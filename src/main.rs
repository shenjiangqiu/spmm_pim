use std::env::args_os;
use std::io;

use clap::{Command, IntoApp, Parser};
use clap_complete::Generator;
use eyre::{Context, Result};
use itertools::Itertools;
use log::{debug, error, info};

use spmm_pim::{
    args::{Args, RunMode},
    result::{self, Results},
    run_2d_unroll_buf,
    settings::Settings,
    two_matrix::TwoMatrix,
    utils::run::run_exp_csr,
};
use sprs::CsMat;

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    clap_complete::generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

fn main() -> Result<()> {
    let args = args_os();
    let args = Args::parse_from(args);
    _main(args)
}

fn _main(args: Args) -> Result<()> {
    let config_str = include_str!("../log_config.yml");
    let config = serde_yaml::from_str(config_str).unwrap();
    log4rs::init_raw_config(config).unwrap_or_else(|err| {
        error!("log4rs init error: {}", err);
    });
    info!("start sim with {:?}", args);

    let start_time = std::time::Instant::now();
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

    println!("config");

    let settings = Settings::new(&config_files).wrap_err("fail to create Setting object")?;
    debug!("{:?}", settings);
    let mtxs = settings.mtx_files.clone();

    match args.run_mode {
        RunMode::Sim => {
            info!("sim start");
            let graph_name = settings.mtx_files;
            let results: Vec<eyre::Result<_>> = graph_name
                .iter()
                .map(|name| {
                    info!("graph: {:?}", name);
                    let csr: CsMat<i32> = sprs::io::read_matrix_market(name)
                        .wrap_err(format!("{:?} is error!", name))?
                        .to_csr();
                    let trans_pose = csr.transpose_view().to_csr();
                    let two_matrix = TwoMatrix::new(csr, trans_pose);
                    spmm_pim::sim::Simulator::run(&settings.mem_settings, two_matrix);
                    info!("finished graph: {:?}", name);
                    Ok(name)
                })
                .collect_vec();
            for r in results {
                match r {
                    Ok(name) => info!("finished graph: {:?}", name),
                    Err(e) => error!("{:?}", e),
                }
            }
            Ok(())
        }
        RunMode::Pim => {
            let mut full_result = Results { all: vec![] };
            let mut ok_list = vec![];
            let mut err_list = vec![];
            // load config into ConfigFile
            for i in mtxs.iter() {
                match sprs::io::read_matrix_market(i) {
                    Ok(tri) => {
                        let csr = tri.to_csr();
                        // run_1d_c_unroll_buf!(i;&csr;&settings.mem_settings;full_result;ok_list;err_list; run_exp_csr; 64,128,256,512,1024,2048);
                        // run_2d_unroll_buf!(i;&csr;&settings.mem_settings; full_result;ok_list;err_list; run_exp_csr; (2,32),(4,16),(8,8),(2,64),(4,32),(8,16),(2,128),(4,64),(8,32),(16,16),(2,256),(4,128),(8,64),(16,32),
                        // (2,512),(4,256),(8,128),(16,64),(32,32), (2,1024),(4,512),(8,256),(16,128),(32,64));
                        run_2d_unroll_buf!(i;&csr;&settings.mem_settings; full_result;ok_list;err_list; run_exp_csr;(1,1),(4,4));
                    }
                    Err(e) => {
                        err_list.push(i);
                        error!("{}", e);
                    }
                }
            }
            let file_name = settings.result_file;
            full_result
                .save_to_file(&file_name)
                .wrap_err("file to save result")?;
            result::save_result_list(&ok_list, &err_list, &file_name)
                .wrap_err("file to save result")?;
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
    }
}
#[cfg(test)]
mod test_main {

    use clap::StructOpt;
    use spmm_pim::args::Args;

    #[test]
    fn test_main() {
        let args = vec![
            "spmm_pim",
            "-r",
            "pim",
            "configs/default.toml",
            "configs/ddr4.toml",
        ];
        let args = Args::parse_from(args);
        println!("hello world!");
        super::_main(args).unwrap();

        let args = vec![
            "spmm_pim",
            "-r",
            "sim",
            "configs/default.toml",
            "configs/ddr4.toml",
        ];
        let args = Args::parse_from(args);
        println!("hello world!");
        super::_main(args).unwrap();
    }
}
