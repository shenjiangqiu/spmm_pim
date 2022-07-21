use std::fmt::Debug;

use crate::{csv_nodata::CsVecNodata, settings::RowMapping};
use genawaiter::{rc::gen, yield_};
use itertools::Itertools;
use log::debug;
use qsim::ResourceId;
use sprs::CsMat;

use super::{
    component::Component, id_translation::get_bank_id_from_row_id, BankTask, BankTaskEnum,
    SpmmStatus,
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
        Box::new(gen!({
            let all_send_task = self
                .matrix_a
                .outer_iterator()
                .map(|x| CsVecNodata::from(x.to_owned()))
                .collect_vec();
            let num_rows = self.matrix_b.rows();
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
                    yield_!(
                        original_status.clone_with_state(super::SpmmStatusEnum::PushBankTask(
                            self.task_sender,
                            BankTaskEnum::PushBankTask(BankTask {
                                from: source_idx,
                                to: target_idx,
                                row,
                                bank_id,
                                row_shift: row_start.start,
                                row_size: row_start.end - row_start.start,
                            }),
                        ))
                    );
                }
                // then send a end signal
                yield_!(
                    original_status.clone_with_state(super::SpmmStatusEnum::PushBankTask(
                        self.task_sender,
                        BankTaskEnum::EndThisTask,
                    ))
                );
            }
        }))
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
    ) -> Self {
        Self {
            matrix_a,
            matrix_b,
            task_sender,
            channels,
            chips,
            banks,
            row_mapping,
        }
    }
}
