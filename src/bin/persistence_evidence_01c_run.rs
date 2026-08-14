//! Gate 4 package 01C equivalent candidate evidence runner.

use std::path::PathBuf;

use vox_proof::persistence_evidence::current_contract::evidence_01c::measurement_worker::{
    child_hold_writer_main, child_interrupt_transition_main, measure_sample_main,
};
use vox_proof::persistence_evidence::run_01c_evidence;

fn main() {
    if let Some(command) = std::env::args().nth(1) {
        let result = match command.as_str() {
            "measure-sample" => measure_sample_main(),
            "child-hold-writer" => child_hold_writer_main(),
            "child-interrupt-transition" => child_interrupt_transition_main(),
            other => Err(format!("unknown 01C worker command {other}")),
        };
        if let Err(error) = result {
            eprintln!("01C worker failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    let run_id = std::env::var("VOXPROOF_01C_RUN_ID")
        .unwrap_or_else(|_| format!("current-contract-v3-01c-{}", timestamp_slug()));
    let output_root = PathBuf::from("evidence/persistence/current-contract-v3/01c").join(&run_id);
    let output = run_01c_evidence(&output_root);
    println!(
        "01C evidence written to {} (readiness: {})",
        output.output_root.display(),
        output.package.mechanism_comparison_readiness
    );
}

fn timestamp_slug() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_secs()
        .to_string()
}
