use std::cell::RefCell;

use log::warn;

#[derive(Debug, Default)]
pub struct QueueTracker {
    pub data: RefCell<Vec<(String, i32)>>,
}
#[derive(Debug, Clone, Copy)]
pub struct QueueTrackerId {
    pub id: usize,
}
impl QueueTracker {
    pub fn add_component_with_name(&self, name: impl Into<String>) -> QueueTrackerId {
        let mut data = self.data.borrow_mut();
        data.push((name.into(), 0));
        QueueTrackerId { id: data.len() - 1 }
    }

    pub fn enq(&self, id: &QueueTrackerId) {
        let mut data = self.data.borrow_mut();
        data[id.id].1 += 1;
    }
    pub fn deq(&self, id: &QueueTrackerId) {
        let mut data = self.data.borrow_mut();
        data[id.id].1 -= 1;
        if data[id.id].1 < 0 {
            warn!("error! queue length is negative: {}", data[id.id].1);
        }
    }

    pub fn show_data(&self) {
        let data = self.data.borrow();
        for (name, count) in data.iter() {
            log::error!("{}:{}", name, count);
        }
    }
}
