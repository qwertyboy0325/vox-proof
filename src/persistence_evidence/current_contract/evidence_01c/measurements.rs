use super::super::measurement::{comparative_measurement_contract, MeasurementFixtureScale};
use super::candidate::{CurrentContractCandidate, CurrentContractCandidateKind};
use super::measurement_ops::peak_metric;
use super::measurement_worker::run_sample_in_worker;
use super::types::{
    fixture_scale_label, MeasurementAggregate, MeasurementSample, MetricAvailability,
};

pub fn run_comparative_measurements(
    append: &CurrentContractCandidate,
    sqlite: &CurrentContractCandidate,
    platform: &str,
    work_root: &std::path::Path,
) -> (Vec<MeasurementAggregate>, Vec<MeasurementAggregate>) {
    run_comparative_measurements_with_kinds(
        append.kind(),
        sqlite.kind(),
        append,
        sqlite,
        platform,
        work_root,
    )
}

pub fn run_comparative_measurements_with_kinds(
    append_kind: CurrentContractCandidateKind,
    sqlite_kind: CurrentContractCandidateKind,
    append: &CurrentContractCandidate,
    sqlite: &CurrentContractCandidate,
    platform: &str,
    work_root: &std::path::Path,
) -> (Vec<MeasurementAggregate>, Vec<MeasurementAggregate>) {
    let contract = comparative_measurement_contract();
    let mut append_samples: std::collections::BTreeMap<
        (String, MeasurementFixtureScale),
        Vec<MeasurementSample>,
    > = std::collections::BTreeMap::new();
    let mut sqlite_samples: std::collections::BTreeMap<
        (String, MeasurementFixtureScale),
        Vec<MeasurementSample>,
    > = std::collections::BTreeMap::new();

    for operation in &contract.operations {
        for scale in &operation.fixture_scales {
            let key = (operation.operation.clone(), *scale);
            append_samples.insert(key.clone(), Vec::new());
            sqlite_samples.insert(key.clone(), Vec::new());
            let total = operation.sample_count + operation.warmup_count;
            for index in 0..total {
                let ordered = if index % 2 == 0 {
                    [(append_kind, "append"), (sqlite_kind, "sqlite")]
                } else {
                    [(sqlite_kind, "sqlite"), (append_kind, "append")]
                };
                for (kind, label) in ordered {
                    let sample_root = work_root.join(format!(
                        "{}/{}/{:03}/{label}",
                        operation.operation,
                        fixture_scale_label(*scale),
                        index
                    ));
                    let _ = std::fs::remove_dir_all(&sample_root);
                    std::fs::create_dir_all(&sample_root).expect("sample root");
                    let sample = run_sample_in_worker(
                        &sample_root,
                        kind,
                        &operation.operation,
                        *scale,
                        index,
                    );
                    if index >= operation.warmup_count {
                        let bucket = if label == "append" {
                            append_samples.get_mut(&key).expect("append key")
                        } else {
                            sqlite_samples.get_mut(&key).expect("sqlite key")
                        };
                        bucket.push(sample);
                    }
                }
            }
        }
    }

    let append_results = append_samples
        .into_iter()
        .map(|((operation, scale), samples)| {
            aggregate(append, &operation, scale, platform, samples)
        })
        .collect();
    let sqlite_results = sqlite_samples
        .into_iter()
        .map(|((operation, scale), samples)| {
            aggregate(sqlite, &operation, scale, platform, samples)
        })
        .collect();
    (append_results, sqlite_results)
}

fn aggregate(
    candidate: &CurrentContractCandidate,
    operation: &str,
    scale: MeasurementFixtureScale,
    platform: &str,
    samples: Vec<MeasurementSample>,
) -> MeasurementAggregate {
    if operation == "compaction_where_supported" && !candidate.compaction_supported() {
        return unsupported_aggregate(candidate, operation, scale, platform);
    }

    let mut elapsed: Vec<u128> = samples
        .iter()
        .filter(|sample| !sample.failed)
        .map(|sample| sample.elapsed_ms)
        .collect();
    elapsed.sort_unstable();
    let count = samples.len() as u32;
    let failure_count = samples.iter().filter(|sample| sample.failed).count() as u32;
    let peak = samples
        .iter()
        .filter(|sample| !sample.failed)
        .map(|sample| sample.peak_memory_bytes)
        .max()
        .unwrap_or(0);
    let storage_before = super::measurement_ops::storage_before_metric(&samples);
    let storage_after = samples
        .iter()
        .filter(|sample| !sample.failed)
        .map(|sample| sample.storage_size_after)
        .max()
        .unwrap_or(0);

    MeasurementAggregate {
        operation: operation.to_owned(),
        fixture_scale: scale,
        candidate_id: candidate.candidate_id().to_owned(),
        candidate_version: candidate.candidate_version().to_owned(),
        platform: platform.to_owned(),
        count,
        minimum_ms: *elapsed.first().unwrap_or(&0),
        median_ms: elapsed.get(elapsed.len() / 2).copied().unwrap_or(0),
        p95_ms: elapsed
            .get(
                ((elapsed.len().saturating_sub(1) as f64 * 0.95) as usize)
                    .min(elapsed.len().saturating_sub(1)),
            )
            .copied()
            .unwrap_or(0),
        maximum_ms: *elapsed.last().unwrap_or(&0),
        failure_count,
        peak_memory_bytes: peak_metric(peak),
        bytes_read: MetricAvailability::unavailable("not observed"),
        bytes_written: MetricAvailability::unavailable("not observed"),
        storage_size_before: storage_before,
        storage_size_after: super::measurement_ops::storage_metric(storage_after),
    }
}

fn unsupported_aggregate(
    candidate: &CurrentContractCandidate,
    operation: &str,
    scale: MeasurementFixtureScale,
    platform: &str,
) -> MeasurementAggregate {
    MeasurementAggregate {
        operation: operation.to_owned(),
        fixture_scale: scale,
        candidate_id: candidate.candidate_id().to_owned(),
        candidate_version: candidate.candidate_version().to_owned(),
        platform: platform.to_owned(),
        count: 0,
        minimum_ms: 0,
        median_ms: 0,
        p95_ms: 0,
        maximum_ms: 0,
        failure_count: 0,
        peak_memory_bytes: MetricAvailability::unavailable("compaction unsupported"),
        bytes_read: MetricAvailability::unavailable("not observed"),
        bytes_written: MetricAvailability::unavailable("not observed"),
        storage_size_before: MetricAvailability::unavailable("compaction unsupported"),
        storage_size_after: MetricAvailability::unavailable("compaction unsupported"),
    }
}
pub fn scale_label(scale: MeasurementFixtureScale) -> String {
    fixture_scale_label(scale)
}
