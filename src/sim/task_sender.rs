use desim::ResourceId;


use super::{component::Component, BankTaskType, SpmmStatusEnum};

pub struct TaskSender {
    pub matrix: Vec<(usize, usize)>,
    pub config: TaskSenderConfig,
    pub task_sender: ResourceId,
}
impl Component for TaskSender {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |_| {
            for (from, to) in self.matrix {
                // build the task
                let task = BankTaskType { from, to };
                
                yield SpmmStatusEnum::PushBankTask(self.task_sender, task).into();
            }
        })
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
