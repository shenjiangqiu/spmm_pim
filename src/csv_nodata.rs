use std::{
    fmt::Debug,
    iter::Sum,
    ops::{Add, Deref, DerefMut},
};

use sprs::{CsVecI, SpIndex};

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct CsVecNodata<I>
where
    I: SpIndex,
{
    pub dim: usize,
    pub indices: Vec<I>,
}

impl<I> Deref for CsVecNodata<I>
where
    I: SpIndex,
{
    type Target = Vec<I>;

    fn deref(&self) -> &Self::Target {
        &self.indices
    }
}

impl<I> DerefMut for CsVecNodata<I>
where
    I: SpIndex,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.indices
    }
}

pub fn get_csvec_nodata<N, I>(csvec: &CsVecI<N, I>) -> CsVecNodata<I>
where
    I: SpIndex,
{
    CsVecNodata {
        dim: csvec.dim(),
        indices: csvec.indices().to_vec(),
    }
}

impl<N, I> From<CsVecI<N, I>> for CsVecNodata<I>
where
    I: SpIndex,
{
    fn from(csvec: CsVecI<N, I>) -> Self {
        get_csvec_nodata(&csvec)
    }
}

impl<T> Add for CsVecNodata<T>
where
    T: SpIndex,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut index1 = self.indices;
        let index2 = other.indices;
        // merge the two sorted index

        index1.extend(index2);
        index1.sort_unstable();
        index1.dedup();

        CsVecNodata {
            dim: self.dim,
            indices: index1,
        }
    }
}

// impl sum for CsVecNodata

impl<T> Sum for CsVecNodata<T>
where
    T: SpIndex,
{
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        // fix bug here, the init dim is not correct, so use reduce instead!
        iter.reduce(|x, y| x + y).unwrap()
    }
}

#[cfg(test)]
mod test {
    use sprs::CsVec;

    use super::{get_csvec_nodata, CsVecNodata};
    #[test]
    fn test() {
        let csvec = CsVec::new(100, vec![0, 1, 4, 5, 6, 9], vec![1, 2, 5, 6, 7, 10]);
        let csvec2 = CsVec::new(100, vec![2, 3, 8, 9], vec![1, 2, 9, 10]);
        let csvec_nodata = get_csvec_nodata(&csvec);
        let csvec_nodata2 = get_csvec_nodata(&csvec2);

        let out = csvec_nodata + csvec_nodata2;
        println!("{:?}", out);
    }

    #[test]
    fn test_sum() {
        let vscs: Vec<CsVecNodata<usize>> = vec![
            CsVec::new(100, vec![0, 1, 4], vec![1, 2, 5]).into(),
            CsVec::new(100, vec![4, 8, 9], vec![1, 2, 9]).into(),
        ];
        let out: CsVecNodata<usize> = vscs.into_iter().sum();
        println!("{:?}", out);
    }
}
