//! full result merger worker
//! it receives the full partial result from the dispatcher and merger them and send it to merger sender
use super::{
    buffer_status::BufferStatusId, component::Component, merger_status::MergerStatusId,
    sim_time::NamedTimeId, FullTaskType, LevelId, SpmmContex, SpmmStatus, SpmmStatusEnum,
    StateWithSharedStatus,
};
use genawaiter::rc::{Co, Gen};
use log::debug;
#[derive(Debug)]
pub struct FullResultMergerWorker {
    pub buffer_status_id: BufferStatusId,
    pub level_id: LevelId,
    // this is the id of this merger
    pub queue_id_partial_sum_sender: usize,
    pub queue_id_partial_sum_in: usize,
    pub queue_id_finished_signal_out: usize,
    // this is the id that the upper used to send to us
    pub self_sender_id: usize,

    // release it when finished
    pub merger_status_id: MergerStatusId,
    pub id: usize,

    // the merger width
    pub merger_width: usize,
    pub named_sim_time: NamedTimeId,
    pub is_bind: bool,
}

impl Component for FullResultMergerWorker {
    fn run(self, original_status: SpmmStatus) -> Box<super::SpmmGenerator> {
        // for merger, we need to release the merger when finished. also, we need to release the buffer when the bind mode is applied.

        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            let mut current_time = 0.;
            loop {
                // first get the full partial Sum:
                let context: SpmmContex = co
                    .yield_(
                        original_status
                            .clone_with_state(SpmmStatusEnum::Pop(self.queue_id_partial_sum_in)),
                    )
                    .await;
                let (_time, status) = context.into_inner();
                let gap = _time - current_time;
                current_time = _time;
                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = status.into_inner();

                shared_status.shared_named_time.add_idle_time(
                    &self.named_sim_time,
                    "get_partial_sum_in",
                    gap,
                );

                let full_result: FullTaskType = status.into_push_full_partial_task().unwrap().1;
                let (target_row, total_result) = full_result;

                debug!(
                    "FULL_RESULT_MERGER_WORKER:{:?}-{}, received target_id: {}",
                    self.level_id, self.id, target_row
                );
                let (add_time, merge_time, partial_sum) =
                    crate::pim::merge_rows_into_one(total_result, self.merger_width);
                // wait time in max(add_time, merge_time)
                let wait_time = std::cmp::max(add_time, merge_time) as f64;

                let context = co
                    .yield_(original_status.clone_with_state(SpmmStatusEnum::Wait(wait_time)))
                    .await;
                let (_time, _status) = context.into_inner();
                let gap = _time - current_time;
                shared_status.shared_named_time.add_idle_time(
                    &self.named_sim_time,
                    "merge_time!",
                    gap,
                );
                current_time = _time;
                // release the resource
                if self.is_bind {
                    shared_status
                        .shared_buffer_status
                        .remove(&self.buffer_status_id, target_row);
                }

                debug!(
                    "FULL_RESULT_MERGER_WORKER:{:?}-{},ready send target_id: {}",
                    self.level_id, self.id, target_row
                );
                // send the partial result to the sender
                let context = co
                    .yield_(
                        original_status.clone_with_state(SpmmStatusEnum::PushPartialTask(
                            self.queue_id_partial_sum_sender,
                            (target_row, self.self_sender_id, partial_sum),
                        )),
                    )
                    .await;
                debug!(
                    "FULL_RESULT_MERGER_WORKER:{:?}-{},finished send target_id: {}",
                    self.level_id, self.id, target_row
                );
                let (_time, _status) = context.into_inner();
                let gap = _time - current_time;
                current_time = _time;
                shared_status.shared_named_time.add_idle_time(
                    &self.named_sim_time,
                    "send_partial_sum_out",
                    gap,
                );

                // fix bug here, do not release the merger now, release the merger after the send have sent the result!
                // shared_status.shared_merger_status.release_merger(
                //     self.merger_status_id,
                //     self.id,
                //     target_row,
                //     self.is_bind,
                // );

                debug!(
                    "FULL_RESULT_MERGER_WORKER:{:?}-{},finished.",
                    self.level_id, self.id
                );
            }
        };

        Box::new(Gen::new(function))
    }
}
