use config::File;
use enum_as_inner::EnumAsInner;
use std::path::Path;
use std::path::PathBuf;

use config::Config;
use eyre::Context;
use eyre::Result;
use itertools::Itertools;
use serde::Deserialize;

/// the toml file do not support enum with value
pub enum RealRowMapping {
    Chunk,
    Interleaved(usize),
}

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
impl RowMapping {
    pub fn to_real_row_mapping(&self, size: usize) -> RealRowMapping {
        match self {
            RowMapping::Chunk => RealRowMapping::Chunk,
            RowMapping::Interleaved => RealRowMapping::Interleaved(size),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default, EnumAsInner)]
pub enum BufferMode {
    #[default]
    BindMerger,
    Standalone,
}
#[derive(Debug, Deserialize, Clone, Default, EnumAsInner)]
pub enum TaskSchedulerMode {
    #[default]
    Sequence,
    Shuffle,
    ChunkShuffle,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MemSettings {
    pub buffer_mode: BufferMode,

    pub row_size: usize,
    pub banks: usize,
    pub chips: usize,
    pub channels: usize,
    pub row_mapping: RowMapping,
    pub interleaved_chunk: usize,

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
    pub sender_store_size: usize,

    // buffer lines
    pub dimm_buffer_lines: usize,
    pub channel_buffer_lines: usize,
    pub chip_buffer_lines: usize,
    pub task_scheduler_mode: TaskSchedulerMode,
    pub task_scheduler_chunk_size: usize,
}

impl Default for MemSettings {
    fn default() -> Self {
        Self {
            buffer_mode: Default::default(),
            row_size: 4,
            banks: 2,
            chips: 2,
            channels: 2,
            row_mapping: RowMapping::Interleaved,
            interleaved_chunk: 1,
            bank_merger_size: 4,
            chip_merger_size: 4,
            channel_merger_size: 4,
            dimm_merger_size: 4,
            bank_merger_count: 2,
            chip_merger_count: 2,
            channel_merger_count: 2,
            dimm_merger_count: 2,
            simd_width: 2,
            parallel_count: Default::default(),
            reorder_count: Default::default(),
            row_change_latency: 2,
            bank_adder_size: 2,
            sender_store_size: 2,
            dimm_buffer_lines: 2,
            channel_buffer_lines: 2,
            chip_buffer_lines: 2,
            task_scheduler_mode: Default::default(),
            task_scheduler_chunk_size: Default::default(),
        }
    }
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
