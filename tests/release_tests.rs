use clap::Parser;
use eyre::{Context, Result};

use itertools::Itertools;
use rayon::prelude::*;
use spmm_pim::{args::Args, run_main};
#[test]
#[ignore]
fn test() -> Result<()> {
    let store_sizes = [32, 64, 128, 256, 512];
    let interleave_chunk_sizes = [1, 2, 4, 8, 16];
    let store_and_interleave = store_sizes
        .iter()
        .cartesian_product(interleave_chunk_sizes.iter());

    store_and_interleave
        .par_bridge()
        .for_each(|(store, inter)| {
            let store_config = format!("configs/store_sizes/{store}.toml");
            let inter_config = format!("configs/interleaving/{inter}.toml");
            let args = vec![
                "spmm_pim",
                "-r",
                "sim",
                "configs/large.toml",
                "configs/ddr4.toml",
                // "configs/scheduler_modes/shuffle.toml",
                "configs/scheduler_modes/sequence.toml",
                &store_config,
                &inter_config,
            ];
            let args = Args::parse_from(args);
            println!("hello world!");
            run_main::main(args).unwrap();
        });

    Ok(())
}
#[test]
#[ignore]
fn collect_data() {
    let mut results = vec![];
    for store in [32, 64, 128, 256, 512] {
        let mut temp_vec = vec![];
        for inter in [1, 2, 4, 8, 16] {
            let result_file = format!(
                "results/full_time_Interleaved{}_{}_{}_{}.json",
                "", store, inter, "Pd"
            );
            let time: f64 = serde_json::from_reader(
                std::fs::File::open(&result_file)
                    .expect(format!("{} not found", &result_file).as_str()),
            )
            .expect("failed to parse json");
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
#[ignore]
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
#[ignore]
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
