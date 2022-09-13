use std::fs;
use std::fs::File;
use std::io::{self};

use super::{
    args::{Args, RunMode},
    result::{self, Results},
    run_2d_unroll_buf,
    settings::Settings,
    two_matrix::TwoMatrix,
    utils::run::run_exp_csr,
};
use crate::init_logger;
use crate::sim::sim_time::AllTimeStats;
use crate::sim::Simulator;
use clap::{Command, IntoApp};
use clap_complete::Generator;
use eyre::{Context, Result};
use itertools::Itertools;
use sprs::CsMat;
use tracing::{debug, error, info};

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    clap_complete::generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}
pub fn main(args: Args) -> Result<()> {
    init_logger();
    let start_time = std::time::Instant::now();
    if let Some(generator) = args.generator {
        let mut cmd = Args::command();
        eprintln!("Generating completion file for {:?}...", generator);
        print_completions(generator, &mut cmd);
        return Ok(());
    }
    info!("start sim with {:?}", args);

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
    let run_mode = args.run_mode.unwrap_or(RunMode::Sim);
    fs::create_dir_all("results")?;
    match run_mode {
        RunMode::Sim => {
            info!("sim start");
            let graph_name = settings.mtx_files;
            let mut all_results = AllTimeStats { data: Vec::new() };
            let results: Vec<eyre::Result<_>> = graph_name
                .iter()
                .map(|name| {
                    info!("graph: {:?}", name);
                    let csr: CsMat<i32> = sprs::io::read_matrix_market(name)
                        .wrap_err(format!("{:?} is error!", name))?
                        .to_csr();
                    let mtx_file_name = name.file_stem().unwrap();
                    let trans_pose = csr.transpose_view().to_csr();
                    let two_matrix = TwoMatrix::new(csr, trans_pose);
                    let (time, time_stats, detailed_time_status, end_time_stats) =
                        Simulator::run(&settings.mem_settings, two_matrix)?;
                    let time_stats = time_stats.to_rate();
                    let detailed_time_status = detailed_time_status.to_rate();
                    let file_path = mtx_file_name.to_string_lossy();
                    let task_queue_size = settings.mem_settings.sender_store_size;
                    let interleaving_chunk_size = settings.mem_settings.interleaved_chunk;
                    let row_mapping=&settings.mem_settings.row_mapping;
                    let scheduler_mode=&settings.mem_settings.task_scheduler_mode;
                    let batch_size=settings.mem_settings.task_scheduler_chunk_size;
                    serde_json::to_writer_pretty(
                        File::create(format!("results/full_time_{row_mapping:?}_{scheduler_mode:?}_{task_queue_size}_{interleaving_chunk_size}_{batch_size}_{file_path}.json"))?,
                        &time,
                    )?;
                    serde_json::to_writer_pretty(
                        File::create(format!("results/time_stats_{row_mapping:?}_{scheduler_mode:?}_{task_queue_size}_{interleaving_chunk_size}_{batch_size}_{file_path}.json"))?,
                        &time_stats,
                    )?;
                    serde_json::to_writer_pretty(
                        File::create(format!("results/detailed_time_{row_mapping:?}_{scheduler_mode:?}_{task_queue_size}_{interleaving_chunk_size}_status_{batch_size}_{file_path}.json"))?,
                        &detailed_time_status,
                    )?;
                    serde_json::to_writer_pretty(
                        File::create(format!("results/end_time_{row_mapping:?}_{scheduler_mode:?}_{task_queue_size}_{interleaving_chunk_size}_stats_{batch_size}_{file_path}.json"))?,
                        &end_time_stats,
                    )?;

                    all_results.data.push((file_path.to_string(), time_stats));
                    // write the result to file

                    Ok(name)
                })
                .collect_vec();
            for r in results {
                match r {
                    Ok(name) => info!("finished graph: {:?}", name),
                    Err(e) => error!("{:?}", e),
                }
            }
            // let time_stats_output = serde_json::to_string_pretty(&all_results)
            //     .wrap_err("fail to serialize all_results")?;
            // // write the result to file "all_results.json"
            // let file_name = "results/time_stats_all_results.json";
            // let mut _file = File::create(&file_name)
            //     .wrap_err(format!("the path: {} is invalid!", file_name))?;
            // writeln!(_file, "{}", time_stats_output)?;
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
