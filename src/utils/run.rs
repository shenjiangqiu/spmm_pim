use std::path::Path;

use crate::{
    bsr::Bsr, pim::Pim, result::SingleResult, settings::MemSettings, two_matrix::TwoMatrix,
};

use eyre::Result;
use itertools::Itertools;
use sprs::CsMat;
use tracing::{debug, Level};

/// run the matrix csr x csr_transpose
pub fn run_exp_csr<'a, const R: usize, const C: usize>(
    path: &'a Path,
    csr: &CsMat<i32>,
    mem_settings: &MemSettings,
) -> Result<SingleResult<'a>> {
    let span = tracing::span!(Level::INFO,"run_exp_csr", path = ?path);
    let _entered = span.enter();
    debug!("original_csr nnz: {}", csr.nnz());
    let oldnnz = csr.nnz();
    let csr_transpose = csr.transpose_view().to_csr();

    let bsr: Bsr<R, C, _> = Bsr::from(csr.clone());
    let bsr_transpose: Bsr<C, R, _> = Bsr::from(csr_transpose);
    let new_nnz = bsr.nnz();
    debug!("bsr_{}_{}_nnz: {}", R, C, bsr.nnz());
    debug!("bsr_{}_{}_element: {}", R, C, bsr.nnz() * C * R);

    let csr: CsMat<_> = bsr.into();
    let csr_transpose: CsMat<_> = bsr_transpose.into();
    let two_mat = TwoMatrix::new(csr, csr_transpose);

    let row_read = two_mat.mem_rows(mem_settings);
    let (bank_merged_cycles, partial_sum) = two_mat.bank_merge(mem_settings);
    let (chip_merged_cycles, partial_sum) = two_mat.chip_merge(mem_settings, &partial_sum);
    let (channel_merged_cycles, partial_sum) = two_mat.channel_merge(mem_settings, &partial_sum);
    let (dimm_merged_cycles, _partial_sum) = two_mat.dimm_merge(mem_settings, &partial_sum);

    // decompose
    let bank_add = bank_merged_cycles.iter().map(|x| x.add_cycle).collect_vec();

    let bank_merge = bank_merged_cycles
        .iter()
        .map(|x| x.merge_cycle)
        .collect_vec();
    let chip_add = chip_merged_cycles.iter().map(|x| x.add_cycle).collect_vec();
    let chip_merge = chip_merged_cycles
        .iter()
        .map(|x| x.merge_cycle)
        .collect_vec();
    let channel_add = channel_merged_cycles
        .iter()
        .map(|x| x.add_cycle)
        .collect_vec();
    let channel_merge = channel_merged_cycles
        .iter()
        .map(|x| x.merge_cycle)
        .collect_vec();
    let dimm_add = dimm_merged_cycles.add_cycle;
    let dimm_merge = dimm_merged_cycles.merge_cycle;

    let single_result = SingleResult {
        file: path,
        r: R,
        c: C,
        block_size: R * C,
        origin_nnz: oldnnz,
        new_nnz,
        new_element: new_nnz * C * R,
        need_speed_up: (new_nnz * C * R) as f32 / (oldnnz as f32),
        row_read,
        bank_add,
        bank_merge,
        chip_add,
        chip_merge,
        channel_add,
        channel_merge,
        dimm_add,
        dimm_merge,
    };
    debug!("{:?}", single_result);
    tracing::info!(?path, "run_exp_csr done");
    Ok(single_result)
}

// pub fn run_exp_filebuf<'a, const R: usize, const C: usize>(
//     path: &'a Path,
//     filebuf: &str,
//     mem_settings: &MemSettings,
// ) -> Result<SingleResult<'a>> {
//     let mut filebuf = BufReader::new(filebuf.as_bytes());
//     let tri: TriMat<i32> = sprs::io::read_matrix_market_from_bufread(&mut filebuf)?;
//     let csr: CsMat<_> = tri.to_csr();
//     debug!("original_csr nnz: {}", csr.nnz());
//     let csr_nnz = csr.nnz();
//     let bsr: Bsr<R, C, _> = Bsr::from(csr);
//     let csr: CsMat<_> = bsr.into();
//     debug!("bsr_{}_{}_nnz: {}", R, C, csr.nnz());
//     debug!("bsr_{}_{}_element: {}", R, C, csr.nnz() * C * R);

//     let row_read = csr.mem_rows(mem_settings);
//     let (bank_merged_cycles, partial_sum) = csr.bank_merge(mem_settings);
//     let (chip_merged_cycles, partial_sum) = csr.chip_merge(mem_settings, &partial_sum);
//     let (channel_merged_cycles, partial_sum) = csr.channel_merge(mem_settings, &partial_sum);
//     let (dimm_merged_cycles, _partial_sum) = csr.dimm_merge(mem_settings, &partial_sum);

