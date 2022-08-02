//! this mod contains the partialsumsender
//!
//!
//!

use log::debug;

use crate::sim::{
    types::{PushPartialSumType, StateWithSharedStatus},
    SpmmStatusEnum,
};

use super::{
    component::Component,
    merger_status::MergerStatusId,
    sim_time::NamedTimeId,
    types::{SpmmContex, SpmmGenerator},
    LevelId, SpmmStatus,
};
use genawaiter::rc::{Co, Gen};

/// the component that collect the partial sum returned by each worker and ready to send the signle to upper and send the real partial sum to partial sum collector.
#[derive(Debug)]
pub struct PartialSumSenderDimm {
    pub level_id: LevelId,
    pub queue_id_partial_sum_in: usize,
    pub queue_id_partial_sum_out: usize,
    pub queue_id_finished_signal_out: usize,
    pub named_sim_time: NamedTimeId,
    pub merger_status_id: MergerStatusId,
    pub is_binding: bool,
    pub id: usize,
}

impl PartialSumSenderDimm {
    pub fn new(
        queue_id_partial_sum_in: usize,
        queue_id_partial_sum_out: usize,
        queue_id_finished_signal_out: usize,
        level_id: LevelId,
        named_sim_time: NamedTimeId,
        merger_status_id: MergerStatusId,
        is_binding: bool,
        id: usize,
    ) -> PartialSumSenderDimm {
        PartialSumSenderDimm {
            queue_id_partial_sum_in,
            queue_id_partial_sum_out,
            queue_id_finished_signal_out,
            level_id,
            named_sim_time,
            merger_status_id,
            is_binding,
            id,
        }
    }
}

impl Component for PartialSumSenderDimm {
    fn run(self, original_status: SpmmStatus) -> Box<SpmmGenerator> {
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            // this is used for record the current time before each yield
            let mut current_time = 0.;
            loop {
                let context: SpmmContex =
                    co.yield_(original_status.clone_with_state(super::SpmmStatusEnum::Pop(
                        self.queue_id_partial_sum_in,
                    )))
                    .await;
                let (time, status) = context.into_inner();
                let _gap = time - current_time;
                current_time = time;
                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = status.into_inner();
                shared_status.shared_named_time.add_idle_time(
                    &self.named_sim_time,
                    "get_partial_sum",
                    _gap,
                );

                let (_resouce_id, partial_task) = status.into_push_partial_task().unwrap();
                let PushPartialSumType {
                    task_id,
                    target_row,
                    sender_id,
                    target_result,
                } = partial_task;
                debug!(
                    "PartialSumSenderDimm-{:?}-{}: receive partial sum: target_id: {}, sender_id: {}",
                    self.level_id, self.id, target_row, sender_id
                );

                // then send the real partial sum out
                let context: SpmmContex = co
                    .yield_(original_status.clone_with_state(
                        super::SpmmStatusEnum::PushPartialTask(
                            self.queue_id_partial_sum_out,
                            PushPartialSumType {
                                task_id,
                                target_row,
                                sender_id,
                                target_result,
                            },
                        ),
                    ))
                    .await;
                debug!(
                    "PartialSumSenderDimm-{:?}-{}: target_id: {}, send data to id: {:?} and release the merger",
                    self.level_id, self.id, target_row, self.queue_id_partial_sum_out
                );
                shared_status.shared_merger_status.release_merger(
                    self.merger_status_id,
                    self.id,
                    task_id,
                    self.is_binding,
                );
                //
                let (time, _status) = context.into_inner();
                let _gap = time - current_time;
                current_time = time;
                shared_status.shared_named_time.add_idle_time(
                    &self.named_sim_time,
                    "send_partial_sum",
                    _gap,
                );
                // now need to send a signal to the dispatcher that a merger is empty!
                // send signal to the dispatcher that it's free now!
                let context = co
                    .yield_(original_status.clone_with_state(
                        SpmmStatusEnum::PushMergerFinishedSignal(self.queue_id_finished_signal_out),
                    ))
                    .await;
                let (_time, _status) = context.into_inner();
                let gap = _time - current_time;
                current_time = _time;
                shared_status.shared_named_time.add_idle_time(
                    &self.named_sim_time,
                    "send_merger_finished_signal",
                    gap,
                );
            }
        };

        Box::new(Gen::new(function))
    }
}
