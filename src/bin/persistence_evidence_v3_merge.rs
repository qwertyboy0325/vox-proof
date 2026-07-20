//! Merge macOS and Windows v3 platform evidence into one run with platform-matrix.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use vox_proof::persistence_evidence::{EvidenceRunResult, build_platform_matrix};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "usage: persistence_evidence_v3_merge <output_run_id> <macos_platform_dir> <windows_platform_dir> [windows_run_id]"
        );
        std::process::exit(1);
    }
    let output_run_id = &args[1];
    let macos_dir = PathBuf::from(&args[2]);
    let windows_dir = PathBuf::from(&args[3]);
    let windows_run_id = args
        .get(4)
        .cloned()
        .unwrap_or_else(|| output_run_id.clone());
    let output_root = PathBuf::from("evidence/persistence").join(output_run_id);
    let windows_platform_label = windows_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "windows".to_string());
    let windows_dest = output_root.join("platforms").join(&windows_platform_label);
    fs::create_dir_all(&windows_dest).expect("windows dir");
    copy_dir_recursive(&windows_dir, &windows_dest);

    let macos_results: EvidenceRunResult =
        read_json(&macos_dir.join("scenario-results.json")).expect("macos results");
    let windows_results: EvidenceRunResult =
        read_json(&windows_dir.join("scenario-results.json")).expect("windows results");

    let mut matrix = build_platform_matrix(
        Some(output_run_id),
        Some(windows_run_id.as_str()),
        &macos_results.scenario_results,
        &windows_results.scenario_results,
    );
    matrix.macos_run_id = Some(output_run_id.clone());
    matrix.windows_run_id = Some(windows_run_id.clone());
    write_json(&output_root.join("platform-matrix.json"), &matrix);
    write_json(
        &output_root.join("scenario-results.json"),
        &serde_json::json!({
            "macos": macos_results.summary,
            "windows": windows_results.summary,
        }),
    );

    let merge_cmd = format!(
        "cargo run --features persistence-spike --bin persistence_evidence_v3_merge -- {} {} {} {}",
        output_run_id,
        macos_dir.display(),
        windows_dir.display(),
        windows_run_id,
    );
    let commands_path = output_root.join("commands.json");
    let mut commands: Vec<String> = if commands_path.exists() {
        read_json(&commands_path).unwrap_or_default()
    } else {
        Vec::new()
    };
    if !commands
        .iter()
        .any(|c| c.contains("persistence_evidence_v3_merge"))
    {
        commands.push(merge_cmd);
    }
    write_json(&commands_path, &commands);
    write_json(
        &output_root.join("checksums.json"),
        &checksum_dir(&output_root),
    );

    println!(
        "Merged platform matrix written to {}",
        output_root.display()
    );
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

fn checksum_dir(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    fn walk(base: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
        let Ok(entries) = fs::read_dir(base) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if path
                    .file_name()
                    .is_some_and(|name| name == "checksums.json")
                {
                    continue;
                }
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                let data = fs::read(&path).expect("read for checksum");
                let hash = Sha256::digest(&data);
                out.insert(rel, format!("sha256:{hash:x}"));
            } else if path.is_dir() {
                walk(&path, root, out);
            }
        }
    }
    walk(root, root, &mut out);
    out
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn write_json(path: &Path, value: &impl serde::Serialize) {
    let json = serde_json::to_vec_pretty(value).expect("serialize");
    fs::write(path, json).expect("write");
}
