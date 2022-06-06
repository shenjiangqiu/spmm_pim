use desim::ResourceId;

use super::component::Component;

pub struct FinalReceiver{
    pub receiver:ResourceId,
}

impl Component for FinalReceiver{
    fn run(self) -> Box<super::SpmmGenerator> {
        todo!()
    }
}