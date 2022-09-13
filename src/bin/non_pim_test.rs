use serde::Serialize;
use spmm_pim::{
    non_pim::NonPim,
    two_matrix::{TwoMatrix, TwoMatrixWrapperForNonPim},
};
use sprs::CsMat;
use tracing::metadata::LevelFilter;

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
    let result_file = "non_pim_result.json";
    let mtx_files = [
        "mtx/Groebner_id2003_aug.mtx",
        "mtx/Franz6_id1959_aug.mtx",
        "mtx/Pd.mtx",
    ];
    #[derive(Serialize)]
    struct Result {
        file: String,
        traffic: usize,
        real_traffic: usize,
        cycle: u64,
        band: f64,
    }
    let mut results = vec![];
    for i in mtx_files {
        let csr: CsMat<i32> = sprs::io::read_matrix_market(i)?.to_csr();
        let csr_trans = csr.transpose_view().to_csr();
        let matrix = TwoMatrix::new(csr, csr_trans);
        let matrix = TwoMatrixWrapperForNonPim::new(matrix, "ddr4config.toml".to_string());
        let (traffic, real_traffic, cycle) = matrix.mem_read_cycle();
        tracing::info!(traffic, cycle);
        let time = cycle as f64 / 1.2e9;
        results.push(Result {
            file: i.to_string(),
            traffic,
            real_traffic,
            cycle,
            band: traffic as f64 / time / 1024.0 / 1024.0 / 1024.0,
        });
    }
    #[derive(Serialize)]
    struct FullResult {
        results: Vec<Result>,
    }
    let full_result = FullResult { results };
    serde_json::to_writer_pretty(std::fs::File::create(result_file)?, &full_result)?;
    // let toml_result = toml::to_string_pretty(&full_result)?;
    // std::fs::write("non_pim_result.toml", toml_result)?;
    Ok(())
}
