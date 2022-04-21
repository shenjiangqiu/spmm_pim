use std::path::PathBuf;

use clap::Parser;
use clap_complete::Shell;


#[derive(Parser, Debug)]
#[clap(author="Jiangqiu Shen",version,about="a spmm pim simulator",long_about=None)]
pub struct Args {
    /// the path of config file, default is "default.toml"
    #[clap(short,long,parse(from_os_str),value_name="FILE")]
    pub config_file: Option<PathBuf>,

    /// Generate completion for the given shell
    #[clap(long = "generate", short = 'g', arg_enum)]
    pub generator: Option<Shell>,
}
