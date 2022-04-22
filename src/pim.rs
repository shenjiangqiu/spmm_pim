use std::ops::Deref;

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

impl<N, I, IptrStorage, IndStorage, DataStorage, Iptr> Pim
    for CsMatBase<N, I, IptrStorage, IndStorage, DataStorage, Iptr>
where
    I: SpIndex,
    Iptr: SpIndex,
    IptrStorage: Deref<Target = [Iptr]>,
    IndStorage: Deref<Target = [I]>,
    DataStorage: Deref<Target = [N]>,
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
