use std::path::Path;
use std::time::Instant;

use super::super::measurement::MeasurementFixtureScale;
use super::candidate::{
    directory_size_bytes, fixture_state_for_scale, measurement_transition_states,
    unique_session_id_for_sample, CurrentContractCandidate, CurrentContractCandidateKind,
};
use super::types::{MeasurementSample, MetricAvailability};

pub const MEASURE_RESULT_PREFIX: &str = "VOXPROOF_MEASURE:";

pub fn execute_measured_operation(
    candidate: &CurrentContractCandidate,
    operation: &str,
    scale: MeasurementFixtureScale,
    sample_index: u32,
) -> MeasurementSample {
    let state = fixture_state_for_scale(scale);
    let storage_scope = candidate.storage_root();
    let started = Instant::now();
    let failed = match operation {
        "create_session" => {
            let isolated = unique_session_id_for_sample(&state, sample_index);
            candidate.create_session(&isolated).is_err()
        }
        "open_cold" => {
            let isolated = unique_session_id_for_sample(&state, sample_index);
            let Ok((session_id, _)) = candidate.create_session(&isolated) else {
                return failed_sample(started, storage_scope);
            };
            let timer = Instant::now();
            let failed = candidate.open_read_only(&session_id).is_err();
            return finalize_sample(timer, failed, storage_scope);
        }
        "open_warm" => {
            let isolated = unique_session_id_for_sample(&state, sample_index);
            let Ok((session_id, _)) = candidate.create_session(&isolated) else {
                return failed_sample(started, storage_scope);
            };
            let _ = candidate.open_read_only(&session_id);
            let timer = Instant::now();
            let reader = candidate.open_read_only(&session_id);
            let failed = match reader {
                Ok(handle) => candidate.close(handle).is_err(),
                Err(_) => true,
            };
            return finalize_sample(timer, failed, storage_scope);
        }
        "append_review_decision"
        | "append_manual_replacement"
        | "append_reusable_promotion"
        | "append_reusable_revocation"
        | "append_reusable_supersession" => {
            let Some((precursor, target)) = measurement_transition_states(operation, scale) else {
                return failed_sample(started, storage_scope);
            };
            let precursor = unique_session_id_for_sample(&precursor, sample_index);
            let target = unique_session_id_for_sample(&target, sample_index);
            let Ok((session_id, _)) = candidate.create_session(&precursor) else {
                return failed_sample(started, storage_scope);
            };
            let Ok(mut writer) = candidate.open_writable(&session_id) else {
                return failed_sample(started, storage_scope);
            };
            let timer = Instant::now();
            let failed = candidate.apply_transition(&mut writer, &target).is_err();
            let _ = candidate.close(writer);
            return finalize_sample(timer, failed, storage_scope);
        }
        "close" => {
            let isolated = unique_session_id_for_sample(&state, sample_index);
            let Ok((session_id, _)) = candidate.create_session(&isolated) else {
                return failed_sample(started, storage_scope);
            };
            let Ok(writer) = candidate.open_writable(&session_id) else {
                return failed_sample(started, storage_scope);
            };
            let timer = Instant::now();
            let failed = candidate.close(writer).is_err();
            return finalize_sample(timer, failed, storage_scope);
        }
        "reopen_and_validate" => {
            let isolated = unique_session_id_for_sample(&state, sample_index);
            let Ok((session_id, _)) = candidate.create_session(&isolated) else {
                return failed_sample(started, storage_scope);
            };
            let Ok(writer) = candidate.open_writable(&session_id) else {
                return failed_sample(started, storage_scope);
            };
            let _ = candidate.close(writer);
            let timer = Instant::now();
            let reader = candidate.open_read_only(&session_id);
            let failed = match reader {
                Ok(handle) => {
                    let ok = candidate.oracle_compare(&isolated, &handle);
                    candidate.close(handle).is_err() || !ok
                }
                Err(_) => true,
            };
            return finalize_sample(timer, failed, storage_scope);
        }
        "semantic_duplication" => {
            let isolated = unique_session_id_for_sample(&state, sample_index);
            let Ok((session_id, _)) = candidate.create_session(&isolated) else {
                return failed_sample(started, storage_scope);
            };
            let Ok(mut writer) = candidate.open_writable(&session_id) else {
                return failed_sample(started, storage_scope);
            };
            let duplicate_id = format!("{}:duplicate", session_id);
            let timer = Instant::now();
            let failed = candidate.duplicate(&mut writer, &duplicate_id).is_err();
            let _ = candidate.close(writer);
            return finalize_sample(timer, failed, storage_scope);
        }
        "derived_rebuild" => {
            return measure_derived_rebuild(candidate, &state, sample_index, storage_scope);
        }
        "compaction_where_supported" => {
            if !candidate.compaction_supported() {
                return unsupported_sample(started, storage_scope);
            }
            return failed_sample(started, storage_scope);
        }
        _ => true,
    };
    finalize_sample(started, failed, storage_scope)
}

