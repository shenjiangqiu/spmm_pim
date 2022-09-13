use itertools::Itertools;
use serde::Serialize;
use spmm_pim::pim::MergeCycle;
mod types;
use types::FullResult;

fn average_max(merge_cycle: &[MergeCycle]) -> AverageMax {
    let add_max = merge_cycle.iter().map(|x| x.add_cycle).max().unwrap_or(0);
    let merge_max = merge_cycle.iter().map(|x| x.merge_cycle).max().unwrap_or(0);
    let add_sum = merge_cycle.iter().map(|x| x.add_cycle).sum::<usize>();
    let merge_sum = merge_cycle.iter().map(|x| x.merge_cycle).sum::<usize>();
    let add_average = add_sum / merge_cycle.len();
    let merge_average = merge_sum / merge_cycle.len();
    AverageMax {
        add_average,
        merge_average,
        add_total: add_sum,
        add_max,
        merge_max,
        merge_total: merge_sum,
    }
}
fn average_max_single(cycles: &[usize]) -> AverageMaxSingle {
    let max = cycles.iter().max().unwrap_or(&0);
    let sum = cycles.iter().sum::<usize>();
    let average = sum / cycles.len();
    AverageMaxSingle {
        average,
        max: *max,
        total: sum,
    }
}
#[derive(Debug, Serialize)]
pub struct AverageMax {
    add_average: usize,
    add_max: usize,
    add_total: usize,
    merge_average: usize,
    merge_max: usize,
    merge_total: usize,
}

#[derive(Debug, Serialize)]
pub struct AverageMaxSingle {
    average: usize,
    max: usize,
    total: usize,
}

#[derive(Debug, Serialize)]
pub struct ResultMaxAverage {
    pub file: String,
    pub mem_rows: AverageMaxSingle,
    pub mem_read: AverageMaxSingle,
    /// ((add_max,merge_max),(add_average,merge_average))
    pub bank_cycles: AverageMax,
    pub bank_sent: AverageMaxSingle,
    pub chip_recv: AverageMaxSingle,
    pub chip_cycles: AverageMax,
    pub chip_sent: AverageMaxSingle,
    pub channel_recv: AverageMaxSingle,
    pub channel_cycles: AverageMax,
    pub channel_sent: AverageMaxSingle,
    pub dimm_recv: usize,
    pub dimm_cycles: MergeCycle,
    pub final_write: usize,
}

fn main() {
    let result_file = "with_pim_result.json";
    let result: FullResult =
        serde_json::from_reader(std::fs::File::open(result_file).unwrap()).unwrap();
    let maped_result = result
        .results
        .into_iter()
        .map(|result| ResultMaxAverage {
            file: result.file,
            mem_rows: average_max_single(&result.mem_rows),
            mem_read: average_max_single(&result.bank_reads),
            bank_cycles: average_max(&result.bank_cycles),
            bank_sent: average_max_single(&result.bank_sent),
            chip_recv: average_max_single(&result.chip_recv),
            chip_cycles: average_max(&result.chip_cycles),
            chip_sent: average_max_single(&result.chip_sent),
            channel_recv: average_max_single(&result.channel_recv),
            channel_cycles: average_max(&result.channel_cycles),
            channel_sent: average_max_single(&result.channel_sent),
            dimm_recv: result.dimm_recv,
            dimm_cycles: result.dimm_cycles,
            final_write: result.final_write,
        })
        .collect_vec();
    println!("{:?}", maped_result);
    serde_json::to_writer_pretty(
        std::fs::File::create("with_pim_result_max_average.json").unwrap(),
        &maped_result,
    )
    .unwrap();
}
