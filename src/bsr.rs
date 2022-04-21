use itertools::Itertools;
use log::debug;
use sprs::{CsMatBase, IndPtr, IndPtrBase, SpIndex};
use std::ops::Deref;

use crate::bsr_row_builder::BsrRowbuilder;
#[derive(Debug, PartialEq)]
pub struct Bsr<
    const R: usize,
    const C: usize,
    N = u32,
    I = usize,
    Iptr = I,
    IptrStorage = Vec<Iptr>,
    IndStorage = Vec<I>,
    DataStorage = Vec<[[N; C]; R]>,
> where
    I: SpIndex,
    Iptr: SpIndex,
    IptrStorage: Deref<Target = [Iptr]>,
    IndStorage: Deref<Target = [I]>,
    DataStorage: Deref<Target = [[[N; C]; R]]>,
{
    data: DataStorage,
    index: IndStorage,
    ptr: IndPtrBase<Iptr, IptrStorage>,
}

impl<const R: usize, const C: usize, N, I, Iptr, IptrStorage, IndStorage, DataStorage>
    Bsr<R, C, N, I, Iptr, IptrStorage, IndStorage, DataStorage>
where
    I: SpIndex,
    Iptr: SpIndex,
    IptrStorage: Deref<Target = [Iptr]>,
    IndStorage: Deref<Target = [I]>,
    DataStorage: Deref<Target = [[[N; C]; R]]>,
{
    pub fn new(data: DataStorage, index: IndStorage, ptr: IndPtrBase<Iptr, IptrStorage>) -> Self {
        Bsr { data, index, ptr }
    }
    pub fn data(&self, index: usize) -> &[[N; C]; R] {
        &self.data[index]
    }
    pub fn index(&self) -> &[I] {
        &self.index
    }
    pub fn ptr(&self) -> &IndPtrBase<Iptr, IptrStorage> {
        &self.ptr
    }
    pub fn nnz(&self) -> usize {
        self.ptr.nnz()
    }
}

impl<const R: usize, const C: usize, N, I, IptrStorage, IndStorage, DataStorage, Iptr>
    From<CsMatBase<N, I, IptrStorage, IndStorage, DataStorage, Iptr>>
    for Bsr<R, C, N, I, Iptr, Vec<Iptr>, Vec<I>, Vec<[[N; C]; R]>>
where
    N: Default + Copy + Clone,
    I: SpIndex,
    Iptr: SpIndex,
    IptrStorage: Deref<Target = [Iptr]>,
    IndStorage: Deref<Target = [I]>,
    DataStorage: Deref<Target = [N]>,
{
    fn from(matrix: CsMatBase<N, I, IptrStorage, IndStorage, DataStorage, Iptr>) -> Self {
        if matrix.is_csc() {
            panic!("CSC matrix is not supported");
        }
        let (rows, cols) = matrix.shape();
        if rows % R != 0 || cols % C != 0 {
            debug!("Matrix shape is not compatible with block size, padding with zeros, matrix shape: {:?}, block size: {:?}", (rows, cols), (R, C));
        }
        let mut iptr: Vec<Iptr> = vec![];
        let mut index: Vec<I> = vec![];
        let mut data = vec![];
        // build the componenets
        let mut current_ptr = 0;
        iptr.push(Iptr::from_usize(current_ptr));

        for chunck in &matrix.outer_iterator().chunks(R) {
            debug!("start to processing ptr: {}'s chunck", current_ptr);

            let chunk_vec = chunck.collect_vec();
            let mut chunk_iter = chunk_vec.iter().map(|x| x.iter().peekable()).collect_vec();

            let mut row_builder = BsrRowbuilder::new();

            loop {
                // choose the min index
                let mut min_index = usize::MAX;
                let mut min_index_row = 0;
                let mut min_value = N::default();
                let mut all_empty = true;

                // find the min index
                for (row, row_vec) in chunk_iter.iter_mut().enumerate() {
                    if row_vec.peek().is_none() {
                        continue;
                    }
                    all_empty = false;
                    let (index, value) = row_vec.peek().unwrap();
                    if *index < min_index {
                        min_index = *index;
                        min_index_row = row;
                        min_value = **value;
                    }
                }
                // nothing left in the chunck
                if all_empty {
                    // get the builder
                    let (mut tindex, mut tdata) = row_builder.into_row();

                    current_ptr += tindex.len();

                    data.append(&mut tdata);
                    index.append(&mut tindex);
                    iptr.push(Iptr::from_usize(current_ptr));
                    break;
                }
                // push the min index
                row_builder.push_element(I::from_usize(min_index), min_value, min_index_row);
                // pop the min
                chunk_iter[min_index_row].next();
            }
        }

        let iptr = IndPtr::new_checked(iptr).unwrap();

        Self::new(data, index, iptr)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::init_log;
    use eyre::Result;
    use sprs::{CsMat, TriMat};
    #[test]
    fn test_bsr() {
        init_log("debug");
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("test.mtx").unwrap();
        let csr: CsMat<_> = matrix.to_csr();
        let bsr: Bsr<2, 2, _> = Bsr::from(csr);
        let true_bsr = Bsr {
            data: vec![
                [[1, 0], [0, 1]],
                [[0, 6], [0, 0]],
                [[0, 0], [0, 2]],
                [[1, 0], [0, -2]],
                [[0, 0], [3, 0]],
                [[1, 0], [0, -1]],
            ],
            index: vec![0, 1, 0, 1, 2, 2],
            ptr: IndPtrBase::new_checked(vec![0, 2, 5, 6]).unwrap(),
        };
        assert_eq!(bsr, true_bsr);
    }

    #[test]
    fn test_unalign() {
        init_log("debug");
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("test.mtx").unwrap();
        let csr: CsMat<_> = matrix.to_csr();
        let bsr: Bsr<4, 4, _> = Bsr::from(csr);
        // let true_bsr = Bsr {
        //     data: vec![
        //         [[1, 0], [0, 1]],
        //         [[0, 6], [0, 0]],
        //         [[0, 0], [0, 2]],
        //         [[1, 0], [0, -2]],
        //         [[0, 0], [3, 0]],
        //         [[1, 0], [0, -1]],
        //     ],
        //     index: vec![0, 1, 0, 1, 2, 2],
        //     ptr: IndPtrBase::new_checked(vec![0, 2, 5, 6]).unwrap(),
        // };
        // assert_eq!(bsr, true_bsr);
        debug!("{:?}", bsr);
    }

    #[test]
    fn test_big() -> Result<()> {
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("test.mtx")?;
        let csr: CsMat<_> = matrix.to_csr();
        let bsr: Bsr<1, 16, _> = Bsr::from(csr);
        let ptr = IndPtrBase::new_checked(vec![0, 1, 2, 3, 4, 5, 6]).map_err(|e| e.1)?;
        let true_value = Bsr {
            data: vec![
                [[1, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
                [[0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
                [[0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
                [[0, 2, 0, -2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
                [[0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
                [[0, 0, 0, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
            ],
            index: vec![0, 0, 0, 0, 0, 0],
            ptr,
        };
        assert_eq!(bsr, true_value);
        Ok(())
    }
}
