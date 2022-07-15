//! receive queue id and fetch partial sum and return to merger

use std::collections::BTreeMap;

use log::debug;

use crate::csv_nodata::CsVecNodata;

use super::{
    component::Component, LevelId, PartialResultTaskType, SpmmContex, StateWithSharedStatus,
};

/// collect the partial sum from lower pe,
/// when all lower pe have returned their partial sum, push it to the full_result_merger_dispatcher
pub struct PartialSumCollector {
    pub level_id: LevelId,
    /// collect ready queue
    pub queue_id_ready_in: usize,
    /// push to
    pub queue_id_full_result_out: usize,
    /// used to signal to signal collector that one entry is freed
    pub queue_id_pop_signal_out: usize,
    // to record how many partial sum have been collected(hint: the old merger_worker will do this!)
}

impl Component for PartialSumCollector {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            // need a struct to store current partial sum
            let mut current_partial_sum = BTreeMap::<usize, Vec<CsVecNodata<usize>>>::new();

            let (_time, original_status) = context.into_inner();
            let mut current_time = 0.;
            loop {
                let ready_queue_context: SpmmContex = yield original_status
                    .clone_with_state(super::SpmmStatusEnum::Pop(self.queue_id_ready_in));
                let (time, ready_queue_status) = ready_queue_context.into_inner();
                let _gap = time - current_time;
                current_time = time;
                let StateWithSharedStatus {
                    status,
                    shared_status: _,
                } = ready_queue_status.into_inner();
                let (ready_queue_id, is_last) = status.into_push_ready_queue_id().unwrap().1;
                debug!(
                    "PartialSumCollector-{:?}: receive ready queue id:{:?}",
                    self.level_id, ready_queue_id
                );
                let partial_sum_context: SpmmContex = yield original_status
                    .clone_with_state(super::SpmmStatusEnum::Pop(ready_queue_id));

                let (time, partial_sum_status) = partial_sum_context.into_inner();
                let _gap = time - current_time;
                current_time = time;
                let StateWithSharedStatus {
                    status,
                    shared_status: _,
                } = partial_sum_status.into_inner();

                let (target_row, _source_pe_id, result): PartialResultTaskType =
                    status.into_push_partial_task().unwrap().1;
                debug!(
                    "PartialSumCollector-{:?}: receive partial sum:{:?}",
                    self.level_id, result
                );
                current_partial_sum
                    .entry(target_row)
                    .or_insert(vec![])
                    .push(result);
                if is_last {
                    let finished_result = current_partial_sum.remove(&target_row).unwrap();
                    debug!(
                        "PartialSumCollector-{:?}: push full partial sum:{:?}",
                        self.level_id, finished_result
                    );
                    // push to partial sum dispatcher
                    yield original_status.clone_with_state(
                        super::SpmmStatusEnum::PushFullPartialTask(
                            self.queue_id_full_result_out,
                            (target_row, finished_result),
                        ),
                    );

                    // push to signal collector
                    yield original_status.clone_with_state(
                        super::SpmmStatusEnum::PushBufferPopSignal(self.queue_id_pop_signal_out),
                    );
                    debug!("PartialSumCollector-{:?}: push signal", self.level_id);
                }
                // need to test if this partial_result is already finished(all sub tasks are finished)

                // collect this partial sum, if it's already full, send it to the full_result_merger_dispatcher
                // we also need some structure to record the buffer status.(this should be shared by data collector and signal collector)
            }
        })
    }
}
