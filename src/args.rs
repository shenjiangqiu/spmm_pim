use std::path::PathBuf;

use clap::Parser;
use clap_complete::Shell;


#[derive(Parser, Debug)]
#[clap(author,version,about,long_about=None)]
pub struct Args {
    #[clap(short,long,parse(from_os_str),value_name="FILE")]
    pub config_file: Option<PathBuf>,
    #[clap(long = "generate", short = 'g', arg_enum)]
    pub generator: Option<Shell>,
}