//     // decompose
//     let bank_add = bank_merged_cycles.iter().map(|x| x.add_cycle).collect_vec();

//     let bank_merge = bank_merged_cycles
//         .iter()
//         .map(|x| x.merge_cycle)
//         .collect_vec();
//     let chip_add = chip_merged_cycles.iter().map(|x| x.add_cycle).collect_vec();
//     let chip_merge = chip_merged_cycles
//         .iter()
//         .map(|x| x.merge_cycle)
//         .collect_vec();
//     let channel_add = channel_merged_cycles
//         .iter()
//         .map(|x| x.add_cycle)
//         .collect_vec();
//     let channel_merge = channel_merged_cycles
//         .iter()
//         .map(|x| x.merge_cycle)
//         .collect_vec();
//     let dimm_add = dimm_merged_cycles.add_cycle;
//     let dimm_merge = dimm_merged_cycles.merge_cycle;

//     let single_result = SingleResult {
//         file: path,
//         r: R,
//         c: C,
//         block_size: R * C,
//         origin_nnz: csr_nnz,
//         new_nnz: csr.nnz(),
//         new_element: csr.nnz() * C * R,
//         need_speed_up: (csr.nnz() * C * R) as f32 / (csr_nnz as f32),
//         row_read,
//         bank_add,
//         bank_merge,
//         chip_add,
//         chip_merge,
//         channel_add,
//         channel_merge,
//         dimm_add,
//         dimm_merge,
//     };
//     debug!("{:?}", single_result);
//     Ok(single_result)
// }

// pub fn run_exp<'a, const R: usize, const C: usize>(
//     path: &'a Path,
//     mem_settings: &MemSettings,
// ) -> Result<SingleResult<'a>> {
//     let tri: TriMat<i32> = sprs::io::read_matrix_market(path)?;
//     let csr: CsMat<_> = tri.to_csr();
//     debug!("original_csr nnz: {}", csr.nnz());
//     let csr_nnz = csr.nnz();
//     let bsr: Bsr<R, C, _> = Bsr::from(csr);
//     let csr: CsMat<_> = bsr.into();
//     debug!("bsr_{}_{}_nnz: {}", R, C, csr.nnz());
//     debug!("csr_{}_{}_element: {}", R, C, csr.nnz() * C * R);

//     let row_read = csr.mem_rows(mem_settings);
//     let (bank_merged_cycles, partial_sum) = csr.bank_merge(mem_settings);
//     let (chip_merged_cycles, partial_sum) = csr.chip_merge(mem_settings, &partial_sum);
//     let (channel_merged_cycles, partial_sum) = csr.channel_merge(mem_settings, &partial_sum);
//     let (dimm_merged_cycles, _partial_sum) = csr.dimm_merge(mem_settings, &partial_sum);

//     // decompose
//     let bank_add = bank_merged_cycles.iter().map(|x| x.add_cycle).collect_vec();

//     let bank_merge = bank_merged_cycles
//         .iter()
//         .map(|x| x.merge_cycle)
//         .collect_vec();
//     let chip_add = chip_merged_cycles.iter().map(|x| x.add_cycle).collect_vec();
//     let chip_merge = chip_merged_cycles
//         .iter()
//         .map(|x| x.merge_cycle)
//         .collect_vec();
//     let channel_add = channel_merged_cycles
//         .iter()
//         .map(|x| x.add_cycle)
//         .collect_vec();
//     let channel_merge = channel_merged_cycles
//         .iter()
//         .map(|x| x.merge_cycle)
//         .collect_vec();
//     let dimm_add = dimm_merged_cycles.add_cycle;
//     let dimm_merge = dimm_merged_cycles.merge_cycle;

//     let single_result = SingleResult {
//         file: path,
//         r: R,
//         c: C,
//         block_size: R * C,
//         origin_nnz: csr_nnz,
//         new_nnz: csr.nnz(),
//         new_element: csr.nnz() * C * R,
//         need_speed_up: (csr.nnz() * C * R) as f32 / (csr_nnz as f32),
//         row_read,
//         bank_add,
//         bank_merge,
//         chip_add,
//         chip_merge,
//         channel_add,
//         channel_merge,
//         dimm_add,
//         dimm_merge,
//     };
//     debug!("{:?}", single_result);
//     Ok(single_result)
// }

#[macro_export]
macro_rules! run_1d_c_unroll {
    ($file:expr;$mem_settings:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; $size0:literal)=>{
        $fun::<1,$size0>($file,$mem_settings).map_or_else(
            |_| {
                $err_list.push($file);

            },
            |x| {
                $full_result.all.push(x);$ok_list.push($file);
            },
        );
    };
    ($file:expr;$mem_settings:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; $size0:literal, $($size:literal),+) => {
        $fun::<1,$size0>($file,$mem_settings).map_or_else(
            |_| {
                $err_list.push($file);
            },

            |x| {$full_result.all.push(x);$ok_list.push($file);
            },
        );

        run_1d_c_unroll!($file;$mem_settings;$full_result;$ok_list;$err_list; $fun;  $($size),+ );
    };

}

