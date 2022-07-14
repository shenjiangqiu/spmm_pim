//! # *⸜( •ᴗ• )⸝*  this module contains the simulation time of each component
//!
//! - the use rate for each component
//!   - get the idle time for each component
//! - the finished time for each row
//!   - get the last finished time for each level!
//! - the unbanlence of each row
//!   - get every level's max time and min time

use std::{cell::UnsafeCell, collections::BTreeMap};

/// the time statistics of all components
///
/// in this struct, a vec of `NamedTime` is stored. each `NamedTime` is a component's time statistics.
/// to add a new component to this struct, you can use the `add_component` method.
/// and later to update the time statistics of a component, you can use the `add_idle_time` method.
///
/// ## Safety
/// it use UnsafeCell to store the time statistics of each component in order to make it shared by all components immutably.
/// see the unsafe block in the functions for more information.
#[derive(Default, Debug)]
pub struct SharedNamedTime {
    data: UnsafeCell<Vec<(String, NamedTime)>>,
}

/// the id to identify a component, use this instead of usize to prevent using arbitrary id or id created by other StaticSimTime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NamedTimeId {
    inner: usize,
}

impl SharedNamedTime {
    pub fn new() -> Self {
        SharedNamedTime {
            data: UnsafeCell::new(Vec::new()),
        }
    }

    /// ## return
    ///- the id of the named time(for a new component)
    /// ## description
    ///- the id should be used to add idle time later in the component by using `add_idle_time`
    pub fn add_component(&self) -> NamedTimeId {
        let default_name = "default".to_string();
        self.add_component_with_name(default_name)
    }

    pub fn add_component_with_name(&self, name: impl Into<String>) -> NamedTimeId {
        unsafe {
            let data = &mut *self.data.get();
            data.push((name.into(), NamedTime::new()));
            NamedTimeId {
                inner: data.len() - 1,
            }
        }
    }

    /// ## Safety
    ///-  the id should be **valid**( which should be created by `add_component`)
    /// ## Parameters
    ///- `id`: the id of the component in the array
    ///- `name`: the name of idle time that need to be added
    ///- `idle_time`: the idle time need to be added to the old one
    pub unsafe fn add_idle_time(&self, id: NamedTimeId, name: &str, idle_time: f64) {
        let data = &mut *self.data.get();
        data.get_unchecked_mut(id.inner)
            .1
            .add_idle_time(name, idle_time);
    }

    pub fn show_data(&self, sim_time: f64) {
        unsafe {
            let data = &*self.data.get();
            for (name, time) in data.iter() {
                println!("{}", name);
                time.show_data(sim_time);
            }
        }
    }
}

/// a dynamic time statistics of a component
/// - it have multiple fields of idle times, which stored in a map indexed by a name.
/// ## usage
/// - you don't need to create a entry, just use `add_idle_time` to add a idle time. if it's the first time to add a idle time, it will create a new entry.
#[derive(Default, Debug)]
struct NamedTime {
    data: BTreeMap<String, f64>,
}
impl NamedTime {
    fn new() -> Self {
        NamedTime {
            data: BTreeMap::new(),
        }
    }

    fn show_data(&self, sim_time: f64) {
        let total_time: f64 = self.data.iter().map(|(_n, t)| t).sum();
        for (name, time) in self.data.iter() {
            println!(
                "{}: {}: {:.1}% :{:.1}%",
                name,
                time,
                time / total_time * 100.0,
                time / sim_time * 100.0
            );
        }
    }
    fn add_idle_time(&mut self, name: &str, idle_time: f64) {
        // if contains the name, add the time, else create a new one
        if let Some(time) = self.data.get_mut(name) {
            *time += idle_time;
        } else {
            self.data.insert(name.to_string(), idle_time);
        }
    }
}

/// # ( Ꙭ) this component contains the idle time of each component
/// - this structure should be instantiated by each component
#[derive(Default, Debug)]
pub struct ComponentTime {
    pub componet_idle_time: UnsafeCell<Vec<(String, Vec<f64>)>>,
}
impl ComponentTime {
    pub fn new() -> Self {
        Default::default()
    }
    /// # (⸝⸝•‧̫•⸝⸝)
    pub fn add_component(&self, name: impl Into<String>) -> usize {
        unsafe {
            let vec = &mut *self.componet_idle_time.get();
            vec.push((name.into(), vec![]));
            vec.len() - 1
        }
    }
    /// # (˶˚ ᗨ ˚˶)
    /// take care! the component_id should be valid that returned by add_component
    /// # Safety
    /// the component_id should be valid
    pub unsafe fn get_idle_time(&self, component_id: usize) -> &(String, Vec<f64>) {
        (*self.componet_idle_time.get()).get_unchecked(component_id)
    }

