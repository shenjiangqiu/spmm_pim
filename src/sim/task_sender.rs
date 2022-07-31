use std::fmt::Debug;

use crate::{csv_nodata::CsVecNodata, settings::RowMapping, sim::StateWithSharedStatus};
use genawaiter::rc::{Co, Gen};
use itertools::Itertools;
use log::{debug, info};
use plotters::{
    prelude::{BitMapBackend, ChartBuilder, Histogram, IntoDrawingArea, IntoSegmentedCoord},
    style::{Color, RED},
};
use qsim::ResourceId;
use sprs::CsMat;

use super::{
    component::Component, id_translation::get_bank_id_from_row_id, queue_tracker::QueueTrackerId,
    BankTask, BankTaskEnum, SpmmContex, SpmmStatus,
};

pub struct TaskSender {
    pub matrix_a: CsMat<i32>,
    pub matrix_b: CsMat<i32>,
    pub task_sender: ResourceId,

    // config
    channels: usize,
    chips: usize,
    banks: usize,
    row_mapping: RowMapping,
    queue_tracker_id_send: QueueTrackerId,
}

impl Debug for TaskSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TaskSender {{ channels: {}, chips: {}, banks: {}, task_sender: {} }}",
            self.channels, self.chips, self.banks, self.task_sender
        )
    }
}

impl Component for TaskSender {
    fn run(self, original_status: SpmmStatus) -> Box<super::SpmmGenerator> {
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            let all_send_task = self
                .matrix_a
                .outer_iterator()
                .map(|x| CsVecNodata::from(x.to_owned()))
                .collect_vec();
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
            let mut bank_dist = vec![0; self.banks * self.chips * self.channels];
            for (_, (_row_id, col_id)) in self.matrix_a.iter() {
                let ((channel, chip), bank) = get_bank_id_from_row_id(
                    col_id,
                    self.channels,
                    self.chips,
                    self.banks,
                    num_rows,
                    &self.row_mapping,
                );
                channel_dist[channel] += 1;
                chip_dist[chip + (channel * self.chips)] += 1;
                bank_dist[bank + (channel * self.chips * self.banks) + (chip * self.banks)] += 1;
            }
            // print the result:
            debug!(target:"spmm_pim::sim::task_sender::histo","TaskSender: channel distribution: {:?}", channel_dist);
            debug!(target:"spmm_pim::sim::task_sender::histo","TaskSender: chip distribution: {:?}", chip_dist);
            debug!(target:"spmm_pim::sim::task_sender::histo","TaskSender: bank distribution: {:?}", bank_dist);
            // then compute the level distribution

            // for each row, first send the index to lower pe, then send a end signal
            for (target_idx, vector) in all_send_task.into_iter().enumerate() {
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
                    debug!("SENDER: {}:{}:{:?}", target_idx, source_idx, row);
                    let row_start = self.matrix_b.indptr().outer_inds_sz(source_idx);
                    let context = co
                        .yield_(original_status.clone_with_state(
                            super::SpmmStatusEnum::PushBankTask(
                                self.task_sender,
                                BankTaskEnum::PushBankTask(BankTask {
                                    from: source_idx,
                                    to: target_idx,
                                    row,
                                    bank_id,
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
            }
        };
        Box::new(Gen::new(function))
    }
}

impl TaskSender {
    pub fn new(
        matrix_a: CsMat<i32>,
        matrix_b: CsMat<i32>,
        task_sender: ResourceId,
        channels: usize,
        chips: usize,
        banks: usize,
        row_mapping: RowMapping,
        queue_tracker_id_send: QueueTrackerId,
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
        }
    }
}
