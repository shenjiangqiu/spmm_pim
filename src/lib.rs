#![feature(generators, generator_trait)]

pub mod args;
pub mod bsr;
pub mod bsr_row_builder;
pub mod csv_nodata;
pub mod pim;
pub mod reorder_calculator;
pub mod reorder_system;
pub mod result;
pub mod run;
pub mod settings;
pub mod sim;
pub mod two_matrix;
pub(self) mod utils;

use std::{io::BufReader, path::Path};

use result::{Results, SingleResult};
use serde::Serialize;
use sprs::{CsMat, TriMat};
use wasm_bindgen::prelude::*;

use crate::{
    run::run_exp_csr,
    settings::{MemSettings, RowMapping},
};
#[derive(Serialize)]
struct CombinedResult<'a> {
    results: Vec<SingleResult<'a>>,
    ok_list: Vec<&'a Path>,
    err_list: Vec<&'a Path>,
}
#[wasm_bindgen]
pub async fn run1(name: String) -> Result<String, JsValue> {
    let res = reqwest::get(format!("https://research.thesjq.com/files/{}", name))
        .await
        .map_err(JsError::from)?
        .text()
        .await
        .map_err(JsError::from)?;

    let mut filebuf = BufReader::new(res.as_bytes());
    let tri: TriMat<i32> =
        sprs::io::read_matrix_market_from_bufread(&mut filebuf).map_err(JsError::from)?;
    let path = Path::new(&name);

    let mut full_result = Results { all: vec![] };
    let mut ok_list = vec![];
    let mut err_list = vec![];
    let mem_settings = MemSettings {
        row_size: 512,
        banks: 8,
        chips: 8,
        channels: 2,
        row_mapping: RowMapping::Chunk,
        bank_merger_size: 8,
        chip_merger_size: 8,
        channel_merger_size: 8,
        dimm_merger_size: 8,
        simd_width: 128,
        parallel_count: 8,
        reorder_count: 8,
        bank_merger_count: 8,
        chip_merger_count: 8,
        channel_merger_count: 8,
        dimm_merger_count: 8,
        row_change_latency: 8,
        bank_adder_size: 8,
    };
    let csr: CsMat<_> = tri.to_csr();

    run_1d_c_unroll_buf!(path;&csr;&mem_settings;full_result;ok_list;err_list; run_exp_csr; 64,128,256,512,1024,2048);
    run_2d_unroll_buf!(path;&csr;&mem_settings;full_result;ok_list;err_list; run_exp_csr; (2,32),(4,16),(8,8),(2,64),(4,32),(8,16),(2,128),(4,64),(8,32),(16,16),(2,256),(4,128),(8,64),(16,32),
        (2,512),(4,256),(8,128),(16,64),(32,32), (2,1024),(4,512),(8,256),(16,128),(32,64));
    if !err_list.is_empty() {
        return Err(JsValue::from_str(&format!("{:?}", err_list)));
    }

    let combined_result = CombinedResult {
        results: full_result.all,
        ok_list,
        err_list,
    };

    serde_json::to_string_pretty(&combined_result).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(test)]
mod test {
    use eyre::Result;
    use log::debug;
    use sprs::{CsMat, TriMat};
    use wasm_bindgen_test::wasm_bindgen_test;

    #[test]
    fn test_csc() -> Result<()> {
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx")?;
        let csc: CsMat<_> = matrix.to_csc();
        debug!("{:?}", csc);
        Ok(())
    }

    #[test]
    fn test_csr() -> Result<()> {
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx")?;
        let csr: CsMat<_> = matrix.to_csr();
        debug!("{:?}", csr);
        Ok(())
    }

    #[test]
    fn test_bsr() -> Result<()> {
        let matrix: TriMat<i32> = sprs::io::read_matrix_market("mtx/test.mtx")?;
        let bsr: super::bsr::Bsr<2, 2, _> = super::bsr::Bsr::from(matrix.to_csr());
        debug!("{:?}", bsr);
        Ok(())
    }
    #[wasm_bindgen_test]
    fn test_wasm() {}
}
