use std::iter::Enumerate;

use itertools::Itertools;
use rand::seq::SliceRandom;
use sprs::CsMat;

use crate::csv_nodata::CsVecNodata;

use super::id_translation::BankID;

pub struct DefaultTaskScheduler {
    data: Enumerate<std::vec::IntoIter<CsVecNodata<usize>>>,
}
impl DefaultTaskScheduler {
    pub fn new(data: Vec<CsVecNodata<usize>>) -> Self {
        Self {
            data: data.into_iter().enumerate(),
        }
    }
}
impl IntoIterator for DefaultTaskScheduler {
    type Item = (usize, CsVecNodata<usize>);

    type IntoIter = Enumerate<std::vec::IntoIter<CsVecNodata<usize>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.data
    }
}

pub struct RandomTaskScheduler {
    data: std::vec::IntoIter<(usize, CsVecNodata<usize>)>,
}
impl RandomTaskScheduler {
    pub fn new(data: Vec<CsVecNodata<usize>>) -> Self {
        let mut iter = data.into_iter().enumerate().collect_vec();
        let mut rng = rand::thread_rng();
        iter.shuffle(&mut rng);
        Self {
            data: iter.into_iter(),
        }
    }
}
impl IntoIterator for RandomTaskScheduler {
    type Item = (usize, CsVecNodata<usize>);

    type IntoIter = std::vec::IntoIter<(usize, CsVecNodata<usize>)>;

    fn into_iter(self) -> Self::IntoIter {
        self.data
    }
}

pub struct BatchShuffleScheduler {
    iter_data: Vec<(usize, CsVecNodata<usize>)>,
}
impl BatchShuffleScheduler {
    pub fn new(chunk_size: usize, data: Vec<CsVecNodata<usize>>) -> Self {
        let grouped_task = data.into_iter().enumerate().chunks(chunk_size);
        let mut grouped_task = grouped_task.into_iter().collect_vec();

        let mut rng = rand::thread_rng();
        grouped_task.shuffle(&mut rng);
        let task_iter = grouped_task.into_iter().flatten().collect_vec();
        Self {
            iter_data: task_iter,
        }
    }
}
impl IntoIterator for BatchShuffleScheduler {
    type Item = (usize, CsVecNodata<usize>);

    fn into_iter(self) -> Self::IntoIter {
        self.iter_data.into_iter()
    }

    type IntoIter = std::vec::IntoIter<(usize, CsVecNodata<usize>)>;
}
#[allow(dead_code)]
pub struct TaskBanlance {
    channel_tasks: Vec<usize>,
    channel_tasks_with_weight: Vec<usize>,
    chip_tasks: Vec<usize>,
    chip_tasks_with_weight: Vec<usize>,
    bank_tasks: Vec<usize>,
    bank_tasks_with_weight: Vec<usize>,
}
impl TaskBanlance {
    #[allow(dead_code)]

    pub fn new(channels: usize, chips: usize, banks: usize) -> Self {
        let channel_tasks = vec![0; channels];
        let channel_tasks_with_weight = vec![0; channels];
        let chip_tasks = vec![0; chips];
        let chip_tasks_with_weight = vec![0; chips];
        let bank_tasks = vec![0; banks];
        let bank_tasks_with_weight = vec![0; banks];
        Self {
            channel_tasks,
            channel_tasks_with_weight,
            chip_tasks,
            chip_tasks_with_weight,
            bank_tasks,
            bank_tasks_with_weight,
        }
    }
    #[allow(dead_code)]

    pub fn add_task(&mut self, bank_id: &BankID, source_id: usize, mat_b: &CsMat<i32>) {
        let ((channel_id, chip_id), bank_id) = bank_id;
        let b_len = mat_b.outer_view(source_id).unwrap().nnz();

        self.channel_tasks[*channel_id] += 1;
        self.channel_tasks_with_weight[*channel_id] += b_len;
        self.chip_tasks[*chip_id] += 1;
        self.chip_tasks_with_weight[*chip_id] += b_len;
        self.bank_tasks[*bank_id] += 1;
        self.bank_tasks_with_weight[*bank_id] += b_len;
    }
}
