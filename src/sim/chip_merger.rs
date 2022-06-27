//! chip-level merger
//!
//!

use desim::ResourceId;

use super::{merger_task_sender::*, BankID};
pub struct ChipMerger {
    pub task_in: ResourceId,
    pub lower_pes: Vec<ResourceId>,
    pub merger_resouce: ResourceId,

    // settings
    pub merger_status_id: usize,
    pub self_level_time_id: usize,
}

impl ChipMerger {
    pub fn new(
        task_in: ResourceId,
        lower_pes: Vec<ResourceId>,
        merger_resouce: ResourceId,
        merger_status_id: usize,
        self_level_time_id: usize,
    ) -> Self {
        Self {
            task_in,
            lower_pes,
            merger_resouce,
            merger_status_id,
            self_level_time_id,
        }
    }
}

impl MergerTaskSender for ChipMerger {
    fn get_lower_id(&self, bank_id: &BankID) -> usize {
        self.lower_pes[bank_id.1]
    }

    fn get_task_in(&self) -> ResourceId {
        self.task_in
    }

    fn get_merger_resouce_id(&self) -> ResourceId {
        self.merger_resouce
    }
    fn get_merger_status_id(&self) -> usize {
        self.merger_status_id
    }

    fn get_lower_pes(&self) -> &[ResourceId] {
        &self.lower_pes
    }
}

#[cfg(test)]
mod tests {

    use std::{cell::RefCell, collections::BTreeMap, path::Path, rc::Rc};

    use desim::{
        resources::{CopyDefault, SimpleResource, Store},
        EndCondition, Simulation,
    };
    use itertools::Itertools;
    use log::debug;

    use crate::{
        settings::RowMapping,
        sim::{
            self,
            bank::{BankPe, BankTaskReorder},
            component::Component,
            final_receiver::FinalReceiver,
            merger_task_worker::MergerWorker,
            sim_time::{ComponentTime, LevelTime, SharedSimTime},
            task_sender::TaskSender,
            SpmmStatus, SpmmStatusEnum,
        },
    };

    use super::*;
    #[test]
    fn test_bank() {
        // ---- first create neccessary status structures
        let config_str = include_str!("../../log_config.yml");
        let config = serde_yaml::from_str(config_str).unwrap();
        log4rs::init_raw_config(config).unwrap_or(());
        let merger_status = Rc::new(RefCell::new(FullMergerStatus::new()));
        let sim_time = Rc::new(SharedSimTime::new());
        let level_time = Rc::new(LevelTime::new());
        let comp_time = Rc::new(ComponentTime::new());
        let status = SpmmStatus::new(
            SpmmStatusEnum::Continue,
            merger_status.clone(),
            Rc::new(RefCell::new(BTreeMap::new())),
            sim_time,
            level_time.clone(),
            comp_time.clone(),
        );
        debug!("start test");
        let mut simulator = Simulation::new();
        let two_mat = sim::create_two_matrix_from_file(Path::new("mtx/test.mtx"));

        // ---- create resources and status id
        let sender_to_chip = simulator.create_resource(Box::new(Store::new(16)));
        let chip_to_bank = simulator.create_resource(Box::new(Store::new(16)));
        let final_partial_return = simulator.create_resource(Box::new(Store::new(16)));
        let bank_to_chip_partial_return = simulator.create_resource(Box::new(Store::new(16)));
        let chip_merger_resouce = simulator.create_resource(Box::new(SimpleResource::new(4)));
        let chip_merger_status_id = merger_status.borrow_mut().create_merger_status(4);
        let bank_to_bank_merger = {
            let mut task_pe = vec![];
            for _i in 0..4 {
                let task_out = simulator.create_resource(Box::new(Store::new(16)));
                task_pe.push(task_out);
            }
            task_pe
        };

        // ---- create processes
        let task_sender = TaskSender::new(
            two_mat.a,
            two_mat.b,
            sender_to_chip,
            1,
            1,
            1,
            RowMapping::Chunk,
        );
        let chip_level_id = level_time.add_level();
        let chip_merger = ChipMerger::new(
            sender_to_chip,
            vec![chip_to_bank],
            chip_merger_resouce,
            chip_merger_status_id,
            chip_level_id,
        );
        let comp_id = comp_time.add_component("13");
        let chip_woker = MergerWorker {
            merger_size: 4,
            merger_status_id: chip_merger_status_id,
            merger_work_resource: chip_merger_resouce,
            partial_sum_sender: final_partial_return,
            task_reciever: bank_to_chip_partial_return,
            task_sender_input_id: sender_to_chip,
            self_level_id: chip_level_id,
            comp_id,
        };
        let comp_id = comp_time.add_component("123");
        let bank_task_reorder = BankTaskReorder::new(
            chip_to_bank,
            bank_to_bank_merger.clone(),
            4,
            ((0, 0), 0),
            33.,
            comp_id,
        );
        let bank_pes = {
            let mut pes = vec![];
            for pe_in in bank_to_bank_merger {
                let comp_id = comp_time.add_component("123");
                let pe_comp = BankPe::new(
                    pe_in,
                    bank_to_chip_partial_return,
                    4,
                    4,
                    chip_to_bank,
                    comp_id,
                );
                pes.push(pe_comp);
            }
            pes
        };
        let final_receiver = FinalReceiver {
            receiver: final_partial_return,
        };

        // ---- add processes to simulator
        let bank_task_reorder = simulator.create_process(bank_task_reorder.run());
        let task_sender = simulator.create_process(task_sender.run());
        let final_receiver_process = simulator.create_process(final_receiver.run());
        let chip_merger = simulator.create_process(chip_merger.run());
        let chip_woker = simulator.create_process(chip_woker.run());
        let bank_woker = bank_pes
            .into_iter()
            .map(|pe| simulator.create_process(pe.run()))
            .collect_vec();
        // ---- schedule processes

        vec![
            bank_task_reorder,
            task_sender,
            final_receiver_process,
            chip_merger,
            chip_woker,
        ]
        .into_iter()
        .for_each(|p| simulator.schedule_event(0., p, status.copy_default()));

        bank_woker
            .into_iter()
            .for_each(|p| simulator.schedule_event(0., p, status.copy_default()));

        // ---- run simulator
        simulator.run(EndCondition::NoEvents);
    }
    // #[test]
    // fn test_bank2() {
    //     // ---- first create neccessary status structures
    //     let config_str = include_str!("../../log_config.yml");
    //     let config = serde_yaml::from_str(config_str).unwrap();
    //     log4rs::init_raw_config(config).unwrap_or(());
    //     let merger_status = Rc::new(RefCell::new(FullMergerStatus::new()));

