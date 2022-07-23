//! receive queue id and fetch partial sum and return to merger

use std::collections::BTreeMap;

use log::debug;

use crate::csv_nodata::CsVecNodata;

use super::{
    buffer_status::BufferStatusId, component::Component, sim_time::NamedTimeId, LevelId,
    PartialResultTaskType, SpmmContex, SpmmStatus, StateWithSharedStatus,
};
use genawaiter::rc::{Co, Gen};

/// collect the partial sum from lower pe,
/// when all lower pe have returned their partial sum, push it to the full_result_merger_dispatcher
#[derive(Debug)]
pub struct PartialSumCollector {
    pub level_id: LevelId,
    /// collect ready queue
    pub queue_id_ready_in: usize,
    /// push to
    pub queue_id_full_result_out: usize,
    /// used to signal to signal collector that one entry is freed
    pub queue_id_pop_signal_out: usize,
    // to record how many partial sum have been collected(hint: the old merger_worker will do this!)
    pub buffer_status_id: BufferStatusId,
    pub named_sim_time: NamedTimeId,
}

impl Component for PartialSumCollector {
    fn run(self, original_status: SpmmStatus) -> Box<super::SpmmGenerator> {
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            // need a struct to store current partial sum
            let mut current_partial_sum = BTreeMap::<usize, Vec<CsVecNodata<usize>>>::new();

            let mut current_time = 0.;
            loop {
                debug!(
                    "PartialSumCollector-{:?}:try to receive queue id at id: {}",
                    self.level_id, self.queue_id_ready_in
                );
                let ready_queue_context: SpmmContex = co
                    .yield_(
                        original_status
                            .clone_with_state(super::SpmmStatusEnum::Pop(self.queue_id_ready_in)),
                    )
                    .await;
                let (time, ready_queue_status) = ready_queue_context.into_inner();
                let _gap = time - current_time;
                current_time = time;

                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = ready_queue_status.into_inner();
                shared_status.shared_named_time.add_idle_time(
                    &self.named_sim_time,
                    "get_queue_id",
                    _gap,
                );

                let (ready_queue_id, target_row, is_last) =
                    status.into_push_ready_queue_id().unwrap().1;
                debug!(
                    "PartialSumCollector-{:?}: receive ready queue id: {:?}",
                    self.level_id, ready_queue_id
                );
                debug!(
                    "PartialSumCollector-{:?}: try to receive data: ready queue id: {:?}",
                    self.level_id, ready_queue_id
                );
                let partial_sum_context: SpmmContex = co
                    .yield_(
                        original_status
                            .clone_with_state(super::SpmmStatusEnum::Pop(ready_queue_id)),
                    )
                    .await;

                let (time, partial_sum_status) = partial_sum_context.into_inner();
                let _gap = time - current_time;
                current_time = time;
                let StateWithSharedStatus {
                    status,
                    shared_status: _,
                } = partial_sum_status.into_inner();
                shared_status.shared_named_time.add_idle_time(
                    &self.named_sim_time,
                    "get_data",
                    _gap,
                );
                let (target_row2, _source_pe_id, result): PartialResultTaskType =
                    status.into_push_partial_task().unwrap().1;
                assert_eq!(target_row, target_row2,"the signal queue target id is not equal to the data id the queue_id is:{ready_queue_id}, check is the queue is poped by other first??");
                debug!(
                    "PartialSumCollector-{:?}: receive partial from sum id: {:?}",
                    self.level_id, ready_queue_id
                );
                current_partial_sum
                    .entry(target_row)
                    .or_insert(vec![])
                    .push(result);
                if is_last {
                    let finished_result = current_partial_sum.remove(&target_row).unwrap();
                    debug!(
                            "PartialSumCollector-{:?}:self_queue_id_in id: {}, try to push full partial sum to id: {},:{:?} of target row:{target_row}",
                            self.level_id,self.queue_id_full_result_out,self.queue_id_ready_in, finished_result
                        );
                    // push to partial sum dispatcher
                    let context = co
                        .yield_(original_status.clone_with_state(
                            super::SpmmStatusEnum::PushFullPartialTask(
                                self.queue_id_full_result_out,
                                (target_row, finished_result),
                            ),
                        ))
                        .await;
                    debug!(
                            "PartialSumCollector-{:?}:self_queue_id_in id: {}, finish push full partial sum of target row:{target_row}",
                            self.level_id,self.queue_id_ready_in
                        );
                    let (time, _status) = context.into_inner();
                    let gap = time - current_time;
                    current_time = time;

                    // fix bug here!
                    shared_status
                        .shared_buffer_status
                        .remove(&self.buffer_status_id, target_row);
                    shared_status.shared_named_time.add_idle_time(
                        &self.named_sim_time,
                        "push_full_partial_task",
                        gap,
                    );
                    // push to signal collector
                    let context = co
                        .yield_(original_status.clone_with_state(
                            super::SpmmStatusEnum::PushBufferPopSignal(
                                self.queue_id_pop_signal_out,
                            ),
                        ))
                        .await;
                    let (time, _status) = context.into_inner();
                    let gap = time - current_time;
                    current_time = time;
                    shared_status.shared_named_time.add_idle_time(
                        &self.named_sim_time,
                        "push_buffer_pop_signal",
                        gap,
                    );
                    debug!("PartialSumCollector-{:?}: push signal", self.level_id);
                }
                // need to test if this partial_result is already finished(all sub tasks are finished)

                // collect this partial sum, if it's already full, send it to the full_result_merger_dispatcher
                // we also need some structure to record the buffer status.(this should be shared by data collector and signal collector)
            }
        };
        Box::new(Gen::new(function))
    }
}
