use std::{cell::RefCell, collections::BTreeMap, fmt::Debug, rc::Rc};

use enum_as_inner::EnumAsInner;
use genawaiter::Coroutine;
use qsim::{resources::CopyDefault, Effect, ResourceId, SimContext, SimState};

use crate::csv_nodata::CsVecNodata;

use super::{
    buffer_status::SharedBufferStatus,
    id_translation::{BankID, LevelId, PeID},
    merger_status::SharedMergerStatus,
    queue_tracker::QueueTracker,
    sim_time::{LevelTime, SharedEndTime, SharedNamedTime, SharedSimTime},
};
// target row, sender_id, target result
#[derive(Debug, Clone)]
pub struct PushPartialSumType {
    pub task_id: usize,
    pub target_row: usize,
    pub sender_id: ResourceId,
    pub target_result: CsVecNodata<usize>,
}
#[derive(Debug, Clone)]

pub struct PushFullSumType {
    pub task_id: usize,
    pub target_row: usize,
    pub target_result: Vec<CsVecNodata<usize>>,
}

#[derive(Debug, Clone, Default)]
pub struct PushBankTaskType {
    pub task_id: usize,
    pub from: usize,
    pub to: usize,
    pub row: CsVecNodata<usize>,
    pub bank_id: BankID,
    pub row_shift: usize,
    pub row_size: usize,
}

pub type SpmmContex = SimContext<SpmmStatus>;
pub type SpmmGenerator =
    dyn Coroutine<Resume = SpmmContex, Yield = SpmmStatus, Return = ()> + Unpin;
//todo: add the type
#[derive(Debug, Clone, EnumAsInner, Default)]
pub enum BankTaskEnum {
    PushBankTask(PushBankTaskType),
    #[default]
    EndThisTask,
}

/// this struct contains the information of the signale that send from the partial sum sender,
/// it should contains: 1. the target id, 2. the source id
/// it will be send by the `partial_sum_sender`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartialSignalType {
    pub task_id: usize,
    pub target_row: usize,
    pub sender_id: usize,
    pub queue_id: usize,
}

#[derive(Debug, Clone)]
pub struct ReadyQueueIdType {
    pub task_id: usize,
    pub target_row: usize,
    pub queue_id: usize,
    pub is_finished: bool,
}

#[derive(Default, Debug, Clone, EnumAsInner)]
pub enum SpmmStatusEnum {
    #[default]
    Continue,
    Wait(f64),
    PushBankTask(ResourceId, BankTaskEnum),
    PushPartialTask(ResourceId, PushPartialSumType),
    PushSignal(ResourceId, PartialSignalType),
    PushReadyQueueId(ResourceId, ReadyQueueIdType),
    PushFullPartialTask(ResourceId, PushFullSumType),
    PushBufferPopSignal(ResourceId),
    PushMergerFinishedSignal(ResourceId),
    Pop(ResourceId),
}

/// all the shared status that can be modified by all processes
#[derive(Debug, Clone, Default)]
pub struct SharedStatus {
    // fix here, this one is not used because we use buffer status to store the status
    // pub shared_merger_status: Rc<RefCell<merger_task_sender::FullMergerStatus>>,
    pub shared_bankpe_status: Rc<RefCell<BTreeMap<PeID, usize>>>,
    pub shared_sim_time: Rc<SharedSimTime>,
    pub shared_level_time: Rc<LevelTime>,
    pub shared_named_time: Rc<SharedNamedTime>,
    pub shared_buffer_status: Rc<SharedBufferStatus>,
    pub shared_merger_status: Rc<SharedMergerStatus>,
    pub shared_end_time: Rc<SharedEndTime>,
    pub queue_tracker: Rc<QueueTracker>,
}
pub struct StateWithSharedStatus {
    pub status: SpmmStatusEnum,
    pub shared_status: SharedStatus,
}

#[derive(Clone)]
pub struct SpmmStatus {
    pub state: SpmmStatusEnum,

    // bank pe to task mapping:
    pub enable_log: bool,
    pub shared_status: SharedStatus,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct MergerId {
    level_id: LevelId,
    id: usize,
}

impl Debug for SpmmStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpmmStatus")
            .field("state", &self.state)
            .finish()
    }
}

impl CopyDefault for SpmmStatus {
    fn copy_default(&self) -> Self {
        let enable_log = self.enable_log;

        let shared_status = self.shared_status.clone();
        Self {
            state: SpmmStatusEnum::Continue,
            enable_log,
            shared_status,
        }
    }
}

impl SpmmStatus {
    pub fn new(state: SpmmStatusEnum, shared_status: SharedStatus) -> Self {
        Self {
            state,
            enable_log: false,
            shared_status,
        }
    }
    // this function is not used any more, because we use buffer status to store the status
    // pub fn get_target_pe_from_target_row(
    //     &self,
    //     status_id: usize,
    //     target_row: usize,
    // ) -> Option<usize> {
    //     self.shared_status
    //         .shared_merger_status
    //         .borrow()
    //         .get_merger_status(status_id)
    //         .current_working_merger
    //         .get(&target_row)
    //         .cloned()
    // }

    pub fn clone_with_state(&self, state: SpmmStatusEnum) -> Self {
        Self {
            state,
            enable_log: self.enable_log,
            shared_status: self.shared_status.clone(),
        }
    }

    pub fn set_state(self, state: SpmmStatusEnum) -> Self {
        Self { state, ..self }
    }
    pub fn set_log(self, enable_log: bool) -> Self {
        Self { enable_log, ..self }
    }

    pub fn new_log(state: SpmmStatusEnum, shared_status: SharedStatus) -> Self {
        Self {
            state,
            enable_log: true,
            shared_status,
        }
    }
    pub fn state(&self) -> &SpmmStatusEnum {
        &self.state
    }

    pub fn into_inner(self) -> StateWithSharedStatus {
        StateWithSharedStatus {
            status: self.state,
            shared_status: self.shared_status,
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
            SpmmStatusEnum::PushSignal(rid, _) => Effect::Push(*rid),
            SpmmStatusEnum::PushReadyQueueId(rid, _) => Effect::Push(*rid),
            SpmmStatusEnum::PushFullPartialTask(rid, _) => Effect::Push(*rid),
            SpmmStatusEnum::PushBufferPopSignal(rid) => Effect::Push(*rid),
            SpmmStatusEnum::PushMergerFinishedSignal(rid) => Effect::Push(*rid),
        }
    }

    fn set_effect(&mut self, _: Effect) {
        panic!("set_effect is not supported");
    }

    fn should_log(&self) -> bool {
        self.enable_log
    }
}
#[allow(dead_code)]
struct SimulationReport {}
#[allow(dead_code)]
struct SimulationErr {}
#[allow(dead_code)]
enum SimulationResult {
    Ok(SimulationReport),
    Err(SimulationErr),
}
impl Ord for PartialSignalType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.task_id.cmp(&other.task_id)
    }
}
impl PartialOrd for PartialSignalType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialSignalType {
    pub fn get_queue_id(&self) -> usize {
        self.queue_id
    }
}
