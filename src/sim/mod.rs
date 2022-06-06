pub mod bank;
pub mod channel_merger;
pub mod chip_merger;
pub mod component;
pub mod dimm_merger;
pub mod final_receiver;
pub mod merger_task_dispather;
pub mod merger_task_sender;
pub mod merger_task_worker;
pub mod task_reorderer;
pub mod task_router;
pub mod task_sender;

use desim::{
    prelude::*,
    resources::{CopyDefault, SimpleResource, Store},
};
use enum_as_inner::EnumAsInner;
use itertools::Itertools;
use std::{cell::RefCell, collections::BTreeMap, ops::Generator, rc::Rc};

use crate::{csv_nodata::CsVecNodata, settings::MemSettings, two_matrix::TwoMatrix};

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
    task_sender::TaskSender,
};
#[derive(Debug, Clone, Default)]
pub struct BankTask {
    pub from: usize,
    pub to: usize,
    pub row: CsVecNodata<usize>,
    pub bank_id: BankID,
}

pub type ChannelID = usize;
pub type ChipID = (ChannelID, usize);

pub type BankID = (ChipID, usize);
pub type PeID = (BankID, usize);

pub fn channel_id_from_chip_id(chip_id: &ChipID) -> &ChannelID {
    &chip_id.0
}

pub fn chip_id_from_bank_id(bank_id: &BankID) -> &ChipID {
    &bank_id.0
}

pub fn channel_id_from_bank_id(bank_id: &BankID) -> &ChannelID {
    channel_id_from_chip_id(chip_id_from_bank_id(bank_id))
}

pub fn bank_id_from_pe_id(pe_id: &PeID) -> &BankID {
    &pe_id.0
}

pub fn chip_id_from_pe_id(pe_id: &PeID) -> &ChipID {
    chip_id_from_bank_id(bank_id_from_pe_id(pe_id))
}

pub fn channel_id_from_pe_id(pe_id: &PeID) -> &ChannelID {
    channel_id_from_chip_id(chip_id_from_pe_id(pe_id))
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
    state: SpmmStatusEnum,

    // bank pe to task mapping:
    enable_log: bool,
    shared_merger_status: Rc<RefCell<merger_task_sender::FullMergerStatus>>,
    shared_bankpe_status: Rc<RefCell<BTreeMap<PeID, usize>>>,
}
impl CopyDefault for SpmmStatus {
    fn copy_default(&self) -> Self {
        let enable_log = self.enable_log;

        let shared_merger_status = self.shared_merger_status.clone();
        let shared_bankpe_status = self.shared_bankpe_status.clone();
        Self {
            state: SpmmStatusEnum::Continue,
            enable_log,
            shared_merger_status,
            shared_bankpe_status,
        }
    }
}

impl SpmmStatus {
    pub fn new(
        state: SpmmStatusEnum,
        shared_merger_status: Rc<RefCell<merger_task_sender::FullMergerStatus>>,
        shared_bankpe_status: Rc<RefCell<BTreeMap<PeID, usize>>>,
    ) -> Self {
        Self {
            state,
            enable_log: false,
            shared_merger_status,
            shared_bankpe_status,
        }
    }

    pub fn clone_with_state(&self, state: SpmmStatusEnum) -> Self {
        Self {
            state,
            enable_log: self.enable_log,
            shared_merger_status: self.shared_merger_status.clone(),
            shared_bankpe_status: self.shared_bankpe_status.clone(),
        }
    }

    pub fn set_state(self, state: SpmmStatusEnum) -> Self {
        Self {
            state,
            enable_log: self.enable_log,
            shared_merger_status: self.shared_merger_status,
            shared_bankpe_status: self.shared_bankpe_status,
        }
    }
    pub fn set_log(self, enable_log: bool) -> Self {
        Self {
            state: self.state,
            enable_log,
            shared_merger_status: self.shared_merger_status,
            shared_bankpe_status: self.shared_bankpe_status,
        }
    }

