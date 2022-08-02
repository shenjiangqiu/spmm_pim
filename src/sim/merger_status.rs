use std::{cell::RefCell, collections::VecDeque};

use itertools::Itertools;
use log::debug;

/// the merger status,
/// each merger status represent a set of mergers for a certain Chip.. or channel.
///
#[derive(Debug, Default)]
pub struct MergerStatus {
    current_merger_working: Vec<bool>,
    current_waiting_task_id: VecDeque<usize>,
}

impl MergerStatus {
    /// create a new merger status
    /// - this one should not contains any current_working_merger
    /// - current_merger_worker_status should be n-mergers with default status
    pub fn new(num_mergers: usize) -> Self {
        Self {
            current_merger_working: vec![false; num_mergers],
            current_waiting_task_id: VecDeque::new(),
        }
    }

    /// select a new merger and push a task into it.
    /// - when it's standalone mode, it will send the lastest one to the first merger.
    /// - it will also delete the waiting task
    pub fn get_next_merger(&mut self, task_id: usize, is_binding: bool) -> Option<usize> {
        // find a merger with least on_going_task_num
        if is_binding {
            debug!("current_ongoing: {:?}", &self.current_merger_working);
            let avaliable = self.current_merger_working.iter().position(|&x| x == false);
            if let Some(id) = avaliable {
                self.current_merger_working[id] = true;
                self.current_waiting_task_id.remove(
                    self.current_waiting_task_id
                        .binary_search(&task_id)
                        .unwrap(),
                );
                return Some(id);
            } else {
                return None;
            }
        } else {
            // standalone mode
            if self.current_waiting_task_id.front().unwrap() == &task_id {
                let avaliable = self.current_merger_working.iter().position(|&x| x == false);
                if let Some(position) = avaliable {
                    self.current_merger_working[position] = true;
                    self.current_waiting_task_id.pop_front();
                    return Some(position);
                } else {
                    return None;
                }
            } else {
                let all_positions = self
                    .current_merger_working
                    .iter()
                    .enumerate()
                    .filter(|&(_, x)| x == &false)
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                match all_positions.len() {
                    0..=1 => None,
                    _ => {
                        let avaliable = all_positions[0];
                        self.current_waiting_task_id.remove(
                            self.current_waiting_task_id
                                .binary_search(&task_id)
                                .unwrap(),
                        );
                        self.current_merger_working[avaliable] = true;
                        Some(avaliable)
                    }
                }
            }
        }
    }
    /// add to the waiting only when it's standalone mode

    pub fn add_waiting(&mut self, task_id: usize, is_binding: bool) {
        // it's standalone mode, and the last is not the new one
        if !is_binding
            && !self
                .current_waiting_task_id
                .back()
                .map_or(false, |&x| x == task_id)
        {
            self.current_waiting_task_id.push_back(task_id);
            // all current_waiting_task should be sorted
            debug_assert!(self
                .current_waiting_task_id
                .iter()
                .tuple_windows()
                .all(|(a, b)| a < b))
        }
    }

    /// means the merger is done.
    /// _is_binding
    pub fn release_merger(&mut self, merger_id: usize, _task_id: usize, _is_binding: bool) {
        assert!(self.current_merger_working[merger_id]);
        self.current_merger_working[merger_id] = false;
    }
}

/// This is the merger status, it is used to store the merger status.
#[derive(Debug, Default)]
pub struct SharedMergerStatus {
    inner: RefCell<Vec<MergerStatus>>,
    is_binding: bool,
}
#[derive(Debug, Clone, Copy)]
pub struct MergerStatusId {
    id: usize,
}

impl SharedMergerStatus {
    pub fn new(is_binding: bool) -> Self {
        Self {
            inner: RefCell::new(vec![]),
            is_binding,
        }
    }
    pub fn add_component(&self, total_merger: usize) -> MergerStatusId {
        let mut inner = self.inner.borrow_mut();
        inner.push(MergerStatus::new(total_merger));
        MergerStatusId {
            id: inner.len() - 1,
        }
    }
    // this target row will need to go to the merger. only standalone mode will take effect on this.
    pub fn add_waiting(&self, id: &MergerStatusId, task_id: usize) {
        let mut inner = self.inner.borrow_mut();
        inner[id.id].add_waiting(task_id, self.is_binding);
    }
    // fetch the next merger for task_id.
    pub fn get_next_merger(
        &self,
        id: MergerStatusId,
        task_id: usize,
        is_binding: bool,
    ) -> Option<usize> {
        let mut inner = self.inner.borrow_mut();
        inner[id.id].get_next_merger(task_id, is_binding)
    }
    // release the merger for task_id.
    pub fn release_merger(
        &self,
        id: MergerStatusId,
        merger_id: usize,
        task_id: usize,
        is_binding: bool,
    ) {
        let mut inner = self.inner.borrow_mut();
        inner[id.id].release_merger(merger_id, task_id, is_binding);
    }
}
