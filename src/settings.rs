use config::File;
use std::path::Path;
use std::path::PathBuf;

use config::Config;
use eyre::Context;
use eyre::Result;
use itertools::Itertools;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum RowMapping {
    Chunk,
    Interleaved,
}
#[derive(Deserialize, Debug)]
pub struct MemSettings {
    pub row_size: usize,
    pub banks: usize,
    pub chips: usize,
    pub channels: usize,
    pub row_mapping: RowMapping,

    // the merger
    pub bank_merger_size: usize,
    pub chip_merger_size: usize,
    pub channel_merger_size: usize,

    // the simd width(for bsr)
    pub simd_width: usize,
}
#[derive(Deserialize, Debug)]
pub struct Settings {
    pub mtx_files: Vec<PathBuf>,
    pub result_file: PathBuf,
    pub mem_settings: MemSettings,
}

impl Settings {
    pub fn new(config: &[impl AsRef<Path>]) -> Result<Self> {
        let names = config
            .iter()
            .map(AsRef::as_ref)
            .map(File::from)
            .collect_vec();
        let ret = Config::builder()
            .add_source(names)
            .build()
            .wrap_err("fail to build setting")?
            .try_deserialize()
            .wrap_err("fail to deserialize")?;

        Ok(ret)
    }
}
