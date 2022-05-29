use std::{fmt::Debug, mem};

use itertools::Itertools;
use log::debug;
use plotlib::{repr::Histogram, view::View};
use sprs::{CsMat, SpIndex};

use crate::{
    csv_nodata::CsVecNodata,
    pim::{self, AdderTaskBuilder, MergeCycle, PartialSum, Pim},
    settings::MemSettings,
};

pub struct TwoMatrix<N1, N2> {
    pub a: CsMat<N1>,
    pub b: CsMat<N2>,
}

impl<N1, N2> TwoMatrix<N1, N2> {
    pub fn new(a: CsMat<N1>, b: CsMat<N2>) -> Self {
        if a.cols() != b.rows() {
            panic!("a.cols()!=b.rows()");
        }
        Self { a, b }
    }
}

impl<N1, N2> Pim for TwoMatrix<N1, N2>
where
    N1: Debug + Clone,
    N2: Debug + Clone,
{
    fn mem_rows(&self, mem_settings: &MemSettings) -> Vec<usize> {
        let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;

        // fisrt calculate the access row stream of each bank
        // the rows of the second vector.
        let num_rows = self.a.cols();
        assert_eq!(num_rows, self.b.rows());
        // contains the rows to read for each bank
        let mut row_stream = vec![vec![]; num_banks];
        // return the bank id and the row id in bank

        for i in self.a.iter() {
            debug!("i: {:?}", i);
            let row_select = i.1 .1.index();
            let bank_id = pim::get_bank_id_from_row_id(row_select, mem_settings, num_rows);
            debug!("bank_id: {:?}", bank_id);
            let row_id_in_bank = pim::get_row_id_in_bank(row_select, mem_settings, num_rows);
            debug!("row_id_in_bank: {:?}", row_id_in_bank);

            let row_size = self.b.outer_view(row_select).unwrap().nnz() * mem::size_of::<N2>();
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
    /// return how many merge operations are needed
    /// return:
    /// - Vec<MergeCycle>: the merge cycles for each bank
    /// - Vec<PartialSum<usize>>: the partial sums for each bank
    fn bank_merge(&self, mem_settings: &MemSettings) -> (Vec<MergeCycle>, Vec<PartialSum<usize>>) {
        let merger_size = mem_settings.bank_merger_size;
        let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;
        let mut bank_tasks = vec![AdderTaskBuilder::default(); num_banks];

        for i in self.a.iter() {
            let row_select = i.1 .1.index();
            debug!("row_select: {:?}", row_select);
            let target_row = i.1 .0.index();
            debug!("target_row: {:?}", target_row);
            let bank_id = pim::get_bank_id_from_row_id(row_select, mem_settings, self.a.cols());
            debug!("bank_id: {:?}", bank_id);
            let row_nnz = self.b.outer_view(row_select).unwrap().nnz();
            debug!("row_nnz: {:?}", row_nnz);
            let input_row: CsVecNodata<_> =
                self.b.outer_view(row_select).unwrap().to_owned().into();
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
                merged_tasks.push(x.1.into());
            });

        (cycles, merged_tasks)
    }

    fn chip_merge(
        &self,
        mem_settings: &MemSettings,
        bank_merge_result: &[PartialSum<usize>],
    ) -> (Vec<MergeCycle>, Vec<PartialSum<usize>>) {
        // just like the bank merge, but istead take the result of bank level result
        let merger_size = mem_settings.chip_merger_size;
        let num_chips = mem_settings.chips * mem_settings.channels;

        pim::internal_merge(bank_merge_result, merger_size, num_chips)
    }

    fn channel_merge(
        &self,
        mem_settings: &MemSettings,
        chip_merge_result: &[PartialSum<usize>],
    ) -> (Vec<MergeCycle>, Vec<PartialSum<usize>>) {
        let num_channel = mem_settings.channels;
        let num_chips = mem_settings.chips * mem_settings.channels;
        assert!(num_chips % num_channel == 0);
        assert_eq!(num_chips, chip_merge_result.len());

        let merger_size = mem_settings.channel_merger_size;
        pim::internal_merge(chip_merge_result, merger_size, num_channel)
    }
    fn dimm_merge(
        &self,
        mem_settings: &MemSettings,
        channel_sum: &[PartialSum<usize>],
    ) -> (MergeCycle, PartialSum<usize>) {
        let mut result = pim::internal_merge(channel_sum, mem_settings.dimm_merger_size, 1);
        assert_eq!(result.0.len(), 1);
        assert_eq!(result.1.len(), 1);

        (result.0.pop().unwrap(), result.1.pop().unwrap())
    }
}
