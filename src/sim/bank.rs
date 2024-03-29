use std::{
    cmp,
    collections::{BTreeMap, VecDeque},
};
const BANK_ROW_SIZE: usize = 2048;
use qsim::ResourceId;
use tracing::debug;

use super::{
    component::Component,
    queue_tracker::QueueTrackerId,
    sim_time::{EndTimeId, NamedTimeId},
    types::{SpmmContex, SpmmGenerator},
    BankID, LevelId, SpmmStatus, SpmmStatusEnum,
};
use crate::{
    pim::merge_rows_into_one,
    sim::types::{BankTaskEnum, PushBankTaskType, PushPartialSumType, StateWithSharedStatus},
};
use genawaiter::rc::{Co, Gen};
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
#[derive(Debug)]
pub struct BankPe {
    pub level_id: LevelId,
    pub pe_id: usize,
    // settings
    pub merger_size: usize,
    pub adder_size: usize,
    // resources
    pub task_in: ResourceId,
    pub partial_out: ResourceId,

    // just for record
    pub task_sender_input_id: ResourceId,

    pub named_idle_time_id: NamedTimeId,
    pub end_time_id: EndTimeId,
}

impl BankPe {
    pub fn new(
        level_id: LevelId,
        pe_id: usize,
        task_in: ResourceId,
        partial_out: ResourceId,
        merger_size: usize,
        adder_size: usize,
        task_sender_input_id: ResourceId,
        named_idle_time_id: NamedTimeId,
        end_time_id: EndTimeId,
    ) -> Self {
        Self {
            level_id,
            pe_id,
            task_in,
            partial_out,
            merger_size,
            adder_size,
            task_sender_input_id,
            named_idle_time_id,
            end_time_id,
        }
    }
}

impl Component for BankPe {
    fn run(self, original_status: SpmmStatus) -> Box<SpmmGenerator> {
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            // first get the task
            let mut current_task_id = 0;
            let mut current_task_target_row = 0;
            let mut tasks = vec![];
            // this is used for record the current time before each yield
            let mut current_time = 0.;
            loop {
                let context: SpmmContex = co
                    .yield_(original_status.clone_with_state(SpmmStatusEnum::Pop(self.task_in)))
                    .await;
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
                shared_status.shared_named_time.add_idle_time(
                    &self.named_idle_time_id,
                    "get_task",
                    gap,
                );
                shared_status
                    .shared_end_time
                    .set_end_time(self.end_time_id, current_time);
                match bank_task {
                    BankTaskEnum::PushBankTask(PushBankTaskType {
                        task_id, to, row, ..
                    }) => {
                        debug!("BANK_PE: receive task: to: row: {},{:?}", to, row);

                        tasks.push(row);
                        current_task_id = task_id;
                        current_task_target_row = to;
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
                            shared_status.shared_named_time.add_idle_time(
                                &self.named_idle_time_id,
                                "compute!",
                                wait_time,
                            );
                            let context = co
                                .yield_(
                                    original_status
                                        .clone_with_state(SpmmStatusEnum::Wait(wait_time)),
                                )
                                .await;
                            let (_time, status) = context.into_inner();
                            current_time = _time;
                            // this could be idle due to upper pressure
                            debug!(
                                "BANK_PE: wait time: {:?} and push to sender: {:?}, the task:{:?}",
                                wait_time,
                                self.partial_out,
                                (current_task_target_row, self.task_sender_input_id, &data)
                            );

                            shared_status
                                .shared_end_time
                                .set_end_time(self.end_time_id, current_time);

                            let context = co
                                .yield_(status.clone_with_state(SpmmStatusEnum::PushPartialTask(
                                    self.partial_out,
                                    PushPartialSumType {
                                        task_id: current_task_id,
                                        target_row: current_task_target_row,
                                        sender_id: self.task_sender_input_id,
                                        target_result: data,
                                    },
                                )))
                                .await;
                            let (_time, _status) = context.into_inner();
                            let return_gap = _time - current_time;
                            current_time = _time;
                            shared_status.shared_named_time.add_idle_time(
                                &self.named_idle_time_id,
                                "return_to_chip",
                                return_gap,
                            );
                        }

                        tasks.clear();
                    }
                };
            }
        };

        Box::new(Gen::new(function))
    }
}

