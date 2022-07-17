pub mod bank;
pub mod buffer_status;
pub mod channel_merger;
pub mod chip_merger;
mod comp_collector;
pub mod component;
pub mod dimm_merger;
pub mod final_receiver;
mod full_result_merger_worker;
pub mod id_translation;
mod merger_status;
pub mod merger_task_dispather;
pub mod merger_task_sender;
pub mod merger_task_worker;
pub mod partial_sum_collector;
pub mod partial_sum_sender;
pub mod partial_sum_signal_collector;
pub mod sim_time;
pub mod task_reorderer;
pub mod task_router;
pub mod task_sender;
use desim::{
    prelude::*,
    resources::{CopyDefault, Store},
};
use enum_as_inner::EnumAsInner;
use id_translation::*;
use itertools::Itertools;
use log::{debug, info};
use sprs::CsMat;
use std::{cell::RefCell, collections::BTreeMap, ops::Generator, path::Path, rc::Rc};

use self::{
    bank::{BankPe, BankTaskReorder},
    buffer_status::SharedBufferStatus,
    channel_merger::ChannelMerger,
    chip_merger::ChipMerger,
    component::Component,
    dimm_merger::DimmMerger,
    final_receiver::FinalReceiver,
    full_result_merger_worker::FullResultMergerWorker,
    merger_status::SharedMergerStatus,
    merger_task_dispather::MergerWorkerDispatcher,
    partial_sum_collector::PartialSumCollector,
    partial_sum_sender::PartialSumSender,
    partial_sum_signal_collector::PartialSumSignalCollector,
    sim_time::{LevelTime, LevelTimeId, SharedNamedTime, SharedSimTime},
    task_sender::TaskSender,
};
use crate::{
    csv_nodata::CsVecNodata, settings::MemSettings, sim::comp_collector::ProcessInfoCollector,
    two_matrix::TwoMatrix,
};
#[derive(Debug, Clone, Default)]
pub struct BankTask {
    pub from: usize,
    pub to: usize,
    pub row: CsVecNodata<usize>,
    pub bank_id: BankID,
    pub row_shift: usize,
    pub row_size: usize,
}
pub fn create_two_matrix_from_file(file_name: &Path) -> TwoMatrix<i32, i32> {
    let csr: CsMat<i32> = sprs::io::read_matrix_market(file_name).unwrap().to_csr();
    let trans_pose = csr.transpose_view().to_csr();
    TwoMatrix::new(csr, trans_pose)
}
pub type SpmmContex = SimContext<SpmmStatus>;
pub type SpmmGenerator = dyn Generator<SpmmContex, Yield = SpmmStatus, Return = ()> + Unpin;
//todo: add the type
#[derive(Debug, Clone, EnumAsInner, Default)]
pub enum BankTaskEnum {
    PushBankTask(BankTask),
    #[default]
    EndThisTask,
}

/// this struct contains the information of the signale that send from the partial sum sender,
/// it should contains: 1. the target id, 2. the source id
/// it will be send by the `partial_sum_sender`
#[derive(Debug, Clone, Default)]
pub struct PartialSignal {
    pub target_id: usize,
    pub self_sender_id: usize,
    pub self_queue_id: usize,
}

impl PartialSignal {
    pub fn get_queue_id(&self) -> usize {
        self.self_queue_id
    }
}

pub type BankTaskType = BankTaskEnum;
// target row, sender_id, target result
pub type PartialResultTaskType = (usize, ResourceId, CsVecNodata<usize>);
// target row, and all partial result
pub type FullTaskType = (usize, Vec<CsVecNodata<usize>>);
pub type BankReadRowTaskType = usize;

