//! # *⸜( •ᴗ• )⸝*  this module contains the simulation time of each component
//!
//! - the use rate for each component
//!   - get the idle time for each component
//! - the finished time for each row
//!   - get the last finished time for each level!
//! - the unbanlence of each row
//!   - get every level's max time and min time

use std::cell::UnsafeCell;

use serde::Serialize;

/// # ( Ꙭ) this component contains the idle time of each component
/// - this structure should be instantiated by each component
#[derive(Default, Debug)]
pub struct ComponentTime {
    pub componet_idle_time: UnsafeCell<Vec<Vec<f64>>>,
}
impl ComponentTime {
    pub fn new() -> Self {
        Default::default()
    }
    /// # (⸝⸝•‧̫•⸝⸝)
    pub fn add_component(&self) -> usize {
        unsafe {
            let vec = &mut *self.componet_idle_time.get();
            vec.push(vec![]);
            vec.len() - 1
        }
    }
    /// # (˶˚ ᗨ ˚˶)
    /// take care! the component_id should be valid that returned by add_component
    /// # Safety
    /// the component_id should be valid
    pub unsafe fn get_idle_time(&self, component_id: usize) -> &Vec<f64> {
        (*self.componet_idle_time.get()).get_unchecked(component_id)
    }
    /// # (,,•́.•̀,,)
    ///  take care! the component_id should be valid that returned by add_component
    /// # Safety
    /// the component_id should be valid
    pub unsafe fn set_idle_time(&self, component_id: usize, idle_time: Vec<f64>) {
        *(*self.componet_idle_time.get()).get_unchecked_mut(component_id) = idle_time;
    }
    /// # (Ծ‸Ծ)
    /// take care! the component_id should be valid that returned by add_component
    /// # Safety
    /// the component_id should be valid
    pub unsafe fn add_idle_time(&self, component_id: usize, idle_time: f64) {
        // add the idle time to current idle time

        (*self.componet_idle_time.get())
            .get_unchecked_mut(component_id)
            .push(idle_time);
    }
}

/// this component is used to record the finished time of each level(like bank, chip, channel)
/// # (◍•ᴗ•◍)
/// - this structure should be instantiated by each level
#[derive(Default,Debug)]
pub struct LevelTime {
    // (f64,f64) means finished time and gap between finished time and first coming time
    pub level_finished_time: UnsafeCell<Vec<Vec<(f64, f64)>>>,
}
impl LevelTime {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn add_level(&self) -> usize {
        unsafe {
            let vec = &mut *self.level_finished_time.get();
            vec.push(vec![]);
            vec.len() - 1
        }
    }
    /// # ( ´◔ ‸◔`)
    /// take care! the level_id should be valid that returned by add_level
    /// # Safety
    /// the level_id should be valid
    /// -  return
    ///  the (finished time, gap time) of all target rows of this level
    pub unsafe fn get_finished_time(&self, level_id: usize) -> &Vec<(f64, f64)> {
        (*self.level_finished_time.get()).get_unchecked(level_id)
    }
    /// # (˶˚ ᗨ ˚˶)
    /// take care! the level_id should be valid that returned by add_level
    /// # Safety
    /// the level_id should be valid
    /// # args
    /// - time: the vec of (finished time, gap time)
    pub unsafe fn set_finished_time(&self, level_id: usize, time: Vec<(f64, f64)>) {
        *(*self.level_finished_time.get()).get_unchecked_mut(level_id) = time;
    }
    /// # (｡•ᴗ-)_
    /// take care! the level_id should be valid that returned by add_level
    /// # Safety
    /// the level_id should be valid
    /// # args:
    /// - time: the (finished time, gap time)
    pub unsafe fn add_finished_time(&self, level_id: usize, time: (f64, f64)) {
        // add the idle time to current idle time

        (*self.level_finished_time.get())
            .get_unchecked_mut(level_id)
            .push(time);
    }
}
/// # ( Ꙭ) this component contains the overall time for the whole simulation
#[derive(Debug, Default)]
pub struct SimTime {
    pub bank_read: f64,
    pub bank_merge: f64,
    pub chip_merge: f64,
    pub channel_merge: f64,
    pub dimm_merge: f64,
}
/// this structure is used to record the simulation time of each banks time break!
#[derive(Debug, Default)]
pub struct SharedSimTime {
   pub inner: UnsafeCell<SimTime>,
}

impl SharedSimTime {
    pub fn new() -> Self {
        Default::default()
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_time() {
        let comp_time = ComponentTime::new();
        let comp1 = comp_time.add_component();
        let comp2 = comp_time.add_component();
        unsafe {
            comp_time.add_idle_time(comp1, 1.3);
            comp_time.add_idle_time(comp1, 1.3);
            comp_time.add_idle_time(comp2, 2.3);
        }
        unsafe {
            assert_eq!(comp_time.get_idle_time(comp1), &vec![1.3, 1.3]);
            assert_eq!(comp_time.get_idle_time(comp2), &vec![2.3]);
        }
    }

    #[test]
    fn test_level_time() {
        let level_time = LevelTime::new();
        let level1 = level_time.add_level();
        let level2 = level_time.add_level();
        unsafe {
            level_time.add_finished_time(level1, (1.3, 1.3));
            level_time.add_finished_time(level1, (2.3, 2.3));
            level_time.add_finished_time(level2, (3.3, 3.3));
        }
        unsafe {
            assert_eq!(
                level_time.get_finished_time(level1),
                &vec![(1.3, 1.3), (2.3, 2.3)]
            );
            assert_eq!(level_time.get_finished_time(level2), &vec![(3.3, 3.3)]);
        }
    }
}
