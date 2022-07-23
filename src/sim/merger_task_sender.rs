use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::sim::StateWithSharedStatus;
use genawaiter::rc::{Co, Gen};
use itertools::Itertools;
use log::debug;
use qsim::{ResourceId, SimContext};

use super::{
    buffer_status::BufferStatusId, component::Component, merger_status::MergerStatusId,
    sim_time::NamedTimeId, BankID, BankTask, BankTaskEnum, SpmmContex, SpmmStatus, SpmmStatusEnum,
};

pub trait MergerTaskSender {
    // lower id should be the resource id that connect to the lower pe
    fn get_lower_id(&self, bank_id: &BankID) -> usize;
    // all resouce ids
    fn get_lower_pes(&self) -> &[ResourceId];
    fn get_task_in(&self) -> ResourceId;
    fn get_merger_resouce_id(&self) -> ResourceId;
    fn get_merger_status_id(&self) -> &MergerStatusId;

    fn get_time_id(&self) -> &NamedTimeId;
    fn get_buffer_id(&self) -> &BufferStatusId;
}
#[derive(Debug, Clone, Default)]
pub struct MergerWorkerStatus {
    pub waiting_banks: BTreeSet<usize>,
}
impl MergerWorkerStatus {
    pub fn new() -> Self {
        Self {
            waiting_banks: BTreeSet::new(),
        }
    }
    pub fn add_lower_record(&mut self, lower_id: usize) {
        self.waiting_banks.insert(lower_id);
    }
    pub fn del_lower_record(&mut self, lower_id: usize) {
        self.waiting_banks.remove(&lower_id);
    }
}

#[derive(Debug, Default)]
pub struct FullMergerStatus {
    pub id_to_mergerstatus: Vec<MergerStatus>,
}

impl FullMergerStatus {
    pub fn new() -> Self {
        Self {
            id_to_mergerstatus: vec![],
        }
    }
    pub fn create_merger_status(&mut self, total_merger: usize) -> usize {
        self.id_to_mergerstatus
            .push(MergerStatus::new(total_merger));
        self.id_to_mergerstatus.len() - 1
    }
    pub fn get_merger_status(&self, id: usize) -> &MergerStatus {
        &self.id_to_mergerstatus[id]
    }
}

/// merger status
/// - total_merger: number of total merger workers
/// - current_working_merger: target id to merger id
/// - current_merger_worker_status: merger status
#[derive(Debug)]
pub struct MergerStatus {
    pub total_merger: usize,
    // target id to merger worker id
    pub current_working_merger: BTreeMap<usize, usize>,
    // merger id to merger status id
    pub current_merger_worker_status: Vec<MergerWorkerStatus>,

    pub idle_merger: VecDeque<usize>,
}

impl MergerStatus {
    /// create a new merger status
    /// - this one should not contains any current_working_merger
    /// - current_merger_worker_status should be n-mergers with default status
    /// - idle_merger should contains 0..total_merger
    pub fn new(total_merger: usize) -> Self {
        // create a dequeue contains 0..total_merger
        let mut idle_merger = VecDeque::new();
        for i in 0..total_merger {
            idle_merger.push_back(i);
        }

        Self {
            total_merger,
            current_merger_worker_status: vec![MergerWorkerStatus::new(); total_merger],
            current_working_merger: BTreeMap::new(),
            idle_merger,
        }
    }

    /// a new target id arrives, find a idle merger if it's the first time, else increase the waiting task
    pub fn push(&mut self, target_row: usize, lower_pe_id: usize) {
        // insert or plus one
        self.current_working_merger
            .entry(target_row)
            .and_modify(|e| self.current_merger_worker_status[*e].add_lower_record(lower_pe_id))
            .or_insert_with(|| {
                let id = self.idle_merger.pop_front().unwrap();
                self.current_merger_worker_status[id].add_lower_record(lower_pe_id);
                id
            });
    }

    pub fn pop(&mut self, target_row: usize, lower_pe_id: usize) {
        // minus one, if it's zero, remove it
        let pe_id = self.current_working_merger[&target_row];
        self.current_merger_worker_status[pe_id].del_lower_record(lower_pe_id);

        if self.current_merger_worker_status[pe_id]
            .waiting_banks
            .is_empty()
        {
            // remove it
            self.current_working_merger.remove(&target_row);
            self.idle_merger.push_back(pe_id);
        }
    }

    pub fn exist(&self, target_row: usize) -> bool {
        self.current_working_merger.contains_key(&target_row)
    }
}

impl<T> Component for T
where
    T: MergerTaskSender + 'static,
{
    /// the merger task sender
    fn run(self, original_status: SpmmStatus) -> Box<super::SpmmGenerator> {
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            let mut current_time = 0.;
            // first get the task
            loop {
                // step 1: get the finished
                let context: SimContext<SpmmStatus> = co
                    .yield_(
                        original_status.clone_with_state(SpmmStatusEnum::Pop(self.get_task_in())),
                    )
                    .await;
                debug!("MERGER_TSK_SD:id:{},{:?}", self.get_task_in(), context);
                let (_time, task) = context.into_inner();
                let gap = _time - current_time;
                current_time = _time;
                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = task.into_inner();
                shared_status
                    .shared_named_time
                    .add_idle_time(self.get_time_id(), "get_task", gap);
                let task = status.into_push_bank_task().unwrap().1;

                match task {
                    super::BankTaskEnum::PushBankTask(BankTask {
                        from,
                        to,
                        row,
                        bank_id,
                        row_shift,
                        row_size,
                    }) => {
                        // then push to target pe
                        let lower_pe_id = self.get_lower_id(&bank_id);

                        // record that the task is on going to lower_pe_id, record it!
                        shared_status.shared_buffer_status.add_waiting(
                            self.get_buffer_id(),
                            to,
                            lower_pe_id,
                        );

                        let context = co
                            .yield_(
                                original_status.clone_with_state(SpmmStatusEnum::PushBankTask(
                                    lower_pe_id,
                                    BankTaskEnum::PushBankTask(BankTask {
                                        from,
                                        to,
                                        row,
                                        bank_id,
                                        row_shift,
                                        row_size,
                                    }),
                                )),
                            )
                            .await;
                        let (_time, _status) = context.into_inner();
                        let gap = _time - current_time;
                        current_time = _time;

                        shared_status.shared_named_time.add_idle_time(
                            self.get_time_id(),
                            "push_bank_task",
                            gap,
                        );
                    }
                    super::BankTaskEnum::EndThisTask => {
                        // push this to every lower pe
                        for lower_pe_id in self.get_lower_pes().iter().cloned().collect_vec() {
                            let context = co
                                .yield_(original_status.clone_with_state(
                                    SpmmStatusEnum::PushBankTask(
                                        lower_pe_id,
                                        super::BankTaskEnum::EndThisTask,
                                    ),
                                ))
                                .await;
                            let (_time, _status) = context.into_inner();
                            let gap = _time - current_time;
                            current_time = _time;
                            shared_status.shared_named_time.add_idle_time(
                                self.get_time_id(),
                                "push_end_bank_task",
                                gap,
                            );
                        }
                    }
                }
            }
        };
        Box::new(Gen::new(function))
    }
}
