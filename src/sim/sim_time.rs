//! # *⸜( •ᴗ• )⸝*  this module contains the simulation time of each component
//!
//! - the use rate for each component
//!   - get the idle time for each component
//! - the finished time for each row
//!   - get the last finished time for each level!
//! - the unbanlence of each row
//!   - get every level's max time and min time

use std::{cell::RefCell, collections::BTreeMap};

use itertools::Itertools;
use log::info;
use serde::Serialize;

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
    /// vec of <name, tags, time>
    data: RefCell<Vec<(String, Vec<String>, NamedTime)>>,
}

#[derive(Default, Debug)]
pub struct SharedEndTime {
    data: RefCell<Vec<(String, f64)>>,
}
#[derive(Debug, Clone, Copy)]
pub struct EndTimeId {
    pub id: usize,
}
#[derive(Serialize)]
pub struct AllTimeStats {
    pub data: Vec<(String, Vec<(String, f64)>)>,
}

impl SharedEndTime {
    pub fn add_component_with_name(&self, name: impl Into<String>) -> EndTimeId {
        self.data.borrow_mut().push((name.into(), 0.0));
        EndTimeId {
            id: self.data.borrow().len() - 1,
        }
    }
    pub fn set_end_time(&self, id: EndTimeId, time: f64) {
        self.data.borrow_mut()[id.id].1 = time;
    }
    pub fn get_end_time(&self, id: EndTimeId) -> f64 {
        self.data.borrow()[id.id].1
    }
    pub fn get_stats(&self, time: f64) -> Vec<(String, f64)> {
        self.data
            .borrow()
            .iter()
            .map(|(name, real_time)| (name.clone(), real_time / time))
            .collect()
    }
}

/// the id to identify a component, use this instead of usize to prevent using arbitrary id or id created by other StaticSimTime.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedTimeId {
    inner: usize,
}
#[derive(Serialize)]
pub struct TimeStats {
    /// real time, total time
    pub status: BTreeMap<String, (f64, f64)>,
}

#[derive(Serialize)]
pub struct DetailedTimeStats {
    /// real time, total time
    pub status: BTreeMap<String, Vec<(f64, f64)>>,
}
impl DetailedTimeStats {
    pub fn to_rate(self) -> Vec<(String, Vec<f64>)> {
        self.status
            .into_iter()
            .map(|(name, stats)| {
                (
                    name,
                    stats
                        .into_iter()
                        .map(|(real, total)| real / total)
                        .collect(),
                )
            })
            .collect_vec()
    }
}

impl TimeStats {
    pub fn to_rate(self) -> Vec<(String, f64)> {
        self.status
            .into_iter()
            .map(|(name, (idle, total))| {
                let rate = idle / total;
                (name, rate)
            })
            .collect()
    }
}

impl SharedNamedTime {
    pub fn new() -> Self {
        SharedNamedTime {
            data: RefCell::new(Vec::new()),
        }
    }

    /// ## return
    ///- the id of the named time(for a new component)
    /// ## description
    ///- the id should be used to add idle time later in the component by using `add_idle_time`
    #[allow(dead_code)]
    fn add_component(&self) -> NamedTimeId {
        let default_name = "default".to_string();
        self.add_component_with_name(default_name, vec!["default"])
    }

    /// ## return
    pub fn add_component_with_name(
        &self,
        name: impl Into<String>,
        tags: Vec<impl Into<String>>,
    ) -> NamedTimeId {
        let mut data = self.data.borrow_mut();
        data.push((
            name.into(),
            tags.into_iter().map(|x| x.into()).collect(),
            NamedTime::new(),
        ));
        NamedTimeId {
            inner: data.len() - 1,
        }
    }

    /// ## Safety
    ///-  the id should be **valid**( which should be created by `add_component`)
    /// ## Parameters
    ///- `id`: the id of the component in the array
    ///- `name`: the name of idle time that need to be added
    ///- `idle_time`: the idle time need to be added to the old one
    pub fn add_idle_time(&self, id: &NamedTimeId, name: &str, idle_time: f64) {
        let mut data = self.data.borrow_mut();
        data.get_mut(id.inner)
            .unwrap()
            .2
            .add_idle_time(name, idle_time);
    }

    pub fn show_data(&self, sim_time: f64) {
        info!("total_time: {}", sim_time);
        let data = self.data.borrow_mut();
        for (name, _tags, time) in data.iter() {
            info!("{}", name);
            time.show_data(sim_time);
        }
    }

    pub fn get_stats(&self, sim_time: f64) -> TimeStats {
        let mut stats = TimeStats {
            status: BTreeMap::new(),
        };
        let data = self.data.borrow();
        for (_name, tags, time) in data.iter() {
            for tag_name in tags {
                for (inner_name, inner_time) in &time.data {
                    let full_name = format!("{}:{}", tag_name, inner_name);
                    let mut entry = stats.status.entry(full_name).or_insert((0.0, 0.0));
                    entry.0 += inner_time;
                    entry.1 += sim_time;
                }
            }
        }
        stats
    }

    pub fn get_detailed_stats(&self, sim_time: f64) -> DetailedTimeStats {
        let mut stats = DetailedTimeStats {
            status: BTreeMap::new(),
        };
        let data = self.data.borrow();
        for (_name, tags, time) in data.iter() {
            for tag_name in tags {
                for (inner_name, inner_time) in &time.data {
                    let full_name = format!("{}:{}", tag_name, inner_name);
                    let entry = stats.status.entry(full_name).or_insert(vec![]);
                    entry.push((*inner_time, sim_time));
                }
            }
        }
        stats
    }
}

