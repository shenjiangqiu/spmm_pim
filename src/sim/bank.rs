use std::{
    cmp,
    collections::{BTreeMap, VecDeque},
};
const BANK_ROW_SIZE: usize = 2048;
use desim::ResourceId;
use log::debug;

use super::{
    component::Component, sim_time::NamedTimeId, BankID, BankTask, SpmmContex, SpmmStatusEnum,
};
use crate::{
    pim::merge_rows_into_one,
    sim::{BankTaskEnum, StateWithSharedStatus},
};
//849191287

/// merger status
/// - total_merger: number of total merger workers
/// - current_working_merger: target id to merger id
/// - current_merger_worker_status: merger status
#[derive(Debug)]
pub struct BankMergerStatus {
    pub total_merger: usize,
    // target id to merger worker id
    pub current_working_merger: BTreeMap<usize, usize>,
    // merger id to merger status id
    pub current_merger_worker_status: Vec<()>,
    pub idle_merger: VecDeque<usize>,
}

impl BankMergerStatus {
    /// create a new merger status
    /// - this one should not contains any current_working_merger
    /// - current_merger_worker_status should be n-mergers with default status
    /// - idle_merger should contains 0..total_merger
    pub fn new(total_merger: usize) -> Self {
        // create a dequeue contains 0..total_merger
        let mut idle_merger = VecDeque::new();
        for i in 0..total_merger {
            idle_merger.push_back(i);
        }

        Self {
            total_merger,
            current_merger_worker_status: vec![(); total_merger],
            current_working_merger: BTreeMap::new(),
            idle_merger,
        }
    }

    /// a new target id arrives, find a idle merger if it's the first time, else increase the waiting task
    pub fn push(&mut self, _target_row: usize, _lower_pe_id: usize) {
        todo!()
    }

    pub fn pop(&mut self, _target_row: usize, _lower_pe_id: usize) {
        // minus one, if it's zero, remove it
        todo!()
    }

    pub fn exist(&self, _target_row: usize) -> bool {
        self.current_working_merger.contains_key(&_target_row)
    }
}
#[derive(Default)]
pub struct FullBankMergerStatus {
    status: Vec<BankMergerStatus>,
}

impl FullBankMergerStatus {
    // generate an empty merger status
    pub fn new() -> Self {
        Default::default()
    }
    pub fn create_bank_merger_status(&mut self, total_merger: usize) -> usize {
        self.status.push(BankMergerStatus::new(total_merger));
        self.status.len() - 1
    }
    pub fn get_bank_merger_status(&self, id: usize) -> &BankMergerStatus {
        &self.status[id]
    }
}

/// BankPe is a component that can receive tasks from chip and perform merge
pub struct BankPe {
    // settings
    pub merger_size: usize,
    pub adder_size: usize,
    // resources
    pub task_in: ResourceId,
    pub partial_out: ResourceId,

    // just for record
    pub task_sender_input_id: ResourceId,

    pub named_idle_time_id: NamedTimeId,
}

impl BankPe {
    pub fn new(
        task_in: ResourceId,
        partial_out: ResourceId,
        merger_size: usize,
        adder_size: usize,
        task_sender_input_id: ResourceId,
        named_idle_time_id: NamedTimeId,
    ) -> Self {
        Self {
            task_in,
            partial_out,
            merger_size,
            adder_size,
            task_sender_input_id,
            named_idle_time_id,
        }
    }
}

