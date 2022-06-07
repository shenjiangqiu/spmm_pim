use std::collections::{BTreeMap, BTreeSet, VecDeque};

use desim::{ResourceId, SimContext};
use itertools::Itertools;

use super::{
    component::Component, BankID, BankTask, BankTaskEnum, SpmmContex, SpmmStatus, SpmmStatusEnum,
};

pub trait MergerTaskSender {
    // lower id should be the resource id that connect to the lower pe
    fn get_lower_id(&self, bank_id: &BankID) -> usize;
    // all resouce ids
    fn get_lower_pes(&self) -> &[ResourceId];
    fn get_task_in(&self) -> ResourceId;
    fn get_merger_resouce_id(&self) -> ResourceId;
    fn get_merger_status_id(&self) -> usize;
}
#[derive(Debug, Clone)]
pub struct MergerWorkerStatus {
    pub waiting_banks: BTreeSet<usize>,
}
impl MergerWorkerStatus {
    pub fn new() -> Self {
        Self {
            waiting_banks: BTreeSet::new(),
        }
    }
    pub fn add_lower_record(&mut self, lower_id: usize) {
        self.waiting_banks.insert(lower_id);
    }
    pub fn del_lower_record(&mut self, lower_id: usize) {
        self.waiting_banks.remove(&lower_id);
    }
}

#[derive(Debug)]
pub struct FullMergerStatus {
    pub id_to_mergerstatus: Vec<MergerStatus>,
}

impl FullMergerStatus {
    pub fn new() -> Self {
        Self {
            id_to_mergerstatus: vec![],
        }
    }
    pub fn create_merger_status(&mut self, total_merger: usize) -> usize {
        self.id_to_mergerstatus
            .push(MergerStatus::new(total_merger));
        self.id_to_mergerstatus.len() - 1
    }
    pub fn get_merger_status(&self, id: usize) -> &MergerStatus {
        &self.id_to_mergerstatus[id]
    }
}

/// merger status
/// - total_merger: number of total merger workers
/// - current_working_merger: target id to merger id
/// - current_merger_worker_status: merger status
#[derive(Debug)]
pub struct MergerStatus {
    pub total_merger: usize,
    // target id to merger worker id
    pub current_working_merger: BTreeMap<usize, usize>,
    // merger id to merger status id
    pub current_merger_worker_status: Vec<MergerWorkerStatus>,

    pub idle_merger: VecDeque<usize>,
}

impl MergerStatus {
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
            current_merger_worker_status: vec![MergerWorkerStatus::new(); total_merger],
            current_working_merger: BTreeMap::new(),
            idle_merger,
        }
    }

    /// a new target id arrives, find a idle merger if it's the first time, else increase the waiting task
    pub fn push(&mut self, target_row: usize, lower_pe_id: usize) {
        // insert or plus one
        self.current_working_merger
            .entry(target_row)
            .and_modify(|e| self.current_merger_worker_status[*e].add_lower_record(lower_pe_id))
            .or_insert_with(|| {
                let id = self.idle_merger.pop_front().unwrap();
                self.current_merger_worker_status[id].add_lower_record(lower_pe_id);
                id
            });
    }

    pub fn pop(&mut self, target_row: usize, lower_pe_id: usize) {
        // minus one, if it's zero, remove it
        self.current_working_merger
            .entry(target_row)
            .and_modify(|e| self.current_merger_worker_status[*e].del_lower_record(lower_pe_id));

        if self.current_merger_worker_status[self.current_working_merger[&target_row]]
            .waiting_banks
            .is_empty()
        {
            // remove it
            self.current_working_merger.remove(&target_row);
        }
    }

    pub fn exist(&self, target_row: usize) -> bool {
        self.current_working_merger.contains_key(&target_row)
    }
}

impl<T> Component for T
where
    T: MergerTaskSender + 'static,
{
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            let (_time, status) = context.into_inner();
            let mut first_task = true;
            // first get the task
            loop {
                // step 1: get the finished
                let context: SimContext<SpmmStatus> =
                    yield status.clone_with_state(SpmmStatusEnum::Pop(self.get_task_in()));
                let (_time, task) = context.into_inner();
                let (_, task, merger_status, _) = task.into_inner();
                let task = task.into_push_bank_task().unwrap().1;
                let status_id = self.get_merger_status_id();

                match task {
                    super::BankTaskEnum::PushBankTask(BankTask {
                        from,
                        to,
                        row,
                        bank_id,
                    }) => {
                        // push to target pe and set the status

                        // first set the status:
                        if first_task {
                            yield status.clone_with_state(SpmmStatusEnum::Acquire(
                                self.get_merger_resouce_id(),
                            ));
                            first_task = false;
                        }
                        // then push to target pe
                        let lower_pe_id = self.get_lower_id(&bank_id);

                        merger_status.borrow_mut().id_to_mergerstatus[status_id]
                            .push(to, lower_pe_id);
                        yield status.clone_with_state(SpmmStatusEnum::PushBankTask(
                            lower_pe_id,
                            BankTaskEnum::PushBankTask(BankTask {
                                from,
                                to,
                                row,
                                bank_id,
                            }),
                        ));
                    }
                    super::BankTaskEnum::EndThisTask => {
                        // push this to every lower pe
                        for lower_pe_id in self.get_lower_pes().iter().cloned().collect_vec() {
                            yield status.clone_with_state(SpmmStatusEnum::PushBankTask(
                                lower_pe_id,
                                super::BankTaskEnum::EndThisTask,
                            ));
                        }
                        first_task = true;
                    }
                }
            }
        })
    }
}
