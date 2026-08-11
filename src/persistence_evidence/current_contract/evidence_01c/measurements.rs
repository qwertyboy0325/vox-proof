use super::super::measurement::{MeasurementFixtureScale, comparative_measurement_contract};
use super::candidate::{CurrentContractCandidate, directory_size_bytes, fixture_state_for_scale};
use super::memory::MemorySampler;
use super::types::{
    MeasurementAggregate, MeasurementSample, MetricAvailability, fixture_scale_label,
};

pub fn run_comparative_measurements(
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
                let ordered: [&CurrentContractCandidate; 2] = if index % 2 == 0 {
                    [append, sqlite]
                } else {
                    [sqlite, append]
                };
                for candidate in ordered {
                    let sample_root = work_root.join(format!(
                        "{}-{:?}-{:03}-{}",
                        operation.operation,
                        scale,
                        index,
                        candidate.candidate_id()
                    ));
                    let sample =
                        run_operation_sample(candidate, &operation.operation, *scale, &sample_root);
                    if index >= operation.warmup_count {
                        if candidate.candidate_id().contains("append") {
                            append_samples
                                .get_mut(&key)
                                .expect("append key")
                                .push(sample);
                        } else {
                            sqlite_samples
                                .get_mut(&key)
                                .expect("sqlite key")
                                .push(sample);
                        }
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

fn run_operation_sample(
    candidate: &CurrentContractCandidate,
    operation: &str,
    scale: MeasurementFixtureScale,
    root: &std::path::Path,
) -> MeasurementSample {
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).expect("sample root");
    let state = fixture_state_for_scale(scale);
    let mut memory = MemorySampler::start();
    let started = std::time::Instant::now();
    let failed = match operation {
        "create_session" => candidate.create_session(&state).is_err(),
        "open_cold" => {
            let Ok((session_id, _path)) = candidate.create_session(&state) else {
                return failed_sample(&mut memory, started);
            };
            candidate.open_read_only(&session_id).is_err()
        }
        "open_warm" => {
            let Ok((session_id, _path)) = candidate.create_session(&state) else {
                return failed_sample(&mut memory, started);
            };
            let _ = candidate.open_read_only(&session_id);
            candidate.open_read_only(&session_id).is_err()
        }
        "append_review_decision"
        | "append_manual_replacement"
        | "append_reusable_promotion"
        | "append_reusable_revocation"
        | "append_reusable_supersession" => {
            let Ok((session_id, _path)) = candidate.create_session(&state) else {
                return failed_sample(&mut memory, started);
            };
            let Ok(mut writer) = candidate.open_writable(&session_id) else {
                return failed_sample(&mut memory, started);
            };
            let next = super::candidate::updated_writer_token_state(state, "writer:measurement");
            candidate.apply_transition(&mut writer, &next).is_err()
        }
        "close" => {
            let Ok((session_id, _path)) = candidate.create_session(&state) else {
                return failed_sample(&mut memory, started);
            };
            let Ok(writer) = candidate.open_writable(&session_id) else {
                return failed_sample(&mut memory, started);
            };
            candidate.close(writer).is_err()
        }
        "reopen_and_validate" => {
            let Ok((session_id, _path)) = candidate.create_session(&state) else {
                return failed_sample(&mut memory, started);
            };
            let Ok(reader) = candidate.open_read_only(&session_id) else {
                return failed_sample(&mut memory, started);
            };
            let ok = candidate.oracle_compare(&state, &reader);
            candidate.close(reader).is_err() || !ok
        }
        "semantic_duplication" => {
            let Ok((session_id, _path)) = candidate.create_session(&state) else {
                return failed_sample(&mut memory, started);
            };
            let Ok(mut writer) = candidate.open_writable(&session_id) else {
                return failed_sample(&mut memory, started);
            };
            candidate
                .duplicate(&mut writer, "session:measurement-duplicate")
                .is_err()
        }
        "derived_rebuild" | "compaction_where_supported" => {
            let Ok((session_id, path)) = candidate.create_session(&state) else {
                return failed_sample(&mut memory, started);
            };
            let Ok(writer) = candidate.open_writable(&session_id) else {
                return failed_sample(&mut memory, started);
            };
            let _ = candidate.close(writer);
            candidate.open_writable(&session_id).is_err() && directory_size_bytes(&path) == 0
        }
        _ => true,
    };
    memory.observe();
    let elapsed_ms = started.elapsed().as_millis();
    let storage_size_after = directory_size_bytes(root);
    MeasurementSample {
        elapsed_ms,
        peak_memory_bytes: memory.peak_since_start(),
        storage_size_before: 0,
        storage_size_after,
        bytes_read: MetricAvailability::unavailable("not observed"),
        bytes_written: MetricAvailability::unavailable("not observed"),
        failed,
    }
}

fn failed_sample(memory: &mut MemorySampler, started: std::time::Instant) -> MeasurementSample {
    memory.observe();
    MeasurementSample {
        elapsed_ms: started.elapsed().as_millis(),
        peak_memory_bytes: memory.peak_since_start(),
        storage_size_before: 0,
        storage_size_after: 0,
        bytes_read: MetricAvailability::unavailable("not observed"),
        bytes_written: MetricAvailability::unavailable("not observed"),
        failed: true,
    }
}

fn aggregate(
    candidate: &CurrentContractCandidate,
    operation: &str,
    scale: MeasurementFixtureScale,
    platform: &str,
    samples: Vec<MeasurementSample>,
) -> MeasurementAggregate {
    let mut elapsed: Vec<u128> = samples.iter().map(|sample| sample.elapsed_ms).collect();
    elapsed.sort_unstable();
    let count = samples.len() as u32;
    let failure_count = samples.iter().filter(|sample| sample.failed).count() as u32;
    let peak = samples
        .iter()
        .map(|sample| sample.peak_memory_bytes)
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
        peak_memory_bytes: MetricAvailability::available(peak),
        bytes_read: MetricAvailability::unavailable("not observed"),
        bytes_written: MetricAvailability::unavailable("not observed"),
        storage_size_before: MetricAvailability::available(
            samples.first().map(|s| s.storage_size_before).unwrap_or(0),
        ),
        storage_size_after: MetricAvailability::available(
            samples.last().map(|s| s.storage_size_after).unwrap_or(0),
        ),
    }
}

pub fn scale_label(scale: MeasurementFixtureScale) -> String {
    fixture_scale_label(scale)
}
