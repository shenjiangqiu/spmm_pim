use desim::ResourceId;

use super::{component::Component, SpmmContex, SpmmStatusEnum};

pub struct MergerWorker {
    pub task_reciever: ResourceId,
    pub partial_sum_sender: ResourceId,
    pub merger_work_resource: ResourceId,
    pub merger_status_id: usize,
    pub merger_size: usize,

    // just recording, not for use to send data
    pub task_sender_input_id: ResourceId,
}

impl Component for MergerWorker {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            // first get the task
            let (_time, status) = context.into_inner();
            let mut tasks = vec![];
            loop {
                let context: SpmmContex =
                    yield status.clone_with_state(SpmmStatusEnum::Pop(self.task_reciever));
                let (_time, pop_status) = context.into_inner();

                // send read request to row buffer.
                let (_enable_log, state, merger_status, _bank_status) = pop_status.into_inner();
                let (_resouce_id, (target_row, task_in_id, target_result)) =
                    state.into_push_partial_task().unwrap();
                // first we need pop from the
                merger_status.borrow_mut().id_to_mergerstatus[self.merger_status_id]
                    .pop(target_row, task_in_id);
                // then test if this is the last
                if merger_status.borrow().id_to_mergerstatus[self.merger_status_id]
                    .exist(target_row)
                {
                    tasks.push(target_result);
                } else {
                    // last, process the result, send the result and release the resource
                    let (add_time, merge_time, partial_sum) =
                        crate::pim::merge_rows_into_one(tasks.clone(), self.merger_size);
                    // wait time in max(add_time, merge_time)
                    let wait_time = std::cmp::max(add_time, merge_time);
                    yield status.clone_with_state(SpmmStatusEnum::Wait(wait_time as f64));
                    // push to upper
                    yield status.clone_with_state(SpmmStatusEnum::PushPartialTask(
                        self.partial_sum_sender,
                        (target_row, self.task_sender_input_id, partial_sum),
                    ));
                    tasks.clear();
                    yield status
                        .clone_with_state(SpmmStatusEnum::Release(self.merger_work_resource));
                }
            }
        })
    }
}
