use eyre::Result;
use tracing::debug;

use spmm_pim::{
    result::{self, Results},
    run_2d_unroll_buf,
    settings::Settings,
    utils::run::run_exp_csr,
};
use sprs::CsMat;
use std::path::{Path, PathBuf};

#[test]
fn test() -> Result<()> {
    let config_files: Vec<PathBuf> = vec!["configs/debug.toml".into(), "configs/ddr4.toml".into()];
    let settings = Settings::new(&config_files)?;
    debug!("{:?}", settings);
    let mtxs = settings.mtx_files;

    let mut full_result = Results { all: vec![] };
    let mut ok_list = vec![];
    let mut err_list = vec![];
    // load config into ConfigFile
    for i in mtxs.iter() {
        let csr: CsMat<i32> = sprs::io::read_matrix_market(i)?.to_csr();

        run_2d_unroll_buf!(i; &csr;&settings.mem_settings; full_result;ok_list;err_list; run_exp_csr; (1,1));
    }
    full_result.save_to_file(Path::new("results/result_test.json"))?;
    result::save_result_list(&ok_list, &err_list, Path::new("results/result_test.json"))?;
    Ok(())
}
