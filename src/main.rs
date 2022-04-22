use std::io;

use clap::{Command, IntoApp, Parser};
use clap_complete::Generator;
use eyre::{Context, Result};
use log::{debug, info};
use spmm_pim::result::save_result_list;
use spmm_pim::run::run_exp;
use spmm_pim::utils::init_log;
use spmm_pim::{args::Args, result::Results, settings::Settings};
use spmm_pim::{run_1d_c_unroll, run_2d_unroll};

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    clap_complete::generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}
fn main() -> Result<()> {
    let start_time = std::time::Instant::now();
    init_log("info");
    let args = Args::parse();

    if let Some(generator) = args.generator {
        let mut cmd = Args::command();
        eprintln!("Generating completion file for {:?}...", generator);
        print_completions(generator, &mut cmd);
        return Ok(());
    }

    debug!("{:?}", args);
    let mut config_files = args.config_file;
    if config_files.is_empty() {
        config_files.push("configs/default.toml".into());
        config_files.push("configs/ddr4.toml".into());
    }

    let settings = Settings::new(&config_files).wrap_err("fail to create Setting object")?;
    debug!("{:?}", settings);
    let mtxs = settings.mtx_files.clone();

    let mut full_result = Results { all: vec![] };
    let mut ok_list = vec![];
    let mut err_list = vec![];
    // load config into ConfigFile
    for i in mtxs.iter() {
        run_1d_c_unroll!(i;full_result;ok_list;err_list; run_exp; 64,128,256,512,1024,2048);
        run_2d_unroll!(i;full_result;ok_list;err_list; run_exp; (2,32),(4,16),(8,8),(2,64),(4,32),(8,16),(2,128),(4,64),(8,32),(16,16),(2,256),(4,128),(8,64),(16,32),
        (2,512),(4,256),(8,128),(16,64),(32,32), (2,1024),(4,512),(8,256),(16,128),(32,64));
    }
    let file_name = settings.result_file;
    full_result
        .save_to_file(&file_name)
        .wrap_err("file to save result")?;
    save_result_list(&ok_list, &err_list, &file_name).wrap_err("file to save result")?;
    info!(
        "running time: {:?}'s",
        std::time::Instant::now()
            .duration_since(start_time)
            .as_secs_f64()
    );
    info!("the list of files succeeded: {:?}", ok_list);
    info!("the list of files failed: {:?}", err_list);
    Ok(())
}
