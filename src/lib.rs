

pub mod bsr;
pub mod utils;

#[cfg(test)]
mod test {
    use sprs::{TriMat, CsMat};

    #[test]
    fn test() {
        let matrix: TriMat<u32> = sprs::io::read_matrix_market("./test.mtx").unwrap();
        let csc: CsMat<_> = matrix.to_csc();
        println!("{:?}", csc);
    }
}
