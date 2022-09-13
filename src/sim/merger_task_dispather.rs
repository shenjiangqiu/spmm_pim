use std::{cmp::Reverse, collections::BinaryHeap};

use crate::{csv_nodata::CsVecNodata, sim::types::PushFullSumType};

use super::{
    component::Component,
    id_translation::LevelId,
    merger_status::MergerStatusId,
    types::{SpmmContex, SpmmGenerator, SpmmStatus, StateWithSharedStatus},
    SpmmStatusEnum,
};
use genawaiter::rc::{Co, Gen};
use qsim::ResourceId;
use tracing::debug;
#[derive(Debug)]
pub struct MergerWorkerDispatcher {
    pub level_id: LevelId,
    // receive from lower pe
    pub full_sum_in: ResourceId,
    // send to merger worker
    pub merger_task_sender: Vec<ResourceId>,

    // the merger status id
    pub merger_status_id: MergerStatusId,
    pub is_binding: bool,
}
#[derive(PartialEq, Eq)]
struct TempFullResult {
    pub task_id: usize,
    pub target_row: usize,
    pub target_result: Vec<CsVecNodata<usize>>,
}

impl Ord for TempFullResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.task_id.cmp(&other.task_id)
    }
}
impl PartialOrd for TempFullResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Component for MergerWorkerDispatcher {
    fn run(self, original_status: SpmmStatus) -> Box<SpmmGenerator> {
        let process = |co: Co<SpmmStatus, SpmmContex>| async move {
            // first get the task
            let mut waiting_tasks = BinaryHeap::new();
            loop {
                let task = co
                    .yield_(original_status.clone_with_state(SpmmStatusEnum::Pop(self.full_sum_in)))
                    .await;
                let (_, ret_status) = task.into_inner();
                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = ret_status.into_inner();
                match status {
                    SpmmStatusEnum::PushFullPartialTask(_, _) => {
                        let PushFullSumType {
                            task_id,
                            target_row,
                            target_result,
                        } = status.into_push_full_partial_task().unwrap().1;
                        debug!(
                            "MergerWorkerDispatcher-{:?}:target_id: {}, from queue: {}",
                            self.level_id, target_row, self.full_sum_in
                        );
                        if let Some(target_pe) = shared_status.shared_merger_status.get_next_merger(
                            self.merger_status_id,
                            task_id,
                            self.is_binding,
                        ) {
                            // find a empty merger!
                            // push the partial result back
                            debug!(
                            "MergerWorkerDispatcher-{:?}: target_id: {target_row} try to send to {},full sum:{:?} to merger worker:{:?},real_id:{:?}",
                            self.level_id,self.merger_task_sender[target_pe], target_result, target_pe,self.merger_task_sender[target_pe]
                            );
                            co.yield_(original_status.clone_with_state(
                                SpmmStatusEnum::PushFullPartialTask(
                                    self.merger_task_sender[target_pe],
                                    PushFullSumType {
                                        task_id,
                                        target_row,
                                        target_result,
                                    },
                                ),
                            ))
                            .await;
                            debug!(
                            "MergerWorkerDispatcher-{:?}: target_id: {target_row} succ to send to {}, to merger worker:{:?},real_id:{:?}",
                            self.level_id,self.merger_task_sender[target_pe], target_pe,self.merger_task_sender[target_pe]
                            );
                        } else {
                            debug!(
                                "MergerWorkerDispatcher-{:?}: target_id: {} failed to find a empty merger",
                                self.level_id,target_row,
                            );
                            waiting_tasks.push(Reverse(TempFullResult {
                                task_id,
                                target_row,
                                target_result,
                            }));
                        }
                    }
                    SpmmStatusEnum::PushMergerFinishedSignal(_) => {
                        debug!(
                            "MergerWorkerDispatcher-{:?}: some_merger_finished",
                            self.level_id,
                        );
                        // some entry is freed, try to push to merger again:
                        for Reverse(TempFullResult {
                            task_id,
                            target_row,
                            target_result,
                        }) in waiting_tasks.pop()
                        {
                            debug!(
                                "MergerWorkerDispatcher-{:?}: start to test target_id: {target_row}",
                                self.level_id,
                            );
                            if let Some(target_pe) = shared_status
                                .shared_merger_status
                                .get_next_merger(self.merger_status_id, task_id, self.is_binding)
                            {
                                // push to that merger
                                // find a empty merger!
                                // push the partial result back
                                debug!("MergerWorkerDispatcher-{:?}: try to send to {},full sum:{:?} to merger worker:{:?},real_id:{:?}",self.level_id,self.merger_task_sender[target_pe], target_result, target_pe,self.merger_task_sender[target_pe]);

                                co.yield_(original_status.clone_with_state(
                                    SpmmStatusEnum::PushFullPartialTask(
                                        self.merger_task_sender[target_pe],
                                        PushFullSumType {
                                            task_id,
                                            target_row,
                                            target_result,
                                        },
                                    ),
                                ))
                                .await;

                                debug!("MergerWorkerDispatcher-{:?}: succ to send to {}, to merger worker:{:?},real_id:{:?}",self.level_id,self.merger_task_sender[target_pe], target_pe,self.merger_task_sender[target_pe]);
                            } else {
                                debug!(
                                    "MergerWorkerDispatcher-{:?}: target_id: {} failed to find a empty merger",
                                    self.level_id,target_row,
                                );
                                waiting_tasks.push(Reverse(TempFullResult {
                                    task_id,
                                    target_row,
                                    target_result,
                                }));
                                break;
                            }
                        }
                    }
                    _ => {
                        unreachable!("cannot be here");
                    }
                }
            }
        };

        Box::new(Gen::new(process))
    }
}
