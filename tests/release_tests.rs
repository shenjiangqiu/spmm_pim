use clap::Parser;
use eyre::Result;
use log::debug;

use spmm_pim::{
    args::Args,
    result::{self, Results},
    run_2d_unroll_buf, run_main,
    settings::Settings,
    utils::run::run_exp_csr,
};
use sprs::CsMat;
use std::path::{Path, PathBuf};

#[test]
fn test() -> Result<()> {
    for store in [32, 64, 128, 256, 512] {
        for inter in [1, 2, 4, 8, 16] {
            let store_config = format!("configs/store_sizes/{store}.toml");
            let inter_config = format!("configs/interleaving/{inter}.toml");
            let args = vec![
                "spmm_pim",
                "-r",
                "sim",
                "configs/large.toml",
                "configs/ddr4.toml",
                &store_config,
                &inter_config,
            ];
            let args = Args::parse_from(args);
            println!("hello world!");
            run_main::main(args).unwrap();
        }
    }
    Ok(())
}
#[test]
fn collect_data() {
    let mut results = vec![];
    for store in [32, 64, 128, 256, 512] {
        let mut temp_vec = vec![];
        for inter in [1, 2, 4, 8, 16] {
            let result_file = format!(
                "results/full_time_{}_{}_Groebner_id2003_aug.json",
                store, inter
            );
            let time: f64 =
                serde_json::from_reader(std::fs::File::open(result_file).unwrap()).unwrap();
            println!("{}", time);
            temp_vec.push(time);
        }
        results.push(temp_vec);
    }
    println!("{:?}", results);
    println!("store: {:?}", [1, 2, 4, 8, 16]);
    for (store, result) in [32, 64, 128, 256, 512].into_iter().zip(results) {
        print!("{:?} ", store);
        for time in result {
            print!("{:?} ", time);
        }
        println!();
    }
}

#[test]
fn run_chunck() -> Result<()> {
    for store in [32, 64, 128, 256, 512] {
        let store_config = format!("configs/store_sizes/{store}.toml");
        let args = vec![
            "spmm_pim",
            "-r",
            "sim",
            "configs/large.toml",
            "configs/ddr4chunk.toml",
            &store_config,
        ];
        let args = Args::parse_from(args);
        println!("hello world!");
        run_main::main(args).unwrap();
    }
    for store in [32, 64, 128, 256, 512] {
        let result_file = format!(
            "results/full_time_Chunk_{}_16_Groebner_id2003_aug.json",
            store
        );
        let time: f64 = serde_json::from_reader(std::fs::File::open(result_file).unwrap()).unwrap();
        println!("{}", time);
    }

    Ok(())
}
#[test]
fn get_chunk() {
    for store in [32, 64, 128, 256, 512] {
        let result_file = format!(
            "results/full_time_Chunk_{}_16_Groebner_id2003_aug.json",
            store
        );
        let time: f64 = serde_json::from_reader(std::fs::File::open(result_file).unwrap()).unwrap();
        println!("{}", time);
    }
}
