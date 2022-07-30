//! dimm-level merger
//!
//!

use qsim::ResourceId;

use super::{
    buffer_status::BufferStatusId, merger_status::MergerStatusId, merger_task_sender::*,
    queue_tracker::QueueTrackerId, sim_time::NamedTimeId, BankID, LevelId,
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
    pub queue_tracker_id_recv: QueueTrackerId,
    pub queue_tracker_id_send: Vec<QueueTrackerId>,
}

impl DimmMerger {
    pub fn new(
        level_id: LevelId,
        task_in: ResourceId,
        lower_pes: Vec<ResourceId>,
        merger_status_id: MergerStatusId,
        time_id: NamedTimeId,
        buffer_status_id: BufferStatusId,
        queue_tracker_id_recv: QueueTrackerId,
        queue_tracker_id_send: Vec<QueueTrackerId>,
    ) -> Self {
        Self {
            level_id,
            task_in,
            lower_pes,
            merger_status_id,
            time_id,
            buffer_status_id,
            queue_tracker_id_recv,
            queue_tracker_id_send,
        }
    }
}

impl MergerTaskSender for DimmMerger {
    // index, resource id
    fn get_lower_id(&self, bank_id: &BankID) -> (usize, usize) {
        (
            *super::channel_id_from_bank_id(bank_id),
            self.lower_pes[*super::channel_id_from_bank_id(bank_id)],
        )
    }

    fn get_lower_pes(&self) -> &[ResourceId] {
        &self.lower_pes
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

    fn get_time_id(&self) -> &NamedTimeId {
        &self.time_id
    }

    fn get_buffer_id(&self) -> &super::buffer_status::BufferStatusId {
        &self.buffer_status_id
    }

    fn get_queue_tracker_id_recv(&self) -> &QueueTrackerId {
        &self.queue_tracker_id_recv
    }

    fn get_queue_tracker_id_send(&self) -> &[QueueTrackerId] {
        &self.queue_tracker_id_send
    }
}