/// this struct receive the task from the chip and send the reordered task to the bank pe
#[derive(Debug)]
pub struct BankTaskReorder {
    pub level_id: LevelId,
    pub task_in: ResourceId,
    pub task_out: Vec<ResourceId>,

    pub total_reorder_size: usize,
    pub self_id: BankID,

    pub bank_change_latency: f64,

    pub comp_id: NamedTimeId,
    pub end_time_id: EndTimeId,
    pub queue_tracker_id_recv: QueueTrackerId,
}

// TODO
impl Component for BankTaskReorder {
    #[allow(unused_assignments)]
    fn run(self, original_status: SpmmStatus) -> Box<SpmmGenerator> {
        let num_pes = self.task_out.len();
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            // todo delete this
            let mut current_target_pe = 0;
            let mut current_row = 0;
            let mut current_time = 0.;
            loop {
                // first get the context
                let context: SpmmContex = co
                    .yield_(original_status.clone_with_state(SpmmStatusEnum::Pop(self.task_in)))
                    .await;
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
                // safety: the comp_id is set by add_comp, that should be valid!
                shared_status.shared_named_time.add_idle_time(
                    &self.comp_id,
                    "get_task_from_chip",
                    gap,
                );
                shared_status.queue_tracker.deq(&self.queue_tracker_id_recv);
                shared_status
                    .shared_end_time
                    .set_end_time(self.end_time_id, time);
                let (_resouce_id, task) = status.into_push_bank_task().unwrap();

                match task {
                    BankTaskEnum::PushBankTask(PushBankTaskType {
                        task_id,
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
                                let context = co
                                    .yield_(
                                        original_status.clone_with_state(SpmmStatusEnum::Wait(16.)),
                                    )
                                    .await;
                                let (_time, _status) = context.into_inner();
                                current_time = _time;
                                shared_status
                                    .shared_end_time
                                    .set_end_time(self.end_time_id, current_time);
                            }
                        } else {
                            for _i in inner_row_id_start..=inner_row_id_end {
                                total_waiting += 16.;
                                let context = co
                                    .yield_(
                                        original_status.clone_with_state(SpmmStatusEnum::Wait(16.)),
                                    )
                                    .await;
                                let (_time, _status) = context.into_inner();
                                current_time = _time;
                                shared_status
                                    .shared_end_time
                                    .set_end_time(self.end_time_id, current_time);
                            }
                        }

                        shared_status.shared_sim_time.add_bank_read(total_waiting);
                        shared_status.shared_named_time.add_idle_time(
                            &self.comp_id,
                            "read_row_buffer",
                            total_waiting,
                        );
                        current_row = inner_row_id_end;

                        let context = co
                            .yield_(
                                original_status.clone_with_state(SpmmStatusEnum::PushBankTask(
                                    self.task_out[current_target_pe],
                                    BankTaskEnum::PushBankTask(PushBankTaskType {
                                        task_id,
                                        from,
                                        to,
                                        row,
                                        bank_id,
                                        row_shift,
                                        row_size,
                                    }),
                                )),
                            )
                            .await;
                        let (_time, _status) = context.into_inner();
                        let gap = _time - current_time;
                        current_time = _time;
                        shared_status.shared_named_time.add_idle_time(
                            &self.comp_id,
                            "push_bank_task",
                            gap,
                        );
                        shared_status
                            .shared_end_time
                            .set_end_time(self.end_time_id, current_time);
                    }
                    BankTaskEnum::EndThisTask => {
                        // end this task
                        // push this to current_taget_pe and switch to the next
                        let context = co
                            .yield_(
                                original_status.clone_with_state(SpmmStatusEnum::PushBankTask(
                                    self.task_out[current_target_pe],
                                    BankTaskEnum::EndThisTask,
                                )),
                            )
                            .await;
                        let (_time, _status) = context.into_inner();
                        let gap = _time - current_time;
                        current_time = _time;
                        shared_status.shared_named_time.add_idle_time(
                            &self.comp_id,
                            "push_bank_task",
                            gap,
                        );
                        shared_status
                            .shared_end_time
                            .set_end_time(self.end_time_id, current_time);

                        current_target_pe = (current_target_pe + 1) % num_pes;
                    }
                }
            }
        };
        Box::new(Gen::new(function))
    }
}

