use desim::ResourceId;

use super::{component::Component};

pub struct TaskSender {
    pub matrix: Vec<(usize, usize)>,
    pub config: TaskSenderConfig,
    pub task_sender: ResourceId,
}
impl Component for TaskSender {
    fn run(self) -> Box<super::SpmmGenerator> {
        todo!()
    }
}
pub struct TaskSenderConfig {}

impl TaskSender {
    pub fn new(
        matrix: Vec<(usize, usize)>,
        task_sender_config: TaskSenderConfig,
        task_sender: ResourceId,
    ) -> Self {
        Self {
            matrix,
            config: task_sender_config,
            task_sender,
        }
    }
}
