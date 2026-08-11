#![cfg(feature = "persistence-spike")]

use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use vox_proof::persistence_evidence::current_contract::AppendCheckpointStatus;
use vox_proof::persistence_evidence::{
    AppendAuthoritativeCandidateAdapter, AppendOpenMode, AppendTailStatus, CurrentContractOracle,
    all_fixture_variants, build_duplicated_session_lineage_state, build_golden_small_state,
    build_original_for_duplication_fixture, build_superseded_state,
};

#[test]
fn equivalence_contract_identifies_the_versioned_01b_append_candidate() {
    assert_eq!(
        vox_proof::persistence_evidence::candidate_equivalence_requirements().append_candidate_id,
        "current-contract-append-authoritative-candidate"
    );
}

fn new_adapter(label: &str) -> AppendAuthoritativeCandidateAdapter {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    AppendAuthoritativeCandidateAdapter::new(
        std::env::temp_dir().join(format!("voxproof-01b-{label}-{nonce}")),
    )
    .expect("candidate root")
}

#[test]
fn child_process_holds_01b_writer_then_aborts() {
    let Ok(root) = std::env::var("VOXPROOF_01B_CHILD_ROOT") else {
        return;
    };
    let session_id = std::env::var("VOXPROOF_01B_CHILD_SESSION_ID").expect("session id");
    let ready_path = std::env::var("VOXPROOF_01B_CHILD_READY").expect("ready path");
    let release_path = std::env::var("VOXPROOF_01B_CHILD_RELEASE").expect("release path");
    let adapter = AppendAuthoritativeCandidateAdapter::new(root).expect("fresh child adapter");
    let _writer = adapter
        .open_existing(session_id, AppendOpenMode::Writable)
        .expect("child writer");
    std::fs::write(ready_path, b"ready").expect("announce ready");
    while !std::path::Path::new(&release_path).exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    std::process::abort();
}

#[test]
fn fresh_adapter_reopens_existing_session_and_child_abort_releases_writer() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("voxproof-01b-child-{nonce}"));
    let state = build_golden_small_state();
    let adapter = AppendAuthoritativeCandidateAdapter::new(&root).expect("parent adapter");
    let session = adapter.create(&state).expect("create");
    let ready = root.join("child.ready");
    let release = root.join("child.release");
    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "child_process_holds_01b_writer_then_aborts",
            "--nocapture",
        ])
        .env("VOXPROOF_01B_CHILD_ROOT", &root)
        .env("VOXPROOF_01B_CHILD_SESSION_ID", session.session_id())
        .env("VOXPROOF_01B_CHILD_READY", &ready)
        .env("VOXPROOF_01B_CHILD_RELEASE", &release)
        .spawn()
        .expect("spawn child");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.exists(), "child obtained the writer lock");
    let fresh = AppendAuthoritativeCandidateAdapter::new(&root).expect("fresh adapter");
    assert_eq!(
        fresh
            .open_existing(session.session_id(), AppendOpenMode::Writable)
            .expect_err("live child writer rejected")
            .code,
        "writer-already-open"
    );
    std::fs::write(&release, b"abort").expect("release child");
    assert!(!child.wait().expect("wait child").success(), "child aborts");
    let reopened = fresh
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("fresh process takeover after abort");
    assert!(CurrentContractOracle::compare(&state, reopened.normalized_state()).passed);
    fresh.close(reopened).expect("close fresh writer");
}

