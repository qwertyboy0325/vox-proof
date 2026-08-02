#![cfg(feature = "persistence-spike")]

use std::time::{SystemTime, UNIX_EPOCH};

use vox_proof::persistence_evidence::{
    AppendAuthoritativeCandidateAdapter, AppendOpenMode, AppendTailStatus, CurrentContractOracle,
    all_fixture_variants, build_golden_small_state, build_superseded_state,
};

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
    let opened = adapter
        .open(&session, AppendOpenMode::Writable)
        .expect("open");
    assert_eq!(
        opened.tail_status,
        AppendTailStatus::IncompleteUncommitted { sequence: 2 }
    );
    assert!(CurrentContractOracle::compare(&state, opened.normalized_state()).passed);
    assert_eq!(
        adapter
            .append_authoritative_transition(&opened, 1, &next)
            .expect_err("tail blocks another authority write")
            .code,
        "incomplete-authoritative-tail"
    );
}

#[test]
fn acknowledged_append_reopens_at_the_exact_committed_transition() {
    let adapter = new_adapter("acknowledged-append");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let opened = adapter
        .open(&session, AppendOpenMode::Writable)
        .expect("open writer");
    let mut next = state.clone();
    next.durable_command_tokens.evidence_writer_token = "writer-token-committed".to_owned();
    let acknowledgement = adapter
        .append_authoritative_transition(&opened, 1, &next)
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
    let writable = adapter
        .open(&session, AppendOpenMode::Writable)
        .expect("open writer after close");
    assert_eq!(
        adapter
            .append_authoritative_transition(&writable, 2, &state)
            .expect_err("historical duplicate rejected before acknowledgement")
            .code,
        "semantic-duplicate-transition"
    );
}

#[test]
fn malformed_newer_duplicate_stale_and_unsafe_inputs_fail_closed() {
    let adapter = new_adapter("boundaries");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let opened = adapter
        .open(&session, AppendOpenMode::Writable)
        .expect("open");
    assert_eq!(
        adapter
            .append_authoritative_transition(&opened, 0, &state)
            .expect_err("stale sequence")
            .code,
        "stale-append-precondition"
    );
    assert_eq!(
        adapter
            .append_authoritative_transition(&opened, 1, &state)
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
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    adapter
        .write_temporary_artifact_for_test(&session, "abandoned-output")
        .expect("temporary hook");
    let plan = adapter.plan_cleanup(&session).expect("cleanup plan");
    assert_eq!(plan.temporary_artifacts, vec!["abandoned-output"]);
    assert!(!plan.destructive_cleanup_permitted);

    let opened = adapter
        .open(&session, AppendOpenMode::Writable)
        .expect("open");
    let duplicate = adapter
        .duplicate(&opened, "duplicate-session")
        .expect("duplicate");
    adapter.close(opened).expect("close original");
    let duplicate_open = adapter
        .open(&duplicate, AppendOpenMode::ReadOnly)
        .expect("open duplicate");
    assert_eq!(
        duplicate_open
            .normalized_state()
            .duplicated_from_session_id
            .as_deref(),
        Some(state.session_id.as_str())
    );
    assert_eq!(
        duplicate_open.normalized_state().session_id,
        "duplicate-session"
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
