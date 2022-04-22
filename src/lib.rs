pub mod args;
pub mod bsr;
pub mod result;
pub mod run;
pub mod settings;
pub mod utils;
pub mod bsr_row_builder;
#[cfg(test)]
mod test {
    use eyre::Result;
    use log::debug;
    use sprs::{CsMat, TriMat};

    use crate::utils::init_log;

    #[test]
    fn test_csc() -> Result<()> {
        init_log("debug");
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx")?;
        let csc: CsMat<_> = matrix.to_csc();
        debug!("{:?}", csc);
        Ok(())
    }

    #[test]
    fn test_csr() -> Result<()> {
        init_log("debug");
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx")?;
        let csr: CsMat<_> = matrix.to_csr();
        debug!("{:?}", csr);
        Ok(())
    }

    #[test]
    fn test_bsr() -> Result<()> {
        init_log("debug");
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx")?;
        let bsr: super::bsr::Bsr<2, 2, _> = super::bsr::Bsr::from(matrix.to_csr());
        debug!("{:?}", bsr);
        Ok(())
    }
}
