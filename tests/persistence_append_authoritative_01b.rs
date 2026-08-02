#![cfg(feature = "persistence-spike")]

use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
        .open(&session, AppendOpenMode::Writable)
        .expect("writer");
    let mut next = state.clone();
    next.durable_command_tokens.evidence_writer_token = "recovered-writer".to_owned();
    adapter
        .append_incomplete_tail_for_test(&session, &next)
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
    partial
        .append_raw_tail_for_test(&partial_session, br#"{"record_kind":"state""#)
        .expect("partial final record");
    let partial_reader = partial
        .open(&partial_session, AppendOpenMode::ReadOnly)
        .expect("partial tail remains inspectable");
    assert_eq!(
        partial_reader.tail_status,
        AppendTailStatus::IncompleteUncommitted { sequence: 2 }
    );
    let recovered = partial
        .open(&partial_session, AppendOpenMode::Writable)
        .expect("writable recovery truncates partial tail");
    assert_eq!(recovered.tail_status, AppendTailStatus::Clean);
}

#[test]
fn committed_state_tampering_and_concurrent_create_fail_closed() {
    let adapter = new_adapter("commit-binding");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    adapter
        .tamper_committed_state_for_test(
            &session,
            &state.durable_command_tokens.evidence_writer_token,
            "writer:tampered-but-oracle-valid",
        )
        .expect("tamper committed payload");
    assert_eq!(
        adapter
            .open(&session, AppendOpenMode::ReadOnly)
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

#[test]
fn round_trips_every_current_contract_fixture_through_the_single_oracle() {
    let adapter = new_adapter("all-fixtures");
    for fixture in all_fixture_variants() {
        let session = adapter.create(&fixture.expected_state).expect("create");
        let opened = adapter
            .open(&session, AppendOpenMode::Writable)
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
            .open(&session, AppendOpenMode::ReadOnly)
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
    adapter
        .append_incomplete_tail_for_test(&session, &next)
        .expect("inject incomplete tail");
    let mut opened = adapter
        .open(&session, AppendOpenMode::Writable)
        .expect("open");
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
        .open(&session, AppendOpenMode::Writable)
        .expect("open writer");
    assert_eq!(
        adapter
            .open(&session, AppendOpenMode::Writable)
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
        .open(&session, AppendOpenMode::ReadOnly)
        .expect("reopen");
    assert_eq!(
        reopened.committed_sequence,
        acknowledgement.committed_sequence
    );
    assert!(CurrentContractOracle::compare(&next, reopened.normalized_state()).passed);
    let mut writable = adapter
        .open(&session, AppendOpenMode::Writable)
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
        .open(&reordered_session, AppendOpenMode::Writable)
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
        .open(&session, AppendOpenMode::Writable)
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
        .append_raw_tail_for_test(&session, b"{not-json}\n")
        .expect("inject hostile tail");
    assert_eq!(
        adapter
            .open(&session, AppendOpenMode::ReadOnly)
            .expect_err("malformed authoritative tail")
            .code,
        "malformed-authoritative-record"
    );

    let newer = new_adapter("newer");
    let newer_session = newer.create(&state).expect("create newer");
    newer
        .set_format_version_for_test(&newer_session, 2)
        .expect("set newer format");
    assert_eq!(
        newer
            .open(&newer_session, AppendOpenMode::Writable)
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
    oversized
        .append_raw_tail_for_test(&oversized_session, &vec![b'x'; 2 * 1024 * 1024 + 1])
        .expect("inject oversized tail");
    let bounded =
        std::panic::catch_unwind(|| oversized.open(&oversized_session, AppendOpenMode::ReadOnly));
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
    adapter
        .write_temporary_artifact_for_test(&session, "abandoned-output")
        .expect("temporary hook");
    let plan = adapter.plan_cleanup(&session).expect("cleanup plan");
    assert_eq!(plan.temporary_artifacts, vec!["abandoned-output"]);
    assert!(!plan.destructive_cleanup_permitted);

    let mut opened = adapter
        .open(&session, AppendOpenMode::Writable)
        .expect("open");
    let duplicate = adapter
        .duplicate(&mut opened, "session:current-contract:duplicate")
        .expect("duplicate");
    adapter.close(opened).expect("close original");
    let duplicate_open = adapter
        .open(&duplicate, AppendOpenMode::ReadOnly)
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

    adapter
        .remove_canonical_log_for_test(&session)
        .expect("remove canonical authority");
    assert_eq!(
        adapter
            .open(&session, AppendOpenMode::ReadOnly)
            .expect_err("derived and temporary files cannot reconstruct canonical truth")
            .code,
        "open-append-log"
    );
}
