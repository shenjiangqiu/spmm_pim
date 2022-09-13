use itertools::Itertools;

use spmm_pim::{pim::Pim, settings::Settings, two_matrix::TwoMatrix};
use sprs::CsMat;
use tracing::metadata::LevelFilter;
mod types;
use types::{FullResult, Result};

fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .try_init()
        .unwrap_or_else(|e| {
            eprintln!("failed to init logger: {}", e);
        });
    let result_file = "with_pim_result.json";
    let mtx_files = [
        "mtx/Groebner_id2003_aug.mtx",
        "mtx/Franz6_id1959_aug.mtx",
        "mtx/Pd.mtx",
    ];
    let settings = Settings::new(&[
        "configs/default.toml",
        "configs/ddr4.toml",
        "configs/large.toml",
    ])?;
    let mem_settings = settings.mem_settings;

    let mut results = vec![];
    for i in mtx_files {
        let csr: CsMat<i32> = sprs::io::read_matrix_market(i)?.to_csr();
        let csr_trans = csr.transpose_view().to_csr();
        let two_matrix = TwoMatrix::new(csr, csr_trans);

        let mem_rows = two_matrix.mem_rows(&mem_settings);
        let bank_reads = mem_rows.iter().cloned().map(|(_, b)| b).collect();
        let mem_rows = mem_rows.iter().cloned().map(|(a, _)| a).collect_vec();
        tracing::info!(?mem_rows);
        let (bank_cycles, bank_partial_sum) = two_matrix.bank_merge(&mem_settings);
        tracing::info!(?bank_cycles);
        let (bank_sent, chip_recv) = two_matrix.chip_fetch_data(&mem_settings, &bank_partial_sum);
        tracing::info!(?bank_sent, ?chip_recv);
        let (chip_cycles, chip_partial_sum) =
            two_matrix.chip_merge(&mem_settings, &bank_partial_sum);
        tracing::info!(?chip_cycles);
        let (chip_sent, channel_recv) =
            two_matrix.channel_fetch_data(&mem_settings, &chip_partial_sum);
        tracing::info!(?chip_sent, ?channel_recv);
        let (channel_cycles, channel_partial_sum) =
            two_matrix.channel_merge(&mem_settings, &chip_partial_sum);
        tracing::info!(?channel_cycles);
        let (channel_sent, dimm_recv) =
            two_matrix.dimm_fetch_data(&mem_settings, &channel_partial_sum);
        tracing::info!(?channel_sent, ?dimm_recv);
        let (dimm_cycles, dimm_result) = two_matrix.dimm_merge(&mem_settings, &channel_partial_sum);
        tracing::info!(?dimm_cycles);
        let final_write = two_matrix.write_result(&mem_settings, &dimm_result);
        tracing::info!(?final_write);
        results.push(Result {
            file: i.to_string(),
            mem_rows,
            bank_reads,
            bank_cycles,
            bank_sent,
            chip_recv,
            chip_cycles,
            chip_sent,
            channel_recv,
            channel_cycles,
            channel_sent,
            dimm_recv,
            dimm_cycles,
            final_write,
        });
    }

    let full_result = FullResult { results };
    serde_json::to_writer_pretty(std::fs::File::create(result_file)?, &full_result)?;
    // let toml_result = toml::to_string_pretty(&full_result)?;
    // std::fs::write("non_pim_result.toml", toml_result)?;
    Ok(())
}
