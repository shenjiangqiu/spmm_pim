use std::{fmt::Debug, mem};

use itertools::Itertools;

use ramu_rs::{
    ddr4,
    memory::{self, MemoryTrait},
    request::ReqType,
};
use sprs::{CsMat, SpIndex};
use tracing::instrument;

use crate::{
    csv_nodata::CsVecNodata,
    non_pim::NonPim,
    pim::{self, AdderTaskBuilder, MergeCycle, MultiplicatableTo, PartialSum, Pim},
    settings::{MemSettings, RealRowMapping, RowMapping},
};

/// two matrix which are going to be multiplied
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

pub struct TwoMatrixWrapperForNonPim<N1, N2> {
    pub matrix: TwoMatrix<N1, N2>,
    pub dram_config: String,
}

impl<N1, N2> TwoMatrixWrapperForNonPim<N1, N2> {
    pub fn new(matrix: TwoMatrix<N1, N2>, dram_config: String) -> Self {
        Self {
            matrix,
            dram_config,
        }
    }
}

impl<N1, N2> NonPim for TwoMatrixWrapperForNonPim<N1, N2>
where
    N1: SpIndex,
    N2: SpIndex,
{
    #[instrument(name = "mem_cycle_non_pim", skip_all,fields(config_name=self.dram_config))]
    fn mem_read_cycle(&self) -> (usize, usize, u64) {
        tracing::info!("start mem_read_cycle");
        let config: ramu_rs::config::Config =
            toml::from_str(std::fs::read_to_string(&self.dram_config).unwrap().as_str()).unwrap();
        let ddr4 = ddr4::DDR4::new(&config);
        let mut dram = memory::SimpleMemory::new(config, ddr4);

        // fisrt calculate the access row stream of each bank
        // the rows of the second vector.
        let num_rows = self.matrix.a.cols();
        assert_eq!(num_rows, self.matrix.b.rows());
        // contains the rows to read for each bank
        // return the bank id and the row id in bank
        let mut addr_of_b = self
            .matrix
            .a
            .iter()
            .map(|(_, (_row, col))| {
                let size =
                    self.matrix.b.outer_view(col).unwrap().nnz() * core::mem::size_of::<N2>();
                // assume each row is 128 bytes
                let addr = (col as u64) * 128;
                (addr, size)
            })
            .peekable();
        let mut sent_reqs = 0;
        let mut total_traffic = 0;
        let mut total_real_traffic = 0;
        while let Some((addr, size)) = addr_of_b.peek_mut() {
            if let Ok(_) = dram.try_send(ramu_rs::request::Request::new(*addr, ReqType::Read)) {
                total_traffic += 64;
                sent_reqs += 1;
                if *size <= 64 {
                    total_real_traffic += *size;
                    addr_of_b.next();
                } else {
                    total_real_traffic += 64;
                    *size -= 64;
                    *addr += 64;
                }
            }
            if let Some(_) = dram.try_recv() {
                sent_reqs -= 1;
            }
            dram.tick();
        }
        while sent_reqs > 0 {
            if let Some(_) = dram.try_recv() {
                sent_reqs -= 1;
            }
            dram.tick();
        }
        (total_traffic, total_real_traffic, dram.get_cycle())
    }
    fn process_cycle(&self) -> u64 {
        0
    }
}

