use std::{error::Error, io::Write};

use env_logger::Env;
use itertools::Itertools;
use serde::Serialize;
use spmm_pim::bsr::Bsr;
use sprs::{CsMat, TriMat};

#[derive(Serialize)]
struct Results {
    all: Vec<SingleResult>,
}
#[derive(Serialize)]
struct SingleResult {
    file: String,
    c: usize,
    origin_nnz: usize,
    new_nnz: usize,
    new_element: usize,
    need_speed_up: f32,
}
fn run<const C: usize>(filename: &str) -> Result<SingleResult, Box<dyn Error>> {
    let tri: TriMat<i32> = sprs::io::read_matrix_market(filename)?;
    let csr: CsMat<_> = tri.to_csr();
    println!("original_csr nnz: {}", csr.nnz());
    let csr_nnz = csr.nnz();
    let bsr: Bsr<1, C, _> = Bsr::from(csr);

    println!("bsr_{}_{}_nnz: {}", 1, C, bsr.nnz());
    println!("bsr_{}_{}_element: {}", 1, C, bsr.nnz() * C);
    let single_result = SingleResult {
        file: String::from(filename),
        c: C,
        origin_nnz: csr_nnz,
        new_nnz: bsr.nnz(),
        new_element: bsr.nnz() * C,
        need_speed_up: (bsr.nnz() * C) as f32 / (csr_nnz as f32),
    };
    Ok(single_result)
}
fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn"))
        .try_init()
        .unwrap_or_default();

    let mtxs = vec![
        "lp_e226.mtx",
        "bcspwr07.mtx",
        "G51.mtx",
        "NotreDame_www.mtx",
        "Erdos971.mtx",
        "jagmesh7.mtx",
        "lp_e226_transposed.mtx",
        "bcspwr09.mtx",
        "lp_share1b.mtx",
        "Franz6_id1959_aug.mtx",
        "Groebner_id2003_aug.mtx",
        "w156.mtx",
        "plskz362.mtx",
        "bcspwr10.mtx",
        "bcspwr08.mtx",
        "young1c.mtx",
        "Pd.mtx",
        "bcspwr06.mtx",
        "dwt_992.mtx",
        "pts5ldd03.mtx",
    ];
    let mtxs = mtxs
        .into_iter()
        .map(|x| String::from("/home/sjq/mtx/") + x)
        .collect_vec();
    let mut full_result = Results { all: vec![] };

    for i in mtxs {
        run::<2048>(&i).map_or_else(
            |x| {
                println!("file: {} ,error: {}", i, x);
            },
            |x| full_result.all.push(x),
        );
        run::<1024>(&i).map_or_else(
            |x| {
                println!("file: {} ,error: {}", i, x);
            },
            |x| full_result.all.push(x),
        );
        run::<512>(&i).map_or_else(
            |x| {
                println!("file: {} ,error: {}", i, x);
            },
            |x| full_result.all.push(x),
        );
        run::<256>(&i).map_or_else(
            |x| {
                println!("file: {} ,error: {}", i, x);
            },
            |x| full_result.all.push(x),
        );
        run::<128>(&i).map_or_else(
            |x| {
                println!("file: {} ,error: {}", i, x);
            },
            |x| full_result.all.push(x),
        );
        run::<64>(&i).map_or_else(
            |x| {
                println!("file: {} ,error: {}", i, x);
            },
            |x| full_result.all.push(x),
        );
    }
    let res = serde_json::to_string_pretty(&full_result).unwrap();
    // write to file
    let file_name = "result.json";
    let mut file = std::fs::File::create(file_name).unwrap();
    file.write_all(res.as_bytes()).unwrap();
}
