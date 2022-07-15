//! channel-level merger
//!
//!

use desim::ResourceId;

use super::{
    buffer_status::BufferStatusId,
    merger_status::MergerStatusId,
    merger_task_sender::*,
    sim_time::{LevelTimeId, NamedTimeId},
    BankID,
};
pub struct ChannelMerger {
    pub task_in: ResourceId,
    pub lower_pes: Vec<ResourceId>,

    // settings
    pub merger_status_id: MergerStatusId,
    pub self_level_time_id: LevelTimeId,
    pub buffer_status_id: BufferStatusId,
    pub sim_time: NamedTimeId,
}

impl ChannelMerger {
    pub fn new(
        task_in: ResourceId,
        lower_pes: Vec<ResourceId>,
        merger_status_id: MergerStatusId,
        self_level_time_id: LevelTimeId,
        sim_time: NamedTimeId,
        buffer_status_id: BufferStatusId,
    ) -> Self {
        Self {
            task_in,
            lower_pes,
            merger_status_id,
            self_level_time_id,
            sim_time,
            buffer_status_id,
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
        panic!("not implemented");
    }
    fn get_merger_status_id(&self) -> &MergerStatusId {
        &self.merger_status_id
    }

    fn get_lower_pes(&self) -> &[ResourceId] {
        &self.lower_pes
    }

    fn get_time_id(&self) -> &NamedTimeId {
        &self.sim_time
    }

    fn get_buffer_id(&self) -> &super::buffer_status::BufferStatusId {
        &self.buffer_status_id
    }
}
