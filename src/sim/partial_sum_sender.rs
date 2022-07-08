//! this mod contains the partialsumsender
//!
//!
//!

use super::{component::Component, PartialSignal, SpmmContex};
/// the component that collect the partial sum returned by each worker and ready to send the signle to upper and send the real partial sum to partial sum collector.
pub struct PartialSumSender {
    queue_id_partial_sum_in: usize,
    queue_id_partial_sum_out: usize,
    queue_id_signal_out: usize,
}

impl PartialSumSender {
    pub fn new(
        queue_id_partial_sum_in: usize,
        queue_id_partial_sum_out: usize,
        queue_id_signal_out: usize,
    ) -> PartialSumSender {
        PartialSumSender {
            queue_id_partial_sum_in,
            queue_id_partial_sum_out,
            queue_id_signal_out,
        }
    }
}

impl Component for PartialSumSender {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            let (_time, original_status) = context.into_inner();
            // this is used for record the current time before each yield
            let mut current_time = 0.;
            loop {
                let context: SpmmContex = yield original_status
                    .clone_with_state(super::SpmmStatusEnum::Pop(self.queue_id_partial_sum_in));
                let (time, status) = context.into_inner();
                let gap = time - current_time;
                current_time = time;
                let (_, st, ..) = status.into_inner();
                let (_resouce_id, partial_task) = st.into_push_partial_task().unwrap();

                // then send the signle out
                let context: SpmmContex = yield original_status.clone_with_state(
                    super::SpmmStatusEnum::PushSignal(self.queue_id_signal_out, PartialSignal {}),
                );
                let (time, status) = context.into_inner();
                let gap = time - current_time;
                current_time = time;

                // then send the real partial sum out
                let context: SpmmContex =
                    yield status.clone_with_state(super::SpmmStatusEnum::PushPartialTask(
                        self.queue_id_partial_sum_out,
                        partial_task,
                    ));
                let (time, status) = context.into_inner();
                let gap = time - current_time;
                current_time = time;
            }
        })
    }
}
