use desim::ResourceId;
use log::debug;

use super::{component::Component, SpmmContex, SpmmStatusEnum};

pub struct MergerWorker {
    pub task_reciever: ResourceId,
    pub partial_sum_sender: ResourceId,
    pub merger_work_resource: ResourceId,
    pub merger_status_id: usize,
    pub merger_size: usize,

    // just recording, not for use to send data
    pub task_sender_input_id: ResourceId,
    pub self_level_id: usize,
    pub comp_id: usize,
}

impl Component for MergerWorker {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            // first get the task
            let (_time, status) = context.into_inner();
            let mut tasks = vec![];
            let mut the_first = true;
            let mut the_first_time = 0.;
            let mut current_time=0.;
            loop {
                let context: SpmmContex =
                    yield status.clone_with_state(SpmmStatusEnum::Pop(self.task_reciever));
                debug!("MERGER_WORKER:id:{},{:?}", self.task_reciever, context);
                let (_time, pop_status) = context.into_inner();
                // FIX BUG HERE, THE _TIME IS SHADDOWED
                let idle_time = _time - current_time;
                if the_first {
                    the_first = false;
                    the_first_time = _time;
                }
                // send read request to row buffer.
                let (_enable_log, state, merger_status, _bank_status, level_time, comp_time) =
                    pop_status.into_inner();
                unsafe {
                    // Safety: the comp_id is valid!
                    comp_time.add_idle_time(self.comp_id, idle_time);
                }
                let (_resouce_id, (target_row, task_in_id, target_result)) =
                    state.into_push_partial_task().unwrap();
                // first we need pop from the
                tasks.push(target_result);

                merger_status.borrow_mut().id_to_mergerstatus[self.merger_status_id]
                    .pop(target_row, task_in_id);
                // then test if this is the last
                if !merger_status.borrow().id_to_mergerstatus[self.merger_status_id]
                    .exist(target_row)
                {
                    // the last one is finished!
                    the_first = true;
                    let gap = _time - the_first_time;
                    // last, process the result, send the result and release the resource
                    let (add_time, merge_time, partial_sum) =
                        crate::pim::merge_rows_into_one(tasks.clone(), self.merger_size);
                    // wait time in max(add_time, merge_time)
                    let wait_time = std::cmp::max(add_time, merge_time) as f64;
                    unsafe {
                        level_time.add_finished_time(self.self_level_id, (wait_time, gap));
                    }
                    yield status.clone_with_state(SpmmStatusEnum::Wait(wait_time));
                    // push to upper
                    yield status.clone_with_state(SpmmStatusEnum::PushPartialTask(
                        self.partial_sum_sender,
                        (target_row, self.task_sender_input_id, partial_sum),
                    ));
                    tasks.clear();
                    yield status
                        .clone_with_state(SpmmStatusEnum::Release(self.merger_work_resource));
                }
                // START TO WAIT FOR THE NEXT TASK
                current_time = _time;

            }
        })
    }
}
