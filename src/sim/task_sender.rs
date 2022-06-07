use desim::ResourceId;
use itertools::Itertools;
use sprs::CsMat;

use crate::settings::RowMapping;

use super::{
    component::Component, id_translation::get_bank_id_from_row_id, BankTask, BankTaskEnum,
    SpmmContex,
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
impl Component for TaskSender {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            let (_time, status) = context.into_inner();
            let all_send_task = self.matrix_a.iter().map(|x| x.1).collect_vec();
            let num_rows = self.matrix_b.rows();
            for (to, from) in all_send_task {
                let bank_id = get_bank_id_from_row_id(
                    from,
                    self.channels,
                    self.chips,
                    self.banks,
                    num_rows,
                    &self.row_mapping,
                );

                let row = self.matrix_b.outer_view(from).unwrap().to_owned().into();
                yield status.clone_with_state(super::SpmmStatusEnum::PushBankTask(
                    self.task_sender,
                    BankTaskEnum::PushBankTask(BankTask {
                        from,
                        to,
                        row,
                        bank_id,
                    }),
                ));
            }
        })
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