    //     let status = SpmmStatus::new(
    //         SpmmStatusEnum::Continue,
    //         merger_status.clone(),
    //         Rc::new(RefCell::new(BTreeMap::new())),
    //     );
    //     debug!("start test");
    //     let mut simulator = Simulation::new();
    //     let two_mat = sim::create_two_matrix_from_file(Path::new("mtx/test.mtx"));

    //     // ---- create resources and status id
    //     let sender_to_chip1 = simulator.create_resource(Box::new(Store::new(16)));
    //     let sender_to_chip2 = simulator.create_resource(Box::new(Store::new(16)));
    //     let chip_to_bank1 = simulator.create_resource(Box::new(Store::new(16)));
    //     let chip_to_bank2 = simulator.create_resource(Box::new(Store::new(16)));
    //     let chip_to_bank3 = simulator.create_resource(Box::new(Store::new(16)));
    //     let chip_to_bank4 = simulator.create_resource(Box::new(Store::new(16)));
    //     let final_partial_return = simulator.create_resource(Box::new(Store::new(16)));
    //     let bank_to_chip_partial_return1 = simulator.create_resource(Box::new(Store::new(16)));
    //     let bank_to_chip_partial_return2 = simulator.create_resource(Box::new(Store::new(16)));
    //     let chip_merger_resouce1 = simulator.create_resource(Box::new(SimpleResource::new(4)));
    //     let chip_merger_resouce2 = simulator.create_resource(Box::new(SimpleResource::new(4)));
    //     let chip_merger_status_id1 = merger_status.borrow_mut().create_merger_status(4);
    //     let chip_merger_status_id2 = merger_status.borrow_mut().create_merger_status(4);

    //     let bank_to_bank_merger = {
    //         let mut task_pe = vec![];
    //         for _i in 0..4 {
    //             let task_out = simulator.create_resource(Box::new(Store::new(16)));
    //             task_pe.push(task_out);
    //         }
    //         task_pe
    //     };

    //     // ---- create processes
    //     let task_sender = TaskSender::new(
    //         two_mat.a,
    //         two_mat.b,
    //         sender_to_chip,
    //         1,
    //         1,
    //         1,
    //         RowMapping::Chunk,
    //     );
    //     let chip_merger = ChipMerger::new(
    //         sender_to_chip,
    //         vec![chip_to_bank],
    //         chip_merger_resouce,
    //         chip_merger_status_id,
    //     );
    //     let chip_woker = MergerWorker {
    //         merger_size: 4,
    //         merger_status_id: chip_merger_status_id,
    //         merger_work_resource: chip_merger_resouce,
    //         partial_sum_sender: final_partial_return,
    //         task_reciever: bank_to_chip_partial_return,
    //         task_sender_input_id: sender_to_chip,
    //     };
    //     let bank_task_reorder =
    //         BankTaskReorder::new(chip_to_bank, bank_to_bank_merger.clone(), 4, ((0, 0), 0));
    //     let bank_pes = {
    //         let mut pes = vec![];
    //         for pe_in in bank_to_bank_merger {
    //             let pe_comp = BankPe::new(pe_in, bank_to_chip_partial_return, 4, 4, chip_to_bank);
    //             pes.push(pe_comp);
    //         }
    //         pes
    //     };
    //     let final_receiver = FinalReceiver {
    //         receiver: final_partial_return,
    //     };

    //     // ---- add processes to simulator
    //     let bank_task_reorder = simulator.create_process(bank_task_reorder.run());
    //     let task_sender = simulator.create_process(task_sender.run());
    //     let final_receiver_process = simulator.create_process(final_receiver.run());
    //     let chip_merger = simulator.create_process(chip_merger.run());
    //     let chip_woker = simulator.create_process(chip_woker.run());
    //     let bank_woker = bank_pes
    //         .into_iter()
    //         .map(|pe| simulator.create_process(pe.run()))
    //         .collect_vec();
    //     // ---- schedule processes

    //     vec![
    //         bank_task_reorder,
    //         task_sender,
    //         final_receiver_process,
    //         chip_merger,
    //         chip_woker,
    //     ]
    //     .into_iter()
    //     .for_each(|p| simulator.schedule_event(0., p, status.copy_default()));

    //     bank_woker
    //         .into_iter()
    //         .for_each(|p| simulator.schedule_event(0., p, status.copy_default()));

    //     // ---- run simulator
    //     simulator.run(EndCondition::NoEvents);
    // }
}
