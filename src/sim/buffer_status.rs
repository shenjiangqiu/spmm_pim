//! the buffer status to control whether to receive a new line from lower pe

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt::Debug,
};

use itertools::Itertools;
use log::debug;

/// the buffer status help to decide whether to receive a new line,
/// # Policy:
/// - if there are more than 2 slots, then receive a new line
/// - if there are only one slot, if the incoming is a latest line, then receive it!
/// - else, drop it
#[derive(Debug)]
pub struct BufferStatus {
    /// self id
    id: usize,
    /// how many rows can be buffered in total
    total_rows: usize,
    /// currently occupied rows(store the target row id)
    occupied_rows: BTreeSet<usize>,
    /// currently not finsihed rows(waiting for reading from lower pe)
    /// a row will be in this queue when add the task, it will be removed when the row is finished reciving!
    /// the front one should be the priority one, and the back one is the latest one.
    /// whe the entry is not enough, the front one will always have a free place to put it in
    waiting_sequence: VecDeque<usize>,
    /// for each waiting rows, the lower id it waits.
    waiting_sub_ids: BTreeMap<usize, BTreeSet<usize>>,
}

impl BufferStatus {
    /// create a new buffer status
    /// - `total_size` the max lines can be buffered in total, should be larger than 2!
    pub fn new(total_size: usize, id: usize) -> Self {
        if total_size < 2 {
            panic!("total_size should be greater than 2");
        }
        Self {
            id,
            total_rows: total_size,
            occupied_rows: BTreeSet::new(),
            waiting_sequence: VecDeque::new(),
            waiting_sub_ids: Default::default(),
        }
    }
    /// add a new target row that will be received later. this should be called by task sender, the row id shoudl be sorted!
    pub fn add_waiting(&mut self, row: usize, sub_id: usize) {
        if !self.waiting_sub_ids.contains_key(&row) {
            self.waiting_sequence.push_back(row);

            assert!(self
                .waiting_sequence
                .iter()
                .tuple_windows()
                .all(|(a, b)| a < b));
        }
        self.waiting_sub_ids
            .entry(row)
            .or_insert(BTreeSet::new())
            .insert(sub_id);
    }
    /// test if the buffer will be availiable to receive a new line
    pub fn can_receive(&self, new_row: usize) -> bool {
        let remaining = self.total_rows - self.occupied_rows.len();
        if self.occupied_rows.contains(&new_row) {
            return true;
        }
        // do not contains this one
        match remaining {
            0 => false,
            1 => {
                if self.waiting_sequence.front().unwrap() == &new_row {
                    true
                } else {
                    if self
                        .occupied_rows
                        .contains(self.waiting_sequence.front().unwrap())
                    {
                        // the first line is already in the buffer, so the next we can receive!
                        true
                    } else {
                        // the last line should be reserved for the latest one!
                        false
                    }
                }
            }
            _ => true,
        }
    }

    /// - receive a new line, if this is the last task, the record will be removed
    /// - ??? no, this is a bug, it should be removed only when the task sent to the merger!!!!
    /// - fixed by sjq
    /// # Returns:
    /// if a ready line is received, return true, else return false
    #[must_use]
    pub fn receive(&mut self, new_row: usize, sub_id: usize) -> bool {
        // todo: first modify current occupied rows, then modify the track status. if all sub pe have returned, then remove the waiting sequence and trace.

        // step 1, add the new row to the occupied rows
        let id = self.id;
        debug!("BUFFER_STATUS:{id} receive row {new_row},subid: {sub_id}");
        assert!(self.can_receive(new_row));
        self.occupied_rows.insert(new_row);
        assert!(self.occupied_rows.len() <= self.total_rows);

        // step 2, remove one of the pe track status
        let entry = self.waiting_sub_ids.get_mut(&new_row).unwrap();

        let removed = entry.remove(&sub_id);
        assert!(removed);
        if entry.is_empty() {
            debug!("BUFFER_STATUS:{id} all sub id received {new_row} {sub_id}");
            // all sub pe have returned, remove the waiting sequence
            self.waiting_sub_ids.remove(&new_row);

            let entry = self.waiting_sequence.binary_search(&new_row).unwrap();
            self.waiting_sequence.remove(entry);

            true
        } else {
            false
        }
    }

    /// when the pe is ready to make a row to merger pe, remove it from the buffer!
    pub fn remove(&mut self, row: usize) {
        let id = self.id;
        debug!("BUFFER_STATUS:id:{id} remove row: {row}");
        let moved = self.occupied_rows.remove(&row);
        assert!(moved);
    }
}

