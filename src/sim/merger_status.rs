use std::{cell::RefCell, collections::VecDeque};

use log::debug;

/// the merger status,
/// each merger status represent a set of mergers for a certain Chip.. or channel.
///
#[derive(Debug, Default)]
pub struct MergerStatus {
    current_merger_working: Vec<bool>,
    current_waiting_target_id: VecDeque<usize>,
}

impl MergerStatus {
    /// create a new merger status
    /// - this one should not contains any current_working_merger
    /// - current_merger_worker_status should be n-mergers with default status
    pub fn new(num_mergers: usize) -> Self {
        Self {
            current_merger_working: vec![false; num_mergers],
            current_waiting_target_id: VecDeque::new(),
        }
    }

    /// select a new merger and push a task into it.
    /// - when it's standalone mode, it will send the lastest one to the first merger.
    /// - it will also delete the waiting task
    pub fn get_next_merger(&mut self, target_id: usize, is_binding: bool) -> Option<usize> {
        // find a merger with least on_going_task_num
        if is_binding {
            debug!("current_ongoing: {:?}", &self.current_merger_working);
            let avaliable = self.current_merger_working.iter().position(|&x| x == false);
            if let Some(id) = avaliable {
                self.current_merger_working[id] = true;
                self.current_waiting_target_id.remove(
                    self.current_waiting_target_id
                        .binary_search(&target_id)
                        .unwrap(),
                );
                return Some(id);
            } else {
                return None;
            }
        } else {
            // standalone mode
            if self.current_waiting_target_id.front().unwrap() == &target_id {
                let avaliable = self.current_merger_working.iter().position(|&x| x == false);
                if let Some(position) = avaliable {
                    self.current_merger_working[position] = true;
                    self.current_waiting_target_id.pop_front();
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
                        self.current_waiting_target_id.remove(
                            self.current_waiting_target_id
                                .binary_search(&target_id)
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

    pub fn add_waiting(&mut self, target_id: usize, is_binding: bool) {
        // it's standalone mode, and the last is not the new one
        if !is_binding
            && !self
                .current_waiting_target_id
                .back()
                .map_or(false, |&x| x == target_id)
        {
            self.current_waiting_target_id.push_back(target_id);
        }
    }

    /// means the merger is done.
    /// _is_binding
    pub fn release_merger(&mut self, merger_id: usize, _target_id: usize, _is_binding: bool) {
        assert!(self.current_merger_working[merger_id]);
        self.current_merger_working[merger_id] = false;
    }
}

/// This is the merger status, it is used to store the merger status.
#[derive(Debug, Default)]
pub struct SharedMergerStatus {
    inner: RefCell<Vec<MergerStatus>>,
}
#[derive(Debug, Clone, Copy)]
pub struct MergerStatusId {
    id: usize,
}

impl SharedMergerStatus {
    pub fn add_component(&self, total_merger: usize) -> MergerStatusId {
        let mut inner = self.inner.borrow_mut();
        inner.push(MergerStatus::new(total_merger));
        MergerStatusId {
            id: inner.len() - 1,
        }
    }
    // this target row will need to go to the merger. only standalone mode will take effect on this.
    pub fn add_waiting(&self, id: &MergerStatusId, target_id: usize, is_binding: bool) {
        let mut inner = self.inner.borrow_mut();
        inner[id.id].add_waiting(target_id, is_binding);
    }
    // fetch the next merger for target_id.
    pub fn get_next_merger(
        &self,
        id: MergerStatusId,
        target_id: usize,
        is_binding: bool,
    ) -> Option<usize> {
        let mut inner = self.inner.borrow_mut();
        inner[id.id].get_next_merger(target_id, is_binding)
    }
    // release the merger for target_id.
    pub fn release_merger(
        &self,
        id: MergerStatusId,
        merger_id: usize,
        target_id: usize,
        is_binding: bool,
    ) {
        let mut inner = self.inner.borrow_mut();
        inner[id.id].release_merger(merger_id, target_id, is_binding);
    }
}
