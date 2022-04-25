pub mod args;
pub mod bsr;
pub mod bsr_row_builder;
pub mod pim;
pub mod result;
pub mod run;
pub mod settings;
pub mod utils;
pub mod csv_nodata;
use result::{Results, SingleResult};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::run::run_exp_filebuf;

#[derive(Serialize)]
struct CombinedResult<'a> {
    results: Vec<SingleResult<'a>>,
    ok_list: Vec<&'a String>,
    err_list: Vec<&'a String>,
}

#[wasm_bindgen]
pub async fn run1(name: String) -> Result<String, JsValue> {
    let res = reqwest::get(format!("https://research.thesjq.com/files/{}", name))
        .await
        .map_err(JsError::from)?
        .text()
        .await
        .map_err(JsError::from)?;

    let mut full_result = Results { all: vec![] };
    let mut ok_list = vec![];
    let mut err_list = vec![];
    run_1d_c_unroll_buf!(&name;&res;full_result;ok_list;err_list; run_exp_filebuf; 64,128,256,512,1024,2048);
    run_2d_unroll_buf!(&name;&res;full_result;ok_list;err_list; run_exp_filebuf; (2,32),(4,16),(8,8),(2,64),(4,32),(8,16),(2,128),(4,64),(8,32),(16,16),(2,256),(4,128),(8,64),(16,32),
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
    #[wasm_bindgen_test]
    fn test_wasm(){
        
    }
}