#[test]
fn read_only_sees_only_committed_prefix_and_writable_retry_recovers_tail() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("voxproof-01b-readonly-tail-{nonce}"));
    let adapter = AppendAuthoritativeCandidateAdapter::new(&root).expect("writer adapter");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let mut writer = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("writer");
    let mut next = state.clone();
    next.durable_command_tokens.evidence_writer_token = "recovered-writer".to_owned();
    adapter
        .append_incomplete_tail_for_test(&mut writer, &next)
        .expect("state without commit");
    let reader = AppendAuthoritativeCandidateAdapter::new(&root).expect("fresh reader");
    let read_only = reader
        .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
        .expect("read-only open during tail");
    assert_eq!(
        read_only.tail_status,
        AppendTailStatus::IncompleteUncommitted { sequence: 2 }
    );
    assert!(CurrentContractOracle::compare(&state, read_only.normalized_state()).passed);
    let acknowledgement = adapter
        .append_authoritative_transition(&mut writer, 1, &next)
        .expect("writer recovers and retries");
    assert_eq!(acknowledgement.committed_sequence, 2);

    let partial = new_adapter("partial-final-record");
    let partial_session = partial.create(&state).expect("create partial");
    let mut partial_writer = partial
        .open_existing(partial_session.session_id(), AppendOpenMode::Writable)
        .expect("partial writer");
    partial
        .append_raw_tail_for_test(&mut partial_writer, br#"{"record_kind":"state""#)
        .expect("partial final record");
    let partial_reader = partial
        .open_existing(partial_session.session_id(), AppendOpenMode::ReadOnly)
        .expect("partial tail remains inspectable");
    assert_eq!(
        partial_reader.tail_status,
        AppendTailStatus::IncompleteUncommitted { sequence: 2 }
    );
    partial.close(partial_writer).expect("close partial writer");
    let recovered = partial
        .open_existing(partial_session.session_id(), AppendOpenMode::Writable)
        .expect("writable recovery truncates partial tail");
    assert_eq!(recovered.tail_status, AppendTailStatus::Clean);
}

#[test]
fn committed_state_tampering_and_concurrent_create_fail_closed() {
    let adapter = new_adapter("commit-binding");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let mut tamper_writer = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("tamper writer");
    adapter
        .tamper_committed_state_for_test(
            &mut tamper_writer,
            &state.durable_command_tokens.evidence_writer_token,
            "writer:tampered-but-oracle-valid",
        )
        .expect("tamper committed payload");
    assert_eq!(
        adapter
            .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
            .expect_err("commit binding detects oracle-valid tampering")
            .code,
        "commit-fingerprint-mismatch"
    );

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("voxproof-01b-concurrent-create-{nonce}"));
    let left_state = build_golden_small_state();
    let right_state = left_state.clone();
    let left_root = root.clone();
    let right_root = root;
    let left = std::thread::spawn(move || {
        AppendAuthoritativeCandidateAdapter::new(left_root)
            .expect("left adapter")
            .create(&left_state)
            .is_ok()
    });
    let right = std::thread::spawn(move || {
        AppendAuthoritativeCandidateAdapter::new(right_root)
            .expect("right adapter")
            .create(&right_state)
            .is_ok()
    });
    assert_ne!(
        left.join().expect("left thread"),
        right.join().expect("right thread")
    );
}

#[cfg(unix)]
#[test]
fn canonical_storage_root_and_static_authority_aliases_fail_closed() {
    use std::os::unix::fs::symlink;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let actual_root = std::env::temp_dir().join(format!("voxproof-01b-alias-root-{nonce}"));
    let alias_root = std::env::temp_dir().join(format!("voxproof-01b-alias-link-{nonce}"));
    std::fs::create_dir(&actual_root).expect("actual root");
    symlink(&actual_root, &alias_root).expect("root symlink");
    let adapter =
        AppendAuthoritativeCandidateAdapter::new(&alias_root).expect("canonicalized root");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create through root alias");
    adapter
        .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
        .expect("canonicalized adapter remains usable");

    let session_root = actual_root.join(session.session_id());
    for (name, expected_code) in [
        ("manifest.json", "aliased-manifest-path"),
        ("canonical.append.jsonl", "aliased-append-log-path"),
        ("authoritative.writer.lock", "aliased-writer-lock-path"),
    ] {
        let path = session_root.join(name);
        let backup = session_root.join(format!("{name}.real"));
        std::fs::rename(&path, &backup).expect("move authority leaf");
        symlink(&backup, &path).expect("alias authority leaf");
        assert_eq!(
            adapter
                .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
                .expect_err("aliased authority leaf rejected")
                .code,
            expected_code
        );
        std::fs::remove_file(&path).expect("remove leaf alias");
        std::fs::rename(&backup, &path).expect("restore authority leaf");
    }

    let external_root = std::env::temp_dir().join(format!("voxproof-01b-external-session-{nonce}"));
    let external =
        AppendAuthoritativeCandidateAdapter::new(&external_root).expect("external adapter");
    external.create(&state).expect("external session");
    std::fs::remove_dir_all(&session_root).expect("remove local test session");
    symlink(external_root.join(session.session_id()), &session_root).expect("session alias");
    assert_eq!(
        adapter
            .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
            .expect_err("session alias rejected")
            .code,
        "aliased-session-path"
    );
}

