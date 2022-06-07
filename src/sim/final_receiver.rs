use desim::ResourceId;

use super::{component::Component, SpmmContex, SpmmStatusEnum};

pub struct FinalReceiver {
    pub receiver: ResourceId,
}

impl Component for FinalReceiver {
    fn run(self) -> Box<super::SpmmGenerator> {
        Box::new(move |context: SpmmContex| {
            let (_time, status) = context.into_inner();

            loop {
                let ret: SpmmContex =
                    yield status.clone_with_state(SpmmStatusEnum::Pop(self.receiver));
                let (_time, pop_status) = ret.into_inner();
                let (_enable_log, state, _merger_status, _bank_status) = pop_status.into_inner();
                let (_resouce_id, (target_row, sender_id, result)) =
                    state.into_push_partial_task().unwrap();
                println!("{}:{}:{:?}", target_row, sender_id, result);
            }
        })
    }
}
