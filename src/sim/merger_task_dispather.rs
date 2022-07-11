use desim::ResourceId;

use super::{component::Component, SpmmContex, SpmmStatusEnum};

pub struct MergerWorkerDispatcher {
    // receive from lower pe
    pub partial_sum_task_in: ResourceId,
    // send to merger worker
    pub merger_task_sender: Vec<ResourceId>,

    // the merger status id
    pub merger_status_id: usize,
}

impl Component for MergerWorkerDispatcher {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            // first get the task

            let (_time, status) = context.into_inner();
            loop {
                let task: SpmmContex =
                    yield status.clone_with_state(SpmmStatusEnum::Pop(self.partial_sum_task_in));
                let (_, ret_status) = task.into_inner();
                let (_, task, merger_status, ..) = ret_status.into_inner();
                let (target_row, task_in_id, target_result) =
                    task.into_push_partial_task().unwrap().1;
                let target_pe = *merger_status
                    .borrow()
                    .get_merger_status(self.merger_status_id)
                    .current_working_merger
                    .get(&target_row)
                    .unwrap();

                // push the partial result back
                yield status.clone_with_state(SpmmStatusEnum::PushPartialTask(
                    self.merger_task_sender[target_pe],
                    (target_row, task_in_id, target_result),
                ));
            }
        })
    }
}
