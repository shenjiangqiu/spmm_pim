//! the pim module

use std::{fmt::Debug, ops::Deref};

use itertools::Itertools;
use log::debug;
use sprs::{vec, CsMatBase, SpIndex};

use crate::{bsr::Bsr, settings::MemSettings};

/// Partial sum
/// for each element in `data`
/// it contains the `(target_index, target_row_size)`

#[derive(Clone, Debug)]
pub struct PartialSum {
    pub data: Vec<(usize, usize)>,
}

impl From<Vec<(usize, usize)>> for PartialSum {
    fn from(data: Vec<(usize, usize)>) -> Self {
        PartialSum { data }
    }
}

impl Deref for PartialSum {
    type Target = Vec<(usize, usize)>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

pub trait Pim {
    fn mem_rows(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn bank_merge(&self, mem_settings: &MemSettings) -> (Vec<usize>, Vec<PartialSum>);
    fn bank_add(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn chip_add(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn chip_merge(
        &self,
        mem_settings: &MemSettings,
        bank_merge_result: &Vec<PartialSum>,
    ) -> (Vec<usize>, Vec<PartialSum>);
    fn channel_add(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn channel_merge(&self, mem_settings: &MemSettings) -> Vec<usize>;
}

fn get_bank_id_from_row_id(row_id: usize, mem_settings: &MemSettings, num_rows: usize) -> usize {
    let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;
    match mem_settings.row_mapping {
        crate::settings::RowMapping::Chunk => {
            let rows_per_bank = num_rows / num_banks;

            let bank_id = if rows_per_bank == 0 {
                0
            } else {
                row_id / rows_per_bank
            };
            if bank_id >= num_banks {
                row_id % num_banks
            } else {
                bank_id
            }
        }
        crate::settings::RowMapping::Interleaved => row_id % num_banks,
    }
}

fn get_row_id_in_bank(row_id: usize, mem_settings: &MemSettings, num_rows: usize) -> usize {
    let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;
    match mem_settings.row_mapping {
        crate::settings::RowMapping::Chunk => {
            let rows_per_bank = num_rows / num_banks;
            if rows_per_bank == 0 {
                row_id
            } else {
                row_id % rows_per_bank
            }
        }
        crate::settings::RowMapping::Interleaved => row_id / num_banks,
    }
}
#[derive(Debug, Clone)]
struct BankMergeTaskBuilder {
    // tasks: targets: (target id, row sizes)
    tasks: Vec<(usize, Vec<usize>)>,
    current_working_target: usize,
}

impl Default for BankMergeTaskBuilder {
    fn default() -> Self {
        BankMergeTaskBuilder {
            tasks: Vec::new(),
            current_working_target: usize::max_value(),
        }
    }
}

#[allow(dead_code)]
impl BankMergeTaskBuilder {
    fn new() -> Self {
        Self::default()
    }

    /// ## Add a new task to the builder.
    /// args:
    /// (to,size) : the target row and the source row size(nnzs)
    ///
    ///
    ///
    fn add_task(&mut self, (to, size): (usize, usize)) {
        if to == self.current_working_target {
            debug!("add_task: to == current_working_target, push the size to last task");
            self.tasks.last_mut().unwrap().1.push(size);
        } else {
            debug!("add_task: to != current_working_target, push a new task");
            self.tasks.push((to, vec![size]));
            self.current_working_target = to;
        }
        debug!("current tasks: {:?}", self.tasks);
    }

    /// return the cycles need to merge
    /// and the tasks that merged(the size of merged row for each target row)
    /// tasks is Vec<(usize,Vec<usize>)>
    /// output: cycle: usize
    ///        merged tasks: Vec<(usize,usize)>
    fn build(self, merger_size: usize) -> (usize, Vec<(usize, usize)>) {
        // TODO fix it! the size of the later task might be samller then simple add the nnzs
        debug!("starting to build the final cycles");

        let mut cycles = 0;
        let mut merged_tasks = Vec::new();
        for i in self.tasks {
            let mut i = i;
            // note that if the i's size is 1 at the begining! we do not merge it so the cycle will be zero for this task
            // it's not a bug!!!
            //
            while i.1.len() > 1 {
                debug!("start merging the task: {:?}", i);

                // TODO: fix it! when the size is 1, no need to count the cycles
                let result_vec =
                    i.1.iter()
                        .chunks(merger_size)
                        .into_iter()
                        .map(|x| x.into_iter().sum::<usize>())
                        .collect::<Vec<_>>();
                debug!("result_vec: {:?}", result_vec);
                debug!("cost: {:?}", result_vec.iter().sum::<usize>());
                cycles += result_vec.iter().sum::<usize>();
                i.1 = result_vec;
            }
            if i.1.len() == 1 {
                merged_tasks.push((i.0, i.1[0]));
            } else {
                merged_tasks.push((i.0, 0));
            }
        }
        (cycles, merged_tasks)
    }
}

impl<N, I, IptrStorage, IndStorage, DataStorage, Iptr> Pim
    for CsMatBase<N, I, IptrStorage, IndStorage, DataStorage, Iptr>
where
    N: Default + Copy + Clone + Debug,
    I: SpIndex,
    Iptr: SpIndex,
    IptrStorage: Deref<Target = [Iptr]>,
    IndStorage: Deref<Target = [I]>,
    DataStorage: Deref<Target = [N]>,
{
    fn mem_rows(&self, mem_settings: &MemSettings) -> Vec<usize> {
        let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;

        // fisrt calculate he access row stream of each bank
        let num_rows = self.rows();
        // contains the rows to read for each bank
        let mut row_stream = vec![vec![]; num_banks];
        // return the bank id and the row id in bank

        for i in self.iter() {
            debug!("i: {:?}", i);
            let row_select = i.1 .1.index();

            let bank_id = get_bank_id_from_row_id(row_select, mem_settings, num_rows);
            debug!("bank_id: {:?}", bank_id);
            let row_id_in_bank = get_row_id_in_bank(row_select, mem_settings, num_rows);
            debug!("row_id_in_bank: {:?}", row_id_in_bank);

            let row_size = self.outer_view(row_select).unwrap().nnz() * 4;
            debug!("row_size: {:?}", row_size);

            let row_buffer_size = mem_settings.row_size;
            for i in 0..((row_size + row_buffer_size - 1) / row_buffer_size) {
                row_stream[bank_id].push(row_id_in_bank + i);
                debug!("band_id: {:?} need read {:?}", bank_id, row_id_in_bank + i);
            }
        }
        debug!("finished build the stream, next count the rows for different banks");
        debug!("{:?}", row_stream);
        let result = row_stream
            .iter()
            .map(|v| {
                v.iter().fold((usize::MAX, 0), |acc, x| {
                    if *x == acc.0 {
                        acc
                    } else {
                        (*x, acc.1 + 1)
                    }
                })
            })
            .map(|(_, y)| y)
            .collect();
        debug!("{:?}", result);
        result
    }
    // return how many merge operations are needed
    fn bank_merge(&self, mem_settings: &MemSettings) -> (Vec<usize>, Vec<PartialSum>) {
        let merger_size = mem_settings.bank_merger_size;
        let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;
        let mut bank_tasks = vec![BankMergeTaskBuilder::default(); num_banks];

        for i in self.iter() {
            let row_select = i.1 .1.index();
            debug!("row_select: {:?}", row_select);
            let target_row = i.1 .0.index();
            debug!("target_row: {:?}", target_row);
            let bank_id = get_bank_id_from_row_id(row_select, mem_settings, self.rows());
            debug!("bank_id: {:?}", bank_id);
            let row_nnz = self.outer_view(row_select).unwrap().nnz();
            debug!("row_nnz: {:?}", row_nnz);

            bank_tasks[bank_id].add_task((target_row, row_nnz));
            debug!("bank_tasks: {:?}", bank_tasks);
        }
        let mut cycles = vec![];
        let mut merged_tasks = vec![];

        bank_tasks
            .into_iter()
            .map(|x| x.build(merger_size))
            .for_each(|x| {
                cycles.push(x.0);
                merged_tasks.push(x.1.into());
            });

        (cycles, merged_tasks)
    }

    fn bank_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        return vec![0; mem_settings.banks * mem_settings.chips * mem_settings.channels];
    }

    fn chip_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        return vec![0; mem_settings.chips * mem_settings.channels];
    }

    fn chip_merge(
        &self,
        mem_settings: &MemSettings,
        bank_merge_result: &Vec<PartialSum>,
    ) -> (Vec<usize>, Vec<PartialSum>) {
        // just like the bank merge, but istead take the result of bank level result
        for (bank_id, bank_result) in bank_merge_result.iter().enumerate() {

            // debug!("i: {:?}", i);
        }

        todo!()
    }

    fn channel_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn channel_merge(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }
}

impl<const R: usize, const C: usize, N, I, IptrStorage, IndStorage, DataStorage, Iptr> Pim
    for Bsr<R, C, N, I, Iptr, IptrStorage, IndStorage, DataStorage>
where
    I: SpIndex,
    Iptr: SpIndex,
    IptrStorage: Deref<Target = [Iptr]>,
    IndStorage: Deref<Target = [I]>,
    DataStorage: Deref<Target = [[[N; C]; R]]>,
{
    fn mem_rows(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn bank_merge(
        &self,
        mem_settings: &MemSettings,
    ) -> (std::vec::Vec<usize>, std::vec::Vec<PartialSum>) {
        todo!()
    }

    fn bank_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn chip_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn channel_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn channel_merge(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn chip_merge(
        &self,
        mem_settings: &MemSettings,
        bank_merge_result: &std::vec::Vec<PartialSum>,
    ) -> (std::vec::Vec<usize>, std::vec::Vec<PartialSum>) {
        todo!()
    }
}

#[cfg(test)]
mod pimtest {
    use log::debug;
    use sprs::{CsMat, TriMat};

    use crate::{settings::MemSettings, utils::init_log};

    use super::Pim;

    #[test]
    fn test_pim() {
        init_log("debug");
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx").unwrap();
        let csr: CsMat<_> = matrix.to_csr();
        let mem_settings = MemSettings {
            banks: 4,
            row_mapping: crate::settings::RowMapping::Chunk,
            row_size: 4,
            chips: 1,
            channels: 1,
            bank_merger_size: 2,
            chip_merger_size: 2,
            channel_merger_size: 2,
        };

        let result = csr.mem_rows(&mem_settings);
        println!("{:?}", result);
    }

    #[test]
    fn test_merge() {
        init_log("debug");
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx").unwrap();
        let csr: CsMat<_> = matrix.to_csr();
        let mem_settings = MemSettings {
            banks: 2,
            row_mapping: crate::settings::RowMapping::Chunk,
            row_size: 4,
            chips: 1,
            channels: 1,
            bank_merger_size: 2,
            chip_merger_size: 2,
            channel_merger_size: 2,
        };

        let result = csr.bank_merge(&mem_settings);
        debug!("{:?}", result);
    }
}