#[test]
fn bounded_manifest_and_outbound_record_preflight_preserve_last_commit() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("voxproof-01b-bounded-input-{nonce}"));
    let adapter = AppendAuthoritativeCandidateAdapter::new(&root).expect("adapter");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let manifest = root.join(session.session_id()).join("manifest.json");
    std::fs::write(&manifest, vec![b' '; 64 * 1024 + 1]).expect("oversized manifest");
    assert_eq!(
        adapter
            .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
            .expect_err("manifest bound enforced")
            .code,
        "manifest-too-large"
    );

    let outbound = new_adapter("outbound-bound");
    let outbound_session = outbound.create(&state).expect("outbound create");
    let mut writer = outbound
        .open_existing(outbound_session.session_id(), AppendOpenMode::Writable)
        .expect("outbound writer");
    let mut oversized_state = state.clone();
    oversized_state.durable_command_tokens.evidence_writer_token = "x".repeat(2 * 1024 * 1024);
    assert_eq!(
        outbound
            .append_authoritative_transition(&mut writer, 1, &oversized_state)
            .expect_err("unreplayable outbound state rejected before write")
            .code,
        "outbound-record-too-large"
    );
    let read_only = outbound
        .open_existing(outbound_session.session_id(), AppendOpenMode::ReadOnly)
        .expect("last commit remains readable");
    assert!(CurrentContractOracle::compare(&state, read_only.normalized_state()).passed);
    outbound
        .validate_record_capacity_for_test(4_094)
        .expect("one final transition fits exactly");
    assert_eq!(
        outbound
            .validate_record_capacity_for_test(4_095)
            .expect_err("partial final transition refused")
            .code,
        "record-count-exhausted"
    );
}

#[test]
fn commit_ack_survives_checkpoint_failure_and_repairs_before_next_write() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("voxproof-01b-checkpoint-{nonce}"));
    let adapter = AppendAuthoritativeCandidateAdapter::new(&root).expect("adapter");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let mut writer = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("writer");
    let manifest_temporary = root.join(session.session_id()).join("manifest.tmp");
    std::fs::create_dir(&manifest_temporary).expect("block manifest checkpoint");
    let mut committed = state.clone();
    committed.durable_command_tokens.evidence_writer_token = "writer:checkpoint-behind".to_owned();
    let acknowledgement = adapter
        .append_authoritative_transition(&mut writer, 1, &committed)
        .expect("Commit sync remains an acknowledgement boundary");
    assert_eq!(acknowledgement.committed_sequence, 2);
    assert_eq!(
        acknowledgement.checkpoint_status,
        AppendCheckpointStatus::Behind {
            last_error_code: "canonical-path-not-regular-file"
        }
    );
    let read_only = adapter
        .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
        .expect("read-only replays the committed log");
    assert_eq!(read_only.committed_sequence, 2);
    assert!(CurrentContractOracle::compare(&committed, read_only.normalized_state()).passed);

    let mut later = committed.clone();
    later.durable_command_tokens.evidence_writer_token =
        "writer:after-checkpoint-repair".to_owned();
    assert_eq!(
        adapter
            .append_authoritative_transition(&mut writer, 2, &later)
            .expect_err("checkpoint repair precedes another State write")
            .code,
        "canonical-path-not-regular-file"
    );
    std::fs::remove_dir(&manifest_temporary).expect("allow checkpoint repair");
    let repaired = adapter
        .append_authoritative_transition(&mut writer, 2, &later)
        .expect("repair then append");
    assert_eq!(repaired.checkpoint_status, AppendCheckpointStatus::Current);
    assert_eq!(repaired.committed_sequence, 3);
}

