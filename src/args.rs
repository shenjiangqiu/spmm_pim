use std::path::PathBuf;

use clap::Parser;


#[derive(Parser, Debug)]
#[clap(author,version,about,long_about=None)]
pub struct Args {
    #[clap(short,long,parse(from_os_str),value_name="FILE")]
    pub config_file: Option<PathBuf>,
}
