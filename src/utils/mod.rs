use std::path::Path;

use sprs::CsMat;

use crate::two_matrix::TwoMatrix;

mod generator;
pub mod plot;
pub mod run;
mod serial_test;
pub fn create_two_matrix_from_file(file_name: &Path) -> TwoMatrix<i32, i32> {
    let csr: CsMat<i32> = sprs::io::read_matrix_market(file_name).unwrap().to_csr();
    let trans_pose = csr.transpose_view().to_csr();
    TwoMatrix::new(csr, trans_pose)
}
#[cfg(test)]
mod test {
    #[test]
    fn simple_test() {
        let a = String::from("123");
        let b = String::from("222");
        let c = vec![&a, &b];
        println!("{:?}", c);
        println!("{:?}", c);
    }
}