#[test]
fn failed_create_and_unguarded_fault_mutation_never_publish_authority() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("voxproof-01b-incomplete-create-{nonce}"));
    let adapter = AppendAuthoritativeCandidateAdapter::new(&root).expect("adapter");
    let mut oversized = build_golden_small_state();
    oversized.durable_command_tokens.evidence_writer_token = "x".repeat(2 * 1024 * 1024);
    assert_eq!(
        adapter
            .create(&oversized)
            .expect_err("oversized initial authority is not published")
            .code,
        "outbound-record-too-large"
    );
    assert_eq!(
        adapter
            .open_existing(&oversized.session_id, AppendOpenMode::ReadOnly)
            .expect_err("incomplete creation is never authority")
            .code,
        "incomplete-session-creation"
    );

    let guarded = new_adapter("guarded-hooks");
    let state = build_golden_small_state();
    let session = guarded.create(&state).expect("guarded session");
    let mut writer = guarded
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("writer");
    let second_adapter_root = std::env::temp_dir();
    let foreign = AppendAuthoritativeCandidateAdapter::new(second_adapter_root)
        .expect("foreign adapter instance");
    assert_eq!(
        foreign
            .append_raw_tail_for_test(&mut writer, b"")
            .expect_err("foreign adapter cannot use live writer")
            .code,
        "foreign-writer-handle"
    );
    let mut read_only = guarded
        .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
        .expect("read-only handle");
    assert_eq!(
        guarded
            .append_raw_tail_for_test(&mut read_only, b"")
            .expect_err("read-only handle cannot mutate")
            .code,
        "not-authoritative-writer"
    );
}

