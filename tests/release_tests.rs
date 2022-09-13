use clap::Parser;
use eyre::Result;

use itertools::Itertools;
use rayon::prelude::*;
use spmm_pim::{args::Args, run_main};
#[test]
#[ignore]
fn test() -> Result<()> {
    let store_sizes = [32, 64, 128, 256, 512];
    let interleave_chunk_sizes = [1, 2, 4, 8, 16];
    let scheduler_mode = ["sequence", "shuffle"];
    let store_and_interleave = store_sizes
        .into_iter()
        .cartesian_product(interleave_chunk_sizes)
        .cartesian_product(scheduler_mode);

    store_and_interleave
        .par_bridge()
        .for_each(|((store, inter), scheduler_mode)| {
            let store_config = format!("configs/store_sizes/{store}.toml");
            let inter_config = format!("configs/interleaving/{inter}.toml");
            let scheduler_config = format!("configs/scheduler_modes/{scheduler_mode}.toml");
            let args = vec![
                "spmm_pim",
                "-r",
                "sim",
                "configs/large.toml",
                "configs/ddr4.toml",
                &scheduler_config,
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
fn test_unlimited() -> Result<()> {
    let store_sizes = ["unlimited"];
    let interleave_chunk_sizes = [1, 2, 4, 8, 16];
    let scheduler_mode = ["sequence", "shuffle"];
    let store_and_interleave = store_sizes
        .into_iter()
        .cartesian_product(interleave_chunk_sizes)
        .cartesian_product(scheduler_mode);

    store_and_interleave
        .par_bridge()
        .for_each(|((store, inter), scheduler_mode)| {
            let store_config = format!("configs/store_sizes/{store}.toml");
            let inter_config = format!("configs/interleaving/{inter}.toml");
            let scheduler_config = format!("configs/scheduler_modes/{scheduler_mode}.toml");
            let args = vec![
                "spmm_pim",
                "-r",
                "sim",
                "configs/large.toml",
                "configs/ddr4.toml",
                &scheduler_config,
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
fn test_chunk() -> Result<()> {
    let store_sizes = [32, 64, 128, 256, 512];
    // let interleave_chunk_sizes = [1, 2, 4, 8, 16];
    let scheduler_mode = ["sequence", "shuffle"];
    let store_and_interleave = store_sizes.into_iter().cartesian_product(scheduler_mode);
    let batch_size = [16, 32, 64];
    let shuffle_tasks = store_and_interleave
        .par_bridge()
        .for_each(|(store, scheduler_mode)| {
            let store_config = format!("configs/store_sizes/{store}.toml");
            let scheduler_config = format!("configs/scheduler_modes/{scheduler_mode}.toml");
            let args = vec![
                "spmm_pim",
                "-r",
                "sim",
                "configs/large.toml",
                "configs/ddr4chunk.toml",
                &scheduler_config,
                &store_config,
            ];
            let args = Args::parse_from(args);
            println!("hello world!");
            run_main::main(args).unwrap();
        });

    Ok(())
}

#[test]
#[ignore]
fn test_batch() -> Result<()> {
    let store_sizes = [32, 64, 128, 256, 512];
    let interleave_chunk_sizes = [1, 2, 4, 8, 16];
    let scheduler_mode = ["batched_shuffle"];
    let batch_size = [16, 32, 64];
    let store_and_interleave = store_sizes
        .into_iter()
        .cartesian_product(interleave_chunk_sizes)
        .cartesian_product(scheduler_mode)
        .cartesian_product(batch_size);

    store_and_interleave
        .par_bridge()
        .for_each(|(((store, inter), scheduler_mode), batch_size)| {
            let store_config = format!("configs/store_sizes/{store}.toml");
            let inter_config = format!("configs/interleaving/{inter}.toml");
            let scheduler_config = format!("configs/scheduler_modes/{scheduler_mode}.toml");
            let batch_size_config =
                format!("configs/scheduler_modes/batch_sizes/{batch_size}.toml");
            let args = vec![
                "spmm_pim",
                "-r",
                "sim",
                "configs/large.toml",
                "configs/ddr4.toml",
                &scheduler_config,
                &store_config,
                &inter_config,
                &batch_size_config,
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
    let store_sizes = ["unlimited"];
    // let interleave_chunk_sizes = [1, 2, 4, 8, 16];
    let scheduler_mode = ["Shuffle", "Sequence"];

    for graph in ["Franz6_id1959_aug", "Groebner_id2003_aug", "Pd"] {
        println!("graph: {}", graph);
        let mut mode_results = vec![];
        for mode in scheduler_mode {
            for batch in [16] {
                println!("mode: {}-{}", mode, batch);
                let mut store_results = vec![];
                for store in store_sizes {
                    print!("{} ", store);
                    let mut inter_results = vec![];
                    for inter in [16] {
                        let result_file = format!(
                            "results/full_time_Chunk_{mode}_{store}_{inter}_{batch}_{graph}.json",
                        );
                        let time: f64 = serde_json::from_reader(
                            std::fs::File::open(&result_file)
                                .expect(format!("{} not found", &result_file).as_str()),
                        )
                        .expect("failed to parse json");
                        inter_results.push(time);
                        print!("{} ", time);
                    }
                    store_results.push(inter_results);
                    println!("");
                }
                println!("");
                mode_results.push(store_results);
            }
        }
        results.push(mode_results);
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
