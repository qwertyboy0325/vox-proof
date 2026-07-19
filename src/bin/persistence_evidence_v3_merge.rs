//! Merge macOS and Windows v3 platform evidence into one run with platform-matrix.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use vox_proof::persistence_evidence::{
    build_platform_matrix, EvidenceRunResult,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "usage: persistence_evidence_v3_merge <output_run_id> <macos_platform_dir> <windows_platform_dir>"
        );
        std::process::exit(1);
    }
    let output_run_id = &args[1];
    let macos_dir = PathBuf::from(&args[2]);
    let windows_dir = PathBuf::from(&args[3]);
    let output_root = PathBuf::from("evidence/persistence").join(output_run_id);
    fs::create_dir_all(output_root.join("platforms/macos")).expect("macos dir");
    fs::create_dir_all(output_root.join("platforms/windows")).expect("windows dir");
    copy_dir_recursive(&macos_dir, &output_root.join("platforms/macos"));
    copy_dir_recursive(&windows_dir, &output_root.join("platforms/windows"));

    let macos_results: EvidenceRunResult =
        read_json(&macos_dir.join("scenario-results.json")).expect("macos results");
    let windows_results: EvidenceRunResult =
        read_json(&windows_dir.join("scenario-results.json")).expect("windows results");

    let matrix = build_platform_matrix(
        Some(output_run_id),
        Some(output_run_id),
        &macos_results.scenario_results,
        &windows_results.scenario_results,
    );
    write_json(&output_root.join("platform-matrix.json"), &matrix);
    write_json(
        &output_root.join("scenario-results.json"),
        &serde_json::json!({
            "macos": macos_results.summary,
            "windows": windows_results.summary,
        }),
    );
    println!("Merged platform matrix written to {}", output_root.display());
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    if !src.exists() {
        return;
    }
    fs::create_dir_all(dst).expect("create dst");
    for entry in fs::read_dir(src).into_iter().flatten().flatten() {
        let ty = entry.file_type().expect("file type");
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).expect("copy file");
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn write_json(path: &Path, value: &impl serde::Serialize) {
    let json = serde_json::to_vec_pretty(value).expect("serialize");
    fs::write(path, json).expect("write");
}
