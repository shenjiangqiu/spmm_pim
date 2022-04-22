use std::path::Path;

use crate::{bsr::Bsr, result::SingleResult};
use eyre::Result;
use log::debug;
use sprs::{CsMat, TriMat};
pub fn run_exp<const R: usize, const C: usize>(filename: &Path) -> Result<SingleResult> {
    let tri: TriMat<i32> = sprs::io::read_matrix_market(filename)?;
    let csr: CsMat<_> = tri.to_csr();
    debug!("original_csr nnz: {}", csr.nnz());
    let csr_nnz = csr.nnz();
    let bsr: Bsr<R, C, _> = Bsr::from(csr);

    debug!("bsr_{}_{}_nnz: {}", R, C, bsr.nnz());
    debug!("bsr_{}_{}_element: {}", R, C, bsr.nnz() * C * R);
    let single_result = SingleResult {
        file: filename,
        r: R,
        c: C,
        block_size: R * C,
        origin_nnz: csr_nnz,
        new_nnz: bsr.nnz(),
        new_element: bsr.nnz() * C * R,
        need_speed_up: (bsr.nnz() * C * R) as f32 / (csr_nnz as f32),
    };
    debug!("{:?}", single_result);
    Ok(single_result)
}

#[macro_export]
macro_rules! run_1d_c_unroll {
    ($file:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; $size0:literal)=>{
        $fun::<1,$size0>($file).map_or_else(
            |_| {
                $err_list.push($file);
            },
            |x| {$full_result.all.push(x);$ok_list.push($file);},
        );
    };
    ($file:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; $size0:literal, $($size:literal),+) => {
        $fun::<1,$size0>($file).map_or_else(
            |_| {
                $err_list.push($file);
            },

            |x| {$full_result.all.push(x);$ok_list.push($file);},
        );

        run_1d_c_unroll!($file;$full_result;$ok_list;$err_list; $fun;  $($size),+ );
    };

}

#[macro_export]
macro_rules! run_2d_unroll {
    ($file:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; ($r:literal,$c:literal))=>{
        $fun::<$r,$c>($file).map_or_else(
            |_| {
                $err_list.push($file);
            },
            |x| {$full_result.all.push(x);$ok_list.push($file);},
        );
    };
    ($file:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; ($r:literal,$c:literal), $(($r1:literal,$c1:literal)),+) => {
        $fun::<$r,$c>($file).map_or_else(
            |_| {
                $err_list.push($file);
            },

            |x| {$full_result.all.push(x);$ok_list.push($file);},
        );

        run_2d_unroll!($file;$full_result;$ok_list;$err_list; $fun;  $(($r1,$c1)),+ );
    };

}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use log::debug;

    use crate::result::Results;
    use crate::run::run_exp;
    use crate::utils::init_log;
    #[test]
    fn test() {
        init_log("debug");
        let path = PathBuf::from("test.mtx");
        let mut full_result = Results { all: vec![] };
        let mut ok_list = vec![];
        let mut err_list = vec![];
        run_1d_c_unroll!(&path;full_result; ok_list;err_list;run_exp; 1,2,3);
        debug!("{:?}", full_result);
    }

    #[test]
    fn test2d() {
        init_log("debug");
        let path = PathBuf::from("test.mtx");
        let mut full_result = Results { all: vec![] };
        let mut ok_list = vec![];
        let mut err_list = vec![];
        run_2d_unroll!(&path;full_result; ok_list;err_list;run_exp;(1,1),(2,2),(3,3),(4,4));
        debug!("{:?}", full_result);
    }
}
