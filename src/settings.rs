use std::path::PathBuf;

use config::Config;
use eyre::eyre;
use eyre::Context;
use eyre::Result;
use serde::Deserialize;
#[derive(Deserialize, Debug)]
pub struct Settings {
    pub mtx_files: Vec<PathBuf>,
    pub result_file: PathBuf,
}

impl Settings {
    pub fn new(config: PathBuf) -> Result<Self> {
        let name = config.to_str().ok_or(eyre!("Invalid path"))?;
        let settings = Config::builder()
            .add_source(config::File::with_name(name))
            .build()
            .wrap_err("cannot build Setting object")?;
        let ret = settings
            .try_deserialize()
            .wrap_err("failed to deserialize")?;
        Ok(ret)
    }
}
