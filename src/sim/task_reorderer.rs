//! this mod contains task reorder component
//!
//!

use qsim::ResourceId;

use crate::sim::types::StateWithSharedStatus;

use super::{
    component::Component,
    types::{SpmmContex, SpmmGenerator},
    SpmmStatus, SpmmStatusEnum,
};
use genawaiter::rc::{Co, Gen};
pub struct TaskReordererSetting {}
pub struct TaskReorderer {
    pub task_in: ResourceId,
    pub task_out: ResourceId,
    pub task_reorderer_config: TaskReordererSetting,
}

impl TaskReorderer {
    pub fn new(
        task_in: ResourceId,
        task_out: ResourceId,
        task_reorderer_config: TaskReordererSetting,
    ) -> Self {
        Self {
            task_in,
            task_out,
            task_reorderer_config,
        }
    }
}

impl Component for TaskReorderer {
    fn run(self, original_status: SpmmStatus) -> Box<SpmmGenerator> {
        let process = |co: Co<SpmmStatus, SpmmContex>| async move {
            loop {
                let task = co
                    .yield_(original_status.clone_with_state(SpmmStatusEnum::Pop(self.task_in)))
                    .await;
                let (_time, state) = task.into_inner();
                let StateWithSharedStatus {
                    status,
                    shared_status: _,
                } = state.into_inner();
                let task = status.into_push_bank_task().unwrap().1;
                // do some reorder work

                // finished, push the task
                co.yield_(
                    original_status
                        .clone_with_state(SpmmStatusEnum::PushBankTask(self.task_out, task)),
                )
                .await;
            }
        };
        Box::new(Gen::new(process))
    }
}