/// the shared buffer status for all process.
/// to get a specifice buffer status, you should get an id from `add_component` and use it to get the buffer status.
/// - see [BufferStatus](self.BufferStatus) for more details.
/// # Example:
/// ```rust
/// use spmm_pim::sim::buffer_status::*;
/// let shared_buffer_status=SharedBufferStatus::default();
/// let id=shared_buffer_status.add_component(10);
/// // do shomething with the buffer status
/// unsafe{
///     shared_buffer_status.add_waiting(&id,1,1);
///     shared_buffer_status.add_waiting(&id,2,2);
///     if(shared_buffer_status.can_receive(&id,1)){
///         shared_buffer_status.receive(&id,1,1);
///
///     }
///     if(shared_buffer_status.can_receive(&id,2)){
///         shared_buffer_status.receive(&id,2,2);
///     }
/// }
///
/// ```
#[derive(Default)]
pub struct SharedBufferStatus {
    pub inner: RefCell<Vec<BufferStatus>>,
}
impl Debug for SharedBufferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SharedBufferStatus {:?}", self.inner.borrow())
    }
}
#[derive(Debug, Clone, Copy)]
pub struct BufferStatusId {
    pub id: usize,
}

impl SharedBufferStatus {
    #[must_use]
    pub fn add_component(&self, buffer_rows: usize) -> BufferStatusId {
        let mut inner = self.inner.borrow_mut();
        let id = inner.len();
        inner.push(BufferStatus::new(buffer_rows, id));
        BufferStatusId { id }
    }
    /// # Safety:
    /// - the id must be valid
    #[must_use]
    pub fn can_receive(&self, comp_id: &BufferStatusId, new_row: usize) -> bool {
        let inner = self.inner.borrow();
        inner.get(comp_id.id).unwrap().can_receive(new_row)
    }
    /// # Safety:
    /// - the id must be valid
    /// # Returns:
    /// if a ready line is received, return true, else return false
    #[must_use]
    pub fn receive(&self, comp_id: &BufferStatusId, new_row: usize, sub_id: usize) -> bool {
        let mut inner = self.inner.borrow_mut();
        let buffer = inner.get_mut(comp_id.id).unwrap();
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
    pub fn add_waiting(&self, comp_id: &BufferStatusId, row: usize, sub_id: usize) {
        let mut inner = self.inner.borrow_mut();
        let buffer = inner.get_mut(comp_id.id).unwrap();
        buffer.add_waiting(row, sub_id);
    }

    /// # Safety:
    /// - the id must be valid
    ///
    /// see the comment of `BufferStatus::remove()`
    pub fn remove(&self, comp_id: &BufferStatusId, row: usize) {
        let mut inner = self.inner.borrow_mut();
        let buffer = inner.get_mut(comp_id.id).unwrap();
        buffer.remove(row);
    }

    ///
    pub fn get_current_status(&self, comp_id: &BufferStatusId) -> String {
        let inner = self.inner.borrow();
        format!("{:?}", inner.get(comp_id.id).unwrap())
    }
}

#[cfg(test)]
mod test {
    use super::SharedBufferStatus;

    #[test]
    fn main_test() {
        let shared_buffer_status = SharedBufferStatus::default();
        let id = shared_buffer_status.add_component(4);
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

        let is_finished = shared_buffer_status.receive(&id, 1, 2);
        println!("{:?}", shared_buffer_status);
        assert!(is_finished);
        shared_buffer_status.remove(&id, 0);
        println!("{:?}", shared_buffer_status);
        shared_buffer_status.remove(&id, 1);
        println!("{:?}", shared_buffer_status);
    }

    #[test]
    fn cannot_add_test() {
        let shared_buffer_status = SharedBufferStatus::default();
        let id = shared_buffer_status.add_component(4);
        //send task
        shared_buffer_status.add_waiting(&id, 0, 0);
        shared_buffer_status.add_waiting(&id, 0, 1);
        shared_buffer_status.add_waiting(&id, 1, 2);
        shared_buffer_status.add_waiting(&id, 2, 2);
        shared_buffer_status.add_waiting(&id, 3, 2);
        shared_buffer_status.add_waiting(&id, 4, 2);

        assert!(shared_buffer_status.can_receive(&id, 0));

        // can receive 1,2,3 because 2 more entry is available
        let _finished = shared_buffer_status.receive(&id, 1, 2);
        let _finished = shared_buffer_status.receive(&id, 2, 2);
        let _finished = shared_buffer_status.receive(&id, 3, 2);

        // cannot receive 4, because 1 entry is available
        assert!(!shared_buffer_status.can_receive(&id, 4));
        // can receive 0, because it's the first one to wait

        println!(
            "after add 1,2,3,4 and receive 1,2,3:\n{:?}",
            shared_buffer_status
        );
        assert!(shared_buffer_status.can_receive(&id, 0));
        let _finished = shared_buffer_status.receive(&id, 0, 0);
        let _finished = shared_buffer_status.receive(&id, 0, 1);
        println!("after receive 0:\n{:?}", shared_buffer_status);
        shared_buffer_status.remove(&id, 0);
        assert!(shared_buffer_status.can_receive(&id, 4));
        let _finished = shared_buffer_status.receive(&id, 4, 2);
        println!("after remove 0 and receive 4\n{:?}", shared_buffer_status);
        shared_buffer_status.remove(&id, 4);
        shared_buffer_status.remove(&id, 3);
        shared_buffer_status.remove(&id, 2);
        shared_buffer_status.remove(&id, 1);
        println!("after remove 1,2,3,4\n {:?}", shared_buffer_status);
    }
}