    pub fn new_log(
        state: SpmmStatusEnum,
        shared_merger_status: Rc<RefCell<merger_task_sender::FullMergerStatus>>,
        shared_bankpe_status: Rc<RefCell<BTreeMap<PeID, usize>>>,
    ) -> Self {
        Self {
            state,
            enable_log: true,
            shared_merger_status,
            shared_bankpe_status,
        }
    }
    pub fn state(&self) -> &SpmmStatusEnum {
        &self.state
    }
    pub fn into_inner(
        self,
    ) -> (
        bool,
        SpmmStatusEnum,
        Rc<RefCell<merger_task_sender::FullMergerStatus>>,
        Rc<RefCell<BTreeMap<PeID, usize>>>,
    ) {
        (
            self.enable_log,
            self.state,
            self.shared_merger_status,
            self.shared_bankpe_status,
        )
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
    pub fn run(mem_settings: &MemSettings, tow_matrix: &TwoMatrix<usize, usize>) {
        // the basic data
        let mut sim = Simulation::new();
        let merger_status = Rc::new(RefCell::new(FullMergerStatus::new()));
        let bankpe_status = Rc::new(RefCell::new(BTreeMap::new()));
        let status = SpmmStatus::new(
            SpmmStatusEnum::Continue,
            merger_status.clone(),
            bankpe_status.clone(),
        );

        // 1. add the task sender and final reciever

        let final_receiver_resouce = sim.create_resource(Box::new(Store::new(1)));
        let final_rev = FinalReceiver {
            receiver: final_receiver_resouce,
        };
        let id = sim.create_process(final_rev.run());
        sim.schedule_event(0., id, status.clone());

        // this store connect the task sender and the Dimm
        let task_send_store = sim.create_resource(Box::new(Store::new(1)));
        let task_sender = TaskSender::new(&tow_matrix.a, task_send_store);
        let id = sim.create_process(task_sender.run());
        sim.schedule_event(0., id, status.clone());

        // 2. add the Dimm
        let num_channels = mem_settings.channels;
        let channel_stores = (0..num_channels)
            .map(|_i| sim.create_resource(Box::new(Store::new(1))))
            .collect_vec();
        let merger_status_id = merger_status.borrow_mut().create_merger_status(10);
        let merger_resouce_id = sim.create_resource(Box::new(SimpleResource::new(10)));
        let dimm = DimmMerger::new(
            task_send_store,
            channel_stores.clone(),
            merger_resouce_id,
            10,
            10,
            merger_status_id,
        );
        let id = sim.create_process(dimm.run());
        sim.schedule_event(0., id, status.clone());

        // create the merger_task_worker
        let mut task_receiver = vec![];
        for _i in 0..mem_settings.dimm_merger_count {
            let resouce = sim.create_resource(Box::new(Store::new(1)));
            let merger_task_worker = MergerWorker {
                merger_size: mem_settings.dimm_merger_size,
                merger_status_id,
                merger_work_resource: merger_resouce_id,
                partial_sum_sender: final_receiver_resouce,
                task_reciever: resouce,
                task_sender_input_id: task_send_store,
            };
            let id = sim.create_process(merger_task_worker.run());
            sim.schedule_event(0., id, status.clone());
            task_receiver.push(resouce);
        }
        // create the dimm merger_task_dispatcher
        let dimm_merger_worker_task_in = sim.create_resource(Box::new(Store::new(1)));
        let merger_task_dispatcher = MergerWorkerDispatcher {
            merger_status_id,
            merger_task_sender: task_receiver,
            partial_sum_task_in: dimm_merger_worker_task_in,
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
                    .map(|_i| sim.create_resource(Box::new(Store::new(1))))
                    .collect_vec();
                let merger_status_id = merger_status.borrow_mut().create_merger_status(10);
                let merger_resouce_id = sim.create_resource(Box::new(SimpleResource::new(10)));

                let channel = ChannelMerger::new(
                    store_id,
                    chip_stores.clone(),
                    merger_resouce_id,
                    10,
                    10,
                    merger_status_id,
                );

                // create the process
                let id = sim.create_process(channel.run());
                sim.schedule_event(0., id, status.clone());

                // create the merger_task_worker
                let mut task_receiver = vec![];
                for _i in 0..mem_settings.channel_merger_count {
                    let resouce = sim.create_resource(Box::new(Store::new(1)));
                    let merger_task_worker = MergerWorker {
                        merger_size: mem_settings.channel_merger_size,
                        merger_status_id,
                        merger_work_resource: merger_resouce_id,
                        partial_sum_sender: dimm_merger_worker_task_in,
                        task_reciever: resouce,
                        task_sender_input_id: store_id,
                    };
                    let id = sim.create_process(merger_task_worker.run());
                    sim.schedule_event(0., id, status.clone());
                    task_receiver.push(resouce);
                }
                // create the channel merger_task_dispatcher
                let channel_merger_worker_task_in = sim.create_resource(Box::new(Store::new(1)));
                let merger_task_dispatcher = MergerWorkerDispatcher {
                    merger_status_id,
                    merger_task_sender: task_receiver,
                    partial_sum_task_in: channel_merger_worker_task_in,
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
                            .map(|_i| sim.create_resource(Box::new(Store::new(1))))
                            .collect_vec();
                        let merger_resouce_id =
                            sim.create_resource(Box::new(SimpleResource::new(10)));
                        let merger_status_id = merger_status.borrow_mut().create_merger_status(10);
                        let chip = ChipMerger::new(
                            store_id,
                            bank_stores.clone(),
                            merger_resouce_id,
                            10,
                            10,
                            merger_status_id,
                        );

                        // create the process
                        let id = sim.create_process(chip.run());
                        sim.schedule_event(0., id, status.clone());

                        // create the merger_task_worker
                        let mut task_receiver = vec![];
                        for _i in 0..mem_settings.chip_merger_count {
                            let resouce = sim.create_resource(Box::new(Store::new(1)));
                            let merger_task_worker = MergerWorker {
                                merger_size: mem_settings.chip_merger_size,
                                merger_status_id,
                                merger_work_resource: merger_resouce_id,
                                partial_sum_sender: channel_merger_worker_task_in,
                                task_reciever: resouce,
                                task_sender_input_id: store_id,
                            };
                            let id = sim.create_process(merger_task_worker.run());
                            sim.schedule_event(0., id, status.clone());
                            task_receiver.push(resouce);
                        }
                        // create the chip merger_task_dispatcher
                        let chip_merger_worker_task_in =
                            sim.create_resource(Box::new(Store::new(1)));
                        let merger_task_dispatcher = MergerWorkerDispatcher {
                            merger_status_id,
                            merger_task_sender: task_receiver,
                            partial_sum_task_in: chip_merger_worker_task_in,
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

                                let bank_pe_stores = (0..10)
                                    .map(|_i| sim.create_resource(Box::new(Store::new(1))))
                                    .collect_vec();
                                let bank = BankTaskReorder::new(
                                    store_id,
                                    bank_pe_stores.clone(),
                                    10,
                                    10,
                                    bank_id,
                                );

                                // create the process
                                let id = sim.create_process(bank.run());
                                sim.schedule_event(0., id, status.clone());

                                for bank_pe_store_id in bank_pe_stores {
                                    let bank_pe = BankPe::new(
                                        bank_pe_store_id,
                                        chip_merger_worker_task_in,
                                        mem_settings.bank_merger_size,
                                        10,
                                        10,
                                    );
                                    let id = sim.create_process(bank_pe.run());
                                    sim.schedule_event(0., id, status.clone());
                                }
                            });
                    });
            });
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

        let process1 = sim.create_process(Box::new(move |init_status: SimContext<SpmmStatus>| {
            let (_time, status) = init_status.into_inner();
            let new_status = status.copy_default();
            yield new_status.copy_default();
            let ret = yield new_status.copy_default();
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
        );
        sim.schedule_event(0., process1, status.copy_default());
        sim.schedule_event(0., process2, status.copy_default());
        let sim = sim.run(EndCondition::NoEvents);
        let events = sim.processed_events();
        for i in events {
            println!("{:?}", i);
        }
    }
}
