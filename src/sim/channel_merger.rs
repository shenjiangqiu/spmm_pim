//! channel-level merger
//!
//!

use desim::ResourceId;

use super::{merger_task_sender::*, BankID, sim_time::{NamedTimeId, LevelTimeId}};
pub struct ChannelMerger {
    pub task_in: ResourceId,
    pub lower_pes: Vec<ResourceId>,
    pub merger_resouce: ResourceId,

    // settings
    pub merger_status_id: usize,
    pub self_level_time_id: LevelTimeId,

    pub sim_time:NamedTimeId,
}

impl ChannelMerger {
    pub fn new(
        task_in: ResourceId,
        lower_pes: Vec<ResourceId>,
        merger_resouce: ResourceId,
        merger_status_id: usize,
        self_level_time_id: LevelTimeId,
        sim_time:NamedTimeId,
    ) -> Self {
        Self {
            task_in,
            lower_pes,
            merger_resouce,
            merger_status_id,
            self_level_time_id,
            sim_time,
        }
    }
}

impl MergerTaskSender for ChannelMerger {
    fn get_lower_id(&self, bank_id: &BankID) -> usize {
        self.lower_pes[super::chip_id_from_bank_id(bank_id).1]
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
        &self.sim_time
    }
}
