//! receive queue id and fetch partial sum and return to merger

use super::{component::Component, PartialResultTaskType, SpmmContex};

//
pub struct PartialSumCollector {
    queue_id_ready_in: usize,
    queue_id_partial_out: usize,
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
                let (_, partial_sum_enum, ..) = partial_sum_status.into_inner();
                let patial_result: PartialResultTaskType =
                    partial_sum_enum.into_push_partial_task().unwrap().1;

                let push_partial_sum_context =
                    yield original_status.clone_with_state(super::SpmmStatusEnum::PushPartialTask(
                        self.queue_id_partial_out,
                        patial_result,
                    ));
                let (time, _) = push_partial_sum_context.into_inner();
                let gap = time - current_time;
                current_time = time;
            }
        })
    }
}
