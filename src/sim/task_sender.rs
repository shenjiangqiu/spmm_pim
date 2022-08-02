use std::fmt::Debug;

use crate::{
    csv_nodata::CsVecNodata,
    pim::get_bank_id_from_row_id,
    settings::RealRowMapping,
    sim::types::{BankTaskEnum, PushBankTaskType, StateWithSharedStatus},
};
use genawaiter::rc::{Co, Gen};
use itertools::Itertools;
use log::{debug, info};

use qsim::ResourceId;
use sprs::CsMat;

use super::{
    component::Component,
    queue_tracker::QueueTrackerId,
    types::{SpmmContex, SpmmGenerator},
    SpmmStatus,
};

pub struct TaskSender<T> {
    pub matrix_a: CsMat<i32>,
    pub matrix_b: CsMat<i32>,
    pub task_sender: ResourceId,

    // config
    channels: usize,
    chips: usize,
    banks: usize,
    row_mapping: RealRowMapping,
    queue_tracker_id_send: QueueTrackerId,

    // contructor
    pub task_generator: T,
}

impl<T> Debug for TaskSender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TaskSender {{ channels: {}, chips: {}, banks: {}, task_sender: {} }}",
            self.channels, self.chips, self.banks, self.task_sender
        )
    }
}

impl<T> Component for TaskSender<T>
where
    T: IntoIterator<Item = (usize, CsVecNodata<usize>)> + 'static,
{
    fn run(self, original_status: SpmmStatus) -> Box<SpmmGenerator> {
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            info!(
                "TaskSender: Total a:rows: {}, total a:cols: {}",
                self.matrix_a.rows(),
                self.matrix_a.cols()
            );
            // compute the distrubution
            // first record all source id

            let num_rows = self.matrix_b.rows();
            let mut all_source_id = vec![0; num_rows];
            for (_, (_row_id, col_id)) in self.matrix_a.iter() {
                all_source_id[col_id] += 1;
            }
            // print the result:
            debug!(target:"spmm_pim::sim::task_sender::histo","TaskSender: total rows: {:?}", num_rows);
            debug!(target:"spmm_pim::sim::task_sender::histo","TaskSender: source id distribution: {:?}", all_source_id);
            let mut channel_dist = vec![0; self.channels];
            let mut chip_dist = vec![0; self.chips * self.channels];
            let mut chip_standalone = vec![0; self.chips];
            let mut bank_dist = vec![0; self.banks * self.chips * self.channels];
            let mut bank_standalone = vec![0; self.banks];
            for (_, (_row_id, col_id)) in self.matrix_a.iter() {
                let (((channel, chip), bank), _) = get_bank_id_from_row_id(
                    col_id,
                    self.channels,
                    self.chips,
                    self.banks,
                    num_rows,
                    &self.row_mapping,
                );
                channel_dist[channel] += 1;
                chip_dist[chip + (channel * self.chips)] += 1;
                chip_standalone[chip] += 1;
                bank_dist[bank + (channel * self.chips * self.banks) + (chip * self.banks)] += 1;
                bank_standalone[bank] += 1;
            }
            // print the result:
            debug!(target:"spmm_pim::sim::task_sender::histo","TaskSender: channel distribution: {:?}", channel_dist);
            debug!(target:"spmm_pim::sim::task_sender::histo","TaskSender: chip distribution: {:?}", chip_dist);
            debug!(target:"spmm_pim::sim::task_sender::histo","TaskSender: chip standalone distribution: {:?}", chip_standalone);
            debug!(target:"spmm_pim::sim::task_sender::histo","TaskSender: bank distribution: {:?}", bank_dist);
            debug!(target:"spmm_pim::sim::task_sender::histo","TaskSender: bank standalone distribution: {:?}", bank_standalone);
            // then compute the level distribution
            let mut task_id = 0;
            // for each row, first send the index to lower pe, then send a end signal
            for (target_idx, vector) in self.task_generator.into_iter() {
                let all_source = vector.iter().cloned().collect_vec();
                // for every col in this row, push a task to lower pe
                for source_idx in all_source {
                    let bank_id = get_bank_id_from_row_id(
                        source_idx,
                        self.channels,
                        self.chips,
                        self.banks,
                        num_rows,
                        &self.row_mapping,
                    );

                    let row = self
                        .matrix_b
                        .outer_view(source_idx)
                        .unwrap()
                        .to_owned()
                        .into();
                    debug!(target:"spmm_pim::sim::task_sender::histo","TASKSENDER:target_idx: {} source_idx: {} target_bank: {:?}", target_idx, source_idx, bank_id);
                    debug!("SENDER: {}:{}:{:?}", target_idx, source_idx, row);
                    let row_start = self.matrix_b.indptr().outer_inds_sz(source_idx);
                    let context = co
                        .yield_(original_status.clone_with_state(
                            super::SpmmStatusEnum::PushBankTask(
                                self.task_sender,
                                BankTaskEnum::PushBankTask(PushBankTaskType {
                                    task_id,
                                    from: source_idx,
                                    to: target_idx,
                                    row,
                                    bank_id: bank_id.0,
                                    row_shift: row_start.start,
                                    row_size: row_start.end - row_start.start,
                                }),
                            ),
                        ))
                        .await;
                    let (_time, status) = context.into_inner();
                    let StateWithSharedStatus {
                        status: _,
                        shared_status,
                    } = status.into_inner();
                    shared_status.queue_tracker.enq(&self.queue_tracker_id_send);
                }
                // then send a end signal
                let context = co
                    .yield_(
                        original_status.clone_with_state(super::SpmmStatusEnum::PushBankTask(
                            self.task_sender,
                            BankTaskEnum::EndThisTask,
                        )),
                    )
                    .await;
                let (_time, status) = context.into_inner();
                let StateWithSharedStatus {
                    status: _,
                    shared_status,
                } = status.into_inner();
                shared_status.queue_tracker.enq(&self.queue_tracker_id_send);
                task_id += 1;
            }
        };
        Box::new(Gen::new(function))
    }
}

impl<T> TaskSender<T> {
    pub fn new(
        matrix_a: CsMat<i32>,
        matrix_b: CsMat<i32>,
        task_sender: ResourceId,
        channels: usize,
        chips: usize,
        banks: usize,
        row_mapping: RealRowMapping,
        queue_tracker_id_send: QueueTrackerId,
        task_generator: T,
    ) -> Self {
        Self {
            matrix_a,
            matrix_b,
            task_sender,
            channels,
            chips,
            banks,
            row_mapping,
            queue_tracker_id_send,
            task_generator,
        }
    }
}
