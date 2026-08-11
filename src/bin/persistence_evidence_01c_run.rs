//! Gate 4 package 01C equivalent candidate evidence runner.

use std::path::PathBuf;

use vox_proof::persistence_evidence::run_01c_evidence;

fn main() {
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
