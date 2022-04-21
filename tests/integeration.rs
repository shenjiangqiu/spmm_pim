use env_logger::Env;
use eyre::Result;
use log::debug;
use spmm_pim::result::save_result_list;
use spmm_pim::run::run_exp;
use spmm_pim::{result::Results, run_unroll, settings::Settings};
use std::path::Path;

#[test]
fn test() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug"))
        .try_init()
        .unwrap_or_default();

    let settings = Settings::new("default.toml".into())?;
    debug!("{:?}", settings);
    let mtxs = settings.mtx_files.clone();

    let mut full_result = Results { all: vec![] };
    let mut ok_list = vec![];
    let mut err_list = vec![];
    // load config into ConfigFile
    for i in mtxs.iter() {
        run_unroll!(i;full_result;ok_list;err_list; run_exp; 64,128,256,512,1024,2048);
    }
    full_result.save_to_file(Path::new("result_test.json"))?;
    save_result_list(&ok_list, &err_list, Path::new("result_test.json"))?;
    Ok(())
}
