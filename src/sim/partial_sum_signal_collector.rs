//! collect the signal and decide which partial sum to be fetch
//!
//!

use super::{component::Component, PartialSignal, SpmmContex};

struct CollectStatus {}

impl CollectStatus {
    fn new() -> CollectStatus {
        CollectStatus {}
    }
    /// update a new partial signal and return ready signal
    fn update(&mut self, _: PartialSignal) -> Vec<PartialSignal> {
        vec![]
    }
}

pub struct PartialSumSignalCollector {
    queue_id_signal_in: usize,
    queue_id_ready_out: usize,
    collect_status: CollectStatus,
}

impl Component for PartialSumSignalCollector {
    fn run(mut self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            let mut current_time = 0.;
            let (time, original_status) = context.into_inner();

            loop {
                // first get the signal
                let signal_context: SpmmContex = yield original_status
                    .clone_with_state(super::SpmmStatusEnum::Pop(self.queue_id_signal_in));
                let (time, signal_status) = signal_context.into_inner();
                let gap = time - current_time;
                current_time = time;

                let (_, signal_enum, ..) = signal_status.into_inner();
                let signal: PartialSignal = signal_enum.into_push_signal().unwrap().1;

                for ready_signal in self.collect_status.update(signal) {
                    let ready_queue = ready_signal.get_queue_id();
                    yield original_status.clone_with_state(
                        super::SpmmStatusEnum::PushReadyQueueId(
                            self.queue_id_ready_out,
                            ready_queue,
                        ),
                    );
                }
            }
        })
    }
}
