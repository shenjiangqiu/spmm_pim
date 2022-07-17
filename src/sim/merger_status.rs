use std::cell::UnsafeCell;

use log::debug;

/// the merger status,
/// each merger status represent a set of mergers for a certain Chip.. or channel.
///
#[derive(Debug, Default)]
pub struct MergerStatus {
    current_on_going_task_num: Vec<usize>,
}

impl MergerStatus {
    /// create a new merger status
    /// - this one should not contains any current_working_merger
    /// - current_merger_worker_status should be n-mergers with default status
    pub fn new(num_mergers: usize) -> Self {
        Self {
            current_on_going_task_num: vec![0; num_mergers],
        }
    }

    /// select a new merger and push a task into it.
    pub fn get_next_merger(&mut self) -> usize {
        // find a merger with least on_going_task_num
        let min = self
            .current_on_going_task_num
            .iter_mut()
            .enumerate()
            .min()
            .unwrap();
        *min.1 += 1;

        debug!("find next merger:id: {}, new_size: {}", min.0, min.1);
        min.0
    }

    pub fn release_merger(&mut self, merger_id: usize) {
        self.current_on_going_task_num[merger_id] -= 1;
        debug!(
            "release_merger: {}, current_value: {}",
            merger_id, self.current_on_going_task_num[merger_id]
        );
    }
}

/// This is the merger status, it is used to store the merger status.
#[derive(Debug, Default)]
pub struct SharedMergerStatus {
    inner: UnsafeCell<Vec<MergerStatus>>,
}
#[derive(Debug, Clone, Copy)]
pub struct MergerStatusId {
    id: usize,
}

impl SharedMergerStatus {
    pub fn add_component(&self, total_merger: usize) -> MergerStatusId {
        let inner = unsafe { &mut *self.inner.get() };
        inner.push(MergerStatus::new(total_merger));
        MergerStatusId {
            id: inner.len() - 1,
        }
    }
    pub fn get_next_merger(&self, id: MergerStatusId) -> usize {
        let inner = unsafe { &mut *self.inner.get() };
        inner[id.id].get_next_merger()
    }
    pub fn release_merger(&self, id: MergerStatusId, merger_id: usize) {
        let inner = unsafe { &mut *self.inner.get() };
        inner[id.id].release_merger(merger_id);
    }

    /// ## Safety
    /// the id should be valid.
    pub unsafe fn get_next_merger_unchecked(&self, id: MergerStatusId) -> usize {
        let inner = &mut *self.inner.get();
        inner.get_unchecked_mut(id.id).get_next_merger()
    }

    /// ## Safety
    /// the id should be valid.
    pub unsafe fn release_merger_unchecked(&self, id: MergerStatusId, merger_id: usize) {
        let inner = &mut *self.inner.get();
        inner.get_unchecked_mut(id.id).release_merger(merger_id);
    }
}
