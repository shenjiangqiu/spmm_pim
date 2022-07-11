//! receive queue id and fetch partial sum and return to merger

use super::{component::Component, PartialResultTaskType, SpmmContex};

/// collect the partial sum from lower pe,
/// when all lower pe have returned their partial sum, push it to the full_result_merger_dispatcher
pub struct PartialSumCollector {
    queue_id_ready_in: usize,
    queue_id_full_result_out: usize,

    // to record how many partial sum have been collected(hint: the old merger_worker will do this!)
    merger_status_id: usize,
}

impl Component for PartialSumCollector {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            let (time, original_status) = context.into_inner();
            let mut current_time = 0.;
            loop {
                let ready_queue_context: SpmmContex = yield original_status
                    .clone_with_state(super::SpmmStatusEnum::Pop(self.queue_id_ready_in));
                let (time, ready_queue_status) = ready_queue_context.into_inner();
                let gap = time - current_time;
                current_time = time;
                let (_, ready_queue_enum, ..) = ready_queue_status.into_inner();
                let ready_queue_id: usize = ready_queue_enum.into_push_ready_queue_id().unwrap().1;

                let partial_sum_context: SpmmContex = yield original_status
                    .clone_with_state(super::SpmmStatusEnum::Pop(ready_queue_id));
                let (time, partial_sum_status) = partial_sum_context.into_inner();
                let gap = time - current_time;
                current_time = time;
                let (_, partial_sum_enum, _, _, _, _, buffer) = partial_sum_status.into_inner();
                let patial_result: PartialResultTaskType =
                    partial_sum_enum.into_push_partial_task().unwrap().1;
                // need to test if this partial_result is already finished(all sub tasks are finished)
                

                // collect this partial sum, if it's already full, send it to the full_result_merger_dispatcher
                // we also need some structure to record the buffer status.(this should be shared by data collector and signal collector)
            }
        })
    }
}
