use std::cmp::max;

use desim::ResourceId;
use log::debug;

use crate::{
    csv_nodata::CsVecNodata,
    pim::{self},
    sim::{BankTask, PE_MAPPING},
};

use super::{component::Component, BankID, SpmmContex, SpmmStatusEnum};

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
        task_in: ResourceId,
        partial_out: ResourceId,
        merger_size: usize,
        adder_size: usize,
        total_rows: usize,
    ) -> Self {
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
                let (_enable_log, state) = task.into_inner();
                let (_resouce_id, bank_task) = state.into_push_bank_task().unwrap();

                let BankTask {
                    from: _,
                    to,
                    inner_bank_id: _,
                    row,
                } = bank_task;
                if to == current_task {
                    // continue the current task
                    // keep receiving the task
                    tasks.push(row);
                } else {
                    // to!=current_task, so we switched to antoher task
                    assert!(to == current_task + 1);
                    current_task = to;
                    if !tasks.is_empty() {
                        // process last tasks
                        let (add_cycle, merge_cycle, data) = self.process_task(tasks.clone());
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
                let (_enable_log, state) = task.into_inner();
                let (_resouce_id, task) = state.into_push_bank_task().unwrap();
                if task.to == self.num_rows {
                    // there is no more task

                    // process the last task
                    // send this to all pe, to shutdown all the pes
                    for i in 0..num_pes {
                        yield SpmmStatusEnum::PushBankTask(self.task_out[i], task.clone()).into();
                    }

                    return;
                }
                // push the task to target PE
                let target_pe_id = (self.self_id, current_target_pe);
                if let Some(current_target) =
                    PE_MAPPING.with(|f| f.borrow().get(&target_pe_id).cloned())
                {
                    if current_target == task.to {
                        yield SpmmStatusEnum::PushBankTask(self.task_out[current_target_pe], task)
                            .into();
                    } else {
                        // it's a new task, push to the next pe
                        current_target_pe = (current_target_pe + 1) % num_pes;
                        yield SpmmStatusEnum::PushBankTask(self.task_out[current_target_pe], task)
                            .into();
                    }
                } else {
                    // no task in that pe! just create one
                    PE_MAPPING.with(|f| f.borrow_mut().insert(target_pe_id, task.to));
                    yield SpmmStatusEnum::PushBankTask(self.task_out[current_target_pe], task)
                        .into();
                }
            }
        })
    }
}

impl BankTaskReorder {
    pub fn new(
        task_in: ResourceId,
        task_out: Vec<ResourceId>,
        num_rows: usize,
        total_reorder_size: usize,
        self_id: BankID,
    ) -> Self {
        Self {
            task_in,
            task_out,
            num_rows,
            total_reorder_size,
            self_id,
        }
    }
}
#[cfg(test)]
mod test {
    use desim::{resources::Store, EndCondition, Simulation};

    use super::*;
    #[test]
    fn test_bank() {
        let mut simulator = Simulation::new();
        let task_in = simulator.create_resource(Box::new(Store::new(1)));
        let task_pe = {
            let mut task_pe = vec![];
            for _i in 0..4 {
                let task_out = simulator.create_resource(Box::new(Store::new(1)));
                task_pe.push(task_out);
            }
            task_pe
        };
        let partial_return = simulator.create_resource(Box::new(Store::new(1)));
        let bank_task_reorder = BankTaskReorder::new(task_in, task_pe.clone(), 5, 4, ((0, 0), 0));
        let bank_pes = {
            let mut pes = vec![];
            for pe_in in task_pe {
                let pe_comp = BankPe::new(pe_in, partial_return, 4, 3, 4);
                pes.push(pe_comp);
            }
            pes
        };
        let bank_task_reorder = simulator.create_process(bank_task_reorder.run());
        simulator.schedule_event(0.0, bank_task_reorder, SpmmStatusEnum::Continue.into());
        for pe in bank_pes {
            let pe_process = simulator.create_process(pe.run());
            simulator.schedule_event(0.0, pe_process, SpmmStatusEnum::Continue.into());
        }

        simulator.run(EndCondition::NoEvents);
    }
}
