//! Gate 4 package 01C equivalent candidate evidence runner.

use std::path::PathBuf;

use vox_proof::persistence_evidence::current_contract::evidence_01c::measurement_worker::{
    child_hold_writer_main, child_interrupt_transition_main, measure_sample_main,
};
use vox_proof::persistence_evidence::run_01c_evidence;
use vox_proof::persistence_evidence::current_contract::evidence_01c::{
    run_01c_evidence_dual_scoped, AppendEvidenceVariant, SqliteEvidenceVariant,
};

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
    let output = if std::env::var("VOXPROOF_01C_DUAL_SCOPED")
        .ok()
        .as_deref()
        .is_some_and(|value| matches!(value, "1" | "true" | "yes"))
    {
        run_01c_evidence_dual_scoped(&output_root)
    } else {
        run_01c_evidence(&output_root)
    };
    let append_label = match output.append_variant {
        AppendEvidenceVariant::Historical01B2 => "01B-2",
        AppendEvidenceVariant::Scoped01B3 => "01B-3",
    };
    let sqlite_label = match output.sqlite_variant {
        SqliteEvidenceVariant::Historical01CSqlite2 => "01C-SQLITE-2",
        SqliteEvidenceVariant::Scoped01CSqlite3 => "01C-SQLITE-3",
    };
    println!(
        "01C evidence written to {} (append: {}, sqlite: {}, readiness: {})",
        output.output_root.display(),
        append_label,
        sqlite_label,
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
