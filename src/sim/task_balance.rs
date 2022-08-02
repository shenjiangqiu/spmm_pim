use std::iter::Enumerate;

use itertools::Itertools;
use rand::seq::SliceRandom;
use sprs::CsMat;

use crate::csv_nodata::CsVecNodata;

use super::id_translation::BankID;

pub trait TaskScheduler {
    fn build(data: Vec<CsVecNodata<usize>>) -> Self;
    fn get_next_row(&mut self) -> Option<(usize, CsVecNodata<usize>)>;
}

pub struct DefaultTaskScheduler {
    data: Enumerate<std::vec::IntoIter<CsVecNodata<usize>>>,
}
impl TaskScheduler for DefaultTaskScheduler {
    fn build(data: Vec<CsVecNodata<usize>>) -> Self {
        Self {
            data: data.into_iter().enumerate(),
        }
    }

    fn get_next_row(&mut self) -> Option<(usize, CsVecNodata<usize>)> {
        self.data.next()
    }
}

pub struct RandomTaskScheduler {
    data: std::vec::IntoIter<(usize, CsVecNodata<usize>)>,
}
impl TaskScheduler for RandomTaskScheduler {
    fn build(data: Vec<CsVecNodata<usize>>) -> Self {
        let mut iter = data.into_iter().enumerate().collect_vec();
        let mut rng = rand::thread_rng();
        iter.shuffle(&mut rng);
        Self {
            data: iter.into_iter(),
        }
    }

    fn get_next_row(&mut self) -> Option<(usize, CsVecNodata<usize>)> {
        self.data.next()
    }
}

pub struct TaskBanlance {
    channel_tasks: Vec<usize>,
    channel_tasks_with_weight: Vec<usize>,
    chip_tasks: Vec<usize>,
    chip_tasks_with_weight: Vec<usize>,
    bank_tasks: Vec<usize>,
    bank_tasks_with_weight: Vec<usize>,
}
impl TaskBanlance {
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