impl<N1, N2> Pim for TwoMatrix<N1, N2>
where
    N1: Debug + Clone,
    N2: Debug + Clone,
    N1: MultiplicatableTo<N2>,
{
    /// return the number of cycles to read the rows of the matrix
    /// - input: mem_settings
    /// - output: number of cycles for each bank
    fn mem_rows(&self, mem_settings: &MemSettings) -> Vec<(usize, usize)> {
        let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;

        // fisrt calculate the access row stream of each bank
        // the rows of the second vector.
        let num_rows = self.a.cols();
        assert_eq!(num_rows, self.b.rows());
        // contains the rows to read for each bank
        let mut row_stream = vec![vec![]; num_banks];
        // return the bank id and the row id in bank
        let real_row_mapping = match mem_settings.row_mapping {
            RowMapping::Chunk => RealRowMapping::Chunk,
            RowMapping::Interleaved => RealRowMapping::Interleaved(mem_settings.interleaved_chunk),
        };
        let mut bank_read_size = vec![0; num_banks];
        for (_node, (_a_row, a_col)) in self.a.iter() {
            // the row in matrix B
            let row_select = a_col;
            let (((channel_id, chip_id), bank_id), row_id_in_bank) = pim::get_bank_id_from_row_id(
                row_select,
                mem_settings.channels,
                mem_settings.chips,
                mem_settings.banks,
                num_rows,
                &real_row_mapping,
            );
            let bank_id = channel_id * mem_settings.chips * mem_settings.banks
                + chip_id * mem_settings.banks
                + bank_id;
            tracing::debug!("bank_id: {:?}", bank_id);
            // TODO: this is a bug, the row id should not get by row_celect, this real row size of bank is not the row size of the matrixs
            //
            tracing::debug!("row_id_in_bank: {:?}", row_id_in_bank);

            let row_size = self.b.outer_view(row_select).unwrap().nnz() * mem::size_of::<N2>();
            tracing::debug!("row_size: {:?}", row_size);
            bank_read_size[bank_id] += row_size;
            let row_buffer_size = mem_settings.row_size;
            for i in 0..((row_size + row_buffer_size - 1) / row_buffer_size) {
                row_stream[bank_id].push(row_id_in_bank + i);
                tracing::debug!("band_id: {:?} need read {:?}", bank_id, row_id_in_bank + i);
            }
        }

        tracing::debug!("finished build the stream, next count the rows for different banks");
        tracing::debug!("{:?}", row_stream);
        let result = row_stream
            .iter()
            .map(|v| {
                // init (last_row, different count)
                // v.iter().fold((usize::MAX, 0), |acc, x| {
                //     if *x == acc.0 {
                //         acc
                //     } else {
                //         (*x, acc.1 + 1)
                //     }
                // })
                v.iter()
                    .tuple_windows()
                    .map(|(x, y)| if x == y { 0 } else { 1 })
                    .sum::<usize>()
                    + 1
            })
            .zip(bank_read_size)
            .collect();
        tracing::debug!("{:?}", result);
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
        let real_row_mapping = match mem_settings.row_mapping {
            RowMapping::Chunk => RealRowMapping::Chunk,
            RowMapping::Interleaved => RealRowMapping::Interleaved(mem_settings.interleaved_chunk),
        };
        for i in self.a.iter() {
            let row_select = i.1 .1.index();
            tracing::debug!("row_select: {:?}", row_select);
            let target_row = i.1 .0.index();
            tracing::debug!("target_row: {:?}", target_row);
            let (((channel_id, chip_id), bank_id), _row_id_in_bank) = pim::get_bank_id_from_row_id(
                row_select,
                mem_settings.channels,
                mem_settings.chips,
                mem_settings.banks,
                self.a.cols(),
                &real_row_mapping,
            );
            let bank_id = channel_id * mem_settings.chips * mem_settings.banks
                + chip_id * mem_settings.banks
                + bank_id;
            tracing::debug!("bank_id: {:?}", bank_id);
            let row_nnz = self.b.outer_view(row_select).unwrap().nnz();
            tracing::debug!("row_nnz: {:?}", row_nnz);
            let input_row: CsVecNodata<_> =
                self.b.outer_view(row_select).unwrap().to_owned().into();
            bank_tasks[bank_id].add_task((target_row, input_row));
            tracing::debug!("bank_tasks: {:?}", bank_tasks);
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

    fn chip_fetch_data(
        &self,
        mem_settings: &MemSettings,
        bank_merge_result: &[PartialSum<usize>],
    ) -> (Vec<usize>, Vec<usize>) {
        let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;
        let num_chips = mem_settings.chips * mem_settings.channels;
        assert!(bank_merge_result.len() == num_banks);
        let bank_result = bank_merge_result
            .iter()
            .map(|partial_sum| {
                partial_sum
                    .iter()
                    .map(|(_target_idx, cs_vec_nodata)| {
                        cs_vec_nodata.nnz() * std::mem::size_of::<N2>()
                    })
                    .sum::<usize>()
            })
            .collect_vec();
        assert!(bank_result.len() == num_banks);
        let chip_result = bank_result
            .iter()
            .chunks(mem_settings.banks)
            .into_iter()
            .map(|x| x.sum())
            .collect_vec();
        assert!(chip_result.len() == num_chips);
        (bank_result, chip_result)
    }

    fn channel_fetch_data(
        &self,
        mem_settings: &MemSettings,
        chip_merge_result: &[PartialSum<usize>],
    ) -> (Vec<usize>, Vec<usize>) {
        let num_chips = mem_settings.chips * mem_settings.channels;
        let num_channel = mem_settings.channels;
        assert!(chip_merge_result.len() == num_chips);
        let chip_result = chip_merge_result
            .into_iter()
            .map(|partial_sum| {
                partial_sum
                    .iter()
                    .map(|(_target_idx, cs_vec_nodata)| {
                        cs_vec_nodata.nnz() * std::mem::size_of::<N2>()
                    })
                    .sum::<usize>()
            })
            .collect_vec();
        assert!(chip_result.len() == num_chips);
        let channel_result = chip_result
            .iter()
            .chunks(mem_settings.chips)
            .into_iter()
            .map(|x| x.sum())
            .collect_vec();
        assert!(channel_result.len() == num_channel);
        (chip_result, channel_result)
    }

    fn dimm_fetch_data(
        &self,
        mem_settings: &MemSettings,
        channel_merge_result: &[PartialSum<usize>],
    ) -> (Vec<usize>, usize) {
        let num_channel = mem_settings.channels;
        assert!(channel_merge_result.len() == num_channel);
        let result = channel_merge_result
            .into_iter()
            .map(|partial_sum| {
                partial_sum
                    .iter()
                    .map(|(_target_idx, cs_vec_nodata)| {
                        cs_vec_nodata.nnz() * std::mem::size_of::<N2>()
                    })
                    .sum::<usize>()
            })
            .collect_vec();
        assert!(result.len() == num_channel);
        let total = result.iter().sum();
        (result, total)
    }

    fn write_result(&self, _mem_settings: &MemSettings, partial_sum: &PartialSum<usize>) -> usize {
        partial_sum
            .iter()
            .map(|(_target_idx, cs_vec_nodata)| cs_vec_nodata.nnz() * std::mem::size_of::<N2>())
            .sum()
    }
}

#[cfg(test)]
mod towmatrix_test {
    use sprs::CsMat;
    use tracing::Level;

    use crate::{init_logger, non_pim::NonPim, pim::Pim, settings::MemSettings};

    use super::{TwoMatrix, TwoMatrixWrapperForNonPim};

    #[test]
    fn test_non_pim() -> eyre::Result<()> {
        init_logger();
        let csr: CsMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx")?.to_csr();
        let csr_trans = csr.transpose_view().to_csr();

        let matrix = TwoMatrix::new(csr, csr_trans);
        let matrix = TwoMatrixWrapperForNonPim::new(matrix, "ddr4config.toml".to_string());
        let (traffic, real_traffic, cycle) = matrix.mem_read_cycle();
        tracing::info!(traffic, real_traffic, cycle);
        Ok(())
    }

    #[test]
    fn matrix_mul() -> eyre::Result<()> {
        let csr: CsMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx")?.to_csr();
        let trans_pose = csr.transpose_view().to_csr();
        let two_matrix = TwoMatrix::new(csr, trans_pose);
        let c = &two_matrix.a * &two_matrix.b;
        println!("{c:?}");
        Ok(())
    }

    #[test]
    fn test_pim() -> eyre::Result<()> {
        tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .try_init()
            .unwrap_or_default();
        tracing::debug!("test_pim");
        let csr: CsMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx")?.to_csr();
        tracing::info!(?csr);
        let trans_pose = csr.transpose_view().to_csr();
        tracing::info!(?trans_pose);
        let two_matrix = TwoMatrix::new(csr, trans_pose);
        let mem_settings = MemSettings::default();
        let mem_rows = two_matrix.mem_rows(&mem_settings);
        tracing::info!(?mem_rows);
        let (bank_cycle, bank_partial_sum) = two_matrix.bank_merge(&mem_settings);
        tracing::info!(?bank_cycle);
        let (bank_sent, chip_recv) = two_matrix.chip_fetch_data(&mem_settings, &bank_partial_sum);
        tracing::info!(?bank_sent, ?chip_recv);
        let (chip_cycle, chip_partial_sum) =
            two_matrix.chip_merge(&mem_settings, &bank_partial_sum);
        tracing::info!(?chip_cycle);
        let (chip_sent, channel_recv) =
            two_matrix.channel_fetch_data(&mem_settings, &chip_partial_sum);
        tracing::info!(?chip_sent, ?channel_recv);
        let (channel_cycle, channel_partial_sum) =
            two_matrix.channel_merge(&mem_settings, &chip_partial_sum);
        tracing::info!(?channel_cycle);
        let (channel_sent, dimm_recv) =
            two_matrix.dimm_fetch_data(&mem_settings, &channel_partial_sum);
        tracing::info!(?channel_sent, ?dimm_recv);
        let (dimm_cycle, dimm_result) = two_matrix.dimm_merge(&mem_settings, &channel_partial_sum);
        tracing::info!(?dimm_cycle);
        let result = two_matrix.write_result(&mem_settings, &dimm_result);
        tracing::info!(?result);

        Ok(())
    }
}
