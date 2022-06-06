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
    pub merger_size: usize,

    pub parallel_merger_num: usize,
    pub merger_status_id: usize,
}

impl DimmMerger {
    pub fn new(
        task_in: ResourceId,
        lower_pes: Vec<ResourceId>,
        merger_resouce: ResourceId,
        merger_size: usize,
        parallel_merger_num: usize,
        merger_status_id: usize,
    ) -> Self {
        Self {
            task_in,
            lower_pes,
            merger_resouce,
            merger_size,
            parallel_merger_num,
            merger_status_id,
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
}
