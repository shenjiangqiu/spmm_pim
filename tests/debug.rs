use eyre::Result;
use log::debug;
use spmm_pim::result::save_result_list;
use spmm_pim::utils::run::run_exp_csr;
use spmm_pim::run_2d_unroll_buf;
use spmm_pim::{result::Results, settings::Settings};
use sprs::CsMat;
use std::path::{Path, PathBuf};
#[test]
fn test_real_matrix() -> Result<()> {
    let config_str = include_str!("../log_config.yml");
    let config = serde_yaml::from_str(config_str).unwrap();
    log4rs::init_raw_config(config).unwrap();

    let config_files: Vec<PathBuf> = vec!["configs/large.toml".into(), "configs/ddr4.toml".into()];
    let settings = Settings::new(&config_files)?;
    debug!("{:?}", settings);
    let mtxs = settings.mtx_files;

    let mut full_result = Results { all: vec![] };
    let mut ok_list = vec![];
    let mut err_list = vec![];
    // load config into ConfigFiles
    for i in mtxs.iter() {
        let csr: CsMat<i32> = sprs::io::read_matrix_market(i)?.to_csr();

        run_2d_unroll_buf!(i; &csr;&settings.mem_settings; full_result;ok_list;err_list; run_exp_csr; (1,1));
    }
    full_result.save_to_file(Path::new("results/result_test.json"))?;
    save_result_list(&ok_list, &err_list, Path::new("results/result_test.json"))?;
    Ok(())
}
