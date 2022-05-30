use desim::ResourceId;



use super::{
    component::Component, PartialResultTaskType, SpmmContex, SpmmStatusEnum,
};

pub struct Merger {
    task_in: ResourceId,
    partial_out: ResourceId,
}

impl Merger {
    fn process_task(&self, _task: PartialResultTaskType) -> (f64, PartialResultTaskType) {
        todo!();
    }
}

impl Component for Merger {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |_: SpmmContex| {
            // first get the task
            loop {
                let task: SpmmContex = yield SpmmStatusEnum::Pop(self.task_in).into();
                let (_, status) = task.into_inner();
                let (_, task) = status.into_inner();
                let task=task.into_push_partial_task().unwrap().1;
                // process
                let (process_time, partial_out) = self.process_task(task);
                
                yield SpmmStatusEnum::Wait(process_time).into();
                // push the partial result back
                yield SpmmStatusEnum::PushPartialTask(self.partial_out, partial_out).into();
            }
        })
    }
}
