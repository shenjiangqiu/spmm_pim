//! full result merger worker
//! it receives the full partial result from the dispatcher and merger them and send it to merger sender

use super::{component::Component, FullTaskType, SpmmContex, SpmmStatusEnum};

struct FullResultMergerWorker {
    // this is the id of this merger
    pub queue_id_partial_sum_sender: usize,
    pub queue_id_partial_sum_in: usize,

    // this is the id that the upper used to send to us
    pub self_sender_id: usize,

    // release it when finished
    pub merger_resource_id: usize,

    // the merger width
    pub merger_width: usize,
}

impl Component for FullResultMergerWorker {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            let (time, original_status) = context.into_inner();

            loop {
                // first get the full partial Sum:
                let context: SpmmContex = yield original_status
                    .clone_with_state(SpmmStatusEnum::Pop(self.queue_id_partial_sum_in));
                let (_time, status) = context.into_inner();
                let (_, st, ..) = status.into_inner();
                let full_result: FullTaskType = st.into_push_full_partial_task().unwrap().1;
                let (target_row, total_result) = full_result;
                let (add_time, merge_time, partial_sum) =
                    crate::pim::merge_rows_into_one(total_result, self.merger_width);
                // wait time in max(add_time, merge_time)
                let wait_time = std::cmp::max(add_time, merge_time) as f64;

                yield original_status.clone_with_state(SpmmStatusEnum::Wait(wait_time));

                // release the resource

                yield original_status
                    .clone_with_state(SpmmStatusEnum::Release(self.merger_resource_id));
                // push the result to sender

                yield original_status.clone_with_state(SpmmStatusEnum::PushPartialTask(
                    self.self_sender_id,
                    (target_row, self.self_sender_id, partial_sum),
                ));
            }
        })
    }
}
