//! collect the signal and decide which partial sum to be fetch
//!
//!

use std::collections::VecDeque;

use super::{
    buffer_status::BufferStatusId, component::Component, sim_time::NamedTimeId, LevelId,
    SpmmContex, SpmmStatus, SpmmStatusEnum, StateWithSharedStatus,
};
use genawaiter::rc::{Co, Gen};
use log::debug;
/// # the signal collector and decide which row to be fetched
#[derive(Debug)]
pub struct PartialSumSignalCollector {
    pub level_id: LevelId,
    pub queue_id_signal_in: usize,
    pub queue_id_ready_out: usize,

    pub buffer_status_id: BufferStatusId,

    pub named_sim_time: NamedTimeId,
}

impl Component for PartialSumSignalCollector {
    fn run(self, original_status: SpmmStatus) -> Box<super::SpmmGenerator> {
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            let mut current_time = 0.;
            // currently cannot receive the data, store the signal
            let mut temp_signal_queue: VecDeque<_> = Default::default();

            loop {
                // first get the signal
                debug!(
                    "PartialSumSignalCollector-{:?}:,try to get signal",
                    self.level_id
                );
                let signal_context: SpmmContex = co
                    .yield_(
                        original_status
                            .clone_with_state(super::SpmmStatusEnum::Pop(self.queue_id_signal_in)),
                    )
                    .await;

                let (time, signal_status) = signal_context.into_inner();
                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = signal_status.into_inner();
                let _gap = time - current_time;
                current_time = time;
                shared_status.shared_named_time.add_idle_time(
                    &self.named_sim_time,
                    "get_signal",
                    _gap,
                );

                match status {
                    SpmmStatusEnum::PushSignal(_rid, signal) => {
                        if shared_status
                            .shared_buffer_status
                            .can_receive(&self.buffer_status_id, signal.target_id)
                        {
                            let finished = shared_status.shared_buffer_status.receive(
                                &self.buffer_status_id,
                                signal.target_id,
                                signal.self_sender_id,
                            );
                            debug!(
                                "PartialSumSignalCollector-{:?}:,target_id:{},finished:{}, receive PushSignal:{:?}",
                                self.level_id,signal.target_id,finished, signal
                            );
                            debug!(
                                "PartialSumSignalCollector-{:?}: try to send PushReadyQueueId:{:?},queue_id:{}",
                                self.level_id,
                                (signal.self_queue_id,signal.target_id, finished),self.queue_id_ready_out
                            );
                            let context = co
                                .yield_(original_status.clone_with_state(
                                    SpmmStatusEnum::PushReadyQueueId(
                                        self.queue_id_ready_out,
                                        (signal.self_queue_id, signal.target_id, finished),
                                    ),
                                ))
                                .await;
                            debug!(
                                    "PartialSumSignalCollector-{:?}: finished send PushReadyQueueId:{:?},queue_id:{}",
                                    self.level_id,
                                    (signal.self_queue_id,signal.target_id, finished),self.queue_id_ready_out
                                );
                            let (time, status) = context.into_inner();
                            let gap = time - current_time;
                            current_time = time;
                            let StateWithSharedStatus {
                                status: _,
                                shared_status,
                            } = status.into_inner();
                            shared_status.shared_named_time.add_idle_time(
                                &self.named_sim_time,
                                "send_ready_queue_id",
                                gap,
                            );
                        } else {
                            // cannot receive now, store it and resume it later
                            debug!("PartialSumSignalCollector-{:?}: receive PushSignal:{:?} but cannot send now",self.level_id, signal);
                            debug!(
                                "the reason cannot receive:{:?}",
                                shared_status
                                    .shared_buffer_status
                                    .get_current_status(&self.buffer_status_id)
                            );
                            temp_signal_queue.push_back(signal);
                        }
                    }
                    SpmmStatusEnum::PushBufferPopSignal(_rid) => {
                        debug!(
                            "PartialSumSignalCollector-{:?}: receive PushBufferPopSignal,current_queue:{:?},start to test",
                            self.level_id,temp_signal_queue
                        );
                        // a buffer entry is popped, resume the signal

                        while let Some(signal) = temp_signal_queue.pop_front() {
                            if shared_status
                                .shared_buffer_status
                                .can_receive(&self.buffer_status_id, signal.target_id)
                            {
                                let finished = shared_status.shared_buffer_status.receive(
                                    &self.buffer_status_id,
                                    signal.target_id,
                                    signal.self_sender_id,
                                );
                                debug!(
                                    "PartialSumSignalCollector-{:?}:try to invoke PushSignal:{:?} queue_id:{}",
                                    self.level_id, signal,self.queue_id_ready_out
                                );
                                let context = co
                                    .yield_(original_status.clone_with_state(
                                        SpmmStatusEnum::PushReadyQueueId(
                                            self.queue_id_ready_out,
                                            (signal.self_queue_id, signal.target_id, finished),
                                        ),
                                    ))
                                    .await;
                                let (time, status) = context.into_inner();
                                let gap = time - current_time;
                                current_time = time;
                                let StateWithSharedStatus {
                                    status: _,
                                    shared_status,
                                } = status.into_inner();
                                shared_status.shared_named_time.add_idle_time(
                                    &self.named_sim_time,
                                    "send_ready_queue_id_for_trigger",
                                    gap,
                                );

                                debug!(
                                    "PartialSumSignalCollector-{:?}: send PushReadyQueueId:{:?}",
                                    self.level_id,
                                    (signal.self_queue_id, finished)
                                );
                            } else {
                                // cannot receive now, store it and resume it later
                                debug!("PartialSumSignalCollector-{:?}: invoke PushSignal:{:?} but cannot send now",self.level_id, signal);
                                temp_signal_queue.push_front(signal);
                                break;
                            }
                        }
                    }
                    _ => {
                        unreachable!("...should never reach here");
                    }
                };
            }
        };

        Box::new(Gen::new(function))
    }
}
