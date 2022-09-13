//! this mod contains the partialsumsender
//!
//!
//!

use tracing::debug;

use crate::sim::types::{PartialSignalType, PushPartialSumType, StateWithSharedStatus};

use super::{
    component::Component,
    sim_time::NamedTimeId,
    types::{SpmmContex, SpmmGenerator},
    LevelId, SpmmStatus,
};
use genawaiter::rc::{Co, Gen};

/// the component that collect the partial sum returned by each worker and ready to send the signle to upper and send the real partial sum to partial sum collector.
#[derive(Debug)]
pub struct PartialSumSenderBank {
    pub(crate) level_id: LevelId,
    pub(crate) queue_id_partial_sum_in: usize,
    pub(crate) queue_id_partial_sum_out: usize,
    pub(crate) queue_id_signal_out: usize,

    pub(crate) named_sim_time: NamedTimeId,
}

impl PartialSumSenderBank {
    pub fn new(
        queue_id_partial_sum_in: usize,
        queue_id_partial_sum_out: usize,
        queue_id_signal_out: usize,
        level_id: LevelId,
        named_sim_time: NamedTimeId,
    ) -> PartialSumSenderBank {
        PartialSumSenderBank {
            queue_id_partial_sum_in,
            queue_id_partial_sum_out,
            queue_id_signal_out,
            level_id,
            named_sim_time,
        }
    }
}

impl Component for PartialSumSenderBank {
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
                    "PartialSumSenderBank-{:?}: receive partial sum: target_id: {}, sender_id: {}",
                    self.level_id, target_row, sender_id
                );

                // then send the signle out
                let context: SpmmContex = co
                    .yield_(
                        original_status.clone_with_state(super::SpmmStatusEnum::PushSignal(
                            self.queue_id_signal_out,
                            PartialSignalType {
                                sender_id,
                                target_row,
                                queue_id: self.queue_id_partial_sum_out,
                                task_id,
                            },
                        )),
                    )
                    .await;
                debug!(
                    "PartialSumSenderBank-{:?}: send signal to:{:?}",
                    self.level_id, self.queue_id_signal_out
                );
                let (time, status) = context.into_inner();
                let _gap = time - current_time;
                current_time = time;
                let StateWithSharedStatus {
                    status: _,
                    shared_status,
                } = status.into_inner();
                shared_status.shared_named_time.add_idle_time(
                    &self.named_sim_time,
                    "send_signal",
                    _gap,
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
                    "PartialSumSenderBank-{:?}: send data to id: {:?}",
                    self.level_id, self.queue_id_partial_sum_out
                );

                let (time, _status) = context.into_inner();
                let _gap = time - current_time;
                current_time = time;
                shared_status.shared_named_time.add_idle_time(
                    &self.named_sim_time,
                    "send_partial_sum",
                    _gap,
                );
            }
        };

        Box::new(Gen::new(function))
    }
}