    /// # (Ծ‸Ծ)
    /// take care! the component_id should be valid that returned by add_component
    /// # Safety
    /// the component_id should be valid
    pub unsafe fn add_idle_time(&self, component_id: usize, idle_time: f64) {
        // add the idle time to current idle time

        (*self.componet_idle_time.get())
            .get_unchecked_mut(component_id)
            .1
            .push(idle_time);
    }
}

/// this component is used to record the finished time of each level(like bank, chip, channel)
/// # (◍•ᴗ•◍)
/// - this structure should be instantiated by each level
#[derive(Default, Debug)]
pub struct LevelTime {
    // (f64,f64) means finished time and gap between finished time and first coming time
    pub level_finished_time: UnsafeCell<Vec<Vec<(f64, f64)>>>,
}

/// levelTimeId is used to identify a level, use this instead of usize to prevent using arbitrary id or id created by other StaticSimTime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LevelTimeId {
    inner: usize,
}

impl LevelTime {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn add_level(&self) -> LevelTimeId {
        unsafe {
            let vec = &mut *self.level_finished_time.get();
            vec.push(vec![]);
            LevelTimeId {
                inner: vec.len() - 1,
            }
        }
    }
    /// # ( ´◔ ‸◔`)
    /// take care! the level_id should be valid that returned by add_level
    /// # Safety
    /// the level_id should be valid
    /// -  return
    ///  the (finished time, gap time) of all target rows of this level
    pub unsafe fn get_finished_time(&self, level_id: LevelTimeId) -> &Vec<(f64, f64)> {
        (*self.level_finished_time.get()).get_unchecked(level_id.inner)
    }
    /// # (˶˚ ᗨ ˚˶)
    /// take care! the level_id should be valid that returned by add_level
    /// # Safety
    /// the level_id should be valid
    /// # args
    /// - time: the vec of (finished time, gap time)
    pub unsafe fn set_finished_time(&self, level_id: LevelTimeId, time: Vec<(f64, f64)>) {
        *(*self.level_finished_time.get()).get_unchecked_mut(level_id.inner) = time;
    }
    /// # (｡•ᴗ-)_
    /// take care! the level_id should be valid that returned by add_level
    /// # Safety
    /// the level_id should be valid
    /// # args:
    /// - time: the (finished time, gap time)
    pub unsafe fn add_finished_time(&self, level_id: LevelTimeId, time: (f64, f64)) {
        // add the idle time to current idle time

        (*self.level_finished_time.get())
            .get_unchecked_mut(level_id.inner)
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
    pub fn get_bank_read(&self) -> f64 {
        unsafe {
            let inner = &*self.inner.get();
            inner.bank_read
        }
    }
    pub fn add_bank_merge(&self, time: f64) {
        unsafe {
            let mut inner = &mut *self.inner.get();
            inner.bank_merge += time;
        }
    }
    pub fn get_bank_merge(&self) -> f64 {
        unsafe {
            let inner = &*self.inner.get();
            inner.bank_merge
        }
    }
    pub fn add_chip_merge(&self, time: f64) {
        unsafe {
            let mut inner = &mut *self.inner.get();
            inner.chip_merge += time;
        }
    }
    pub fn get_chip_merge(&self) -> f64 {
        unsafe {
            let inner = &*self.inner.get();
            inner.chip_merge
        }
    }

    pub fn add_channel_merge(&self, time: f64) {
        unsafe {
            let mut inner = &mut *self.inner.get();
            inner.channel_merge += time;
        }
    }
    pub fn get_channel_merge(&self) -> f64 {
        unsafe {
            let inner = &*self.inner.get();
            inner.channel_merge
        }
    }

    pub fn add_dimm_merge(&self, time: f64) {
        unsafe {
            let mut inner = &mut *self.inner.get();
            inner.dimm_merge += time;
        }
    }
    pub fn get_dimm_merge(&self) -> f64 {
        unsafe {
            let inner = &*self.inner.get();
            inner.dimm_merge
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_time() {
        let comp_time = ComponentTime::new();
        let comp1 = comp_time.add_component("comp1".to_string());
        let comp2 = comp_time.add_component("comp2".to_string());
        unsafe {
            comp_time.add_idle_time(comp1, 1.3);
            comp_time.add_idle_time(comp1, 1.3);
            comp_time.add_idle_time(comp2, 2.3);
        }
        unsafe {
            assert_eq!(comp_time.get_idle_time(comp1).1, vec![1.3, 1.3]);
            assert_eq!(comp_time.get_idle_time(comp2).1, vec![2.3]);
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
