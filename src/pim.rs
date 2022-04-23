use std::{fmt::Debug, ops::Deref};

use itertools::Itertools;
use log::debug;
use sprs::{CsMatBase, SpIndex};

use crate::{bsr::Bsr, settings::MemSettings};
pub trait Pim {
    fn mem_rows(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn bank_merge(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn bank_add(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn chip_add(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn chip_merge(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn channel_add(&self, mem_settings: &MemSettings) -> Vec<usize>;
    fn channel_merge(&self, mem_settings: &MemSettings) -> Vec<usize>;
}

fn get_bank_id_from_row_id(row_id: usize, mem_settings: &MemSettings, num_rows: usize) -> usize {
    let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;
    match mem_settings.row_mapping {
        crate::settings::RowMapping::Chunck => {
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
        crate::settings::RowMapping::Chunck => {
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

struct BankMergeTaskBuilder{
    tasks: Vec<Vec<usize>>,
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
    fn bank_merge(&self, mem_settings: &MemSettings) -> Vec<usize> {
        let merger_size = mem_settings.bank_merger_size;
        let num_banks = mem_settings.banks * mem_settings.chips * mem_settings.channels;
        let bank_tasks=vec![vec![]; num_banks];
        for i in self.iter() {
            let row_select = i.1 .1.index();
            let bank_id = get_bank_id_from_row_id(row_select, mem_settings, self.rows());
            let row_id_in_bank = get_row_id_in_bank(row_select, mem_settings, self.rows());
            let row_size = self.outer_view(row_select).unwrap().nnz() * 4;
            let row_buffer_size = mem_settings.row_size;
            for i in 0..((row_size + row_buffer_size - 1) / row_buffer_size) {
                bank_tasks[bank_id].push(row_id_in_bank + i);
            }
        }
        todo!()
    }

    fn bank_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn chip_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn chip_merge(&self, mem_settings: &MemSettings) -> Vec<usize> {
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

    fn bank_merge(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn bank_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn chip_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn chip_merge(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn channel_add(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }

    fn channel_merge(&self, mem_settings: &MemSettings) -> Vec<usize> {
        todo!()
    }
}

#[cfg(test)]
mod pimtest {
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
            row_mapping: crate::settings::RowMapping::Chunck,
            row_size: 4,
            chips: 1,
            channels: 1,
        };

        let result = csr.mem_rows(&mem_settings);
        println!("{:?}", result);
    }
}