/// a dynamic time statistics of a component
/// - it have multiple fields of idle times, which stored in a map indexed by a name.
/// ## usage
/// - you don't need to create a entry, just use `add_idle_time` to add a idle time. if it's the first time to add a idle time, it will create a new entry.
#[derive(Default, Debug)]
struct NamedTime {
    pub data: BTreeMap<String, f64>,
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
            info!(
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
    pub componet_idle_time: RefCell<Vec<(String, Vec<f64>)>>,
}
impl ComponentTime {
    pub fn new() -> Self {
        Default::default()
    }
    /// # (⸝⸝•‧̫•⸝⸝)
    pub fn add_component(&self, name: impl Into<String>) -> usize {
        let mut vec = self.componet_idle_time.borrow_mut();
        vec.push((name.into(), vec![]));
        vec.len() - 1
    }
    /// # (˶˚ ᗨ ˚˶)
    /// take care! the component_id should be valid that returned by add_component
    /// # Safety
    /// the component_id should be valid
    pub fn get_idle_time(&self, component_id: usize) -> (String, Vec<f64>) {
        let vec = self.componet_idle_time.borrow();
        let v = vec.get(component_id).unwrap();
        v.clone()
    }

    /// # (Ծ‸Ծ)
    /// take care! the component_id should be valid that returned by add_component
    /// # Safety
    /// the component_id should be valid
    pub fn add_idle_time(&self, component_id: usize, idle_time: f64) {
        // add the idle time to current idle time

        self.componet_idle_time
            .borrow_mut()
            .get_mut(component_id)
            .unwrap()
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
    pub level_finished_time: RefCell<Vec<Vec<(f64, f64)>>>,
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
        let mut vec = self.level_finished_time.borrow_mut();
        vec.push(vec![]);
        LevelTimeId {
            inner: vec.len() - 1,
        }
    }
    /// # ( ´◔ ‸◔`)
    /// take care! the level_id should be valid that returned by add_level
    /// # Safety
    /// the level_id should be valid
    /// -  return
    ///  the (finished time, gap time) of all target rows of this level
    pub fn get_finished_time(&self, level_id: LevelTimeId) -> Vec<(f64, f64)> {
        self.level_finished_time
            .borrow()
            .get(level_id.inner)
            .unwrap()
            .clone()
    }
    /// # (˶˚ ᗨ ˚˶)
    /// take care! the level_id should be valid that returned by add_level
    /// # Safety
    /// the level_id should be valid
    /// # args
    /// - time: the vec of (finished time, gap time)
    pub fn set_finished_time(&self, level_id: LevelTimeId, time: Vec<(f64, f64)>) {
        *self
            .level_finished_time
            .borrow_mut()
            .get_mut(level_id.inner)
            .unwrap() = time;
    }
    /// # (｡•ᴗ-)_
    /// take care! the level_id should be valid that returned by add_level
    /// # Safety
    /// the level_id should be valid
    /// # args:
    /// - time: the (finished time, gap time)
    pub fn add_finished_time(&self, level_id: LevelTimeId, time: (f64, f64)) {
        // add the idle time to current idle time

        self.level_finished_time
            .borrow_mut()
            .get_mut(level_id.inner)
            .unwrap()
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
    pub inner: RefCell<SimTime>,
}

impl SharedSimTime {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_bank_read(&self, time: f64) {
        let mut inner = self.inner.borrow_mut();
        inner.bank_read += time;
    }
    pub fn get_bank_read(&self) -> f64 {
        let inner = self.inner.borrow();
        inner.bank_read
    }
    pub fn add_bank_merge(&self, time: f64) {
        let mut inner = self.inner.borrow_mut();
        inner.bank_merge += time;
    }
    pub fn get_bank_merge(&self) -> f64 {
        let inner = self.inner.borrow();
        inner.bank_merge
    }
    pub fn add_chip_merge(&self, time: f64) {
        let mut inner = self.inner.borrow_mut();
        inner.chip_merge += time;
    }
    pub fn get_chip_merge(&self) -> f64 {
        let inner = self.inner.borrow();
        inner.chip_merge
    }

    pub fn add_channel_merge(&self, time: f64) {
        let mut inner = self.inner.borrow_mut();
        inner.channel_merge += time;
    }
    pub fn get_channel_merge(&self) -> f64 {
        let inner = self.inner.borrow();
        inner.channel_merge
    }

    pub fn add_dimm_merge(&self, time: f64) {
        let mut inner = self.inner.borrow_mut();
        inner.dimm_merge += time;
    }
    pub fn get_dimm_merge(&self) -> f64 {
        let inner = self.inner.borrow();
        inner.dimm_merge
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
        comp_time.add_idle_time(comp1, 1.3);
        comp_time.add_idle_time(comp1, 1.3);
        comp_time.add_idle_time(comp2, 2.3);
        assert_eq!(comp_time.get_idle_time(comp1).1, vec![1.3, 1.3]);
        assert_eq!(comp_time.get_idle_time(comp2).1, vec![2.3]);
    }

    #[test]
    fn test_level_time() {
        let level_time = LevelTime::new();
        let level1 = level_time.add_level();
        let level2 = level_time.add_level();
        level_time.add_finished_time(level1, (1.3, 1.3));
        level_time.add_finished_time(level1, (2.3, 2.3));
        level_time.add_finished_time(level2, (3.3, 3.3));
        assert_eq!(
            level_time.get_finished_time(level1),
            vec![(1.3, 1.3), (2.3, 2.3)]
        );
        assert_eq!(level_time.get_finished_time(level2), vec![(3.3, 3.3)]);
    }
}
