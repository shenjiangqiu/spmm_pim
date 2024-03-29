//! the pim module
//! this module define the trait Pim.

use std::{
    collections::BTreeMap,
    fmt::Debug,
    iter::Sum,
    ops::{Deref, Mul},
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sprs::SpIndex;
use tracing::debug;

use crate::{
    csv_nodata::CsVecNodata,
    settings::{MemSettings, RealRowMapping},
    sim::id_translation::BankID,
};

fn transpose<const R: usize, const C: usize, T>(m: [[T; C]; R]) -> [[T; R]; C] {
    let mut iters = m.map(|r| r.into_iter());

    use std::array;

    // safety, iters have R elements, so get_unchecked_mut is safe because inner loop have r element
    // next will be safe because each iter will have C elements
    array::from_fn(|_| {
        array::from_fn(|i| unsafe { iters.get_unchecked_mut(i).next().unwrap_unchecked() })
    })
}
#[cfg(test)]
#[test]
fn test_transpose() {
    let m = [[1, 2, 3], [4, 5, 6]];
    let m_t = transpose(m);
    assert_eq!(m_t, [[1, 4,], [2, 5,], [3, 6,]]);
}
fn mul_vec<const R: usize, T: Mul<Output = T> + Sum>(a: [T; R], b: [T; R]) -> T {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
/// Partial sum
/// for each element in `data`
/// it contains the `(target_index, target_row_size)`

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartialSumSize {
    pub data: Vec<(usize, usize)>,
}

/// the data types that can be operated in Matrix Multiplication like Mat<i32> * Mat<i32> = Mat<i32>
pub trait MultiplicatableTo<Other> {
    type Output;
    fn multiple(&self, other: &Other) -> Self::Output;
}

impl MultiplicatableTo<f64> for f64 {
    type Output = f64;
    fn multiple(&self, other: &Self) -> Self {
        self * other
    }
}
impl MultiplicatableTo<f32> for f32 {
    type Output = f32;
    fn multiple(&self, other: &Self) -> Self {
        self * other
    }
}
impl MultiplicatableTo<i32> for i32 {
    type Output = i32;
    fn multiple(&self, other: &Self) -> Self {
        self * other
    }
}
impl MultiplicatableTo<i64> for i64 {
    type Output = i64;
    fn multiple(&self, other: &Self) -> Self {
        self * other
    }
}
impl MultiplicatableTo<u32> for u32 {
    type Output = u32;
    fn multiple(&self, other: &Self) -> Self {
        self * other
    }
}
impl MultiplicatableTo<u64> for u64 {
    type Output = u64;
    fn multiple(&self, other: &Self) -> Self {
        self * other
    }
}
impl MultiplicatableTo<usize> for usize {
    type Output = usize;
    fn multiple(&self, other: &Self) -> Self {
        self * other
    }
}
impl MultiplicatableTo<isize> for isize {
    type Output = isize;
    fn multiple(&self, other: &Self) -> Self {
        self * other
    }
}
impl<T: Mul<Output = T> + Sum + Copy, const R: usize, const C: usize, const C2: usize>
    MultiplicatableTo<[[T; C2]; C]> for [[T; C]; R]
{
    type Output = [[T; C2]; R];
    fn multiple(&self, other: &[[T; C2]; C]) -> Self::Output {
        let t_b = transpose(*other);

        self.map(|x| t_b.map(|y| mul_vec(x, y)))
    }
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeCycle {
    pub add_cycle: usize,
    pub merge_cycle: usize,
}
impl From<(usize, usize)> for MergeCycle {
    fn from(data: (usize, usize)) -> Self {
        MergeCycle {
            add_cycle: data.0,
            merge_cycle: data.1,
        }
    }
}

/// the pim trait
/// for a matrix, or two matrix to implement this trait, it can get the number of cycles to perform matrix multiplication in this matrix.
pub trait Pim {
    /// the cycles to read memory rows. and the data read from memory
    fn mem_rows(&self, mem_settings: &MemSettings) -> Vec<(usize, usize)>;
    /// the cycles to perform merge in bank level.
    /// output: (merge cycle for each bank  , partial sum for each bank)
    fn bank_merge(&self, mem_settings: &MemSettings) -> (Vec<MergeCycle>, Vec<PartialSum<usize>>);
    /// the cycles to fetch partial sum from bank
    /// - input bank_merge_result will have the partial sum for each bank
    /// - output: will have cycles for (each bank sent,each chip received)
    fn chip_fetch_data(
        &self,
        mem_settings: &MemSettings,
        bank_merge_result: &[PartialSum<usize>],
    ) -> (Vec<usize>, Vec<usize>);

    /// the cycles to perform merge in chip level.
    /// output: (merge cycle for each chip  , partial sum for each chip)
    fn chip_merge(
        &self,
        mem_settings: &MemSettings,
        bank_merge_result: &[PartialSum<usize>],
    ) -> (Vec<MergeCycle>, Vec<PartialSum<usize>>);
    /// the cycles to fetch partial sum from chip
    /// - input chip_merge_result will have the partial sum for each chip
    /// - output: will have cycles for each channel
    fn channel_fetch_data(
        &self,
        mem_settings: &MemSettings,
        chip_merge_result: &[PartialSum<usize>],
    ) -> (Vec<usize>, Vec<usize>);
    /// the cycles to perform merge in channel level.
    /// output: (merge cycle for each channel  , partial sum for each channel)
    fn channel_merge(
        &self,
        mem_settings: &MemSettings,
        chip_merge_result: &[PartialSum<usize>],
    ) -> (Vec<MergeCycle>, Vec<PartialSum<usize>>);
    /// the cycles to perform merge in dimm level.
    /// output: (merge cycle for each dimm  , partial sum for each dimm)
    fn dimm_merge(
        &self,
        mem_settings: &MemSettings,
        channel_merge_result: &[PartialSum<usize>],
    ) -> (MergeCycle, PartialSum<usize>);

    /// the cycles to fetch partial sum from channel
    /// - input channel_merge_result will have the partial sum for each channel
    /// - output: will have cycles for each dimm
    fn dimm_fetch_data(
        &self,
        mem_settings: &MemSettings,
        channel_merge_result: &[PartialSum<usize>],
    ) -> (Vec<usize>, usize);

    /// the cycles to write back to memory
    fn write_result(&self, mem_settings: &MemSettings, partial_sum: &PartialSum<usize>) -> usize;
}
pub fn get_bank_id_from_flat_bank_id(
    flat_bank_id: usize,
    num_channel: usize,
    num_chip: usize,
    num_bank: usize,
) -> BankID {
    let channel_id = flat_bank_id / (num_chip * num_bank);
    assert!(channel_id < num_channel);
    let chip_id = (flat_bank_id - channel_id * num_chip * num_bank) / num_bank;

    let bank_id = flat_bank_id - channel_id * num_chip * num_bank - chip_id * num_bank;

    ((channel_id, chip_id), bank_id)
}
/// return (BankID, row_id in bank)
pub fn get_bank_id_from_row_id(
    row_id: usize,
    channels: usize,
    chips: usize,
    banks: usize,
    num_rows: usize,
    row_mapping: &RealRowMapping,
) -> (BankID, usize) {
    let num_banks = banks * chips * channels;
    match row_mapping {
        crate::settings::RealRowMapping::Chunk => {
            let rows_per_bank = num_rows / num_banks;

            let bank_id = if rows_per_bank == 0 {
                0
            } else {
                row_id / rows_per_bank
            };
            let row_id_in_bank = row_id % rows_per_bank;

            if bank_id >= num_banks {
                let target_bank_flat = bank_id % num_banks;
                (
                    get_bank_id_from_flat_bank_id(target_bank_flat, channels, chips, banks),
                    row_id_in_bank,
                )
            } else {
                (
                    get_bank_id_from_flat_bank_id(bank_id, channels, chips, banks),
                    row_id_in_bank,
                )
            }
        }
        crate::settings::RealRowMapping::Interleaved(chunk_size) => {
            let row_id = row_id / chunk_size;
            let channel_id = row_id % channels;
            let row_id = row_id / channels;
            let chip_id = row_id % chips;
            let row_id = row_id / chips;
            let bank_id = row_id % banks;
            let row_id = row_id / banks;
            (((channel_id, chip_id), bank_id), row_id)
        }
    }
}

// pub fn get_row_id_in_bank(row_id: usize, mem_settings: &MemSettings, num_rows: usize) -> usize {
//     let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;
//     match mem_settings.row_mapping {
//         crate::settings::RowMapping::Chunk => {
//             let rows_per_bank = num_rows / num_banks;
//             if rows_per_bank == 0 {
//                 row_id
//             } else {
//                 row_id % rows_per_bank
//             }
//         }
//         crate::settings::RowMapping::Interleaved => {
//             row_id / mem_settings.interleaved_chunk / num_banks
//         }
//     }
// }

#[derive(Debug, Clone)]
pub struct AdderTaskBuilder<I>
where
    I: SpIndex,
{
    // tasks: targets: (target id, row sizes)
    tasks: Vec<(usize, Vec<CsVecNodata<I>>)>,
    current_working_target: usize,
}
/// - merget a list of tasks into one patrial sum
/// - merger_size: the number of merger heads
/// - output: (merger_cycle, add_cycle, partial_sum)
pub fn merge_rows_into_one(
    tasks: Vec<CsVecNodata<usize>>,
    merger_size: usize,
) -> (usize, usize, CsVecNodata<usize>) {
    let mut tasks = tasks;
    let mut merge_cycles = 0usize;
    let mut add_cycles = 0usize;
    while tasks.len() > 1 {
        let result_vec = tasks
            .into_iter()
            .chunks(merger_size)
            .into_iter()
            .map(|x| {
                let mut x = x.collect_vec();
                if x.len() == 1 {
                    ((0, 0), x.pop().unwrap())
                } else {
                    let (old_len, result_vec) = x
                        .into_iter()
                        .fold((0, CsVecNodata::default()), |(total_len, cal_vec), y| {
                            (total_len + y.len(), cal_vec + y)
                        });
                    let new_len = result_vec.len();
                    // add cycle, merge cycle and result vec
                    ((old_len - new_len, old_len), result_vec)
                }
            })
            .collect_vec();

        debug!("result_vec: {:?}", result_vec);
        let merge_cycle = result_vec.iter().map(|x| x.0 .1).sum::<usize>();
        debug!("merge_cycle: {:?}", merge_cycle);
        merge_cycles += merge_cycle;
        let add_cycle = result_vec.iter().map(|x| x.0 .0).sum::<usize>();
        debug!("add_cycle: {:?}", add_cycle);
        add_cycles += add_cycle;

        tasks = result_vec.into_iter().map(|x| x.1).collect();
        debug!("end merge ---------------------\n");
    }
    (merge_cycles, add_cycles, tasks.pop().unwrap())
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
    pub fn new() -> Self {
        Self::default()
    }

    /// ## Add a new task to the builder.
    /// args:
    /// (to,size) : the target row and the source row size(nnzs)
    ///
    ///
    ///
    pub fn add_task(&mut self, (to, vec): (usize, CsVecNodata<T>)) {
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
    pub fn build(self, merger_size: usize) -> (MergeCycle, Vec<(usize, CsVecNodata<T>)>) {
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
            debug!("---------start mergeing: i: {:?}", i);
            while i.1.len() > 1 {
                debug!("start merge round ---------------------");
                debug!("start merging the task: {:?}", i.1);

                // TODO: fix it! when the size is 1, no need to count the cycles
                // now fixed, no worry!

                let result_vec =
                    i.1.into_iter()
                        .chunks(merger_size)
                        .into_iter()
                        .map(|x| {
                            let mut x = x.collect_vec();
                            if x.len() == 1 {
                                ((0, 0), x.pop().unwrap())
                            } else {
                                let (old_len, result_vec) = x.into_iter().fold(
                                    (0, CsVecNodata::default()),
                                    |(total_len, cal_vec), y| (total_len + y.len(), cal_vec + y),
                                );
                                let new_len = result_vec.len();
                                // add cycle, merge cycle and result vec
                                ((old_len - new_len, old_len), result_vec)
                            }
                        })
                        .collect_vec();

                debug!("result_vec: {:?}", result_vec);
                let merge_cycle = result_vec.iter().map(|x| x.0 .1).sum::<usize>();
                debug!("merge_cycle: {:?}", merge_cycle);
                merge_cycles += merge_cycle;
                let add_cycle = result_vec.iter().map(|x| x.0 .0).sum::<usize>();
                debug!("add_cycle: {:?}", add_cycle);
                add_cycles += add_cycle;

                i.1 = result_vec.into_iter().map(|x| x.1).collect();
                debug!("end merge ---------------------\n");
            }
            debug!("---------end mergeing: i: {:?}", i);

            merged_tasks.push((i.0, i.1[0].clone()));
        }
        ((add_cycles, merge_cycles).into(), merged_tasks)
    }
}

pub fn internal_merge(
    input: &[PartialSum<usize>],
    merger_size: usize,
    output_elements: usize,
) -> (Vec<MergeCycle>, Vec<PartialSum<usize>>) {
    // just like the bank merge, but istead take the result of bank level result

    let mut output_tasks = vec![AdderTaskBuilder::default(); output_elements];
    let num_task_per_output = input.len() / output_elements;
    // merge inside the chip
    assert_eq!(input.chunks(num_task_per_output).len(), output_elements);

    for (output_tasks, chip_task) in input.chunks(num_task_per_output).zip(&mut output_tasks) {
        // build a set of all chip_sum
        let mut output_sum_map: BTreeMap<usize, Vec<CsVecNodata<usize>>> = BTreeMap::new();
        output_tasks.iter().for_each(|x| {
            x.deref().iter().for_each(|y| {
                output_sum_map
                    .entry(y.0)
                    .or_insert(vec![])
                    .push(y.1.clone());
            });
        });
        // build the task from the map
        output_sum_map.into_iter().for_each(|(k, v)| {
            v.into_iter().for_each(|size| {
                chip_task.add_task((k, size));
            });
        });
    }

    let mut cycles = vec![];
    let mut merged_tasks = vec![];

    output_tasks
        .into_iter()
        .map(|x| x.build(merger_size))
        .for_each(|x| {
            cycles.push(x.0);
            merged_tasks.push(x.1.into());
        });

    assert_eq!(cycles.len(), output_elements);
    assert_eq!(merged_tasks.len(), output_elements);

    (cycles, merged_tasks)
}

// impl<N, IptrStorage, IndStorage, DataStorage, Iptr> Pim
//     for CsMatBase<N, usize, IptrStorage, IndStorage, DataStorage, Iptr>
// where
//     N: Copy + Clone + Debug,
//     Iptr: SpIndex,
//     IptrStorage: Deref<Target = [Iptr]>,
//     IndStorage: Deref<Target = [usize]>,
//     DataStorage: Deref<Target = [N]>,
// {
//     fn mem_rows(&self, mem_settings: &MemSettings) -> Vec<usize> {
//         let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;

//         // fisrt calculate he access row stream of each bank
//         let num_rows = self.rows();
//         // contains the rows to read for each bank
//         let mut row_stream = vec![vec![]; num_banks];
//         // return the bank id and the row id in bank

//         for i in self.iter() {
//             debug!("i: {:?}", i);
//             let row_select = i.1 .1.index();

//             let bank_id = get_bank_id_from_row_id(row_select, mem_settings, num_rows);
//             debug!("bank_id: {:?}", bank_id);
//             let row_id_in_bank = get_row_id_in_bank(row_select, mem_settings, num_rows);
//             debug!("row_id_in_bank: {:?}", row_id_in_bank);

//             let row_size = self.outer_view(row_select).unwrap().nnz() * mem::size_of::<N>();
//             debug!("row_size: {:?}", row_size);

//             let row_buffer_size = mem_settings.row_size;
//             for i in 0..((row_size + row_buffer_size - 1) / row_buffer_size) {
//                 row_stream[bank_id].push(row_id_in_bank + i);
//                 debug!("band_id: {:?} need read {:?}", bank_id, row_id_in_bank + i);
//             }
//         }

//         debug!("finished build the stream, next count the rows for different banks");
//         debug!("{:?}", row_stream);
//         let result = row_stream
//             .iter()
//             .map(|v| {
//                 v.iter().fold((usize::MAX, 0), |acc, x| {
//                     if *x == acc.0 {
//                         acc
//                     } else {
//                         (*x, acc.1 + 1)
//                     }
//                 })
//             })
//             .map(|(_, y)| y)
//             .collect();
//         debug!("{:?}", result);
//         result
//     }
//     // return how many merge operations are needed
//     fn bank_merge(&self, mem_settings: &MemSettings) -> (Vec<MergeCycle>, Vec<PartialSum<usize>>) {
//         let merger_size = mem_settings.bank_merger_size;
//         let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;
//         let mut bank_tasks = vec![AdderTaskBuilder::default(); num_banks];

//         for i in self.iter() {
//             let row_select = i.1 .1.index();
//             debug!("row_select: {:?}", row_select);
//             let target_row = i.1 .0.index();
//             debug!("target_row: {:?}", target_row);
//             let bank_id = get_bank_id_from_row_id(row_select, mem_settings, self.rows());
//             debug!("bank_id: {:?}", bank_id);
//             let row_nnz = self.outer_view(row_select).unwrap().nnz();
//             debug!("row_nnz: {:?}", row_nnz);
//             let input_row: CsVecNodata<_> = self.outer_view(row_select).unwrap().to_owned().into();
//             bank_tasks[bank_id].add_task((target_row, input_row));
//             debug!("bank_tasks: {:?}", bank_tasks);
//         }
//         let mut cycles = vec![];
//         let mut merged_tasks = vec![];

//         bank_tasks
//             .into_iter()
//             .map(|x| x.build(merger_size))
//             .for_each(|x| {
//                 cycles.push(x.0);
//                 merged_tasks.push(x.1.into());
//             });

//         (cycles, merged_tasks)
//     }

//     fn chip_merge(
//         &self,
//         mem_settings: &MemSettings,
//         bank_merge_result: &[PartialSum<usize>],
//     ) -> (Vec<MergeCycle>, Vec<PartialSum<usize>>) {
//         // just like the bank merge, but istead take the result of bank level result
//         let merger_size = mem_settings.chip_merger_size;
//         let num_chips = mem_settings.chips * mem_settings.channels;

//         internal_merge(bank_merge_result, merger_size, num_chips)
//     }

//     fn channel_merge(
//         &self,
//         mem_settings: &MemSettings,
//         chip_merge_result: &[PartialSum<usize>],
//     ) -> (Vec<MergeCycle>, Vec<PartialSum<usize>>) {
//         let num_channel = mem_settings.channels;
//         let num_chips = mem_settings.chips * mem_settings.channels;
//         assert!(num_chips % num_channel == 0);
//         assert_eq!(num_chips, chip_merge_result.len());

//         let merger_size = mem_settings.channel_merger_size;
//         internal_merge(chip_merge_result, merger_size, num_channel)
//     }
//     fn dimm_merge(
//         &self,
//         mem_settings: &MemSettings,
//         channel_sum: &[PartialSum<usize>],
//     ) -> (MergeCycle, PartialSum<usize>) {
//         let mut result = internal_merge(channel_sum, mem_settings.dimm_merger_size, 1);
//         assert_eq!(result.0.len(), 1);
//         assert_eq!(result.1.len(), 1);

//         (result.0.pop().unwrap(), result.1.pop().unwrap())
//     }
// }

// #[cfg(test)]
// mod pimtest {
//     use sprs::{CsMat, TriMat};

//     use crate::{pim::Pim, settings::MemSettings};
//     use tracing::debug;

//     fn read_mtx() -> CsMat<i32> {

//         let matrix: TriMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx").unwrap();
//         let csr: CsMat<_> = matrix.to_csr();
//         csr
//     }

//     mod csr {

//         use super::*;

//         #[test]
//         fn test_pim() {
//             let csr = read_mtx();
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 4,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 1,
//                 channels: 1,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };

//             let result = csr.mem_rows(&mem_settings);
//             println!("{:?}", result);
//         }

//         #[test]
//         fn test_merge() {

//             let csr = read_mtx();
//             debug!("csr: {:?}", csr);
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 2,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 1,
//                 channels: 1,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };

//             let result = csr.bank_merge(&mem_settings);
//             debug!("result: {:?}", result);
//         }

//         #[test]
//         fn test_chip_merge() {
//             let csr = read_mtx();
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 1,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 2,
//                 channels: 1,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };
//             let bank_merge_result = csr.bank_merge(&mem_settings);
//             debug!("bank result: {:?}", bank_merge_result);
//             let result = csr.chip_merge(&mem_settings, &bank_merge_result.1);
//             debug!("chip result: {:?}", result);
//         }
//         #[test]
//         fn test_chip_merge2() {
//             let csr = read_mtx();
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 2,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 1,
//                 channels: 1,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };
//             let bank_merge_result = csr.bank_merge(&mem_settings);
//             debug!("bank result: {:?}", bank_merge_result);
//             let result = csr.chip_merge(&mem_settings, &bank_merge_result.1);
//             debug!("chip result: {:?}", result);
//         }

//         #[test]
//         fn test_channel_merge() {
//             let csr = read_mtx();
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 1,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 1,
//                 channels: 2,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };
//             let bank_merge_result = csr.bank_merge(&mem_settings).1;
//             debug!("bank result: {:?}", bank_merge_result);
//             let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result).1;
//             debug!("chip result: {:?}", chip_merge_result);
//             let result = csr.channel_merge(&mem_settings, &chip_merge_result);
//             debug!("channel result: {:?}", result);
//         }

//         #[test]
//         fn test_channel_merge2() {
//             let csr = read_mtx();
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 2,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 1,
//                 channels: 1,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };
//             let bank_merge_result = csr.bank_merge(&mem_settings).1;
//             debug!("bank result: {:?}", bank_merge_result);
//             let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result).1;
//             debug!("chip result: {:?}", chip_merge_result);
//             let result = csr.channel_merge(&mem_settings, &chip_merge_result);
//             debug!("channel result: {:?}", result);
//         }

//         #[test]
//         fn test_channel_merge3() {
//             let csr = read_mtx();
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 1,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 2,
//                 channels: 1,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };
//             let bank_merge_result = csr.bank_merge(&mem_settings).1;
//             debug!("bank result: {:?}", bank_merge_result);
//             let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result).1;
//             debug!("chip result: {:?}", chip_merge_result);
//             let result = csr.channel_merge(&mem_settings, &chip_merge_result);
//             debug!("channel result: {:?}", result);
//         }
//     }

//     mod bsr {

//         use crate::bsr::Bsr;

//         use super::*;

//         #[test]
//         fn test_pim_row_read() {
//             let csr = read_mtx();
//             let bsr = Bsr::<2, 2, _>::from(csr);
//             debug!("bsr: {:?}", bsr);
//             let csr: CsMat<_> = bsr.into();
//             debug!("csr: {:?}", csr);

//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 2,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 1,
//                 channels: 1,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };

//             let result = csr.mem_rows(&mem_settings);
//             println!("{:?}", result);
//         }

//         #[test]
//         fn test_chip_merge() {
//             let csr = read_mtx();
//             let bsr = Bsr::<2, 2, _>::from(csr);
//             debug!("bsr: {:?}", bsr);
//             let csr: CsMat<_> = bsr.into();
//             debug!("csr: {:?}", csr);
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 1,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 2,
//                 channels: 1,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };
//             let bank_merge_result = csr.bank_merge(&mem_settings).1;
//             debug!("bank result: {:?}", bank_merge_result);
//             let result = csr.chip_merge(&mem_settings, &bank_merge_result);
//             debug!("chip result: {:?}", result);
//         }

//         #[test]
//         fn test_chip_merge2() {
//             let csr = read_mtx();
//             let bsr = Bsr::<2, 2, _>::from(csr);
//             debug!("bsr: {:?}", bsr);
//             let csr: CsMat<_> = bsr.into();
//             debug!("csr: {:?}", csr);
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 2,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 1,
//                 channels: 1,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };
//             let bank_merge_result = csr.bank_merge(&mem_settings);
//             debug!("bank result: {:?}", bank_merge_result);
//             let result = csr.chip_merge(&mem_settings, &bank_merge_result.1);
//             debug!("chip result: {:?}", result);
//         }

//         #[test]
//         fn test_channel_merge() {
//             let csr = read_mtx();
//             let bsr = Bsr::<2, 2, _>::from(csr);
//             debug!("bsr: {:?}", bsr);
//             let csr: CsMat<_> = bsr.into();
//             debug!("csr: {:?}", csr);
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 1,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 1,
//                 channels: 2,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };
//             let bank_merge_result = csr.bank_merge(&mem_settings).1;
//             debug!("bank result: {:?}", bank_merge_result);
//             let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result).1;
//             debug!("chip result: {:?}", chip_merge_result);
//             let result = csr.channel_merge(&mem_settings, &chip_merge_result);
//             debug!("channel result: {:?}", result);
//         }

//         #[test]
//         fn test_channel_merge2() {
//             let csr = read_mtx();
//             let bsr = Bsr::<2, 2, _>::from(csr);
//             debug!("bsr: {:?}", bsr);
//             let csr: CsMat<_> = bsr.into();
//             debug!("csr: {:?}", csr);
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 2,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 1,
//                 channels: 1,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };
//             let bank_merge_result = csr.bank_merge(&mem_settings);
//             debug!("bank result: {:?}", bank_merge_result);
//             let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result.1);
//             debug!("chip result: {:?}", chip_merge_result);
//             let result = csr.channel_merge(&mem_settings, &chip_merge_result.1);
//             debug!("channel result: {:?}", result);
//         }

//         #[test]
//         fn test_channel_merge3() {
//             let csr = read_mtx();
//             let bsr = Bsr::<2, 2, _>::from(csr);
//             debug!("bsr: {:?}", bsr);
//             let csr: CsMat<_> = bsr.into();
//             debug!("csr: {:?}", csr);
//             let mem_settings = MemSettings {
//                 simd_width: 4,
//                 banks: 1,
//                 row_mapping: crate::settings::RowMapping::Chunk,
//                 row_size: 4,
//                 chips: 2,
//                 channels: 1,
//                 bank_merger_size: 2,
//                 chip_merger_size: 2,
//                 channel_merger_size: 2,
//                 dimm_merger_size: 2,
//                 ..Default::default()
//             };
//             let bank_merge_result = csr.bank_merge(&mem_settings).1;
//             debug!("bank result: {:?}", bank_merge_result);
//             let chip_merge_result = csr.chip_merge(&mem_settings, &bank_merge_result).1;
//             debug!("chip result: {:?}", chip_merge_result);
//             let result = csr.channel_merge(&mem_settings, &chip_merge_result);
//             debug!("channel result: {:?}", result);
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_slice_iter() {
        let a = [[1, 2], [2, 3], [3, 4]];
        let b = [[1, 2], [2, 3]];
        let c = a.map(|x| {
            let t_b = transpose(b);
            t_b.map(|y| mul_vec(x, y))
        });
        println!("{:?}", c);
    }
}
