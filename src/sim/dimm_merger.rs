//! dimm-level merger
//!
//!

use desim::ResourceId;

use super::{
    buffer_status::BufferStatusId, merger_status::MergerStatusId, merger_task_sender::*,
    sim_time::NamedTimeId, BankID, LevelId,
};
#[derive(Debug)]
pub struct DimmMerger {
    pub level_id: LevelId,
    pub task_in: ResourceId,
    pub lower_pes: Vec<ResourceId>,

    // settings
    pub merger_status_id: MergerStatusId,
    pub buffer_status_id: BufferStatusId,
    // stats ids
    pub time_id: NamedTimeId,
}

impl DimmMerger {
    pub fn new(
        level_id: LevelId,
        task_in: ResourceId,
        lower_pes: Vec<ResourceId>,
        merger_status_id: MergerStatusId,
        time_id: NamedTimeId,
        buffer_status_id: BufferStatusId,
    ) -> Self {
        Self {
            level_id,
            task_in,
            lower_pes,
            merger_status_id,
            time_id,
            buffer_status_id,
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
        panic!("not implemented");
    }
    fn get_merger_status_id(&self) -> &MergerStatusId {
        &self.merger_status_id
    }

    fn get_lower_pes(&self) -> &[ResourceId] {
        &self.lower_pes
    }

    fn get_time_id(&self) -> &NamedTimeId {
        &self.time_id
    }

    fn get_buffer_id(&self) -> &super::buffer_status::BufferStatusId {
        &self.buffer_status_id
    }
}