impl BankTaskReorder {
    pub fn new(
        level_id: LevelId,
        task_in: ResourceId,
        task_out: Vec<ResourceId>,
        total_reorder_size: usize,
        self_id: BankID,
        bank_change_latency: f64,
        comp_id: NamedTimeId,
        end_time_id: EndTimeId,
        queue_tracker_id_recv: QueueTrackerId,
    ) -> Self {
        Self {
            level_id,
            task_in,
            task_out,
            total_reorder_size,
            self_id,
            bank_change_latency,
            comp_id,
            end_time_id,
            queue_tracker_id_recv,
        }
    }
}
#[cfg(test)]
mod test {
    use std::{cell::RefCell, path::Path, rc::Rc};

    use itertools::Itertools;
    use qsim::{resources::Store, EndCondition, Simulation};

    use crate::{
        csv_nodata::CsVecNodata,
        init_logger,
        settings::RealRowMapping,
        sim::{
            final_receiver::FinalReceiver,
            sim_time::{SharedEndTime, SharedNamedTime},
            task_balance::DefaultTaskScheduler,
            task_sender::TaskSender,
            SharedStatus, SpmmStatus,
        },
    };

    use super::*;
    #[test]
    fn test_bank() {
        init_logger();
        let shared_named_time: Rc<SharedNamedTime> = Rc::new(Default::default());
        let shared_end_time: Rc<SharedEndTime> = Rc::new(Default::default());
        let status = SpmmStatus::new(
            SpmmStatusEnum::Continue,
            SharedStatus {
                shared_named_time: shared_named_time.clone(),
                shared_end_time: shared_end_time.clone(),
                ..Default::default()
            },
        );
        debug!("start test");
        let mut simulator = Simulation::new();
        let two_mat = crate::utils::create_two_matrix_from_file(Path::new("mtx/test.mtx"));

        let task_in = simulator.create_resource(Box::new(Store::new(16)), "test");
        // create a final receiver for partial sum:
        let partial_return = simulator.create_resource(Box::new(Store::new(16)), "test");
        let all_received = Rc::new(RefCell::new(Vec::new()));
        let final_receiver = FinalReceiver::new(partial_return, false, &two_mat, all_received);
        let final_receiver_process = simulator.create_process(final_receiver.run(status.clone()));
        simulator.schedule_event(
            0.0,
            final_receiver_process,
            status.clone_with_state(SpmmStatusEnum::Continue),
        );
        let queue_id_send = status
            .shared_status
            .queue_tracker
            .add_component_with_name("123");
        let all_send_task = two_mat
            .a
            .outer_iterator()
            .map(|x| CsVecNodata::from(x.to_owned()))
            .collect_vec();
        let task_sender = TaskSender::<DefaultTaskScheduler>::new(
            two_mat.a,
            two_mat.b,
            task_in,
            1,
            1,
            1,
            RealRowMapping::Chunk,
            queue_id_send,
            DefaultTaskScheduler::new(all_send_task),
        );

        let task_pe = {
            let mut task_pe = vec![];
            for _i in 0..4 {
                let task_out = simulator.create_resource(Box::new(Store::new(16)), "test");
                task_pe.push(task_out);
            }
            task_pe
        };

        let comp_id = shared_named_time.add_component_with_name("123", vec!["test"]);
        let end_time_id = shared_end_time.add_component_with_name("123");

        let bank_task_reorder = BankTaskReorder::new(
            LevelId::Dimm,
            task_in,
            task_pe.clone(),
            4,
            ((0, 0), 0),
            33.,
            comp_id,
            end_time_id,
            queue_id_send,
        );
        let bank_pes = {
            let mut pes = vec![];
            for pe_in in task_pe {
                let comp_id = shared_named_time.add_component_with_name("123", vec!["test"]);
                let end_time_id = shared_end_time.add_component_with_name("123");
                let pe_comp = BankPe::new(
                    LevelId::Bank(Default::default()),
                    0,
                    pe_in,
                    partial_return,
                    4,
                    4,
                    task_in,
                    comp_id,
                    end_time_id,
                );
                pes.push(pe_comp);
            }
            pes
        };
        let bank_task_reorder = simulator.create_process(bank_task_reorder.run(status.clone()));
        let task_sender = simulator.create_process(task_sender.run(status.clone()));

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
            let pe_process = simulator.create_process(pe.run(status.clone()));
            simulator.schedule_event(
                0.0,
                pe_process,
                status.clone_with_state(SpmmStatusEnum::Continue),
            );
        }

        simulator.run(EndCondition::NoEvents);
    }
}
