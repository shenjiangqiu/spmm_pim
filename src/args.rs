use std::path::PathBuf;

use clap::{Parser, ValueHint};
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[clap(author="Jiangqiu Shen",version,about="a spmm pim simulator",long_about=None,trailing_var_arg=true)]
pub struct Args {
    /// Generate completion for the given shell
    #[clap(long = "generate", short = 'g', arg_enum)]
    pub generator: Option<Shell>,
    #[clap(long = "run-mode", short = 'r', arg_enum)]
    pub run_mode: RunMode,
    /// the path of config file, default is "default.toml"
    #[clap(parse(from_os_str),value_hint=ValueHint::FilePath)]
    pub config_file: Vec<PathBuf>,
}
#[derive(Debug, Clone, clap::ArgEnum)]
pub enum RunMode {
    Sim,
    Pim,
}
