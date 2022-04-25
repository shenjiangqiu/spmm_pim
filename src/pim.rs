//! the pim module

use std::{collections::BTreeMap, fmt::Debug, mem, ops::Deref};

use itertools::Itertools;
use log::debug;
use sprs::{CsMatBase, SpIndex};

use crate::{bsr::Bsr, csv_nodata::CsVecNodata, settings::MemSettings};

/// Partial sum
/// for each element in `data`
/// it contains the `(target_index, target_row_size)`

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartialSumSize {
    pub data: Vec<(usize, usize)>,
}

impl From<Vec<(usize, usize)>> for PartialSumSize {
    fn from(data: Vec<(usize, usize)>) -> Self {
        PartialSumSize { data }
    }
}

impl Deref for PartialSumSize {
    type Target = Vec<(usize, usize)>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[allow(dead_code)]
impl PartialSumSize {
    fn new() -> Self {
        PartialSumSize { data: vec![] }
    }
    fn add_data(&mut self, data: Vec<(usize, usize)>) {
        self.data.extend(data);
    }
    fn add_item(&mut self, item: (usize, usize)) {
        self.data.push(item);
    }
}

/// Partial sum
/// for each element in `data`
/// it contains the `(target_index, target_row_size)`

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartialSum<I>
where
    I: SpIndex,
{
    pub data: Vec<(usize, CsVecNodata<I>)>,
}

impl<I> From<Vec<(usize, CsVecNodata<I>)>> for PartialSum<I>
where
    I: SpIndex,
{
    fn from(data: Vec<(usize, CsVecNodata<I>)>) -> Self {
        PartialSum { data }
    }
}

impl<I> Deref for PartialSum<I>
where
    I: SpIndex,
{
    type Target = Vec<(usize, CsVecNodata<I>)>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[allow(dead_code)]
impl<I> PartialSum<I>
where
    I: SpIndex,
{
    fn new() -> Self {
        PartialSum { data: vec![] }
    }
    fn add_data(&mut self, data: Vec<(usize, CsVecNodata<I>)>) {
        self.data.extend(data);
    }
    fn add_item(&mut self, item: (usize, CsVecNodata<I>)) {
        self.data.push(item);
    }
}

pub trait Pim {
    fn mem_rows(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn bank_merge(&self, mem_settings: &MemSettings) -> (Vec<usize>, Vec<PartialSum<usize>>);
    fn bank_add(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn chip_add(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn chip_merge(
        &self,
        mem_settings: &MemSettings,
        bank_merge_result: &[PartialSum<usize>],
    ) -> (Vec<usize>, Vec<PartialSum<usize>>);
    fn channel_add(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn channel_merge(
        &self,
        mem_settings: &MemSettings,
        chip_merge_result: &[PartialSum<usize>],
    ) -> (Vec<usize>, Vec<PartialSum<usize>>);
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
struct AdderTaskBuilder<I>
where
    I: SpIndex,
{
    // tasks: targets: (target id, row sizes)
    tasks: Vec<(usize, Vec<CsVecNodata<I>>)>,
    current_working_target: usize,
}

impl<T> Default for AdderTaskBuilder<T>
where
    T: SpIndex,
{
    fn default() -> Self {
        AdderTaskBuilder {
            tasks: Vec::new(),
            current_working_target: usize::max_value(),
        }
    }
}

#[allow(dead_code)]
impl<T> AdderTaskBuilder<T>
where
    T: SpIndex,
{
    fn new() -> Self {
        Self::default()
    }

    /// ## Add a new task to the builder.
    /// args:
    /// (to,size) : the target row and the source row size(nnzs)
    ///
    ///
    ///
    fn add_task(&mut self, (to, vec): (usize, CsVecNodata<T>)) {
        if to == self.current_working_target {
            debug!("add_task: to == current_working_target, push the size to last task");
            self.tasks.last_mut().unwrap().1.push(vec);
        } else {
            debug!("add_task: to != current_working_target, push a new task");
            self.tasks.push((to, vec![vec]));
            self.current_working_target = to;
        }
    }

    /// return the cycles need to merge
    /// and the tasks that merged(the size of merged row for each target row)
    /// tasks is Vec<(usize,Vec<usize>)>
    /// output: cycle: usize
    ///        merged tasks: Vec<(usize,usize)>
    /// return (add_cycles, merge_cycles, Vec<target_id,result_vec>)
    fn build(self, merger_size: usize) -> ((usize, usize), Vec<(usize, CsVecNodata<T>)>) {
        // TODO fix it! the size of the later task might be samller then simple add the nnzs
        debug!("starting to build the final cycles");
        let mut add_cycles = 0;
        let mut merge_cycles = 0;
        let mut merged_tasks = vec![];
        for i in self.tasks {
            let mut i = i;
            // note that if the i's size is 1 at the begining! we do not merge it so the cycle will be zero for this task
            // it's not a bug!!!
            //
            while i.1.len() > 1 {
                debug!("start merging the task: {:?}", i);

                // TODO: fix it! when the size is 1, no need to count the cycles
                // now fixed, no worry!

                let result_vec =
                    i.1.into_iter()
                        .chunks(merger_size)
                        .into_iter()
                        .map(|x| {
                            x.fold((0, CsVecNodata::default()), |(total_len, cal_vec), y| {
                                (total_len + y.len(), cal_vec + y)
                            })
                        })
                        .collect_vec();

                debug!("result_vec: {:?}", result_vec);
                merge_cycles += result_vec.iter().map(|x| x.0).sum::<usize>();
                add_cycles += merge_cycles - result_vec.iter().map(|x| x.1.len()).sum::<usize>();

                i.1 = result_vec.into_iter().map(|x| x.1).collect();
            }

            merged_tasks.push((i.0, i.1[0].clone()));
        }
        ((add_cycles, merge_cycles), merged_tasks)
    }
}

// #[derive(Debug, Clone)]
// struct AdderTaskBuilder {
//     // tasks: targets: (target id, row sizes)
//     tasks: Vec<(usize, Vec<usize>)>,
//     current_working_target: usize,
// }

// impl Default for AdderTaskBuilder {
//     fn default() -> Self {
//         AdderTaskBuilder {
//             tasks: Vec::new(),
//             current_working_target: usize::max_value(),
//         }
//     }
// }

// #[allow(dead_code)]
// impl AdderTaskBuilder {
//     fn new() -> Self {
//         Self::default()
//     }

//     /// ## Add a new task to the builder.
//     /// args:
//     /// (to,size) : the target row and the source row size(nnzs)
//     ///
//     ///
//     ///
//     fn add_task(&mut self, (to, size): (usize, usize)) {
//         if to == self.current_working_target {
//             debug!("add_task: to == current_working_target, push the size to last task");
//             self.tasks.last_mut().unwrap().1.push(size);
//         } else {
//             debug!("add_task: to != current_working_target, push a new task");
//             self.tasks.push((to, vec![size]));
//             self.current_working_target = to;
//         }
//         debug!("current tasks: {:?}", self.tasks);
//     }

//     /// return the cycles need to merge
//     /// and the tasks that merged(the size of merged row for each target row)
//     /// tasks is Vec<(usize,Vec<usize>)>
//     /// output: cycle: usize
//     ///        merged tasks: Vec<(usize,usize)>
//     fn build(self, merger_size: usize) -> (usize, PartialSum<usize>) {
//         // TODO fix it! the size of the later task might be samller then simple add the nnzs
//         debug!("starting to build the final cycles");

//         let mut cycles = 0;
//         let mut merged_tasks = PartialSum::<usize>::new();
//         for i in self.tasks {
//             let mut i = i;
//             // note that if the i's size is 1 at the begining! we do not merge it so the cycle will be zero for this task
//             // it's not a bug!!!
//             //
//             while i.1.len() > 1 {
//                 debug!("start merging the task: {:?}", i);

//                 // TODO: fix it! when the size is 1, no need to count the cycles
//                 let result_vec =
//                     i.1.iter()
//                         .chunks(merger_size)
//                         .into_iter()
//                         .map(|x| x.into_iter().sum::<usize>())
//                         .collect::<Vec<_>>();
//                 debug!("result_vec: {:?}", result_vec);
//                 debug!("cost: {:?}", result_vec.iter().sum::<usize>());
//                 cycles += result_vec.iter().sum::<usize>();
//                 i.1 = result_vec;
//             }
//             if i.1.len() == 1 {
//                 merged_tasks.add_item((i.0, i.1[0]));
//             } else {
//                 merged_tasks.add_item((i.0, 0));
//             }
//         }
//         (cycles, merged_tasks)
//     }
// }

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

            let row_size = self.outer_view(row_select).unwrap().nnz() * mem::size_of::<N>();
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
    fn bank_merge(&self, mem_settings: &MemSettings) -> (Vec<usize>, Vec<PartialSum<usize>>) {
        let merger_size = mem_settings.bank_merger_size;
        let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;
        let mut bank_tasks = vec![AdderTaskBuilder::default(); num_banks];

        for i in self.iter() {
            let row_select = i.1 .1.index();
            debug!("row_select: {:?}", row_select);
            let target_row = i.1 .0.index();
            debug!("target_row: {:?}", target_row);
            let bank_id = get_bank_id_from_row_id(row_select, mem_settings, self.rows());
            debug!("bank_id: {:?}", bank_id);
            let row_nnz = self.outer_view(row_select).unwrap().nnz();
            debug!("row_nnz: {:?}", row_nnz);
            let input_row: CsVecNodata<_> = self.outer_view(row_select).unwrap().to_owned().into();
            bank_tasks[bank_id].add_task((target_row, input_row));
            debug!("bank_tasks: {:?}", bank_tasks);
        }
        let mut cycles = vec![];
        let mut merged_tasks = vec![];

        bank_tasks
            .into_iter()
            .map(|x| x.build(merger_size))
            .for_each(|x| {
                cycles.push(x.0);
                merged_tasks.push(x.1);
            });

        (cycles, merged_tasks.into())
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
        bank_merge_result: &[PartialSum<usize>],
    ) -> (Vec<usize>, Vec<PartialSum<usize>>) {
        // just like the bank merge, but istead take the result of bank level result
        let merger_size = mem_settings.chip_merger_size;
        let num_chips = mem_settings.chips * mem_settings.channels;

        let num_banks_per_chip = mem_settings.banks;

        assert_eq!(
            bank_merge_result.len(),
            mem_settings.chips * num_banks_per_chip * mem_settings.channels
        );

        let mut chip_tasks = vec![AdderTaskBuilder::default(); num_chips];

        // merge inside the chip
        for (chip_sum, chip_task) in bank_merge_result
            .chunks(num_banks_per_chip)
            .zip(&mut chip_tasks)
        {
            assert_eq!(chip_sum.len(), num_banks_per_chip);
            // build a set of all chip_sum
            let mut chip_sum_set: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            chip_sum.iter().for_each(|x| {
                x.deref().iter().for_each(|y| {
                    chip_sum_set.entry(y.0).or_insert(vec![]).push(y.1);
                });
            });
            // build the task from the map
            chip_sum_set.into_iter().for_each(|(k, v)| {
                v.iter().for_each(|size| {
                    chip_task.add_task((k, *size));
                });
            });
        }

        let mut cycles = vec![];
        let mut merged_tasks = vec![];

        chip_tasks
            .into_iter()
            .map(|x| x.build(merger_size))
            .for_each(|x| {
                cycles.push(x.0);
                merged_tasks.push(x.1);
            });

        assert_eq!(cycles.len(), num_chips);
        assert_eq!(merged_tasks.len(), num_chips);

        (cycles, merged_tasks)
    }

    fn channel_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        return vec![0; mem_settings.channels];
    }

    fn channel_merge(
        &self,
        mem_settings: &MemSettings,
        chip_merge_result: &[PartialSum<usize>],
    ) -> (Vec<usize>, Vec<PartialSum<usize>>) {
        let num_channel = mem_settings.channels;
        let num_chips = mem_settings.chips * mem_settings.channels;
        let num_chips_per_channel = mem_settings.chips;
        assert!(num_chips % num_channel == 0);
        assert_eq!(num_chips, chip_merge_result.len());

        let merger_size = mem_settings.channel_merger_size;
        let mut channel_tasks = vec![AdderTaskBuilder::default(); num_channel];

        // merge inside the channel

        for (channel_sum, channel_task) in chip_merge_result
            .chunks(num_chips_per_channel)
            .zip(&mut channel_tasks)
        {
            assert_eq!(channel_sum.len(), num_chips_per_channel);
            // build a set of all channel_sum
            let mut channel_sum_set: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            channel_sum.iter().for_each(|x| {
                x.deref().iter().for_each(|y| {
                    channel_sum_set.entry(y.0).or_insert(vec![]).push(y.1);
                });
            });
            // build the task from the map
            channel_sum_set.into_iter().for_each(|(k, v)| {
                v.iter().for_each(|size| {
                    channel_task.add_task((k, *size));
                });
            });
        }

        let mut cycles = vec![];
        let mut merged_tasks = vec![];

        channel_tasks
            .into_iter()
            .map(|x| x.build(merger_size))
            .for_each(|x| {
                cycles.push(x.0);
                merged_tasks.push(x.1);
            });

        assert_eq!(cycles.len(), num_channel);
        assert_eq!(merged_tasks.len(), num_channel);

        (cycles, merged_tasks)
    }
}

// do not use this one, the bsr can be translate to csr with no overhead, use csr instead
#[allow(dead_code, unused_variables)]
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
    ) -> (std::vec::Vec<usize>, std::vec::Vec<PartialSum<usize>>) {
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

    fn channel_merge(
        &self,
        mem_settings: &MemSettings,
        chip_merge_result: &[PartialSum<usize>],
    ) -> (Vec<usize>, Vec<PartialSum<usize>>) {
        todo!()
    }

    fn chip_merge(
        &self,
        mem_settings: &MemSettings,
        bank_merge_result: &[PartialSum<usize>],
    ) -> (std::vec::Vec<usize>, std::vec::Vec<PartialSum<usize>>) {
        todo!()
    }
}

#[cfg(test)]
mod pimtest {
    use sprs::{CsMat, TriMat};

    use crate::utils::init_log;
    use crate::{
        pim::{PartialSum, Pim},
        settings::MemSettings,
    };
    use log::debug;

    use super::AdderTaskBuilder;
    fn read_mtx() -> CsMat<i32> {
        init_log("debug");
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx").unwrap();
        let csr: CsMat<_> = matrix.to_csr();
        csr
    }

    #[test]
    fn test_adder_builder() {
        let adder_builder = AdderTaskBuilder::default();
    }

    mod csr {

        use super::*;

        #[test]
        fn test_pim() {
            let csr = read_mtx();
            let mem_settings = MemSettings {
                simd_width: 4,
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
            let csr = read_mtx();
            let mem_settings = MemSettings {
                simd_width: 4,
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
            assert_eq!(
                result,
                (
                    vec![17, 4],
                    vec![
                        PartialSum {
                            data: vec![(0, 7), (1, 3), (2, 1), (3, 2)]
                        },
                        PartialSum {
                            data: vec![(0, 3), (3, 4), (4, 1), (5, 1)]
                        }
                    ]
                )
            );
        }

        #[test]
        fn test_chip_merge() {
            let csr = read_mtx();
            let mem_settings = MemSettings {
                simd_width: 4,
                banks: 1,
                row_mapping: crate::settings::RowMapping::Chunk,
                row_size: 4,
                chips: 2,
                channels: 1,
                bank_merger_size: 2,
                chip_merger_size: 2,
                channel_merger_size: 2,
            };
            let bank_merge_result = csr.bank_merge(&mem_settings).1;
            debug!("bank result: {:?}", bank_merge_result);
            let result = csr.chip_merge(&mem_settings, &bank_merge_result);
            debug!("chip result: {:?}", result);
        }
        #[test]
        fn test_chip_merge2() {
            let csr = read_mtx();
            let mem_settings = MemSettings {
                simd_width: 4,
                banks: 2,
                row_mapping: crate::settings::RowMapping::Chunk,
                row_size: 4,
                chips: 1,
                channels: 1,
                bank_merger_size: 2,
                chip_merger_size: 2,
                channel_merger_size: 2,
            };
            let bank_merge_result = csr.bank_merge(&mem_settings).1;
            debug!("bank result: {:?}", bank_merge_result);
            let result = csr.chip_merge(&mem_settings, &bank_merge_result);
            debug!("chip result: {:?}", result);
        }

        #[test]
        fn test_channel_merge() {
            let csr = read_mtx();
            let mem_settings = MemSettings {
                simd_width: 4,
                banks: 1,
                row_mapping: crate::settings::RowMapping::Chunk,
                row_size: 4,
                chips: 1,
                channels: 2,
                bank_merger_size: 2,
                chip_merger_size: 2,
                channel_merger_size: 2,
            };
            let bank_merge_result = csr.bank_merge(&mem_settings).1;
            debug!("bank result: {:?}", bank_merge_result);
            let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result).1;
            debug!("chip result: {:?}", chip_merge_result);
            let result = csr.channel_merge(&mem_settings, &chip_merge_result);
            debug!("channel result: {:?}", result);
        }

        #[test]
        fn test_channel_merge2() {
            let csr = read_mtx();
            let mem_settings = MemSettings {
                simd_width: 4,
                banks: 2,
                row_mapping: crate::settings::RowMapping::Chunk,
                row_size: 4,
                chips: 1,
                channels: 1,
                bank_merger_size: 2,
                chip_merger_size: 2,
                channel_merger_size: 2,
            };
            let bank_merge_result = csr.bank_merge(&mem_settings).1;
            debug!("bank result: {:?}", bank_merge_result);
            let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result).1;
            debug!("chip result: {:?}", chip_merge_result);
            let result = csr.channel_merge(&mem_settings, &chip_merge_result);
            debug!("channel result: {:?}", result);
        }

        #[test]
        fn test_channel_merge3() {
            let csr = read_mtx();
            let mem_settings = MemSettings {
                simd_width: 4,
                banks: 1,
                row_mapping: crate::settings::RowMapping::Chunk,
                row_size: 4,
                chips: 2,
                channels: 1,
                bank_merger_size: 2,
                chip_merger_size: 2,
                channel_merger_size: 2,
            };
            let bank_merge_result = csr.bank_merge(&mem_settings).1;
            debug!("bank result: {:?}", bank_merge_result);
            let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result).1;
            debug!("chip result: {:?}", chip_merge_result);
            let result = csr.channel_merge(&mem_settings, &chip_merge_result);
            debug!("channel result: {:?}", result);
        }
    }

    mod bsr {

        use crate::bsr::Bsr;

        use super::*;

        #[test]
        fn test_pim() {
            let csr = read_mtx();
            let bsr = Bsr::<2, 2, _>::from(csr);
            debug!("bsr: {:?}", bsr);
            let csr: CsMat<_> = bsr.into();
            debug!("csr: {:?}", csr);

            let mem_settings = MemSettings {
                simd_width: 4,
                banks: 2,
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
            // let csr = read_mtx();
            // let bsr = Bsr::<2, 2, _>::from(csr);
            // debug!("bsr: {:?}", bsr);
            // let csr: CsMat<_> = bsr.into();
            // debug!("csr: {:?}", csr);
            // let mem_settings = MemSettings {
            //     simd_width: 4,
            //     banks: 2,
            //     row_mapping: crate::settings::RowMapping::Chunk,
            //     row_size: 4,
            //     chips: 1,
            //     channels: 1,
            //     bank_merger_size: 2,
            //     chip_merger_size: 2,
            //     channel_merger_size: 2,
            // };
        }

        #[test]
        fn test_chip_merge() {
            let csr = read_mtx();
            let bsr = Bsr::<2, 2, _>::from(csr);
            debug!("bsr: {:?}", bsr);
            let csr: CsMat<_> = bsr.into();
            debug!("csr: {:?}", csr);
            let mem_settings = MemSettings {
                simd_width: 4,
                banks: 1,
                row_mapping: crate::settings::RowMapping::Chunk,
                row_size: 4,
                chips: 2,
                channels: 1,
                bank_merger_size: 2,
                chip_merger_size: 2,
                channel_merger_size: 2,
            };
            let bank_merge_result = csr.bank_merge(&mem_settings).1;
            debug!("bank result: {:?}", bank_merge_result);
            let result = csr.chip_merge(&mem_settings, &bank_merge_result);
            debug!("chip result: {:?}", result);
        }
        #[test]
        fn test_chip_merge2() {
            let csr = read_mtx();
            let bsr = Bsr::<2, 2, _>::from(csr);
            debug!("bsr: {:?}", bsr);
            let csr: CsMat<_> = bsr.into();
            debug!("csr: {:?}", csr);
            let mem_settings = MemSettings {
                simd_width: 4,
                banks: 2,
                row_mapping: crate::settings::RowMapping::Chunk,
                row_size: 4,
                chips: 1,
                channels: 1,
                bank_merger_size: 2,
                chip_merger_size: 2,
                channel_merger_size: 2,
            };
            let bank_merge_result = csr.bank_merge(&mem_settings).1;
            debug!("bank result: {:?}", bank_merge_result);
            let result = csr.chip_merge(&mem_settings, &bank_merge_result);
            debug!("chip result: {:?}", result);
        }

        #[test]
        fn test_channel_merge() {
            let csr = read_mtx();
            let bsr = Bsr::<2, 2, _>::from(csr);
            debug!("bsr: {:?}", bsr);
            let csr: CsMat<_> = bsr.into();
            debug!("csr: {:?}", csr);
            let mem_settings = MemSettings {
                simd_width: 4,
                banks: 1,
                row_mapping: crate::settings::RowMapping::Chunk,
                row_size: 4,
                chips: 1,
                channels: 2,
                bank_merger_size: 2,
                chip_merger_size: 2,
                channel_merger_size: 2,
            };
            let bank_merge_result = csr.bank_merge(&mem_settings).1;
            debug!("bank result: {:?}", bank_merge_result);
            let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result).1;
            debug!("chip result: {:?}", chip_merge_result);
            let result = csr.channel_merge(&mem_settings, &chip_merge_result);
            debug!("channel result: {:?}", result);
        }

        #[test]
        fn test_channel_merge2() {
            let csr = read_mtx();
            let bsr = Bsr::<2, 2, _>::from(csr);
            debug!("bsr: {:?}", bsr);
            let csr: CsMat<_> = bsr.into();
            debug!("csr: {:?}", csr);
            let mem_settings = MemSettings {
                simd_width: 4,
                banks: 2,
                row_mapping: crate::settings::RowMapping::Chunk,
                row_size: 4,
                chips: 1,
                channels: 1,
                bank_merger_size: 2,
                chip_merger_size: 2,
                channel_merger_size: 2,
            };
            let bank_merge_result = csr.bank_merge(&mem_settings).1;
            debug!("bank result: {:?}", bank_merge_result);
            let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result).1;
            debug!("chip result: {:?}", chip_merge_result);
            let result = csr.channel_merge(&mem_settings, &chip_merge_result);
            debug!("channel result: {:?}", result);
        }

        #[test]
        fn test_channel_merge3() {
            let csr = read_mtx();
            let bsr = Bsr::<2, 2, _>::from(csr);
            debug!("bsr: {:?}", bsr);
            let csr: CsMat<_> = bsr.into();
            debug!("csr: {:?}", csr);
            let mem_settings = MemSettings {
                simd_width: 4,
                banks: 1,
                row_mapping: crate::settings::RowMapping::Chunk,
                row_size: 4,
                chips: 2,
                channels: 1,
                bank_merger_size: 2,
                chip_merger_size: 2,
                channel_merger_size: 2,
            };
            let bank_merge_result = csr.bank_merge(&mem_settings).1;
            debug!("bank result: {:?}", bank_merge_result);
            let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result).1;
            debug!("chip result: {:?}", chip_merge_result);
            let result = csr.channel_merge(&mem_settings, &chip_merge_result);
            debug!("channel result: {:?}", result);
        }
    }
}
