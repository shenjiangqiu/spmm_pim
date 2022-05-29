use std::cmp::max;

use desim::ResourceId;
use log::debug;
use sprs::CsMat;

use crate::{
    csv_nodata::CsVecNodata,
    pim::{self, PartialSum},
    sim::{BankTask, PE_MAPPING},
};

use super::{
    component::Component, BankID, BankTaskType, PartialResultTaskType, SpmmContex, SpmmStatusEnum,
};
pub struct BankPeInport {
    pub task_in: ResourceId,
    pub row_buffer_receiver: ResourceId,
}

pub struct BankPeOutport {
    pub task_in: ResourceId,
    pub partial_out: ResourceId,
}
/// BankPe is a component that can receive tasks from chip and perform merge
pub struct BankPe {
    // settings
    pub merger_size: usize,
    pub adder_size: usize,
    pub total_rows: usize,
    // resources
    pub task_in: ResourceId,
    pub partial_out: ResourceId,
}

impl BankPe {
    pub fn new(
        ports_in: BankPeInport,
        ports_out: BankPeOutport,
        merger_size: usize,
        adder_size: usize,
        total_rows: usize,
    ) -> Self {
        let BankPeInport {
            task_in,
            row_buffer_receiver,
        } = ports_in;
        let BankPeOutport {
            task_in,
            partial_out,
        } = ports_out;
        Self {
            task_in,
            partial_out,
            merger_size,
            adder_size,
            total_rows,
        }
    }
    /// return: add_cycle,merge_cycle, result
    fn process_task(&self, rows: Vec<CsVecNodata<usize>>) -> (usize, usize, CsVecNodata<usize>) {
        let (add_cycle, merge_cycle, result) = pim::merge_rows_into_one(rows, self.merger_size);

        (add_cycle, merge_cycle, result)
        // get the line of the matrix
    }
}

impl Component for BankPe {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |_: SpmmContex| {
            // first get the task
            let mut current_task = 0;
            let mut tasks = vec![];

            loop {
                let context: SpmmContex = yield SpmmStatusEnum::Pop(self.task_in).into();
                let (time, task) = context.into_inner();
                debug!("time: {},received taske: {:?}", time, task);

                // send read request to row buffer.
                let (_enable_log, state, _) = task.into_inner();
                let (_resouce_id, bank_task) = state.into_push_bank_task().unwrap();
                let BankTask {
                    from,
                    to,
                    inner_bank_id,
                    row,
                } = bank_task;
                if to == current_task {
                    // continue the current task
                    // keep receiving the task
                    tasks.push(row);
                } else {
                    assert!(to == current_task + 1);
                    current_task = to;
                    if !tasks.is_empty() {
                        // process last tasks
                        let (add_cycle, merge_cycle, data) = self.process_task(tasks);
                        // todo: refine the add cycle according to the adder size
                        yield SpmmStatusEnum::Wait(max(add_cycle, merge_cycle) as f64).into();
                        yield SpmmStatusEnum::PushPartialTask(self.partial_out, data).into();
                    }
                    if to == self.total_rows {
                        // there is no more task
                        return;
                    }
                    tasks.clear();
                }
            }
        })
    }
}

/// this struct receive the task from the chip and send the reordered task to the bank pe
pub struct BankTaskReorder {
    pub task_in: ResourceId,
    pub task_out: Vec<ResourceId>,
    pub row_buffer_sender: ResourceId,
    pub partial_ret: ResourceId,
    pub num_rows: usize,

    pub total_reorder_size: usize,
    pub self_id: BankID,
}

impl Component for BankTaskReorder {
    fn run(self) -> Box<super::SpmmGenerator> {
        let num_pes = self.task_out.len();
        Box::new(move |_: SpmmContex| {
            // todo delete this
            yield SpmmStatusEnum::Continue.into();
            let mut current_target_pe = 0;
            loop {
                // first get the context
                let context: SpmmContex = yield SpmmStatusEnum::Pop(self.task_in).into();
                let (time, task) = context.into_inner();
                debug!("time: {},received taske: {:?}", time, task);
                let (_enable_log, state, _) = task.into_inner();
                let (_resouce_id, task) = state.into_push_bank_task().unwrap();
                if task.to == self.num_rows {
                    // there is no more task

                    // process the last task
                    // send this to all pe, to shutdown all the pes
                    for i in 0..num_pes {
                        yield SpmmStatusEnum::PushBankTask(self.task_out[i], task).into();
                    }

                    return;
                }
                // push the task to target PE
                let target_pe_id = (self.self_id, current_target_pe);
                if let Some(current_target) = PE_MAPPING.read().unwrap().get(&target_pe_id) {
                    if current_target == task.to {
                        yield SpmmStatusEnum::PushBankTask(self.task_out[current_target_pe], task)
                            .into();
                    } else {
                        // it's a new task, push to the next pe
                        current_target_pe = (current_target_pe + 1) % num_pes;
                        yield SpmmStatusEnum::PushBankTask(self.task_out[current_target_pe], task)
                            .into();
                    }
                }
                // if the task is the last
            }
        })
    }
}

impl BankTaskReorder {
    pub fn new(
        task_in: ResourceId,
        task_out: Vec<ResourceId>,
        row_buffer_sender: ResourceId,
        partial_ret: ResourceId,
        num_rows: usize,
        total_reorder_size: usize,
        self_id: BankID,
    ) -> Self {
        Self {
            task_in,
            task_out,
            row_buffer_sender,
            partial_ret,
            num_rows,
            total_reorder_size,
            self_id,
        }
    }
}
