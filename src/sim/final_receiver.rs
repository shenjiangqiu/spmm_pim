use desim::ResourceId;
use log::debug;

use crate::sim::StateWithSharedStatus;

use super::{component::Component, SpmmContex, SpmmStatusEnum};
#[derive(Debug)]
pub struct FinalReceiver {
    pub receiver: ResourceId,
    pub collect_result: bool,
}

impl Component for FinalReceiver {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            let (_time, original_status) = context.into_inner();
            let mut all_rows_collected = vec![];
            loop {
                let ret: SpmmContex =
                    yield original_status.clone_with_state(SpmmStatusEnum::Pop(self.receiver));
                debug!("FINIAL_RECIEVER: received final result: {:?}", ret);
                let (_time, pop_status) = ret.into_inner();
                let StateWithSharedStatus {
                    status,
                    shared_status: _,
                } = pop_status.into_inner();
                let (_resouce_id, (target_row, sender_id, result)) =
                    status.into_push_partial_task().unwrap();
                debug!("FINIAL_RECIEVER: {}:{}:{:?}", target_row, sender_id, result);
                if self.collect_result {
                    all_rows_collected.push(target_row);
                    debug!(
                        "FINIAL_RECIEVER: all_rows_collected: {:?}",
                        all_rows_collected
                    );
                }
            }
        })
    }
}
