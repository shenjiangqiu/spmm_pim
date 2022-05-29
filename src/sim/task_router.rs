use desim::ResourceId;

use super::{component::Component, SpmmGenerator, SpmmStatusEnum};

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
    fn run(self) -> Box<SpmmGenerator> {
        Box::new(move |_| {
            yield SpmmStatusEnum::Continue.into();
            

        })
    }
}
