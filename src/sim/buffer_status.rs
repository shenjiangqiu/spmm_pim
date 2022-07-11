use std::{
    cell::UnsafeCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt::Debug,
};

/// this struct is used to track if all sub-task are return!
#[derive(Debug, Default)]
struct SubTaskTrackStatus {
    // for each target row, there are multiple sub-tasks(like bank for chip, chip for channel),
    //this struct record currently what sub-task is still outstanding
    waiting_sub_ids: BTreeMap<usize, BTreeSet<usize>>,
}

impl SubTaskTrackStatus {
    /// add a sub_task, should be called from task scheduler
    fn add_task(&mut self, target_row: usize, sub_id: usize) {
        let sub_ids = self
            .waiting_sub_ids
            .entry(target_row)
            .or_insert(BTreeSet::new());
        sub_ids.insert(sub_id);
    }

    fn contains(&self, target_row: usize) -> bool {
        self.waiting_sub_ids.contains_key(&target_row)
    }

    /// delete a sub_task and return true if all sub_task are done
    fn del_task(&mut self, target_row: usize, sub_id: usize) -> bool {
        let entry = self.waiting_sub_ids.get_mut(&target_row).unwrap();
        let removed = entry.remove(&sub_id);
        assert!(removed);
        if entry.is_empty() {
            self.waiting_sub_ids.remove(&target_row);
            true
        } else {
            false
        }
    }
}

/// the buffer status help to decide whether to receive a new line,
/// # Policy:
/// - if there are more than 2 slots, then receive a new line
/// - if there are only one slot, if the incoming is a latest line, then receive it!
/// - else, drop it
#[derive(Debug)]
pub struct BufferStatus {
    total_rows: usize,
    occupied_rows: BTreeSet<usize>,
    waiting_sequence: VecDeque<usize>,
    sub_task_track_status: SubTaskTrackStatus,
}

impl BufferStatus {
    pub fn new(total_size: usize) -> Self {
        if total_size < 2 {
            panic!("total_size should be greater than 2");
        }
        Self {
            total_rows: total_size,
            occupied_rows: BTreeSet::new(),
            waiting_sequence: VecDeque::new(),
            sub_task_track_status: SubTaskTrackStatus::default(),
        }
    }
    /// add a new target row that will be received later. this should be called by task sender
    pub fn add_waiting(&mut self, row: usize, sub_id: usize) {
        if !self.sub_task_track_status.contains(row) {
            self.waiting_sequence.push_back(row);
        }
        self.sub_task_track_status.add_task(row, sub_id);
    }
    /// test if the buffer will be availiable to receive a new line
    pub fn can_receive(&self, new_row: usize) -> bool {
        // already in it, receive it!
        if self.occupied_rows.contains(&new_row) || self.occupied_rows.len() <= self.total_rows - 2
        // alwasy receive when it's already in the buffer
        {
            true
        } else if self.occupied_rows.len() == self.total_rows - 1 {
            // have only one slot, if it's the latest line, receive it
            self.waiting_sequence.front().unwrap() == &new_row
        } else {
            false
        }
    }

    /// - receive a new line, if this is the last task, the record will be removed
    /// - ??? no, this is a bug, it should be removed only when the task sent to the merger!!!!
    /// - fixed by sjq
    #[must_use]
    pub fn receive(&mut self, new_row: usize, sub_id: usize) -> bool {
        assert!(self.can_receive(new_row));
        self.occupied_rows.insert(new_row);
        assert!(self.occupied_rows.len() <= self.total_rows);
        let finished = self.sub_task_track_status.del_task(new_row, sub_id);
        if finished {
            self.remove(new_row);
            true
        } else {
            false
        }
    }

    /// when all submerger are done, we can clear the buffer status
    fn remove(&mut self, row: usize) {
        let moved = self.occupied_rows.remove(&row);
        assert!(moved);
        self.waiting_sequence.pop_front().unwrap();
    }
}

#[derive(Default)]
pub struct SharedBufferStatus {
    pub inner: UnsafeCell<Vec<BufferStatus>>,
}
impl Debug for SharedBufferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SharedBufferStatus {:?}", unsafe { &*self.inner.get() })
    }
}

pub struct BufferStatusId {
    pub id: usize,
}

impl SharedBufferStatus {
    #[must_use]
    pub fn add_component(&self, buffer_rows: usize) -> BufferStatusId {
        let inner = unsafe { &mut *self.inner.get() };
        let id = inner.len();
        inner.push(BufferStatus::new(buffer_rows));
        BufferStatusId { id }
    }
    /// # Safety:
    /// - the id must be valid
    #[must_use]
    pub unsafe fn can_receive(&self, comp_id: &BufferStatusId, new_row: usize) -> bool {
        let inner = &mut *self.inner.get();
        inner.get_unchecked(comp_id.id).can_receive(new_row)
    }
    /// # Safety:
    /// - the id must be valid
    #[must_use]
    pub unsafe fn receive(&self, comp_id: &BufferStatusId, new_row: usize, sub_id: usize) -> bool {
        let inner = &mut *self.inner.get();
        let buffer = inner.get_unchecked_mut(comp_id.id);
        buffer.receive(new_row, sub_id)
    }
    /// # Safety:
    /// - the id must be valid
    // unsafe fn remove(&self, comp_id: &BufferStatusId, row: usize) {
    //     let inner = &mut *self.inner.get();
    //     let buffer = inner.get_unchecked_mut(comp_id.id);
    //     buffer.remove(row);
    // }
    /// # Safety:
    /// - the id must be valid
    pub unsafe fn add_waiting(&self, comp_id: &BufferStatusId, row: usize, sub_id: usize) {
        let inner = &mut *self.inner.get();
        let buffer = inner.get_unchecked_mut(comp_id.id);
        buffer.add_waiting(row, sub_id);
    }
}

#[cfg(test)]
mod test {
    use super::SharedBufferStatus;

    #[test]
    fn main_test() {
        let shared_buffer_status = SharedBufferStatus::default();
        let id = shared_buffer_status.add_component(4);
        unsafe {
            //send task
            shared_buffer_status.add_waiting(&id, 0, 0);
            shared_buffer_status.add_waiting(&id, 0, 1);
            shared_buffer_status.add_waiting(&id, 1, 2);
            println!("{:?}", shared_buffer_status);
            // start to receive data
            assert!(shared_buffer_status.can_receive(&id, 0));
            let is_finished = shared_buffer_status.receive(&id, 0, 0);
            println!("{:?}", shared_buffer_status);

            assert!(!is_finished);
            assert!(shared_buffer_status.can_receive(&id, 0));
            let is_finished = shared_buffer_status.receive(&id, 0, 1);
            println!("{:?}", shared_buffer_status);

            assert!(is_finished);
        }
    }
}
