

pub mod bsr;
pub mod utils;

#[cfg(test)]
mod test {
    use sprs::{TriMat, CsMat};

    use crate::utils::test::init_log;

    #[test]
    fn test_csc() {
        init_log();
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("./test.mtx").unwrap();
        let csc: CsMat<_> = matrix.to_csc();
        println!("{:?}", csc);
    }

    #[test]
    fn test_csr(){
        init_log();
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("./test.mtx").unwrap();
        let csr: CsMat<_> = matrix.to_csr();
        println!("{:?}", csr);
    }

    #[test]
    fn test_bsr(){
        init_log();
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("./test.mtx").unwrap();
        let bsr: super::bsr::Bsr<2, 2, _> = super::bsr::Bsr::from(matrix.to_csr());
        println!("{:?}", bsr);
    }
}
