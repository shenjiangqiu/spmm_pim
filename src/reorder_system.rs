use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Req {
    pub from: u32,
    pub to: u32,
}
impl Req {
    pub fn new(from: u32, to: u32) -> Self {
        Req { from, to }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AddResult {
    Ok,
    TooManyTarget,
    TooManySource,
}

#[derive(Debug)]
pub struct ReorderSystem {
    // the number of parallel target node to be merged
    pub parallel_count: u32,
    pub working_set: BTreeMap<u32, Vec<u32>>,
    pub buffer_size: usize,
}

impl Default for ReorderSystem {
    fn default() -> Self {
        ReorderSystem {
            parallel_count: 4,
            working_set: BTreeMap::new(),
            buffer_size: 64,
        }
    }
}

impl ReorderSystem {
    /// ## create a new reorder system with max parallel count: `parallel_count`
    pub fn new(parallel_count: u32, buffer_size: usize) -> Self {
        ReorderSystem {
            parallel_count,
            buffer_size,
            ..Default::default()
        }
    }

    /// ## add a new working set
    pub fn add_req(&mut self, req: Req) -> AddResult {
        let from = req.from;
        let to = req.to;

        let key = to;
        let value = from;
        let total_size: usize = self.working_set.values().map(Vec::len).sum();

        if let Some(vec) = self.working_set.get_mut(&key) {
            // add to existing working set
            if total_size < self.buffer_size {
                vec.push(value);
            } else {
                return AddResult::TooManySource;
            }
        } else if self.working_set.len() < self.parallel_count as usize {
            self.working_set.insert(key, vec![value]);
        } else {
            return AddResult::TooManyTarget;
        }

        AddResult::Ok
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test() {
        let mut system = ReorderSystem::new(2, 4);
        let req = Req::new(1, 2);
        assert_eq!(system.add_req(req), AddResult::Ok);
        let req = Req::new(2, 3);
        assert_eq!(system.add_req(req), AddResult::Ok);
        let req = Req::new(3, 4);
        assert_eq!(system.add_req(req), AddResult::TooManyTarget);
    }

    #[test]
    fn float_test() {
        let mut numbers = vec![f64::NAN, 1.0, 2.0];
        numbers.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        println!("{:?}", numbers);
    }
}
