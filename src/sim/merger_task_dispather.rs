use super::{
    component::Component, merger_status::MergerStatusId, LevelId, SpmmContex, SpmmStatus,
    SpmmStatusEnum, StateWithSharedStatus,
};
use genawaiter::rc::{Co, Gen};
use log::debug;
use qsim::ResourceId;
#[derive(Debug)]
pub struct MergerWorkerDispatcher {
    pub level_id: LevelId,
    // receive from lower pe
    pub full_sum_in: ResourceId,
    // send to merger worker
    pub merger_task_sender: Vec<ResourceId>,

    // the merger status id
    pub merger_status_id: MergerStatusId,
}

impl Component for MergerWorkerDispatcher {
    fn run(self, original_status: SpmmStatus) -> Box<super::SpmmGenerator> {
        let process = |co: Co<SpmmStatus, SpmmContex>| async move {
            // first get the task

            loop {
                let task: SpmmContex = co
                    .yield_(original_status.clone_with_state(SpmmStatusEnum::Pop(self.full_sum_in)))
                    .await;
                let (_, ret_status) = task.into_inner();
                let StateWithSharedStatus {
                    status,
                    shared_status,
                } = ret_status.into_inner();
                let (target_row, target_result) = status.into_push_full_partial_task().unwrap().1;
                debug!(
                    "MergerWorkerDispatcher-{:?}: receive full sum:{:?} from queue: {}",
                    self.level_id, target_result, self.full_sum_in
                );
                let target_pe = shared_status
                    .shared_merger_status
                    .get_next_merger(self.merger_status_id);
                // find a empty merger!
                // push the partial result back
                debug!(
                    "MergerWorkerDispatcher-{:?}: try to send to {},full sum:{:?} to merger worker:{:?},real_id:{:?}",
                    self.level_id,self.merger_task_sender[target_pe], target_result, target_pe,self.merger_task_sender[target_pe]
                );
                co.yield_(
                    original_status.clone_with_state(SpmmStatusEnum::PushFullPartialTask(
                        self.merger_task_sender[target_pe],
                        (target_row, target_result),
                    )),
                )
                .await;
                debug!(
                    "MergerWorkerDispatcher-{:?}: succ to send to {}, to merger worker:{:?},real_id:{:?}",
                    self.level_id,self.merger_task_sender[target_pe], target_pe,self.merger_task_sender[target_pe]
                );
            }
        };

        Box::new(Gen::new(process))
    }
}
