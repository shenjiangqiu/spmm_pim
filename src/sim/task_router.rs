use qsim::ResourceId;

use super::{
    component::Component,
    types::{SpmmContex, SpmmGenerator},
    SpmmStatus, SpmmStatusEnum,
};
use genawaiter::rc::{Co, Gen};
pub struct TaskRouterConfig {}

pub struct TaskRouter {
    pub task_in: ResourceId,
    pub task_out_ports: Vec<ResourceId>,
    pub task_router_config: TaskRouterConfig,
}
impl TaskRouter {
    pub fn new(
        task_in: ResourceId,
        task_out_ports: Vec<ResourceId>,
        task_router_config: TaskRouterConfig,
    ) -> Self {
        Self {
            task_in,
            task_out_ports,
            task_router_config,
        }
    }
}

impl Component for TaskRouter {
    fn run(self, original_status: SpmmStatus) -> Box<SpmmGenerator> {
        let process = |co: Co<SpmmStatus, SpmmContex>| async move {
            co.yield_(original_status.clone_with_state(SpmmStatusEnum::Continue))
                .await;
        };
        Box::new(Gen::new(process))
    }
}
