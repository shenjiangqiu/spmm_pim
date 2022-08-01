use std::env::args_os;

use clap::Parser;
use eyre::Result;
use spmm_pim::{args::Args, run_main};
fn main() -> Result<()> {
    let args = args_os();
    let args = Args::parse_from(args);
    run_main::main(args)
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
            "sim",
            "configs/large.toml",
            "configs/ddr4.toml",
            "configs/store_sizes/4.toml",
        ];
        let args = Args::parse_from(args);
        println!("hello world!");
        super::run_main::main(args).unwrap();
    }
}
