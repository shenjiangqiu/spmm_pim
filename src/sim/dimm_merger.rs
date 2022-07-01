//! dimm-level merger
//!
//!

use desim::ResourceId;

use super::{
    merger_task_sender::*,
    sim_time::{LevelTimeId, NamedTimeId},
    BankID,
};
pub struct DimmMerger {
    pub task_in: ResourceId,
    pub lower_pes: Vec<ResourceId>,
    pub merger_resouce: ResourceId,

    // settings
    pub merger_status_id: usize,

    // stats ids
    pub time_id: NamedTimeId,
}

impl DimmMerger {
    pub fn new(
        task_in: ResourceId,
        lower_pes: Vec<ResourceId>,
        merger_resouce: ResourceId,
        merger_status_id: usize,
        time_id: NamedTimeId,
    ) -> Self {
        Self {
            task_in,
            lower_pes,
            merger_resouce,
            merger_status_id,
            time_id,
        }
    }
}

impl MergerTaskSender for DimmMerger {
    fn get_lower_id(&self, bank_id: &BankID) -> usize {
        self.lower_pes[*super::channel_id_from_bank_id(bank_id)]
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

    fn get_time_id(&self) -> &NamedTimeId {
        &self.time_id
    }
}
