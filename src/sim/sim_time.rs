use std::cell::UnsafeCell;
#[derive(Debug,Default)]
pub struct SimTime {
    pub bank_read: f64,
    pub bank_merge: f64,
    pub chip_merge: f64,
    pub channel_merge: f64,
    pub dimm_merge: f64,
}
#[derive(Debug)]
pub struct SharedSimTime {
    inner: UnsafeCell<SimTime>,
}

impl SharedSimTime {
    pub fn new() -> Self {
        SharedSimTime {
            inner: UnsafeCell::new(SimTime {
                bank_read: 0.,
                bank_merge: 0.,
                chip_merge: 0.,
                channel_merge: 0.,
                dimm_merge: 0.,
            }),
        }
    }

    pub fn add_bank_read(&self, time: f64) {
        unsafe {
            let mut inner = &mut *self.inner.get();
            inner.bank_read += time;
        }
    }
    pub fn add_bank_merge(&self, time: f64) {
        unsafe {
            let mut inner = &mut *self.inner.get();
            inner.bank_merge += time;
        }
    }
    pub fn add_chip_merge(&self, time: f64) {
        unsafe {
            let mut inner = &mut *self.inner.get();
            inner.chip_merge += time;
        }
    }

    pub fn add_channel_merge(&self, time: f64) {
        unsafe {
            let mut inner = &mut *self.inner.get();
            inner.channel_merge += time;
        }
    }

    pub fn add_dimm_merge(&self, time: f64) {
        unsafe {
            let mut inner = &mut *self.inner.get();
            inner.dimm_merge += time;
        }
    }
}
