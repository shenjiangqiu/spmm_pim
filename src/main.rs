use clap::Parser;
use env_logger::Env;
use log::debug;
use spmm_pim::run::run_exp;
use spmm_pim::{args::Args, result::Results, run_unroll, settings::Settings};
use std::io::Write;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn"))
        .try_init()
        .unwrap_or_default();

    let args = Args::parse();
    println!("{:?}", args);

    let settings = Settings::new(args.config_file.unwrap_or("default.toml".into()));
    debug!("{:?}", settings);
    let mtxs = settings.mtx_files().clone();

    let mut full_result = Results { all: vec![] };
    // load config into ConfigFile
    for i in mtxs.iter() {
        run_unroll!(i;full_result;run_exp; 64,128,256,512,1024,2048);
    }
    let res = serde_json::to_string_pretty(&full_result).unwrap();
    // write to file
    let file_name = "result.json";
    let mut file = std::fs::File::create(file_name).unwrap();
    file.write_all(res.as_bytes()).unwrap();
}