#[derive(Default, Debug, Clone, EnumAsInner)]
pub enum SpmmStatusEnum {
    #[default]
    Continue,
    Wait(f64),
    PushBankTask(ResourceId, BankTaskType),
    PushPartialTask(ResourceId, PartialResultTaskType),
    PushReadBankTask(ResourceId, BankReadRowTaskType),
    PushSignal(ResourceId, PartialSignal),
    /// (queue_id, is_last)
    PushReadyQueueId(ResourceId, (usize, bool)),
    PushFullPartialTask(ResourceId, FullTaskType),
    PushBufferPopSignal(ResourceId),
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
}
pub struct StateWithSharedStatus {
    pub status: SpmmStatusEnum,
    pub shared_status: SharedStatus,
}

#[derive(Debug, Clone)]
pub struct SpmmStatus {
    pub state: SpmmStatusEnum,

    // bank pe to task mapping:
    pub enable_log: bool,
    pub shared_status: SharedStatus,
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
#[derive(Debug, Clone, EnumAsInner, PartialEq, Eq, PartialOrd, Ord)]
pub enum LevelId {
    Dimm,
    Channel(ChannelID),
    Bank(BankID),
    Chip(ChipID),
}

#[derive(Debug, Clone, EnumAsInner, PartialEq, PartialOrd, Eq, Ord)]
pub enum PureLevelId {
    Dimm,
    Channel,
    Bank,
    Chip,
}
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct MergerId {
    level_id: LevelId,
    id: usize,
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
            SpmmStatusEnum::PushSignal(rid, _) => Effect::Push(*rid),
            SpmmStatusEnum::PushReadyQueueId(rid, _) => Effect::Push(*rid),
            SpmmStatusEnum::PushFullPartialTask(rid, _) => Effect::Push(*rid),
            SpmmStatusEnum::PushBufferPopSignal(rid) => Effect::Push(*rid),
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
/// # safety:
/// the comp_ids should all be valid!
// unsafe fn calculate_raition_rate(
//     end_time: f64,
//     comp_ids: &[usize],
//     shared_comp_time: &ComponentTime,
// ) -> f64 {
//     let total_time = end_time * comp_ids.len() as f64;
//     let idle_time = comp_ids
//         .iter()
//         .map(|comp_id| {
//             shared_comp_time
//                 .get_idle_time(*comp_id)
//                 .1
//                 .iter()
//                 .sum::<f64>()
//         })
//         .sum::<f64>();
//     idle_time / total_time
// }

fn build_dimm(
    mem_settings: &MemSettings,
    store_size: usize,
    sim: &mut Simulation<SpmmStatus>,
    task_send_store: usize,
    status: SpmmStatus,
    final_data_receiver: usize,
    _dimm_level_id: LevelTimeId,
    channel_level_id: LevelTimeId,
    chip_level_id: LevelTimeId,
    bank_level_id: LevelTimeId,
    p_collector: &mut ProcessInfoCollector,
) -> eyre::Result<()> {
    let shared_status = status.shared_status.clone();
    // 2. add the Dimm
    let num_channels = mem_settings.channels;
    let channel_stores = (0..num_channels)
        .map(|_i| sim.create_resource(Box::new(Store::new(store_size)), "dimm_to_channel"))
        .collect_vec();
    let merger_status_id = shared_status
        .shared_merger_status
        .add_component(mem_settings.dimm_merger_count);

    let sim_time_id = shared_status
        .shared_named_time
        .add_component_with_name("DIMMSENDER_GETID");
    let buffer_status_id = shared_status
        .shared_buffer_status
        .add_component(mem_settings.buffer_lines);

    let signal_in = sim.create_resource(Box::new(Store::new(128)), "signal_dimm");
    let ready_id_queue = sim.create_resource(Box::new(Store::new(128)), "ready_dimm");
    let dimm_signal_collector = PartialSumSignalCollector {
        queue_id_signal_in: signal_in,
        queue_id_ready_out: ready_id_queue,
        buffer_status_id,
        level_id: LevelId::Dimm,
    };
    p_collector.create_process_and_schedule(sim, dimm_signal_collector, &status);
    let collector_to_dispatcher = sim.create_resource(
        Box::new(Store::new(store_size)),
        "collector_to_dispatcher_dimm",
    );

    let dimm_partial_sum_data_collector = PartialSumCollector {
        queue_id_ready_in: ready_id_queue,
        queue_id_full_result_out: collector_to_dispatcher,
        queue_id_pop_signal_out: signal_in,
        level_id: LevelId::Dimm,
        buffer_status_id,
    };
    p_collector.create_process_and_schedule(sim, dimm_partial_sum_data_collector, &status);
    let dimm = DimmMerger::new(
        LevelId::Dimm,
        task_send_store,
        channel_stores.clone(),
        merger_status_id,
        sim_time_id,
        buffer_status_id,
    );

    p_collector.create_process_and_schedule(sim, dimm, &status);
    // create the merger_task_worker
    let mut task_receiver = vec![];
    for i in 0..mem_settings.dimm_merger_count {
        let full_partial_sum_in = sim.create_resource(
            Box::new(Store::new(store_size)),
            "dispatcher_to_merger_dimm",
        );

        let merger_task_worker = FullResultMergerWorker {
            level_id: LevelId::Dimm,
            id: i,
            queue_id_partial_sum_sender: final_data_receiver,
            queue_id_partial_sum_in: full_partial_sum_in,
            self_sender_id: task_send_store,
            merger_status_id,
            merger_width: mem_settings.dimm_merger_size,
        };
        p_collector.create_process_and_schedule(sim, merger_task_worker, &status);
        task_receiver.push(full_partial_sum_in);
    }

    // create the dimm merger_task_dispatcher
    let merger_task_dispatcher = MergerWorkerDispatcher {
        level_id: LevelId::Dimm,
        merger_status_id,
        merger_task_sender: task_receiver,
        full_sum_in: collector_to_dispatcher,
    };

    p_collector.create_process_and_schedule(sim, merger_task_dispatcher, &status);
    build_channel(
        mem_settings,
        sim,
        status,
        store_size,
        channel_level_id,
        chip_level_id,
        bank_level_id,
        channel_stores,
        signal_in,
        p_collector,
    )?;
    Ok(())
}

fn build_channel(
    mem_settings: &MemSettings,
    sim: &mut Simulation<SpmmStatus>,
    status: SpmmStatus,
    store_size: usize,
    channel_level_id: LevelTimeId,
    chip_level_id: LevelTimeId,
    bank_level_id: LevelTimeId,
    channel_task_senders: Vec<usize>,
    dimm_signal_in: usize,
    p_collector: &mut ProcessInfoCollector,
) -> eyre::Result<()> {
    let shared_status = status.shared_status.clone();

    // 3. add the Channel
    for (channel_id, dimm_to_channel_task_sender) in channel_task_senders.into_iter().enumerate() {
        // create the channel!
        let num_chips = mem_settings.chips;

        // the channel that send task to the chip from this channel
        let chip_stores = (0..num_chips)
            .map(|_i| sim.create_resource(Box::new(Store::new(store_size)), "channel_to_chip"))
            .collect_vec();

        let merger_status_id = shared_status
            .shared_merger_status
            .add_component(mem_settings.channel_merger_count);

        let sim_time = shared_status
            .shared_named_time
            .add_component_with_name("channel_sender");
        let buffer_status_id = shared_status
            .shared_buffer_status
            .add_component(mem_settings.buffer_lines);

        let signal_in = sim.create_resource(Box::new(Store::new(128)), "signal_channel");
        let ready_queueid = sim.create_resource(Box::new(Store::new(128)), "ready_channel");
        let channel_signal_collector = PartialSumSignalCollector {
            queue_id_signal_in: signal_in,
            queue_id_ready_out: ready_queueid,
            buffer_status_id,
            level_id: LevelId::Channel(channel_id),
        };

        p_collector.create_process_and_schedule(sim, channel_signal_collector, &status);

        let collector_to_dispatcher =
            sim.create_resource(Box::new(Store::new(1)), "collector_to_dispatcher_channel");
        let channel_partial_sum_data_collector = PartialSumCollector {
            queue_id_ready_in: ready_queueid,
            queue_id_full_result_out: collector_to_dispatcher,
            queue_id_pop_signal_out: signal_in,
            level_id: LevelId::Channel(channel_id),
            buffer_status_id,
        };

        p_collector.create_process_and_schedule(sim, channel_partial_sum_data_collector, &status);

        let channel = ChannelMerger::new(
            LevelId::Channel(channel_id),
            dimm_to_channel_task_sender,
            chip_stores.clone(),
            merger_status_id,
            channel_level_id,
            sim_time,
            buffer_status_id,
        );

        // create the process
        p_collector.create_process_and_schedule(sim, channel, &status);
        // create the merger_task_worker
        let mut task_receiver = vec![];
        for i in 0..mem_settings.channel_merger_count {
            let merger_to_sender_queue =
                sim.create_resource(Box::new(Store::new(1)), "merger_to_sender_channel");
            let channel_signal_sender = PartialSumSender {
                queue_id_partial_sum_in: merger_to_sender_queue,
                queue_id_partial_sum_out: sim
                    .create_resource(Box::new(Store::new(1)), "signal_data_sender_channel"),
                queue_id_signal_out: dimm_signal_in,
                level_id: LevelId::Channel(channel_id),
            };
            p_collector.create_process_and_schedule(sim, channel_signal_sender, &status);

            let resouce = sim.create_resource(
                Box::new(Store::new(store_size)),
                "dispatcher_to_merger_channel",
            );

            let merger_task_worker = FullResultMergerWorker {
                level_id: LevelId::Channel(channel_id),
                id: i,
                queue_id_partial_sum_sender: merger_to_sender_queue,
                queue_id_partial_sum_in: resouce,
                self_sender_id: dimm_to_channel_task_sender,
                merger_status_id,
                merger_width: mem_settings.channel_merger_size,
            };
            p_collector.create_process_and_schedule(sim, merger_task_worker, &status);
            task_receiver.push(resouce);
        }
        // create the channel merger_task_dispatcher
        let merger_task_dispatcher = MergerWorkerDispatcher {
            level_id: LevelId::Channel(channel_id),
            merger_status_id,
            merger_task_sender: task_receiver,
            full_sum_in: collector_to_dispatcher,
        };
        p_collector.create_process_and_schedule(sim, merger_task_dispatcher, &status);
        build_chip(
            mem_settings,
            sim,
            status.clone(),
            store_size,
            chip_level_id,
            bank_level_id,
            chip_stores,
            channel_id,
            signal_in,
            p_collector,
        )?;
    }
    Ok(())
}
fn build_chip(
    mem_settings: &MemSettings,
    sim: &mut Simulation<SpmmStatus>,
    status: SpmmStatus,
    store_size: usize,
    chip_level_id: LevelTimeId,
    bank_level_id: LevelTimeId,
    chip_stores: Vec<usize>,
    channel_id: ChannelID,
    channel_signal_in: usize,
    p_collector: &mut ProcessInfoCollector,
) -> eyre::Result<()> {
    let shared_status = status.shared_status.clone();
    // 4. add the chip
    for (chip_id, store_id) in chip_stores.into_iter().enumerate() {
        // create the chip!
        let chip_id = (channel_id, chip_id);
        let num_banks = mem_settings.banks;
        let bank_stores = (0..num_banks)
            .map(|_i| sim.create_resource(Box::new(Store::new(store_size)), "chip_to_bank"))
            .collect_vec();

        let merger_status_id = shared_status
            .shared_merger_status
            .add_component(mem_settings.chip_merger_count);
        let sim_time_id = shared_status
            .shared_named_time
            .add_component_with_name("chip_sender");
        let buffer_status_id = shared_status
            .shared_buffer_status
            .add_component(mem_settings.buffer_lines);

        let signal_in = sim.create_resource(Box::new(Store::new(128)), "signal_chip");
        let ready_queueid = sim.create_resource(Box::new(Store::new(128)), "ready_chip");
        let chip_signal_collector = PartialSumSignalCollector {
            queue_id_signal_in: signal_in,
            queue_id_ready_out: ready_queueid,
            buffer_status_id,
            level_id: LevelId::Chip(chip_id),
        };
        p_collector.create_process_and_schedule(sim, chip_signal_collector, &status);
        let collector_to_dispatcher =
            sim.create_resource(Box::new(Store::new(1)), "collector_to_dispatcher_chip");
        let chip_partial_sum_data_collector = PartialSumCollector {
            queue_id_ready_in: ready_queueid,
            queue_id_full_result_out: collector_to_dispatcher,
            queue_id_pop_signal_out: signal_in,
            level_id: LevelId::Chip(chip_id),
            buffer_status_id,
        };
        p_collector.create_process_and_schedule(sim, chip_partial_sum_data_collector, &status);
        let chip = ChipMerger::new(
            LevelId::Chip(chip_id),
            store_id,
            bank_stores.clone(),
            merger_status_id,
            chip_level_id,
            sim_time_id,
            buffer_status_id,
        );

        // create the process
        p_collector.create_process_and_schedule(sim, chip, &status);
        // create the merger_task_worker
        let mut task_receiver = vec![];
        for i in 0..mem_settings.chip_merger_count {
            // build partial sum sender, signal collector and data collector
            let merger_to_sender_queue =
                sim.create_resource(Box::new(Store::new(1)), "merger_to_sender_chip");
            let chip_signal_sender = PartialSumSender {
                queue_id_partial_sum_in: merger_to_sender_queue,
                queue_id_partial_sum_out: sim
                    .create_resource(Box::new(Store::new(1)), "signal_data_sender_chip"),
                queue_id_signal_out: channel_signal_in,
                level_id: LevelId::Chip(chip_id),
            };
            p_collector.create_process_and_schedule(sim, chip_signal_sender, &status);

            let resouce = sim.create_resource(
                Box::new(Store::new(store_size)),
                "dispatcher_to_merger_chip",
            );

            let merger_task_worker = FullResultMergerWorker {
                level_id: LevelId::Chip(chip_id),
                id: i,
                queue_id_partial_sum_sender: merger_to_sender_queue,
                queue_id_partial_sum_in: resouce,
                self_sender_id: store_id,
                merger_status_id,
                merger_width: mem_settings.chip_merger_size,
            };
            p_collector.create_process_and_schedule(sim, merger_task_worker, &status);
            task_receiver.push(resouce);
        }
        // create the chip merger_task_dispatcher
        let merger_task_dispatcher = MergerWorkerDispatcher {
            level_id: LevelId::Chip(chip_id),
            merger_status_id,
            merger_task_sender: task_receiver,
            full_sum_in: collector_to_dispatcher,
        };
        p_collector.create_process_and_schedule(sim, merger_task_dispatcher, &status);

        build_bank(
            mem_settings,
            sim,
            status.clone(),
            store_size,
            bank_level_id,
            bank_stores,
            chip_id,
            signal_in,
            p_collector,
        )?;
    }
    // start

    Ok(())
    // what we should to output?
}

fn build_bank(
    mem_settings: &MemSettings,
    sim: &mut Simulation<SpmmStatus>,
    status: SpmmStatus,
    store_size: usize,
    _bank_level_id: LevelTimeId,
    bank_stores: Vec<usize>,
    chip_id: ChipID,
    chip_signal_in: usize,
    p_collector: &mut ProcessInfoCollector,
) -> eyre::Result<()> {
    let shared_status = status.shared_status.clone();
    // 5. add the bank
    for (bank_id, store_id) in bank_stores.into_iter().enumerate() {
        // create the bank!
        let bank_id = (chip_id, bank_id);

        let bank_pe_stores = (0..mem_settings.bank_merger_count)
            .map(|_i| sim.create_resource(Box::new(Store::new(store_size)), "bank_to_pe"))
            .collect_vec();

        let comp_id = shared_status
            .shared_named_time
            .add_component_with_name("bank_sender");
        let bank = BankTaskReorder::new(
            LevelId::Bank(bank_id),
            store_id,
            bank_pe_stores.clone(),
            mem_settings.reorder_count,
            bank_id,
            mem_settings.row_change_latency as f64,
            comp_id,
        );

        // create the process
        p_collector.create_process_and_schedule(sim, bank, &status);

        for (bank_pe_id, bank_pe_store_id) in bank_pe_stores.into_iter().enumerate() {
            // create the partial sum sender, signal collector and data collector
            let merger_to_sender =
                sim.create_resource(Box::new(Store::new(1)), "merger_to_sender_bank");
            let bank_signal_sender = PartialSumSender {
                queue_id_partial_sum_in: merger_to_sender,
                queue_id_partial_sum_out: sim
                    .create_resource(Box::new(Store::new(1)), "signal_data_sender_bank"),
                queue_id_signal_out: chip_signal_in,
                level_id: LevelId::Bank(bank_id),
            };
            p_collector.create_process_and_schedule(sim, bank_signal_sender, &status);

            let comp_id = shared_status
                .shared_named_time
                .add_component_with_name(format!("bank_pe_{bank_id:?}_{bank_pe_id}"));

            let bank_pe = BankPe::new(
                LevelId::Bank(bank_id),
                bank_pe_id,
                bank_pe_store_id,
                merger_to_sender,
                mem_settings.bank_merger_size,
                mem_settings.bank_adder_size,
                store_id,
                comp_id,
            );
            p_collector.create_process_and_schedule(sim, bank_pe, &status);
        }
    }

    Ok(())
}

pub struct Simulator {}
impl Simulator {
    /// run the simulator
    pub fn run(
        mem_settings: &MemSettings,
        input_matrix: TwoMatrix<i32, i32>,
    ) -> Result<(), eyre::Report> {
        let store_size = mem_settings.store_size;
        // now we need a stucture to map the sim_time id to the real component time

        // the statistics
        let mut p_collector = ProcessInfoCollector::new(true);
        // 1.---- the basic data
        debug!("start to run");
        let mut sim = Simulation::new();
        // let merger_status = Rc::new(RefCell::new(FullMergerStatus::new()));
        let bankpe_status = Rc::new(RefCell::new(BTreeMap::new()));
        let shared_level_time = Rc::new(LevelTime::new());

        let dimm_level_id = shared_level_time.add_level();
        let channel_level_id = shared_level_time.add_level();
        let chip_level_id = shared_level_time.add_level();
        let bank_level_id = shared_level_time.add_level();

        let shared_named_time = Rc::new(SharedNamedTime::new());
        let shared_buffer_status = Rc::new(SharedBufferStatus::default());
        let sim_time = Rc::new(SharedSimTime::new());
        let merger_status = Default::default();
        let shared_status = SharedStatus {
            shared_bankpe_status: bankpe_status,
            shared_sim_time: sim_time,
            shared_level_time,
            shared_named_time,
            shared_buffer_status,
            shared_merger_status: merger_status,
        };
        let status = SpmmStatus::new(SpmmStatusEnum::Continue, shared_status.clone());

        let final_receiver_resouce =
            sim.create_resource(Box::new(Store::new(store_size)), "final_receiver");
        let final_rev = FinalReceiver {
            receiver: final_receiver_resouce,
            collect_result: true,
        };

        p_collector.create_process_and_schedule(&mut sim, final_rev, &status);
        // this store connect the task sender and the Dimm
        let task_send_store =
            sim.create_resource(Box::new(Store::new(store_size)), "task_send_store");
        let task_sender = TaskSender::new(
            input_matrix.a,
            input_matrix.b,
            task_send_store,
            mem_settings.channels,
            mem_settings.chips,
            mem_settings.banks,
            mem_settings.row_mapping.clone(),
        );
        p_collector.create_process_and_schedule(&mut sim, task_sender, &status);

        build_dimm(
            mem_settings,
            store_size,
            &mut sim,
            task_send_store,
            status.clone(),
            final_receiver_resouce,
            dimm_level_id,
            channel_level_id,
            chip_level_id,
            bank_level_id,
            &mut p_collector,
        )?;
        p_collector.show_data();

        let sim = sim.run(EndCondition::NoEvents);
        info!("{}", sim.print_resources());
        let time = sim.time();
        status.shared_status.shared_named_time.show_data(time);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use desim::resources::Store;
    use sprs::CsMat;

    use crate::settings::RowMapping;

    use super::*;
    #[test]
    pub fn run() {
        let mut sim = Simulation::new();
        let queue1 = sim.create_resource(Box::new(Store::new(16)), "test");
        let queue2 = sim.create_resource(Box::new(Store::new(12)), "test");

        let process1 = sim.create_process(Box::new(move |init_status: SimContext<SpmmStatus>| {
            let (_time, status) = init_status.into_inner();
            let new_status = status.copy_default();
            yield new_status
                .clone_with_state(SpmmStatusEnum::PushBankTask(queue1, Default::default()));
            let ret = yield new_status.clone_with_state(SpmmStatusEnum::Pop(queue2));
            println!(
                "ret: {:?}",
                ret.state().state().as_push_bank_task().unwrap()
            );
        }));

        let process2 = sim.create_process(Box::new(move |context: SimContext<SpmmStatus>| {
            let (_time, status) = context.into_inner();

            yield status.clone_with_state(SpmmStatusEnum::Wait(5.));
            let ret = yield status.clone_with_state(SpmmStatusEnum::Pop(queue1));
            println!(
                "ret: {:?}",
                ret.state().state().as_push_bank_task().unwrap()
            );
            yield status.clone_with_state(SpmmStatusEnum::PushBankTask(queue2, Default::default()));
        }));

        let status = SpmmStatus::new(SpmmStatusEnum::Continue, SharedStatus::default());
        sim.schedule_event(0., process1, status.copy_default());
        sim.schedule_event(0., process2, status.copy_default());
        let sim = sim.run(EndCondition::NoEvents);
        let events = sim.processed_events();
        for i in events {
            println!("{:?}", i);
        }
    }

    #[test]
    fn sim_test() {
        // ---- first create neccessary status structures
        let config_str = include_str!("../../log_config_debug.yml");
        let config = serde_yaml::from_str(config_str).unwrap();
        log4rs::init_raw_config(config).unwrap_or(());

        debug!("start");
        let csr: CsMat<i32> = sprs::io::read_matrix_market("mtx/bfwa62.mtx")
            .unwrap()
            .to_csr();
        let trans_pose = csr.transpose_view().to_csr();
        let two_matrix = TwoMatrix::new(csr, trans_pose);
        let mem_settings = MemSettings {
            row_size: 512,
            banks: 2,
            chips: 2,
            channels: 2,
            row_mapping: RowMapping::Chunk,
            bank_merger_size: 2,
            chip_merger_size: 2,
            channel_merger_size: 2,
            dimm_merger_size: 2,
            simd_width: 128,
            parallel_count: 8,
            reorder_count: 8,
            bank_merger_count: 2,
            chip_merger_count: 2,
            channel_merger_count: 2,
            dimm_merger_count: 2,
            row_change_latency: 8,
            bank_adder_size: 8,
            store_size: 1,
            buffer_lines: 10,
        };
        Simulator::run(&mem_settings, two_matrix).unwrap();
    }
}
