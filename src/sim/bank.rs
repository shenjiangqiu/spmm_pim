use std::cmp;

use desim::ResourceId;
use log::debug;

use super::{component::Component, BankID, BankTask, SpmmContex, SpmmStatusEnum};
use crate::{pim::merge_rows_into_one, sim::BankTaskEnum};

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
}

impl Component for BankPe {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            // first get the task
            let (_time, status) = context.into_inner();
            let mut current_task = 0;
            let mut tasks = vec![];
            loop {
                let context: SpmmContex =
                    yield status.clone_with_state(SpmmStatusEnum::Pop(self.task_in));
                let (time, pop_status) = context.into_inner();
                debug!("time: {},received taske: {:?}", time, pop_status);

                // send read request to row buffer.
                let (_enable_log, state, _merger_status, _bank_status) = pop_status.into_inner();
                let (_resouce_id, bank_task) = state.into_push_bank_task().unwrap();

                match bank_task {
                    BankTaskEnum::PushBankTask(BankTask {
                        from: _,
                        to,
                        row,
                        bank_id: _,
                    }) => {
                        tasks.push(row);
                        current_task = to;
                    }
                    BankTaskEnum::EndThisTask => {
                        // end this task
                        // compute the task
                        if !tasks.is_empty() {
                            // process last tasks
                            let (add_cycle, merge_cycle, data) =
                                merge_rows_into_one(tasks.clone(), self.merger_size);
                            // todo: refine the add cycle according to the adder size
                            yield status.clone_with_state(SpmmStatusEnum::Wait(cmp::max(
                                add_cycle,
                                merge_cycle,
                            )
                                as f64));
                            yield status.clone_with_state(SpmmStatusEnum::PushPartialTask(
                                self.partial_out,
                                (current_task, self.task_in, data),
                            ));
                        }

                        tasks.clear();
                    }
                };
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
        Box::new(move |context: SpmmContex| {
            // todo delete this
            let mut current_target_pe = 0;
            let (_time, status) = context.into_inner();
            loop {
                // first get the context
                let context: SpmmContex =
                    yield status.clone_with_state(SpmmStatusEnum::Pop(self.task_in));
                let (time, pop_status) = context.into_inner();
                debug!("time: {},received taske: {:?}", time, pop_status);
                let (_enable_log, state, _merger_status, _bank_status) = pop_status.into_inner();
                let (_resouce_id, task) = state.into_push_bank_task().unwrap();

                match task {
                    BankTaskEnum::PushBankTask(bank_task) => {
                        // keep push this task to the current_task_pe
                        yield status.clone_with_state(SpmmStatusEnum::PushBankTask(
                            self.task_out[current_target_pe],
                            BankTaskEnum::PushBankTask(bank_task),
                        ));
                    }
                    BankTaskEnum::EndThisTask => {
                        // end this task
                        // push this to current_taget_pe and switch to the next
                        yield status.clone_with_state(SpmmStatusEnum::PushBankTask(
                            self.task_out[current_target_pe],
                            BankTaskEnum::EndThisTask,
                        ));
                        current_target_pe = (current_target_pe + 1) % num_pes;
                    }
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
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

    use desim::{resources::Store, EndCondition, Simulation};

    use crate::sim::{merger_task_sender::FullMergerStatus, SpmmStatus};

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
        let status = SpmmStatus::new(
            SpmmStatusEnum::Continue,
            Rc::new(RefCell::new(FullMergerStatus::new())),
            Rc::new(RefCell::new(BTreeMap::new())),
        );
        simulator.schedule_event(
            0.0,
            bank_task_reorder,
            status.clone_with_state(SpmmStatusEnum::Continue),
        );
        for pe in bank_pes {
            let pe_process = simulator.create_process(pe.run());
            simulator.schedule_event(
                0.0,
                pe_process,
                status.clone_with_state(SpmmStatusEnum::Continue),
            );
        }

        simulator.run(EndCondition::NoEvents);
    }
}
