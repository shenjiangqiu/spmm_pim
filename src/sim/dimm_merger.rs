//! dimm-level merger
//!
//!

use desim::ResourceId;

use super::{merger_task_sender::*, BankID};
pub struct DimmMerger {
    pub task_in: ResourceId,
    pub lower_pes: Vec<ResourceId>,
    pub merger_resouce: ResourceId,

    // settings
    pub merger_status_id: usize,
    pub self_level_time_id: usize,

    pub get_id: usize,
    pub send_id: usize,
    pub aquer_id: usize,
    pub release_id: usize,
}

impl DimmMerger {
    pub fn new(
        task_in: ResourceId,
        lower_pes: Vec<ResourceId>,
        merger_resouce: ResourceId,
        merger_status_id: usize,
        self_level_time_id: usize,
        get_id: usize,
        send_id: usize,
        aquer_id: usize,
        release_id: usize,
    ) -> Self {
        Self {
            task_in,
            lower_pes,
            merger_resouce,
            merger_status_id,
            self_level_time_id,
            get_id,
            send_id,
            aquer_id,
            release_id,
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

    fn get_task_get_idle_id(&self) -> usize {
        self.get_id
    }

    fn get_task_send_idle_id(&self) -> usize {
        self.send_id
    }

    fn get_slot_aquer_id(&self) -> usize {
        self.aquer_id
    }

    fn get_slot_release_id(&self) -> usize {
        self.release_id
    }
}
