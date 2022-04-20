use std::{error::Error, path::Path};

use sprs::{CsMat, TriMat};

use crate::{bsr::Bsr, result::SingleResult};
pub fn run_exp<const C: usize>(filename: &Path) -> Result<SingleResult, Box<dyn Error>> {
    let tri: TriMat<i32> = sprs::io::read_matrix_market(filename)?;
    let csr: CsMat<_> = tri.to_csr();
    println!("original_csr nnz: {}", csr.nnz());
    let csr_nnz = csr.nnz();
    let bsr: Bsr<1, C, _> = Bsr::from(csr);

    println!("bsr_{}_{}_nnz: {}", 1, C, bsr.nnz());
    println!("bsr_{}_{}_element: {}", 1, C, bsr.nnz() * C);
    let single_result = SingleResult {
        file: filename,
        c: C,
        origin_nnz: csr_nnz,
        new_nnz: bsr.nnz(),
        new_element: bsr.nnz() * C,
        need_speed_up: (bsr.nnz() * C) as f32 / (csr_nnz as f32),
    };
    Ok(single_result)
}
#[macro_export]
macro_rules! run_unroll {
    ($file:expr;$full_result:expr;$fun:ident; $size0:literal)=>{
        println!("runing {:?} with {:?}", $file, $size0);
        $fun::<$size0>($file).map_or_else(
            |x| {
                println!("file: {:?} ,error: {}", $file, x);
            },
            |x| $full_result.all.push(x),
        );
    };
    ($file:expr;$full_result:expr;$fun:ident; $size0:literal, $($size:literal),+) => {
        println!("runing {:?} with {:?}", $file, $size0);
        $fun::<$size0>($file).map_or_else(
            |x| {
                println!("file: {:?} ,error: {}", $file, x);
            },
            |x| $full_result.all.push(x),
        );

        run_unroll!($file;$full_result;$fun;  $($size),+ );
    };

}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::result::Results;
    use crate::run::run_exp;
    #[test]
    fn test() {
        let path = PathBuf::from("test.mtx");
        let mut full_result = Results { all: vec![] };

        run_unroll!(&path;full_result;run_exp; 1,2,3);
        println!("{:?}", full_result);
    }
}
