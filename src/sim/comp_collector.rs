#![allow(dead_code)]

use std::fmt::Debug;

use log::info;
use qsim::{resources::Store, Simulation};

use super::{component::Component, SpmmStatus};

#[derive(Default)]
pub struct QueueIdCollector {
    all_queue_ids: Vec<usize>,
}
impl QueueIdCollector {
    pub fn create_queue(&mut self, sim: &mut Simulation<SpmmStatus>, len: usize) -> usize {
        let id = sim.create_resource(Box::new(Store::new(len)), "name");
        self.all_queue_ids.push(id);
        id
    }
}
pub struct ProcessInfoCollector {
    should_collect: bool,
    all_process_infos: Vec<String>,
}

impl ProcessInfoCollector {
    pub fn new(should_collect: bool) -> Self {
        ProcessInfoCollector {
            should_collect,
            all_process_infos: Vec::new(),
        }
    }
    pub fn create_process_and_schedule<T>(
        &mut self,
        sim: &mut Simulation<SpmmStatus>,
        process: T,
        status: &SpmmStatus,
    ) where
        T: Debug + Component + 'static,
    {
        self.all_process_infos.push(format!("{:?}", process));
        let id = sim.create_process(process.run(status.clone()));
        sim.schedule_event(
            0.0,
            id,
            status.clone_with_state(super::SpmmStatusEnum::Continue),
        );
    }

    pub fn show_data(&self) {
        for process_info in &self.all_process_infos {
            info!("{}", process_info);
        }
    }
}
