use std::collections::BTreeSet;

use crate::sim::{StateWithSharedStatus, MEM_ST};
use genawaiter::rc::{Co, Gen};
use itertools::Itertools;
use log::debug;
use qsim::{ResourceId, SimContext};

use super::{
    buffer_status::BufferStatusId, component::Component, merger_status::MergerStatusId,
    sim_time::NamedTimeId, BankID, BankTask, BankTaskEnum, SpmmContex, SpmmStatus, SpmmStatusEnum,
};

pub trait MergerTaskSender {
    // lower id should be the resource id that connect to the lower pe
    fn get_lower_id(&self, bank_id: &BankID) -> usize;
    // all resouce ids
    fn get_lower_pes(&self) -> &[ResourceId];
    fn get_task_in(&self) -> ResourceId;
    fn get_merger_resouce_id(&self) -> ResourceId;
    fn get_merger_status_id(&self) -> &MergerStatusId;

    fn get_time_id(&self) -> &NamedTimeId;
    fn get_buffer_id(&self) -> &BufferStatusId;
}
#[derive(Debug, Clone, Default)]
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

impl<T> Component for T
where
    T: MergerTaskSender + 'static,
{
    /// the merger task sender
    fn run(self, original_status: SpmmStatus) -> Box<super::SpmmGenerator> {
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            let mut current_time = 0.;
            // first get the task
            loop {
                // step 1: get the finished
                let context: SimContext<SpmmStatus> = co
                    .yield_(
                        original_status.clone_with_state(SpmmStatusEnum::Pop(self.get_task_in())),
                    )
                    .await;
                debug!("MERGER_TSK_SD:id:{},{:?}", self.get_task_in(), context);
                let (_time, task) = context.into_inner();
                let gap = _time - current_time;
                current_time = _time;
                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = task.into_inner();
                shared_status
                    .shared_named_time
                    .add_idle_time(self.get_time_id(), "get_task", gap);
                let task = status.into_push_bank_task().unwrap().1;

                match task {
                    super::BankTaskEnum::PushBankTask(BankTask {
                        from,
                        to,
                        row,
                        bank_id,
                        row_shift,
                        row_size,
                    }) => {
                        // then push to target pe
                        let lower_pe_id = self.get_lower_id(&bank_id);

                        // record that the task is on going to lower_pe_id, record it!
                        shared_status.shared_buffer_status.add_waiting(
                            self.get_buffer_id(),
                            to,
                            lower_pe_id,
                        );

                        // record the merger that the target row is about to come!
                        shared_status.shared_merger_status.add_waiting(
                            self.get_merger_status_id(),
                            to,
                            MEM_ST.get().unwrap().buffer_mode.is_bind_merger(),
                        );

                        let context = co
                            .yield_(
                                original_status.clone_with_state(SpmmStatusEnum::PushBankTask(
                                    lower_pe_id,
                                    BankTaskEnum::PushBankTask(BankTask {
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
                            self.get_time_id(),
                            "push_bank_task",
                            gap,
                        );
                    }
                    super::BankTaskEnum::EndThisTask => {
                        // push this to every lower pe
                        for lower_pe_id in self.get_lower_pes().iter().cloned().collect_vec() {
                            let context = co
                                .yield_(original_status.clone_with_state(
                                    SpmmStatusEnum::PushBankTask(
                                        lower_pe_id,
                                        super::BankTaskEnum::EndThisTask,
                                    ),
                                ))
                                .await;
                            let (_time, _status) = context.into_inner();
                            let gap = _time - current_time;
                            current_time = _time;
                            shared_status.shared_named_time.add_idle_time(
                                self.get_time_id(),
                                "push_end_bank_task",
                                gap,
                            );
                        }
                    }
                }
            }
        };
        Box::new(Gen::new(function))
    }
}
