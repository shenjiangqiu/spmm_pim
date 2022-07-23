//! this mod contains the partialsumsender
//!
//!
//!

use log::debug;

use super::{
    component::Component, sim_time::NamedTimeId, LevelId, PartialResultTaskType, PartialSignal,
    SpmmContex, SpmmStatus, StateWithSharedStatus,
};
use genawaiter::rc::{Co, Gen};

/// the component that collect the partial sum returned by each worker and ready to send the signle to upper and send the real partial sum to partial sum collector.
#[derive(Debug)]
pub struct PartialSumSender {
    pub level_id: LevelId,
    pub queue_id_partial_sum_in: usize,
    pub queue_id_partial_sum_out: usize,
    pub queue_id_signal_out: usize,

    pub named_sim_time: NamedTimeId,
}

impl PartialSumSender {
    pub fn new(
        queue_id_partial_sum_in: usize,
        queue_id_partial_sum_out: usize,
        queue_id_signal_out: usize,
        level_id: LevelId,
        named_sim_time: NamedTimeId,
    ) -> PartialSumSender {
        PartialSumSender {
            queue_id_partial_sum_in,
            queue_id_partial_sum_out,
            queue_id_signal_out,
            level_id,
            named_sim_time,
        }
    }
}

impl Component for PartialSumSender {
    fn run(self, original_status: SpmmStatus) -> Box<super::SpmmGenerator> {
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

                let (_resouce_id, partial_task): (usize, PartialResultTaskType) =
                    status.into_push_partial_task().unwrap();
                debug!(
                    "PartialSumSender-{:?}: receive partial sum:{},{}",
                    self.level_id, partial_task.0, partial_task.1
                );

                let target_id = partial_task.0;
                let self_sender_id = partial_task.1;
                // then send the signle out
                let context: SpmmContex = co
                    .yield_(
                        original_status.clone_with_state(super::SpmmStatusEnum::PushSignal(
                            self.queue_id_signal_out,
                            PartialSignal {
                                self_sender_id,
                                target_id,
                                self_queue_id: self.queue_id_partial_sum_out,
                            },
                        )),
                    )
                    .await;
                debug!(
                    "PartialSumSender-{:?}: send signal to:{:?}",
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
                debug!(
                    "PartialSumSender-{:?}: ready to provide data at queue id: {} data:{:?}",
                    self.level_id, self.queue_id_partial_sum_out, partial_task
                );
                // then send the real partial sum out
                let context: SpmmContex = co
                    .yield_(original_status.clone_with_state(
                        super::SpmmStatusEnum::PushPartialTask(
                            self.queue_id_partial_sum_out,
                            partial_task,
                        ),
                    ))
                    .await;
                debug!(
                    "PartialSumSender-{:?}: send data to id: {:?}",
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
