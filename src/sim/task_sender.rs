use desim::ResourceId;
use sprs::CsMat;

use super::component::Component;

pub struct TaskSender<'a> {
    pub matrix: &'a CsMat<usize>,
    pub task_sender: ResourceId,
}
impl<'a> Component for TaskSender<'a> {
    fn run(self) -> Box<super::SpmmGenerator> {
        todo!()
    }
}

impl<'a> TaskSender<'a> {
    pub fn new(
        matrix: &'a CsMat<usize>,
        task_sender: ResourceId,
    ) -> Self {
        Self {
            matrix,
            task_sender,
        }
    }
}
