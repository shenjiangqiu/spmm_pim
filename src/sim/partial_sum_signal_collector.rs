//! collect the signal and decide which partial sum to be fetch
//!
//!

use std::collections::VecDeque;

use log::debug;

use super::{
    buffer_status::BufferStatusId, component::Component, LevelId, SpmmContex, SpmmStatusEnum,
    StateWithSharedStatus,
};
/// # the signal collector and decide which row to be fetched
#[derive(Debug)]
pub struct PartialSumSignalCollector {
    pub level_id: LevelId,
    pub queue_id_signal_in: usize,
    pub queue_id_ready_out: usize,

    pub buffer_status_id: BufferStatusId,
}

impl Component for PartialSumSignalCollector {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            let mut current_time = 0.;
            // currently cannot receive the data, store the signal
            let mut temp_signal_queue: VecDeque<_> = Default::default();

            let (_time, original_status) = context.into_inner();

            loop {
                // first get the signal
                let signal_context: SpmmContex = yield original_status
                    .clone_with_state(super::SpmmStatusEnum::Pop(self.queue_id_signal_in));
                let (time, signal_status) = signal_context.into_inner();
                let _gap = time - current_time;
                current_time = time;

                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = signal_status.into_inner();
                match status {
                    SpmmStatusEnum::PushSignal(_rid, signal) => {
                        if unsafe {
                            shared_status
                                .shared_buffer_status
                                .can_receive(&self.buffer_status_id, signal.target_id)
                        } {
                            let finished = unsafe {
                                shared_status.shared_buffer_status.receive(
                                    &self.buffer_status_id,
                                    signal.target_id,
                                    signal.self_sender_id,
                                )
                            };
                            debug!(
                                "PartialSumSignalCollector-{:?}:,target_id:{},finished:{}, receive PushSignal:{:?}",
                                self.level_id,signal.target_id,finished, signal
                            );
                            debug!(
                                "PartialSumSignalCollector-{:?}: send PushReadyQueueId:{:?},queue_id:{}",
                                self.level_id,
                                (signal.self_queue_id,signal.target_id, finished),self.queue_id_ready_out
                            );
                            yield original_status.clone_with_state(
                                SpmmStatusEnum::PushReadyQueueId(
                                    self.queue_id_ready_out,
                                    (signal.self_queue_id, signal.target_id, finished),
                                ),
                            );
                        } else {
                            // cannot receive now, store it and resume it later
                            debug!("PartialSumSignalCollector-{:?}: receive PushSignal:{:?} but cannot send now",self.level_id, signal);
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
                            if unsafe {
                                shared_status
                                    .shared_buffer_status
                                    .can_receive(&self.buffer_status_id, signal.target_id)
                            } {
                                let finished = unsafe {
                                    shared_status.shared_buffer_status.receive(
                                        &self.buffer_status_id,
                                        signal.target_id,
                                        signal.self_sender_id,
                                    )
                                };
                                debug!(
                                    "PartialSumSignalCollector-{:?}: invoke PushSignal:{:?}",
                                    self.level_id, signal
                                );
                                yield original_status.clone_with_state(
                                    SpmmStatusEnum::PushReadyQueueId(
                                        self.queue_id_ready_out,
                                        (signal.self_queue_id, signal.target_id, finished),
                                    ),
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
                        panic!("PartialSumSignalCollector-{:?}: error!", self.level_id)
                    }
                };
            }
        })
    }
}