#[test]
fn duplicate_uses_latest_commit_has_independent_identity_and_remains_independent() {
    let adapter = new_adapter("duplicate-latest");
    let original = build_original_for_duplication_fixture();
    let source = adapter.create(&original).expect("source");
    let mut source_writer = adapter
        .open_existing(source.session_id(), AppendOpenMode::Writable)
        .expect("source writer");
    let mut latest = original.clone();
    latest.durable_command_tokens.evidence_writer_token =
        "writer:latest-before-duplicate".to_owned();
    adapter
        .append_authoritative_transition(&mut source_writer, 1, &latest)
        .expect("latest source commit");

    let duplicate_id = "alias:latest-before-duplicate";
    let duplicate = adapter
        .duplicate(&mut source_writer, duplicate_id)
        .expect("duplicate latest replay");
    let mut expected_duplicate = latest.clone();
    expected_duplicate.duplicated_from_session_id = Some(latest.session_id.clone());
    expected_duplicate.session_id = duplicate_id.to_owned();
    expected_duplicate
        .durable_command_tokens
        .evidence_writer_token = format!(
        "writer-encoded-session-id-v1:{}",
        duplicate_id
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let duplicate_read = adapter
        .open_existing(duplicate.session_id(), AppendOpenMode::ReadOnly)
        .expect("duplicate read");
    assert!(
        CurrentContractOracle::compare(&expected_duplicate, duplicate_read.normalized_state())
            .passed
    );
    assert_ne!(
        expected_duplicate
            .durable_command_tokens
            .evidence_writer_token,
        latest.durable_command_tokens.evidence_writer_token
    );

    let duplicate_writer = adapter
        .open_existing(duplicate.session_id(), AppendOpenMode::Writable)
        .expect("duplicate has independent writer ownership");
    assert_eq!(
        adapter
            .open_existing(duplicate.session_id(), AppendOpenMode::Writable)
            .expect_err("second duplicate writer refused")
            .code,
        "writer-already-open"
    );
    adapter
        .close(duplicate_writer)
        .expect("close duplicate writer");

    let mut later_source = latest.clone();
    later_source.durable_command_tokens.evidence_writer_token = "writer:after-duplicate".to_owned();
    adapter
        .append_authoritative_transition(&mut source_writer, 2, &later_source)
        .expect("later source commit");
    let source_read = adapter
        .open_existing(source.session_id(), AppendOpenMode::ReadOnly)
        .expect("source read");
    let duplicate_read = adapter
        .open_existing(duplicate.session_id(), AppendOpenMode::ReadOnly)
        .expect("duplicate remains readable");
    assert!(CurrentContractOracle::compare(&later_source, source_read.normalized_state()).passed);
    assert!(
        CurrentContractOracle::compare(&expected_duplicate, duplicate_read.normalized_state())
            .passed
    );
}

#[test]
fn round_trips_every_current_contract_fixture_through_the_single_oracle() {
    let adapter = new_adapter("all-fixtures");
    for fixture in all_fixture_variants() {
        let session = adapter.create(&fixture.expected_state).expect("create");
        let opened = adapter
            .open_existing(session.session_id(), AppendOpenMode::Writable)
            .expect("open");
        assert_eq!(
            opened.tail_status,
            AppendTailStatus::Clean,
            "{}",
            fixture.variant_id
        );
        assert!(
            CurrentContractOracle::compare(&fixture.expected_state, opened.normalized_state())
                .passed,
            "{}",
            fixture.variant_id
        );
        adapter.close(opened).expect("close");
        let reopened = adapter
            .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
            .expect("reopen");
        assert!(
            CurrentContractOracle::compare(&fixture.expected_state, reopened.normalized_state())
                .passed,
            "{}",
            fixture.variant_id
        );
    }
}

#[test]
fn incomplete_tail_is_detected_and_never_becomes_authority() {
    let adapter = new_adapter("incomplete-tail");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let mut next = state.clone();
    next.durable_command_tokens.evidence_writer_token = "writer-token-after-boundary".to_owned();
    let mut opened = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("open");
    adapter
        .append_incomplete_tail_for_test(&mut opened, &next)
        .expect("inject incomplete tail");
    assert_eq!(opened.tail_status, AppendTailStatus::Clean);
    assert!(CurrentContractOracle::compare(&state, opened.normalized_state()).passed);
    let acknowledgement = adapter
        .append_authoritative_transition(&mut opened, 1, &next)
        .expect("recovered writer can retry append");
    assert_eq!(acknowledgement.committed_sequence, 2);
}

#[test]
fn acknowledged_append_reopens_at_the_exact_committed_transition() {
    let adapter = new_adapter("acknowledged-append");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let mut opened = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("open writer");
    assert_eq!(
        adapter
            .open_existing(session.session_id(), AppendOpenMode::Writable)
            .expect_err("second writer refused")
            .code,
        "writer-already-open"
    );
    let mut next = state.clone();
    next.durable_command_tokens.evidence_writer_token = "writer-token-committed".to_owned();
    let acknowledgement = adapter
        .append_authoritative_transition(&mut opened, 1, &next)
        .expect("durable append acknowledgement");
    assert_eq!(acknowledgement.committed_sequence, 2);
    adapter.close(opened).expect("close");
    let reopened = adapter
        .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
        .expect("reopen");
    assert_eq!(
        reopened.committed_sequence,
        acknowledgement.committed_sequence
    );
    assert!(CurrentContractOracle::compare(&next, reopened.normalized_state()).passed);
    let mut writable = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("open writer after close");
    assert_eq!(
        adapter
            .append_authoritative_transition(&mut writable, 2, &state)
            .expect_err("historical duplicate rejected before acknowledgement")
            .code,
        "semantic-duplicate-transition"
    );

    let reordered_adapter = new_adapter("normalized-duplicate");
    let reordered_state = build_superseded_state();
    let reordered_session = reordered_adapter
        .create(&reordered_state)
        .expect("create reordered state");
    let mut reordered_opened = reordered_adapter
        .open_existing(reordered_session.session_id(), AppendOpenMode::Writable)
        .expect("open reordered state");
    let mut same_canonical_state = reordered_state.clone();
    same_canonical_state.review_cases.swap(0, 1);
    assert_eq!(
        reordered_adapter
            .append_authoritative_transition(&mut reordered_opened, 1, &same_canonical_state)
            .expect_err("normalized duplicate rejected before acknowledgement")
            .code,
        "semantic-duplicate-transition"
    );
}

#[test]
fn malformed_newer_duplicate_stale_and_unsafe_inputs_fail_closed() {
    let adapter = new_adapter("boundaries");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let mut opened = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("open");
    assert_eq!(
        adapter
            .append_authoritative_transition(&mut opened, 0, &state)
            .expect_err("stale sequence")
            .code,
        "stale-append-precondition"
    );
    assert_eq!(
        adapter
            .append_authoritative_transition(&mut opened, 1, &state)
            .expect_err("duplicate canonical identity")
            .code,
        "semantic-duplicate-transition"
    );
    adapter
        .append_raw_tail_for_test(&mut opened, b"{not-json}\n")
        .expect("inject hostile tail");
    assert_eq!(
        adapter
            .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
            .expect_err("malformed authoritative tail")
            .code,
        "malformed-authoritative-record"
    );

    let newer = new_adapter("newer");
    let newer_session = newer.create(&state).expect("create newer");
    let mut newer_writer = newer
        .open_existing(newer_session.session_id(), AppendOpenMode::Writable)
        .expect("newer writer");
    newer
        .set_format_version_for_test(&mut newer_writer, 2)
        .expect("set newer format");
    newer.close(newer_writer).expect("close newer writer");
    assert_eq!(
        newer
            .open_existing(newer_session.session_id(), AppendOpenMode::Writable)
            .expect_err("newer is not writable")
            .code,
        "unsupported-newer-format"
    );

    let mut unsafe_state = state;
    unsafe_state.session_id = "../outside".to_owned();
    assert_eq!(
        adapter
            .create(&unsafe_state)
            .expect_err("path escape rejected")
            .code,
        "unsafe-session-id"
    );

    let oversized = new_adapter("oversized");
    let oversized_session = oversized
        .create(&build_golden_small_state())
        .expect("create oversized");
    let mut oversized_writer = oversized
        .open_existing(oversized_session.session_id(), AppendOpenMode::Writable)
        .expect("oversized writer");
    oversized
        .append_raw_tail_for_test(&mut oversized_writer, &vec![b'x'; 2 * 1024 * 1024 + 1])
        .expect("inject oversized tail");
    let bounded = std::panic::catch_unwind(|| {
        oversized.open_existing(oversized_session.session_id(), AppendOpenMode::ReadOnly)
    });
    assert!(bounded.is_ok(), "oversized hostile input must not panic");
    assert_eq!(
        bounded
            .expect("no panic")
            .expect_err("oversized tail rejected")
            .code,
        "record-too-large"
    );

    let mut reordered = build_superseded_state();
    reordered.review_ledger_events.swap(0, 1);
    assert_eq!(
        new_adapter("event-order")
            .create(&reordered)
            .expect_err("event order cannot be accepted")
            .code,
        "current-contract-oracle-rejected"
    );
}

#[test]
fn canonical_loss_anchor_forgery_duplication_and_cleanup_are_bounded() {
    let adapter = new_adapter("canonical-loss");
    let state = build_original_for_duplication_fixture();
    let session = adapter.create(&state).expect("create");
    let mut opened = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("open");
    adapter
        .write_temporary_artifact_for_test(&mut opened, "abandoned-output")
        .expect("temporary hook");
    let plan = adapter.plan_cleanup(&session).expect("cleanup plan");
    assert_eq!(plan.temporary_artifacts, vec!["abandoned-output"]);
    assert!(!plan.destructive_cleanup_permitted);

    let duplicate = adapter
        .duplicate(&mut opened, "session:current-contract:duplicate")
        .expect("duplicate");
    adapter.close(opened).expect("close original");
    let duplicate_open = adapter
        .open_existing(duplicate.session_id(), AppendOpenMode::ReadOnly)
        .expect("open duplicate");
    let expected_duplicate = build_duplicated_session_lineage_state();
    assert!(
        CurrentContractOracle::compare(&expected_duplicate, duplicate_open.normalized_state())
            .passed
    );
    assert_ne!(
        duplicate_open
            .normalized_state()
            .durable_command_tokens
            .evidence_writer_token,
        state.durable_command_tokens.evidence_writer_token
    );

    let mut forged_anchor = state.clone();
    forged_anchor.review_cases[0].observed_source_bytes = "forged".to_owned();
    assert_eq!(
        adapter
            .create(&forged_anchor)
            .expect_err("forged anchor rejected")
            .code,
        "current-contract-oracle-rejected"
    );

    let mut destructive_writer = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("destructive writer");
    adapter
        .remove_canonical_log_for_test(&mut destructive_writer)
        .expect("remove canonical authority");
    assert_eq!(
        adapter
            .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
            .expect_err("derived and temporary files cannot reconstruct canonical truth")
            .code,
        "stat-candidate-path"
    );
}

#[cfg(unix)]
#[test]
fn shared_canonical_log_distinct_writer_locks_fail_closed() {
    use std::os::unix::fs::MetadataExt;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root_a = std::env::temp_dir().join(format!("voxproof-01b-hardlink-a-{nonce}"));
    let root_b = std::env::temp_dir().join(format!("voxproof-01b-hardlink-b-{nonce}"));
    let state_a = build_golden_small_state();
    let mut state_b = state_a.clone();
    state_b.session_id = "session:current-contract:hardlink-b".to_owned();

    let adapter_a = AppendAuthoritativeCandidateAdapter::new(&root_a).expect("adapter a");
    let session_a = adapter_a.create(&state_a).expect("create session a");
    let source_log = root_a
        .join(session_a.session_id())
        .join("canonical.append.jsonl");
    let source_bytes = std::fs::read(&source_log).expect("source log bytes");
    assert_eq!(
        std::fs::metadata(&source_log)
            .expect("source log metadata")
            .nlink(),
        1
    );

    std::fs::create_dir_all(&root_b).expect("root b");
    let session_b_root = root_b.join(&state_b.session_id);
    std::fs::create_dir(&session_b_root).expect("session b root");
    std::fs::create_dir(session_b_root.join("temporary")).expect("session b temporary");
    std::fs::copy(
        root_a.join(session_a.session_id()).join("manifest.json"),
        session_b_root.join("manifest.json"),
    )
    .expect("copy manifest b");
    let mut manifest_b: serde_json::Value =
        serde_json::from_slice(&std::fs::read(session_b_root.join("manifest.json")).unwrap())
            .expect("manifest json");
    manifest_b["session_id"] = serde_json::Value::String(state_b.session_id.clone());
    std::fs::write(
        session_b_root.join("manifest.json"),
        serde_json::to_vec(&manifest_b).expect("manifest bytes"),
    )
    .expect("write manifest b");
    std::fs::hard_link(&source_log, session_b_root.join("canonical.append.jsonl"))
        .expect("hard link canonical log");
    std::fs::write(session_b_root.join("authoritative.writer.lock"), b"lock-b")
        .expect("distinct writer lock");

    let linked_log = session_b_root.join("canonical.append.jsonl");
    assert_ne!(
        root_a
            .join(session_a.session_id())
            .join("authoritative.writer.lock"),
        session_b_root.join("authoritative.writer.lock")
    );
    assert!(
        std::fs::metadata(&linked_log)
            .expect("linked log metadata")
            .nlink()
            > 1,
        "canonical log must be hard-linked across session roots"
    );

    let adapter_b = AppendAuthoritativeCandidateAdapter::new(&root_b).expect("adapter b");
    assert_eq!(
        adapter_a
            .open_existing(session_a.session_id(), AppendOpenMode::ReadOnly)
            .expect_err("session a read-only rejects aliased canonical log")
            .code,
        "authority-leaf-hard-linked"
    );
    assert_eq!(
        adapter_a
            .open_existing(session_a.session_id(), AppendOpenMode::Writable)
            .expect_err("session a writable rejects aliased canonical log")
            .code,
        "authority-leaf-hard-linked"
    );
    assert_eq!(
        adapter_b
            .open_existing(&state_b.session_id, AppendOpenMode::ReadOnly)
            .expect_err("session b read-only rejects aliased canonical log")
            .code,
        "authority-leaf-hard-linked"
    );
    assert_eq!(
        adapter_b
            .open_existing(&state_b.session_id, AppendOpenMode::Writable)
            .expect_err("session b writable rejects aliased canonical log")
            .code,
        "authority-leaf-hard-linked"
    );
    assert_eq!(
        std::fs::read(&source_log).expect("source log unchanged"),
        source_bytes
    );
}

#[cfg(unix)]
#[test]
fn hard_linked_manifest_fail_closed_before_parse() {
    use std::os::unix::fs::MetadataExt;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("voxproof-01b-hardlink-manifest-{nonce}"));
    let adapter = AppendAuthoritativeCandidateAdapter::new(&root).expect("adapter");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let session_root = root.join(session.session_id());
    let manifest = session_root.join("manifest.json");
    let manifest_backup = session_root.join("manifest.real.json");
    std::fs::rename(&manifest, &manifest_backup).expect("move manifest");
    std::fs::hard_link(&manifest_backup, &manifest).expect("hard link manifest");
    assert!(
        std::fs::metadata(&manifest)
            .expect("manifest metadata")
            .nlink()
            > 1,
        "manifest must be hard-linked"
    );

    for mode in [AppendOpenMode::ReadOnly, AppendOpenMode::Writable] {
        assert_eq!(
            adapter
                .open_existing(session.session_id(), mode)
                .expect_err("hard-linked manifest rejected")
                .code,
            "authority-leaf-hard-linked"
        );
    }
}

#[cfg(unix)]
#[test]
fn hard_linked_writer_lock_fail_closed_before_authority() {
    use std::os::unix::fs::MetadataExt;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("voxproof-01b-hardlink-lock-{nonce}"));
    let adapter = AppendAuthoritativeCandidateAdapter::new(&root).expect("adapter");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let session_root = root.join(session.session_id());
    let lock = session_root.join("authoritative.writer.lock");
    let lock_backup = session_root.join("authoritative.writer.real.lock");
    std::fs::rename(&lock, &lock_backup).expect("move lock");
    std::fs::hard_link(&lock_backup, &lock).expect("hard link lock");
    assert!(
        std::fs::metadata(&lock).expect("lock metadata").nlink() > 1,
        "writer lock must be hard-linked"
    );

    assert_eq!(
        adapter
            .open_existing(session.session_id(), AppendOpenMode::Writable)
            .expect_err("hard-linked writer lock rejected")
            .code,
        "authority-leaf-hard-linked"
    );
    adapter
        .open_existing(session.session_id(), AppendOpenMode::ReadOnly)
        .expect("read-only open does not acquire writer lock");
}
