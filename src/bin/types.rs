use serde::{Deserialize, Serialize};
use spmm_pim::pim::MergeCycle;

#[derive(Serialize, Deserialize, Debug)]
pub struct Result {
    pub file: String,
    pub mem_rows: Vec<usize>,
    pub bank_reads: Vec<usize>,
    pub bank_cycles: Vec<MergeCycle>,
    pub bank_sent: Vec<usize>,
    pub chip_recv: Vec<usize>,
    pub chip_cycles: Vec<MergeCycle>,
    pub chip_sent: Vec<usize>,
    pub channel_recv: Vec<usize>,
    pub channel_cycles: Vec<MergeCycle>,
    pub channel_sent: Vec<usize>,
    pub dimm_recv: usize,
    pub dimm_cycles: MergeCycle,
    pub final_write: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FullResult {
    pub results: Vec<Result>,
}
#[allow(unused)]
fn main() {}
