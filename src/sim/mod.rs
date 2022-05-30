pub mod bank;
pub mod component;
pub mod merger;
pub mod task_reorderer;
pub mod task_router;
pub mod task_sender;
use desim::prelude::*;
use enum_as_inner::EnumAsInner;
use std::{cell::RefCell, collections::BTreeMap, ops::Generator};

thread_local! {
    pub static  PE_MAPPING: RefCell<BTreeMap::<PeID, usize>> =
        RefCell::new(BTreeMap::<PeID, usize>::new());
}

use crate::{csv_nodata::CsVecNodata};
#[derive(Debug, Clone, Default)]
pub struct BankTask {
    pub from: usize,
    pub to: usize,
    pub row: CsVecNodata<usize>,
    pub inner_bank_id: usize,
}

pub type ChannelID = usize;
pub type ChipID = (ChannelID, usize);
pub type BankID = (ChipID, usize);
pub type PeID = (BankID, usize);

pub type SpmmContex = SimContext<SpmmStatus>;
pub type SpmmGenerator = dyn Generator<SpmmContex, Yield = SpmmStatus, Return = ()> + Unpin;
//todo: add the type
pub type BankTaskType = BankTask;
pub type PartialResultTaskType = CsVecNodata<usize>;
pub type BankReadRowTaskType = usize;

#[derive(Default, Debug, Clone, EnumAsInner)]
pub enum SpmmStatusEnum {
    #[default]
    Continue,
    Wait(f64),
    PushBankTask(ResourceId, BankTaskType),
    PushPartialTask(ResourceId, PartialResultTaskType),
    PushReadBankTask(ResourceId, BankReadRowTaskType),

    Pop(ResourceId),
}

#[derive(Debug, Clone)]
pub struct SpmmStatus {
    state: SpmmStatusEnum,

    // bank pe to task mapping:
    enable_log: bool,
}

impl From<SpmmStatusEnum> for SpmmStatus {
    fn from(state: SpmmStatusEnum) -> Self {
        Self {
            state,
            enable_log: false,
        }
    }
}
impl SpmmStatus {
    pub fn new(state: SpmmStatusEnum) -> Self {
        Self {
            state,
            enable_log: false,
        }
    }

    pub fn new_log(state: SpmmStatusEnum) -> Self {
        Self {
            state,
            enable_log: true,
        }
    }
    pub fn state(&self) -> &SpmmStatusEnum {
        &self.state
    }
    pub fn into_inner(self) -> (bool, SpmmStatusEnum) {
        (self.enable_log, self.state)
    }
}

impl Default for SpmmStatus {
    fn default() -> Self {
        Self {
            enable_log: false,
            state: Default::default(),
        }
    }
}

impl SimState for SpmmStatus {
    fn get_effect(&self) -> Effect {
        match &self.state {
            SpmmStatusEnum::Continue => Effect::TimeOut(0.),
            SpmmStatusEnum::Wait(time) => Effect::TimeOut(*time),
            SpmmStatusEnum::PushBankTask(rid, _) => Effect::Push(*rid),
            SpmmStatusEnum::Pop(rid) => Effect::Pop(*rid),
            SpmmStatusEnum::PushPartialTask(rid, _) => Effect::Push(*rid),
            SpmmStatusEnum::PushReadBankTask(rid, _) => Effect::Push(*rid),
        }
    }

    fn set_effect(&mut self, _: Effect) {
        panic!("set_effect is not supported");
    }

    fn should_log(&self) -> bool {
        self.enable_log
    }
}
#[cfg(test)]
mod test {
    use desim::resources::Store;

    use super::*;
    #[test]
    pub fn run() {
        let mut sim = Simulation::new();
        let queue1 = sim.create_resource(Box::new(Store::new(16)));
        let queue2 = sim.create_resource(Box::new(Store::new(12)));

        let process1 = sim.create_process(Box::new(move |_: SimContext<SpmmStatus>| {
            yield SpmmStatusEnum::Wait(10.).into();
            yield SpmmStatusEnum::PushBankTask(queue1, Default::default()).into();
            let ret = yield SpmmStatusEnum::Pop(queue2).into();
            println!(
                "ret: {:?}",
                ret.state().state().as_push_bank_task().unwrap()
            );
        }));

        let process2 = sim.create_process(Box::new(move |_: SimContext<SpmmStatus>| {
            yield SpmmStatusEnum::Wait(5.).into();
            let ret = yield SpmmStatusEnum::Pop(queue1).into();
            println!(
                "ret: {:?}",
                ret.state().state().as_push_bank_task().unwrap()
            );
            yield SpmmStatusEnum::PushBankTask(queue2, Default::default()).into();
        }));

        sim.schedule_event(0., process1, SpmmStatus::default());
        sim.schedule_event(0., process2, Default::default());
        let sim = sim.run(EndCondition::NoEvents);
        let events = sim.processed_events();
        for i in events {
            println!("{:?}", i);
        }
    }
}