fn measure_derived_rebuild(
    candidate: &CurrentContractCandidate,
    state: &super::super::model::CurrentContractState,
    sample_index: u32,
    storage_scope: &Path,
) -> MeasurementSample {
    let isolated = unique_session_id_for_sample(state, sample_index);
    let Ok((session_id, _)) = candidate.create_session(&isolated) else {
        return failed_sample(Instant::now(), storage_scope);
    };
    if matches!(candidate.kind(), CurrentContractCandidateKind::Sqlite) {
        let Ok(mut writer) = candidate.open_writable(&session_id) else {
            return failed_sample(Instant::now(), storage_scope);
        };
        if candidate
            .tamper_derived_cache_for_test(&mut writer, "not-a-derived-projection")
            .is_err()
        {
            let _ = candidate.close(writer);
            return failed_sample(Instant::now(), storage_scope);
        }
        let _ = candidate.close(writer);
        let timer = Instant::now();
        let reopened = candidate.open_writable(&session_id);
        let failed = match reopened {
            Ok(handle) => {
                let ok = candidate.oracle_compare(&isolated, &handle);
                candidate.close(handle).is_err() || !ok
            }
            Err(_) => true,
        };
        return finalize_sample(timer, failed, storage_scope);
    }
    let Ok(mut writer) = candidate.open_writable(&session_id) else {
        return failed_sample(Instant::now(), storage_scope);
    };
    if candidate
        .tamper_derived_projection_for_test(&mut writer)
        .is_err()
    {
        let _ = candidate.close(writer);
        return failed_sample(Instant::now(), storage_scope);
    }
    let _ = candidate.close(writer);
    let timer = Instant::now();
    let reopened = candidate.open_read_only(&session_id);
    let failed = match reopened {
        Ok(handle) => {
            let ok = candidate.oracle_compare(&isolated, &handle);
            candidate.close(handle).is_err() || !ok
        }
        Err(_) => true,
    };
    finalize_sample(timer, failed, storage_scope)
}

fn finalize_sample(timer: Instant, failed: bool, storage_scope: &Path) -> MeasurementSample {
    let peak = super::memory::current_peak_bytes();
    MeasurementSample {
        elapsed_ms: timer.elapsed().as_nanos() / 1_000_000,
        peak_memory_bytes: peak,
        storage_size_before: None,
        storage_size_after: directory_size_bytes(storage_scope),
        bytes_read: MetricAvailability::unavailable("not observed"),
        bytes_written: MetricAvailability::unavailable("not observed"),
        failed,
    }
}

fn failed_sample(timer: Instant, storage_scope: &Path) -> MeasurementSample {
    finalize_sample(timer, true, storage_scope)
}

fn unsupported_sample(timer: Instant, storage_scope: &Path) -> MeasurementSample {
    MeasurementSample {
        elapsed_ms: timer.elapsed().as_nanos() / 1_000_000,
        peak_memory_bytes: 0,
        storage_size_before: None,
        storage_size_after: directory_size_bytes(storage_scope),
        bytes_read: MetricAvailability::unavailable("not observed"),
        bytes_written: MetricAvailability::unavailable("not observed"),
        failed: false,
    }
}

pub fn peak_metric(bytes: u64) -> MetricAvailability<u64> {
    if bytes > 0 {
        MetricAvailability::available(bytes)
    } else {
        MetricAvailability::unavailable("peak rss not observed")
    }
}

pub fn storage_metric(bytes: u64) -> MetricAvailability<u64> {
    if bytes > 0 {
        MetricAvailability::available(bytes)
    } else {
        MetricAvailability::unavailable("zero-byte storage sentinel rejected")
    }
}

pub fn storage_before_metric(samples: &[MeasurementSample]) -> MetricAvailability<u64> {
    if samples
        .iter()
        .all(|sample| sample.storage_size_before.is_none())
    {
        MetricAvailability::unavailable("isolated per-sample measurement root")
    } else {
        samples
            .iter()
            .find_map(|sample| sample.storage_size_before)
            .map(storage_metric)
            .unwrap_or_else(|| MetricAvailability::unavailable("no storage_size_before sample"))
    }
}