impl Component for BankPe {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            // first get the task
            let (_time, original_status) = context.into_inner();
            let mut current_task = 0;
            let mut tasks = vec![];
            // this is used for record the current time before each yield
            let mut current_time = 0.;
            loop {
                let context: SpmmContex =
                    yield original_status.clone_with_state(SpmmStatusEnum::Pop(self.task_in));
                let (time, pop_status) = context.into_inner();
                debug!("BANK_PE: time: {},received taske: {:?}", time, pop_status);
                let gap = time - current_time;
                current_time = time;

                // send read request to row buffer.

                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = pop_status.into_inner();

                let (_resouce_id, bank_task) = status.into_push_bank_task().unwrap();
                unsafe {
                    shared_status.shared_named_time.add_idle_time(
                        self.named_idle_time_id,
                        "get_task",
                        gap,
                    );
                }
                match bank_task {
                    BankTaskEnum::PushBankTask(BankTask { to, row, .. }) => {
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
                            let wait_time = cmp::max(add_cycle, merge_cycle) as f64;
                            shared_status.shared_sim_time.add_bank_merge(wait_time);
                            let context = yield original_status
                                .clone_with_state(SpmmStatusEnum::Wait(wait_time));
                            let (_time, status) = context.into_inner();
                            current_time = _time;
                            // this could be idle due to upper pressure
                            let context =
                                yield status.clone_with_state(SpmmStatusEnum::PushPartialTask(
                                    self.partial_out,
                                    (current_task, self.task_sender_input_id, data),
                                ));
                            let (_time, _status) = context.into_inner();
                            let return_gap = _time - current_time;
                            current_time = _time;
                            unsafe {
                                shared_status.shared_named_time.add_idle_time(
                                    self.named_idle_time_id,
                                    "return_to_chip",
                                    return_gap,
                                );
                            }
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

    pub total_reorder_size: usize,
    pub self_id: BankID,

    pub bank_change_latency: f64,

    pub comp_id: NamedTimeId,
}

// TODO
impl Component for BankTaskReorder {
    #[allow(unused_assignments)]
    fn run(self) -> Box<super::SpmmGenerator> {
        let num_pes = self.task_out.len();
        Box::new(move |context: SpmmContex| {
            // todo delete this
            let mut current_target_pe = 0;
            let mut current_row = 0;
            let (_time, original_status) = context.into_inner();
            let mut current_time = 0.;
            loop {
                // first get the context
                let context: SpmmContex =
                    yield original_status.clone_with_state(SpmmStatusEnum::Pop(self.task_in));
                let (time, pop_status) = context.into_inner();
                let gap = time - current_time;
                // TODO: add the idle time to the comp_time
                current_time = time;
                debug!(
                    "TASK_REORDER: time: {},received taske: {:?}",
                    time, pop_status
                );
                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = pop_status.into_inner();
                unsafe {
                    // safety: the comp_id is set by add_comp, that should be valid!
                    shared_status.shared_named_time.add_idle_time(
                        self.comp_id,
                        "get_task_from_chip",
                        gap,
                    );
                }

                let (_resouce_id, task) = status.into_push_bank_task().unwrap();

                match task {
                    BankTaskEnum::PushBankTask(BankTask {
                        from,
                        to,
                        row,
                        bank_id,
                        row_shift,
                        row_size,
                    }) => {
                        // keep push this task to the current_task_pe
                        // calculate the innder id
                        let row_start = row_shift * 4;
                        let row_end = row_start + row_size * 4;
                        let inner_row_id_start = row_start / BANK_ROW_SIZE;
                        let inner_row_id_end = row_end / BANK_ROW_SIZE;
                        // read the bank
                        let mut total_waiting = 0.;
                        if inner_row_id_start == current_row {
                            for _i in inner_row_id_start..inner_row_id_end {
                                // TODO , read the setting
                                total_waiting += 16.;
                                let context = yield original_status
                                    .clone_with_state(SpmmStatusEnum::Wait(16.));
                                let (_time, _status) = context.into_inner();
                                current_time = _time;
                            }
                        } else {
                            for _i in inner_row_id_start..=inner_row_id_end {
                                total_waiting += 16.;
                                let context = yield original_status
                                    .clone_with_state(SpmmStatusEnum::Wait(16.));
                                let (_time, _status) = context.into_inner();
                                current_time = _time;
                            }
                        }

                        shared_status.shared_sim_time.add_bank_read(total_waiting);
                        current_row = inner_row_id_end;

                        let context =
                            yield original_status.clone_with_state(SpmmStatusEnum::PushBankTask(
                                self.task_out[current_target_pe],
                                BankTaskEnum::PushBankTask(BankTask {
                                    from,
                                    to,
                                    row,
                                    bank_id,
                                    row_shift,
                                    row_size,
                                }),
                            ));
                        let (_time, _status) = context.into_inner();
                        current_time = _time;
                    }
                    BankTaskEnum::EndThisTask => {
                        // end this task
                        // push this to current_taget_pe and switch to the next
                        let context =
                            yield original_status.clone_with_state(SpmmStatusEnum::PushBankTask(
                                self.task_out[current_target_pe],
                                BankTaskEnum::EndThisTask,
                            ));
                        let (_time, _status) = context.into_inner();
                        current_time = _time;
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
        total_reorder_size: usize,
        self_id: BankID,
        bank_change_latency: f64,
        comp_id: NamedTimeId,
    ) -> Self {
        Self {
            task_in,
            task_out,
            total_reorder_size,
            self_id,
            bank_change_latency,
            comp_id,
        }
    }
}
#[cfg(test)]
mod test {
    use std::{path::Path, rc::Rc};

    use desim::{resources::Store, EndCondition, Simulation};

    use crate::{
        settings::RowMapping,
        sim::{
            final_receiver::FinalReceiver, sim_time::SharedNamedTime, task_sender::TaskSender,
            SpmmStatus,
        },
    };

    use super::*;
    use crate::sim;
    #[test]
    fn test_bank() {
        let config_str = include_str!("../../log_config.yml");
        let config = serde_yaml::from_str(config_str).unwrap();
        log4rs::init_raw_config(config).unwrap_or(());
        let shared_comp_time: Rc<SharedNamedTime> = Rc::new(Default::default());
        let status = SpmmStatus::new(SpmmStatusEnum::Continue, Default::default());
        debug!("start test");
        let mut simulator = Simulation::new();
        let two_mat = sim::create_two_matrix_from_file(Path::new("mtx/test.mtx"));

        let task_in = simulator.create_resource(Box::new(Store::new(16)));

        let task_sender =
            TaskSender::new(two_mat.a, two_mat.b, task_in, 1, 1, 1, RowMapping::Chunk);

        let task_pe = {
            let mut task_pe = vec![];
            for _i in 0..4 {
                let task_out = simulator.create_resource(Box::new(Store::new(16)));
                task_pe.push(task_out);
            }
            task_pe
        };

        let partial_return = simulator.create_resource(Box::new(Store::new(16)));
        let comp_id = shared_comp_time.add_component_with_name("123");
        let bank_task_reorder =
            BankTaskReorder::new(task_in, task_pe.clone(), 4, ((0, 0), 0), 33., comp_id);
        let bank_pes = {
            let mut pes = vec![];
            for pe_in in task_pe {
                let comp_id = shared_comp_time.add_component_with_name("123");
                let pe_comp = BankPe::new(pe_in, partial_return, 4, 4, task_in, comp_id);
                pes.push(pe_comp);
            }
            pes
        };
        let bank_task_reorder = simulator.create_process(bank_task_reorder.run());
        let task_sender = simulator.create_process(task_sender.run());

        simulator.schedule_event(
            0.0,
            bank_task_reorder,
            status.clone_with_state(SpmmStatusEnum::Continue),
        );
        simulator.schedule_event(
            0.,
            task_sender,
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

        // create a final receiver for partial sum:
        let final_receiver = FinalReceiver {
            receiver: partial_return,
        };
        let final_receiver_process = simulator.create_process(final_receiver.run());
        simulator.schedule_event(
            0.0,
            final_receiver_process,
            status.clone_with_state(SpmmStatusEnum::Continue),
        );

        simulator.run(EndCondition::NoEvents);
    }
}
