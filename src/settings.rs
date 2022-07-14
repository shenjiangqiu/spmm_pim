use config::File;
use std::path::Path;
use std::path::PathBuf;

use config::Config;
use eyre::Context;
use eyre::Result;
use itertools::Itertools;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum RowMapping {
    Chunk,
    Interleaved,
}
impl Default for RowMapping {
    fn default() -> Self {
        RowMapping::Chunk
    }
}
#[derive(Deserialize, Debug, Default)]
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

    /// the size of one merger!
    pub dimm_merger_size: usize,

    // the merger
    pub bank_merger_count: usize,
    pub chip_merger_count: usize,
    pub channel_merger_count: usize,

    /// the number of mergers
    pub dimm_merger_count: usize,

    // the simd width(for bsr)
    pub simd_width: usize,

    // the reorder engine
    pub parallel_count: usize,
    pub reorder_count: usize,

    // mem read latency
    pub row_change_latency: usize,

    // add size
    pub bank_adder_size: usize,

    // the store buffer size
    pub store_size: usize,
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

impl MemSettings {
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
