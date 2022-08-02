//! collect the signal and decide which partial sum to be fetch
//!
//!

use std::{cmp::Reverse, collections::BinaryHeap};

use crate::sim::types::{ReadyQueueIdType, StateWithSharedStatus};

use super::{
    buffer_status::BufferStatusId,
    component::Component,
    sim_time::NamedTimeId,
    types::{PartialSignalType, SpmmContex, SpmmGenerator},
    LevelId, SpmmStatus, SpmmStatusEnum,
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
    fn run(self, original_status: SpmmStatus) -> Box<SpmmGenerator> {
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            let mut current_time = 0.;
            // currently cannot receive the data, store the signal
            let mut temp_signal_queue: BinaryHeap<Reverse<PartialSignalType>> = Default::default();

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
                    SpmmStatusEnum::PushSignal(
                        _rid,
                        PartialSignalType {
                            task_id,
                            target_row,
                            sender_id,
                            queue_id,
                        },
                    ) => {
                        if shared_status
                            .shared_buffer_status
                            .can_receive(&self.buffer_status_id, task_id)
                        {
                            let is_finished = shared_status.shared_buffer_status.receive(
                                &self.buffer_status_id,
                                task_id,
                                sender_id,
                            );
                            debug!(
                                "PartialSumSignalCollector-{:?}:,target_id: {},finished:{}, receive PushSignal:{:?}",
                                self.level_id,target_row,is_finished, target_row
                            );
                            debug!(
                                "PartialSumSignalCollector-{:?}: try to send PushReadyQueueId:{:?},queue_id:{}",
                                self.level_id,
                                (sender_id,target_row, is_finished),self.queue_id_ready_out
                            );
                            let context = co
                                .yield_(original_status.clone_with_state(
                                    SpmmStatusEnum::PushReadyQueueId(
                                        self.queue_id_ready_out,
                                        ReadyQueueIdType {
                                            task_id,
                                            target_row,
                                            queue_id,
                                            is_finished,
                                        },
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
                                "send_ready_queue_id",
                                gap,
                            );
                        } else {
                            // cannot receive now, store it and resume it later
                            debug!("PartialSumSignalCollector-{:?}: receive PushSignal:{:?} but cannot send now",self.level_id, target_row);
                            debug!(
                                "the reason cannot receive:{:?}",
                                shared_status
                                    .shared_buffer_status
                                    .get_current_status(&self.buffer_status_id)
                            );
                            temp_signal_queue.push(Reverse(PartialSignalType {
                                task_id,
                                target_row,
                                sender_id,
                                queue_id,
                            }));
                        }
                    }
                    SpmmStatusEnum::PushBufferPopSignal(_rid) => {
                        debug!(
                            "PartialSumSignalCollector-{:?}: receive PushBufferPopSignal,current_queue:{:?},start to test",
                            self.level_id,temp_signal_queue
                        );
                        // a buffer entry is popped, resume the signal

                        while let Some(Reverse(PartialSignalType {
                            task_id,
                            target_row,
                            sender_id,
                            queue_id,
                        })) = temp_signal_queue.pop()
                        {
                            if shared_status
                                .shared_buffer_status
                                .can_receive(&self.buffer_status_id, task_id)
                            {
                                let finished = shared_status.shared_buffer_status.receive(
                                    &self.buffer_status_id,
                                    task_id,
                                    sender_id,
                                );
                                debug!(
                                    "PartialSumSignalCollector-{:?}:try to invoke PushSignal:{:?} queue_id:{}",
                                    self.level_id, task_id,self.queue_id_ready_out
                                );
                                let context = co
                                    .yield_(original_status.clone_with_state(
                                        SpmmStatusEnum::PushReadyQueueId(
                                            self.queue_id_ready_out,
                                            ReadyQueueIdType {
                                                task_id,
                                                target_row,
                                                queue_id,
                                                is_finished: finished,
                                            },
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
                                    self.level_id, target_row
                                );
                            } else {
                                // cannot receive now, store it and resume it later
                                debug!(
                                    "the reason cannot receive:{:?}",
                                    shared_status
                                        .shared_buffer_status
                                        .get_current_status(&self.buffer_status_id)
                                );
                                temp_signal_queue.push(Reverse(PartialSignalType {
                                    task_id,
                                    target_row,
                                    sender_id,
                                    queue_id,
                                }));
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
