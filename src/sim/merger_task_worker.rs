use log::{debug, info};
use qsim::ResourceId;

use crate::sim::StateWithSharedStatus;

use super::{
    component::Component,
    merger_status::MergerStatusId,
    sim_time::{LevelTimeId, NamedTimeId},
    SpmmContex, SpmmStatus, SpmmStatusEnum,
};
use genawaiter::rc::{Co, Gen};
/// - `MergerWorker`: this is the worker for each level, there should be multile instance for each level!
struct MergerWorker {
    /// the id (0.. merger_count-1) of a single chip or channel
    pub id: usize,
    pub task_reciever: ResourceId,
    pub partial_sum_sender: ResourceId,
    pub merger_status_id: MergerStatusId,
    pub merger_size: usize,
    pub task_sender_input_id: usize,

    // just recording, not for use to send data
    pub level_time: LevelTimeId,
    pub time_id: NamedTimeId,
}

impl Component for MergerWorker {
    fn run(self, original_status: SpmmStatus) -> Box<super::SpmmGenerator> {
        info!("level_time:{:?}", self.level_time);
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            // first get the task
            let mut current_time = 0.;
            loop {
                let context: SpmmContex = co
                    .yield_(
                        original_status.clone_with_state(SpmmStatusEnum::Pop(self.task_reciever)),
                    )
                    .await;
                debug!("MERGER_WORKER:id:{},{:?}", self.task_reciever, context);
                let (_time, pop_status) = context.into_inner();
                // FIX BUG HERE, THE _TIME IS SHADDOWED
                let idle_time = _time - current_time;
                current_time = _time;
                debug!("current_time:{}", current_time);
                // send read request to row buffer.
                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = pop_status.into_inner();
                unsafe {
                    // Safety: the comp_id is valid!
                    shared_status.shared_named_time.add_idle_time(
                        &self.time_id,
                        "wait_task",
                        idle_time,
                    );
                }
                let (_resouce_id, (target_row, target_result)) =
                    status.into_push_full_partial_task().unwrap();
                // first we need pop from the

                // the last one is finished!
                // last, process the result, send the result and release the resource
                let (add_time, merge_time, partial_sum) =
                    crate::pim::merge_rows_into_one(target_result, self.merger_size);
                // wait time in max(add_time, merge_time)
                let wait_time = add_time.max(merge_time) as f64;

                let context = co
                    .yield_(original_status.clone_with_state(SpmmStatusEnum::Wait(wait_time)))
                    .await;
                let (_time, _wait_status) = context.into_inner();
                current_time = _time;
                // push to upper
                let context = co
                    .yield_(
                        original_status.clone_with_state(SpmmStatusEnum::PushPartialTask(
                            self.partial_sum_sender,
                            (target_row, self.task_sender_input_id, partial_sum),
                        )),
                    )
                    .await;
                let (_time, _push_status) = context.into_inner();
                let return_idle_time = _time - current_time;
                unsafe {
                    shared_status.shared_named_time.add_idle_time(
                        &self.time_id,
                        "push_partial",
                        return_idle_time,
                    );
                }
                current_time = _time;

                // release the merger status
                shared_status
                    .shared_merger_status
                    .release_merger(self.merger_status_id, self.id);
            }
        };
        Box::new(Gen::new(function))
    }
}
