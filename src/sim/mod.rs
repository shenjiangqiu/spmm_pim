pub mod bank;
pub mod channel_merger;
pub mod chip_merger;
pub mod component;
pub mod dimm_merger;
pub mod final_receiver;
pub mod id_translation;
pub mod merger_task_dispather;
pub mod merger_task_sender;
pub mod merger_task_worker;
pub mod sim_time;
pub mod task_reorderer;
pub mod task_router;
pub mod task_sender;
use desim::{
    prelude::*,
    resources::{CopyDefault, SimpleResource, Store},
};
use enum_as_inner::EnumAsInner;
use hdrhistogram::Histogram;
use id_translation::*;
use itertools::Itertools;
use log::{debug, info};
use sprs::CsMat;
use std::{cell::RefCell, collections::BTreeMap, ops::Generator, path::Path, rc::Rc};

use crate::{csv_nodata::CsVecNodata, settings::MemSettings, two_matrix::TwoMatrix};
const STORE_SIZE: usize = 4;
use self::{
    bank::{BankPe, BankTaskReorder},
    channel_merger::ChannelMerger,
    chip_merger::ChipMerger,
    component::Component,
    dimm_merger::DimmMerger,
    final_receiver::FinalReceiver,
    merger_task_dispather::MergerWorkerDispatcher,
    merger_task_sender::FullMergerStatus,
    merger_task_worker::MergerWorker,
    sim_time::{ComponentTime, LevelTime, SharedSimTime},
    task_sender::TaskSender,
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
pub type BankTaskType = BankTaskEnum;
// target row, sender_id, target result
pub type PartialResultTaskType = (usize, ResourceId, CsVecNodata<usize>);
pub type BankReadRowTaskType = usize;

#[derive(Default, Debug, Clone, EnumAsInner)]
pub enum SpmmStatusEnum {
    #[default]
    Continue,
    Wait(f64),
    PushBankTask(ResourceId, BankTaskType),
    PushPartialTask(ResourceId, PartialResultTaskType),
    PushReadBankTask(ResourceId, BankReadRowTaskType),
    Acquire(ResourceId),
    Release(ResourceId),
    Pop(ResourceId),
}

#[derive(Debug, Clone)]
pub struct SpmmStatus {
    pub state: SpmmStatusEnum,

    // bank pe to task mapping:
    pub enable_log: bool,
    pub shared_merger_status: Rc<RefCell<merger_task_sender::FullMergerStatus>>,
    pub shared_bankpe_status: Rc<RefCell<BTreeMap<PeID, usize>>>,
    pub shared_sim_time: Rc<SharedSimTime>,
    pub shared_level_time: Rc<LevelTime>,
    pub shared_comp_time: Rc<ComponentTime>,
}
impl CopyDefault for SpmmStatus {
    fn copy_default(&self) -> Self {
        let enable_log = self.enable_log;

        let shared_merger_status = self.shared_merger_status.clone();
        let shared_bankpe_status = self.shared_bankpe_status.clone();
        let shared_sim_time = self.shared_sim_time.clone();
        let shared_level_time = self.shared_level_time.clone();
        let shared_comp_time = self.shared_comp_time.clone();
        Self {
            state: SpmmStatusEnum::Continue,
            enable_log,
            shared_merger_status,
            shared_bankpe_status,
            shared_sim_time,
            shared_level_time,
            shared_comp_time,
        }
    }
}

type StatusTuple = (
    bool,
    SpmmStatusEnum,
    Rc<RefCell<merger_task_sender::FullMergerStatus>>,
    Rc<RefCell<BTreeMap<PeID, usize>>>,
    Rc<LevelTime>,
    Rc<ComponentTime>,
);

impl SpmmStatus {
    pub fn new(
        state: SpmmStatusEnum,
        shared_merger_status: Rc<RefCell<merger_task_sender::FullMergerStatus>>,
        shared_bankpe_status: Rc<RefCell<BTreeMap<PeID, usize>>>,
        shared_sim_time: Rc<SharedSimTime>,
        shared_level_time: Rc<LevelTime>,
        shared_comp_time: Rc<ComponentTime>,
    ) -> Self {
        Self {
            state,
            enable_log: false,
            shared_merger_status,
            shared_bankpe_status,
            shared_sim_time,
            shared_level_time,
            shared_comp_time,
        }
    }

    pub fn clone_with_state(&self, state: SpmmStatusEnum) -> Self {
        Self {
            state,
            enable_log: self.enable_log,
            shared_merger_status: self.shared_merger_status.clone(),
            shared_bankpe_status: self.shared_bankpe_status.clone(),
            shared_sim_time: self.shared_sim_time.clone(),
            shared_level_time: self.shared_level_time.clone(),
            shared_comp_time: self.shared_comp_time.clone(),
        }
    }

    pub fn set_state(self, state: SpmmStatusEnum) -> Self {
        Self { state, ..self }
    }
    pub fn set_log(self, enable_log: bool) -> Self {
        Self { enable_log, ..self }
    }

    pub fn new_log(
        state: SpmmStatusEnum,
        shared_merger_status: Rc<RefCell<merger_task_sender::FullMergerStatus>>,
        shared_bankpe_status: Rc<RefCell<BTreeMap<PeID, usize>>>,
        shared_sim_time: Rc<SharedSimTime>,
        shared_level_time: Rc<LevelTime>,
        shared_comp_time: Rc<ComponentTime>,
    ) -> Self {
        Self {
            state,
            enable_log: true,
            shared_merger_status,
            shared_bankpe_status,
            shared_sim_time,
            shared_level_time,
            shared_comp_time,
        }
    }
    pub fn state(&self) -> &SpmmStatusEnum {
        &self.state
    }
    pub fn into_inner(self) -> StatusTuple {
        (
            self.enable_log,
            self.state,
            self.shared_merger_status,
            self.shared_bankpe_status,
            self.shared_level_time,
            self.shared_comp_time,
        )
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
            SpmmStatusEnum::Acquire(rid) => Effect::Request(*rid),
            SpmmStatusEnum::Release(rid) => Effect::Release(*rid),
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

pub struct Simulator {}
impl Simulator {
    pub fn run(mem_settings: &MemSettings, input_matrix: TwoMatrix<i32, i32>) {
        // the statistics
        let mut comp_ids = BTreeMap::new();
        let mut level_ids = BTreeMap::new();

        // 1.---- the basic data
        debug!("start to run");
        let mut sim = Simulation::new();
        let merger_status = Rc::new(RefCell::new(FullMergerStatus::new()));
        let bankpe_status = Rc::new(RefCell::new(BTreeMap::new()));
        let shared_level_time = Rc::new(LevelTime::new());
        let shared_comp_time = Rc::new(ComponentTime::new());

        let sim_time = Rc::new(SharedSimTime::new());
        let status = SpmmStatus::new(
            SpmmStatusEnum::Continue,
            merger_status.clone(),
            bankpe_status,
            sim_time,
            shared_level_time.clone(),
            shared_comp_time.clone(),
        );

        let final_receiver_resouce = sim.create_resource(Box::new(Store::new(STORE_SIZE)));
        let final_rev = FinalReceiver {
            receiver: final_receiver_resouce,
        };
        let id = sim.create_process(final_rev.run());
        sim.schedule_event(0., id, status.clone());

        // this store connect the task sender and the Dimm
        let task_send_store = sim.create_resource(Box::new(Store::new(STORE_SIZE)));
        let task_sender = TaskSender::new(
            input_matrix.a,
            input_matrix.b,
            task_send_store,
            mem_settings.channels,
            mem_settings.chips,
            mem_settings.banks,
            mem_settings.row_mapping.clone(),
        );
        let id = sim.create_process(task_sender.run());
        sim.schedule_event(0., id, status.clone());

        // 2. add the Dimm
        let num_channels = mem_settings.channels;
        let channel_stores = (0..num_channels)
            .map(|_i| sim.create_resource(Box::new(Store::new(STORE_SIZE))))
            .collect_vec();
        let merger_status_id = merger_status
            .borrow_mut()
            .create_merger_status(mem_settings.dimm_merger_count);
        let merger_resouce_id = sim.create_resource(Box::new(SimpleResource::new(
            mem_settings.dimm_merger_count,
        )));
        let dimm_id = shared_level_time.add_level();
        level_ids.insert(PureLevelId::Dimm, dimm_id);
        let dimm = DimmMerger::new(
            task_send_store,
            channel_stores.clone(),
            merger_resouce_id,
            merger_status_id,
            dimm_id,
        );
        let id = sim.create_process(dimm.run());
        sim.schedule_event(0., id, status.clone());

        // create the merger_task_worker
        let mut task_receiver = vec![];
        for i in 0..mem_settings.dimm_merger_count {
            let resouce = sim.create_resource(Box::new(Store::new(STORE_SIZE)));
            let comp_id = shared_comp_time.add_component(format!("dimm-{i}"));
            comp_ids.insert(
                MergerId {
                    level_id: LevelId::Dimm,
                    id: i,
                },
                comp_id,
            );
            let merger_task_worker = MergerWorker {
                merger_size: mem_settings.dimm_merger_size,
                merger_status_id,
                merger_work_resource: merger_resouce_id,
                partial_sum_sender: final_receiver_resouce,
                task_reciever: resouce,
                task_sender_input_id: task_send_store,
                self_level_id: dimm_id,
                comp_id,
            };
            let id = sim.create_process(merger_task_worker.run());
            sim.schedule_event(0., id, status.clone());
            task_receiver.push(resouce);
        }

        // create the dimm merger_task_dispatcher
        let dimm_merger_worker_task_in = sim.create_resource(Box::new(Store::new(STORE_SIZE)));
        let merger_task_dispatcher = MergerWorkerDispatcher {
            merger_status_id,
            merger_task_sender: task_receiver,
            partial_sum_task_in: dimm_merger_worker_task_in,
            self_level_id: dimm_id,
        };

        let id = sim.create_process(merger_task_dispatcher.run());
        sim.schedule_event(0., id, status.clone());

        // 3. add the Channel
        channel_stores
            .into_iter()
            .enumerate()
            .for_each(|(channel_id, store_id)| {
                // create the channel!
                let num_chips = mem_settings.chips;
                let chip_stores = (0..num_chips)
                    .map(|_i| sim.create_resource(Box::new(Store::new(STORE_SIZE))))
                    .collect_vec();
                let merger_status_id = merger_status
                    .borrow_mut()
                    .create_merger_status(mem_settings.channel_merger_count);
                let merger_resouce_id = sim.create_resource(Box::new(SimpleResource::new(
                    mem_settings.channel_merger_count,
                )));
                let level_time_id = shared_level_time.add_level();
                level_ids.insert(PureLevelId::Channel, channel_id);
                let channel = ChannelMerger::new(
                    store_id,
                    chip_stores.clone(),
                    merger_resouce_id,
                    merger_status_id,
                    level_time_id,
                );

                // create the process
                let id = sim.create_process(channel.run());
                sim.schedule_event(0., id, status.clone());

                // create the merger_task_worker
                let mut task_receiver = vec![];
                for i in 0..mem_settings.channel_merger_count {
                    let resouce = sim.create_resource(Box::new(Store::new(STORE_SIZE)));
                    let comp_id = shared_comp_time.add_component("123");
                    comp_ids.insert(
                        MergerId {
                            level_id: LevelId::Channel(channel_id),
                            id: i,
                        },
                        comp_id,
                    );
                    let merger_task_worker = MergerWorker {
                        merger_size: mem_settings.channel_merger_size,
                        merger_status_id,
                        merger_work_resource: merger_resouce_id,
                        partial_sum_sender: dimm_merger_worker_task_in,
                        task_reciever: resouce,
                        task_sender_input_id: store_id,
                        self_level_id: channel_id,
                        comp_id,
                    };
                    let id = sim.create_process(merger_task_worker.run());
                    sim.schedule_event(0., id, status.clone());
                    task_receiver.push(resouce);
                }
                // create the channel merger_task_dispatcher
                let channel_merger_worker_task_in =
                    sim.create_resource(Box::new(Store::new(STORE_SIZE)));
                let merger_task_dispatcher = MergerWorkerDispatcher {
                    merger_status_id,
                    merger_task_sender: task_receiver,
                    partial_sum_task_in: channel_merger_worker_task_in,
                    self_level_id: channel_id,
                };
                let id = sim.create_process(merger_task_dispatcher.run());
                sim.schedule_event(0., id, status.clone());

                // 4. add the chip
                chip_stores
                    .into_iter()
                    .enumerate()
                    .for_each(|(chip_id, store_id)| {
                        // create the chip!
                        let chip_id = (channel_id, chip_id);
                        let num_banks = mem_settings.banks;
                        let bank_stores = (0..num_banks)
                            .map(|_i| sim.create_resource(Box::new(Store::new(STORE_SIZE))))
                            .collect_vec();
                        let merger_resouce_id = sim.create_resource(Box::new(SimpleResource::new(
                            mem_settings.chip_merger_count,
                        )));
                        let merger_status_id = merger_status
                            .borrow_mut()
                            .create_merger_status(mem_settings.chip_merger_count);
                        let chip_level_id = shared_level_time.add_level();
                        level_ids.insert(PureLevelId::Chip, chip_level_id);
                        let chip = ChipMerger::new(
                            store_id,
                            bank_stores.clone(),
                            merger_resouce_id,
                            merger_status_id,
                            chip_level_id,
                        );

                        // create the process
                        let id = sim.create_process(chip.run());
                        sim.schedule_event(0., id, status.clone());

                        // create the merger_task_worker
                        let mut task_receiver = vec![];
                        for _i in 0..mem_settings.chip_merger_count {
                            let resouce = sim.create_resource(Box::new(Store::new(STORE_SIZE)));
                            let comp_id = shared_comp_time.add_component("123");
                            comp_ids.insert(
                                MergerId {
                                    level_id: LevelId::Chip(chip_id),
                                    id: _i,
                                },
                                comp_id,
                            );
                            let merger_task_worker = MergerWorker {
                                merger_size: mem_settings.chip_merger_size,
                                merger_status_id,
                                merger_work_resource: merger_resouce_id,
                                partial_sum_sender: channel_merger_worker_task_in,
                                task_reciever: resouce,
                                task_sender_input_id: store_id,
                                self_level_id: chip_level_id,
                                comp_id,
                            };
                            let id = sim.create_process(merger_task_worker.run());
                            sim.schedule_event(0., id, status.clone());
                            task_receiver.push(resouce);
                        }
                        // create the chip merger_task_dispatcher
                        let chip_merger_worker_task_in =
                            sim.create_resource(Box::new(Store::new(STORE_SIZE)));
                        let merger_task_dispatcher = MergerWorkerDispatcher {
                            merger_status_id,
                            merger_task_sender: task_receiver,
                            partial_sum_task_in: chip_merger_worker_task_in,
                            self_level_id: chip_level_id,
                        };
                        let id = sim.create_process(merger_task_dispatcher.run());
                        sim.schedule_event(0., id, status.clone());

                        // 5. add the bank
                        bank_stores
                            .into_iter()
                            .enumerate()
                            .for_each(|(bank_id, store_id)| {
                                // create the bank!
                                let bank_id = (chip_id, bank_id);

                                let bank_pe_stores = (0..mem_settings.bank_merger_count)
                                    .map(|_i| sim.create_resource(Box::new(Store::new(STORE_SIZE))))
                                    .collect_vec();

                                let comp_id = shared_comp_time.add_component("1123");
                                let bank = BankTaskReorder::new(
                                    store_id,
                                    bank_pe_stores.clone(),
                                    mem_settings.reorder_count,
                                    bank_id,
                                    mem_settings.row_change_latency as f64,
                                    comp_id,
                                );

                                // create the process
                                let id = sim.create_process(bank.run());
                                sim.schedule_event(0., id, status.clone());

                                for bank_pe_store_id in bank_pe_stores {
                                    let comp_id = shared_comp_time.add_component("33");
                                    comp_ids.insert(
                                        MergerId {
                                            level_id: LevelId::Bank(bank_id),
                                            id: bank_pe_store_id,
                                        },
                                        comp_id,
                                    );
                                    let bank_pe = BankPe::new(
                                        bank_pe_store_id,
                                        chip_merger_worker_task_in,
                                        mem_settings.bank_merger_size,
                                        mem_settings.bank_adder_size,
                                        store_id,
                                        comp_id,
                                    );
                                    let id = sim.create_process(bank_pe.run());
                                    sim.schedule_event(0., id, status.clone());
                                }
                            });
                    });
            });
        // start
        sim.run(EndCondition::NoEvents);
        // finished, print the result
        // the result is the comp_time and level_time

        // build the histogram
        let histograms_idle = unsafe { &*shared_comp_time.componet_idle_time.get() }
            .iter()
            .map(|(_x, v)| {
                v.iter()
                    .fold(Histogram::<u64>::new(3).unwrap(), |mut acc, item| {
                        acc += *item as u64;
                        acc
                    })
            })
            .collect_vec();
        let histogram_level_finished = unsafe { &*shared_level_time.level_finished_time.get() }
            .iter()
            .map(|x| {
                x.iter()
                    .map(|x| x.0)
                    .fold(Histogram::<u64>::new(3).unwrap(), |mut acc, item| {
                        acc += item as u64;
                        acc
                    })
            })
            .collect_vec();
        let histogram_work_gap = unsafe { &*shared_level_time.level_finished_time.get() }
            .iter()
            .map(|x| {
                x.iter()
                    .map(|x| x.1)
                    .fold(Histogram::<u64>::new(3).unwrap(), |mut acc, item| {
                        acc += item as u64;
                        acc
                    })
            })
            .collect_vec();
        // show the result
        for (k, v) in comp_ids {
            info!("comp_ids:{:?}:{:?}", k, v);
            let histogram = histograms_idle.get(v).unwrap();
            info!(
                "comp_idle_time:mean: {:?} max: {} min: {}, 25qt: {}, 50qt: {}, 75qt: {} ",
                histogram.mean(),
                histogram.max(),
                histogram.min(),
                histogram.value_at_quantile(0.25),
                histogram.value_at_quantile(0.5),
                histogram.value_at_quantile(0.75)
            );
        }

        for (k, v) in level_ids {
            info!("level_ids:{:?}:{:?}", k, v);
            let histogram = histogram_level_finished.get(v).unwrap();
            info!(
                "level_finished_time:mean: {:?} max: {} min: {}, 25qt: {}, 50qt: {}, 75qt: {} ",
                histogram.mean(),
                histogram.max(),
                histogram.min(),
                histogram.value_at_quantile(0.25),
                histogram.value_at_quantile(0.5),
                histogram.value_at_quantile(0.75)
            );
            let histogram = histogram_work_gap.get(v).unwrap();
            info!(
                "level_work_gap:mean: {:?} max: {} min: {}, 25qt: {}, 50qt: {}, 75qt: {} ",
                histogram.mean(),
                histogram.max(),
                histogram.min(),
                histogram.value_at_quantile(0.25),
                histogram.value_at_quantile(0.5),
                histogram.value_at_quantile(0.75)
            );
        }
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
        let queue1 = sim.create_resource(Box::new(Store::new(16)));
        let queue2 = sim.create_resource(Box::new(Store::new(12)));

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

        let status = SpmmStatus::new(
            SpmmStatusEnum::Continue,
            Rc::new(RefCell::new(merger_task_sender::FullMergerStatus::new())),
            Rc::new(RefCell::new(BTreeMap::new())),
            Rc::new(SharedSimTime::new()),
            Rc::new(LevelTime::new()),
            Rc::new(ComponentTime::new()),
        );
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
        let config_str = include_str!("../../log_config.yml");
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
        };
        Simulator::run(&mem_settings, two_matrix);
    }
}
