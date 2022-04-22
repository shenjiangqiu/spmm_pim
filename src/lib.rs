pub mod args;
pub mod bsr;
pub mod bsr_row_builder;
pub mod pim;
pub mod result;
pub mod run;
pub mod settings;
pub mod utils;

use std::{io::BufReader, path::Path};

use bsr::Bsr;
use result::{Results, SingleResult};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn run(name: String) -> String {
    let res = reqwest::get(format!("http://localhost:8081/{}", name))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let mut buf = BufReader::new(res.as_bytes());
    let trimat = sprs::io::read_matrix_market_from_bufread(&mut buf).unwrap();

    let csr = trimat.to_csr();
    let original_nnz = csr.nnz();

    let bsr: Bsr<2, 2, i32> = csr.into();

    let path = Path::new("123.mtx");
    let single_result = SingleResult {
        file: path,
        r: 2,
        c: 2,
        block_size: 2 * 2,
        origin_nnz: original_nnz,
        new_nnz: bsr.nnz(),
        new_element: bsr.nnz() * 2 * 2,
        need_speed_up: (bsr.nnz() * 2 * 2) as f32 / (original_nnz as f32),
    };
    let mut all_result = Results { all: vec![] };
    all_result.all.push(single_result);
    serde_json::to_string_pretty(&all_result).unwrap()
}

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