#[macro_export]
macro_rules! run_2d_unroll {
    ($file:expr;$mem_settings:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; ($r:literal,$c:literal))=>{
        $fun::<$r,$c>($file,$mem_settings).map_or_else(
            |_| {
                $err_list.push($file);
            },
            |x| {$full_result.all.push(x);$ok_list.push($file);},
        );
    };
    ($file:expr;$mem_settings:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; ($r:literal,$c:literal), $(($r1:literal,$c1:literal)),+) => {
        $fun::<$r,$c>($file,$mem_settings).map_or_else(
            |_| {
                $err_list.push($file);
            },

            |x| {$full_result.all.push(x);$ok_list.push($file);},
        );

        run_2d_unroll!($file;$mem_settings;$full_result;$ok_list;$err_list; $fun;  $(($r1,$c1)),+ );
    };

}

#[macro_export]
macro_rules! run_1d_c_unroll_buf {
    ($file:expr;$file_buf:expr;$mem_settings:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; $size0:literal)=>{
        $fun::<1,$size0>($file,$file_buf,$mem_settings).map_or_else(
            |_| {
                $err_list.push($file);
            },
            |x| {
                $full_result.all.push(x);$ok_list.push($file);
            },
        );
    };
    ($file:expr;$file_buf:expr;$mem_settings:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; $size0:literal, $($size:literal),+) => {
        $fun::<1,$size0>($file,$file_buf,$mem_settings).map_or_else(
            |_| {
                $err_list.push($file);
            },

            |x| {
                $full_result.all.push(x);$ok_list.push($file);
            },
        );


        run_1d_c_unroll_buf!($file;$file_buf;$mem_settings;$full_result;$ok_list;$err_list; $fun;  $($size),+ );
    };

}

#[macro_export]
macro_rules! run_2d_unroll_buf {
    ($file:expr;$file_buf:expr;$mem_settings:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; ($r:literal,$c:literal))=>{
        $fun::<$r,$c>($file,$file_buf,$mem_settings).map_or_else(
            |_| {
                $err_list.push($file);
            },
            |x| {$full_result.all.push(x);$ok_list.push($file);},
        );
    };
    ($file:expr;$file_buf:expr;$mem_settings:expr;$full_result:expr;$ok_list:expr;$err_list:expr;$fun:ident; ($r:literal,$c:literal), $(($r1:literal,$c1:literal)),+) => {
        $fun::<$r,$c>($file,$file_buf,$mem_settings).map_or_else(
            |_| {
                $err_list.push($file);
            },

            |x| {$full_result.all.push(x);$ok_list.push($file);},
        );

        run_2d_unroll_buf!($file;$file_buf;$mem_settings;$full_result;$ok_list;$err_list; $fun;  $(($r1,$c1)),+ );
    };

}

#[cfg(test)]
mod test {
    // use std::path::PathBuf;

    // use tracing::debug;

    // use crate::result::Results;
    // use crate::settings::MemSettings;
    // #[test]
    // fn test() {
    //     let path = PathBuf::from("test.mtx");
    //     let mut full_result = Results { all: vec![] };
    //     let mut ok_list = vec![];
    //     let mut err_list = vec![];
    //     let mem_settings = MemSettings {
    //         row_size: 16,
    //         banks: 2,
    //         chips: 2,
    //         channels: 2,
    //         row_mapping: crate::settings::RowMapping::Chunk,
    //         bank_merger_size: 2,
    //         chip_merger_size: 2,
    //         channel_merger_size: 2,
    //         dimm_merger_size: 2,
    //         simd_width: 2,
    //         ..Default::default()
    //     };

    //     run_1d_c_unroll!(&path;&mem_settings; full_result; ok_list;err_list;run_exp; 1,2,3);
    //     debug!("{:?}", full_result);
    // }

    // #[test]
    // fn test2d() {
    //     let path = PathBuf::from("test.mtx");
    //     let mut full_result = Results { all: vec![] };
    //     let mut ok_list = vec![];
    //     let mut err_list = vec![];
    //     let mem_settings = MemSettings {
    //         row_size: 16,
    //         banks: 2,
    //         chips: 2,
    //         channels: 2,
    //         row_mapping: crate::settings::RowMapping::Chunk,
    //         bank_merger_size: 2,
    //         chip_merger_size: 2,
    //         channel_merger_size: 2,
    //         dimm_merger_size: 2,
    //         simd_width: 2,
    //         ..Default::default()
    //     };
    //     run_2d_unroll!(&path;&mem_settings; full_result; ok_list;err_list;run_exp;(1,1),(2,2),(3,3),(4,4));
    //     debug!("{:?}", full_result);
    // }
}
