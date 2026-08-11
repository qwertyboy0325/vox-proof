#![cfg(feature = "persistence-spike")]

use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use vox_proof::persistence_evidence::{
    CurrentContractOracle, CurrentContractPreconditions, SQLITE_AUTHORITATIVE_CANDIDATE_ID,
    SQLITE_AUTHORITATIVE_CANDIDATE_VERSION, SqliteAuthoritativeCandidateAdapter, SqliteOpenMode,
    all_fixture_variants, build_base_manual_replacement_state, build_candidate_rejected_state,
    build_golden_small_state, build_original_for_duplication_fixture, build_promoted_active_state,
    build_revoked_historical_state, build_superseded_state,
};

fn new_root(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("voxproof-01c-sqlite-{label}-{nonce}"))
}

fn new_adapter(label: &str) -> SqliteAuthoritativeCandidateAdapter {
    SqliteAuthoritativeCandidateAdapter::new(new_root(label)).expect("candidate root")
}

fn updated_state(
    mut state: vox_proof::persistence_evidence::CurrentContractState,
    token: &str,
) -> vox_proof::persistence_evidence::CurrentContractState {
    state.durable_command_tokens.evidence_writer_token = token.to_owned();
    vox_proof::persistence_evidence::current_contract::finalize_derived_fields(&mut state);
    state.normalize()
}

#[test]
fn candidate_identity_and_equivalence_contract_name_the_distinct_v3_sqlite_candidate() {
    let adapter = new_adapter("identity");
    assert_eq!(adapter.candidate_id(), SQLITE_AUTHORITATIVE_CANDIDATE_ID);
    assert_eq!(adapter.candidate_version(), "01C-SQLITE-2");
    assert_eq!(adapter.format_version(), 1);
    assert_eq!(
        vox_proof::persistence_evidence::candidate_equivalence_requirements().sqlite_candidate_id,
        SQLITE_AUTHORITATIVE_CANDIDATE_ID
    );
    assert_eq!(SQLITE_AUTHORITATIVE_CANDIDATE_VERSION, "01C-SQLITE-2");
}

#[test]
fn every_v3_fixture_round_trips_through_typed_relational_authority_and_oracle_v3() {
    let adapter = new_adapter("all-fixtures");
    for fixture in all_fixture_variants() {
        let session = adapter
            .create(&fixture.expected_state)
            .expect("create fixture");
        let opened = adapter
            .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
            .expect("fresh read-only reopen");
        assert!(
            CurrentContractOracle::compare(&fixture.expected_state, opened.normalized_state())
                .passed,
            "{}",
            fixture.variant_id
        );
        adapter.close(opened).expect("close read-only");
    }
}

#[test]
fn fresh_candidate_instance_reopens_and_acknowledged_transition_survives() {
    let root = new_root("fresh-reopen");
    let adapter = SqliteAuthoritativeCandidateAdapter::new(&root).expect("candidate root");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let mut writer = adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("open writer");
    let next = updated_state(state.clone(), "writer:sqlite-v3-ack");
    let preconditions = writer.authoritative_preconditions();
    let ack = adapter
        .apply_authoritative_transition(&mut writer, &preconditions, &next)
        .expect("committed acknowledgement");
    assert_eq!(ack.committed_generation, 2);
    adapter.close(writer).expect("close writer");

    let fresh = SqliteAuthoritativeCandidateAdapter::new(&root).expect("fresh candidate root");
    let reopened = fresh
        .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
        .expect("fresh process reopen");
    assert_eq!(reopened.committed_generation, ack.committed_generation);
    assert!(CurrentContractOracle::compare(&next, reopened.normalized_state()).passed);
}

#[test]
fn manual_replacement_and_each_reuse_lifecycle_shape_survive_relational_reconstruction() {
    let adapter = new_adapter("authority-shapes");
    for state in [
        build_base_manual_replacement_state(),
        build_candidate_rejected_state(),
        build_promoted_active_state(),
        build_revoked_historical_state(),
        build_superseded_state(),
    ] {
        let session = adapter.create(&state).expect("create lifecycle fixture");
        let reopened = adapter
            .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
            .expect("reopen lifecycle fixture");
        assert!(CurrentContractOracle::compare(&state, reopened.normalized_state()).passed);
        adapter.close(reopened).expect("close");
    }
}

#[test]
fn stale_generation_review_reuse_and_analysis_preconditions_fail_independently() {
    let adapter = new_adapter("stale-preconditions");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let mut writer = adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("writer");
    let next = updated_state(state, "writer:sqlite-v3-stale");
    let baseline = writer.authoritative_preconditions();

    let stale_generation = CurrentContractPreconditions {
        expected_generation: 0,
        ..baseline.clone()
    };
    assert_eq!(
        adapter
            .apply_authoritative_transition(&mut writer, &stale_generation, &next)
            .expect_err("stale generation rejected")
            .code,
        "stale-generation-precondition"
    );
    let stale_review = CurrentContractPreconditions {
        review_ledger_head: baseline.review_ledger_head + 1,
        ..baseline.clone()
    };
    assert_eq!(
        adapter
            .apply_authoritative_transition(&mut writer, &stale_review, &next)
            .expect_err("stale review rejected")
            .code,
        "stale-review-ledger-precondition"
    );
    let stale_reuse = CurrentContractPreconditions {
        reuse_governance_head: baseline.reuse_governance_head + 1,
        ..baseline.clone()
    };
    assert_eq!(
        adapter
            .apply_authoritative_transition(&mut writer, &stale_reuse, &next)
            .expect_err("stale reuse rejected")
            .code,
        "stale-reuse-governance-precondition"
    );
    let stale_analysis = CurrentContractPreconditions {
        active_analysis_snapshot_identity: "analysis:stale".to_owned(),
        ..baseline
    };
    assert_eq!(
        adapter
            .apply_authoritative_transition(&mut writer, &stale_analysis, &next)
            .expect_err("stale analysis rejected")
            .code,
        "stale-analysis-selection-precondition"
    );
}

#[test]
fn live_writer_exclusion_allows_only_read_only_view() {
    let root = new_root("writer-exclusion");
    let adapter = SqliteAuthoritativeCandidateAdapter::new(&root).expect("candidate root");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let writer = adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("first writer");
    let fresh = SqliteAuthoritativeCandidateAdapter::new(&root).expect("fresh candidate");
    assert_eq!(
        fresh
            .open_existing(session.session_id(), SqliteOpenMode::Writable)
            .expect_err("second writer refused")
            .code,
        "writer-already-open"
    );
    let reader = fresh
        .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
        .expect("read-only view remains allowed");
    assert!(CurrentContractOracle::compare(&state, reader.normalized_state()).passed);
    fresh.close(reader).expect("close reader");
    adapter.close(writer).expect("close writer");
}

#[test]
fn stale_writer_cannot_regain_authority_after_epoch_takeover() {
    let root = new_root("stale-writer-epoch");
    let first = SqliteAuthoritativeCandidateAdapter::new(&root).expect("candidate root");
    let state = build_golden_small_state();
    let session = first.create(&state).expect("create");
    let mut stale = first
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("first writer");
    first
        .expire_writer_lease_for_test(&stale)
        .expect("expire first lease");
    let successor = SqliteAuthoritativeCandidateAdapter::new(&root).expect("successor adapter");
    let takeover = successor
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("take over expired lease");
    let next = updated_state(state, "writer:stale-epoch-attempt");
    let stale_preconditions = stale.authoritative_preconditions();
    assert_eq!(
        first
            .apply_authoritative_transition(&mut stale, &stale_preconditions, &next)
            .expect_err("old writer cannot regain authority")
            .code,
        "writer-epoch-mismatch"
    );
    first
        .close(stale)
        .expect("old close cannot clear successor");
    successor.close(takeover).expect("close successor");
}

#[test]
fn read_only_handle_cannot_apply_authoritative_transition() {
    let adapter = new_adapter("read-only-refusal");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let mut reader = adapter
        .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
        .expect("reader");
    let preconditions = reader.authoritative_preconditions();
    let next = updated_state(state, "writer:read-only-refusal");
    assert_eq!(
        adapter
            .apply_authoritative_transition(&mut reader, &preconditions, &next)
            .expect_err("read-only authority refused")
            .code,
        "not-authoritative-writer"
    );
}

#[test]
fn child_process_holds_sqlite_v3_writer_then_aborts() {
    let Ok(root) = std::env::var("VOXPROOF_01C_SQLITE_CHILD_ROOT") else {
        return;
    };
    let session_id = std::env::var("VOXPROOF_01C_SQLITE_CHILD_SESSION_ID").expect("session id");
    let ready = std::env::var("VOXPROOF_01C_SQLITE_CHILD_READY").expect("ready path");
    let release = std::env::var("VOXPROOF_01C_SQLITE_CHILD_RELEASE").expect("release path");
    let adapter = SqliteAuthoritativeCandidateAdapter::new(root).expect("child adapter");
    let _writer = adapter
        .open_existing(session_id, SqliteOpenMode::Writable)
        .expect("child writer");
    std::fs::write(ready, b"ready").expect("announce ready");
    while !std::path::Path::new(&release).exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    std::process::abort();
}

#[test]
fn child_abort_expires_lease_and_new_writer_takes_over_without_old_authority() {
    let root = new_root("child-takeover");
    let adapter = SqliteAuthoritativeCandidateAdapter::new(&root).expect("candidate root");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    adapter
        .set_lease_duration_for_test(&session, 1_000)
        .expect("child lease long enough to prove live-writer exclusion");
    let ready = root.join("child.ready");
    let release = root.join("child.release");
    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "child_process_holds_sqlite_v3_writer_then_aborts",
            "--nocapture",
        ])
        .env("VOXPROOF_01C_SQLITE_CHILD_ROOT", &root)
        .env("VOXPROOF_01C_SQLITE_CHILD_SESSION_ID", session.session_id())
        .env("VOXPROOF_01C_SQLITE_CHILD_READY", &ready)
        .env("VOXPROOF_01C_SQLITE_CHILD_RELEASE", &release)
        .spawn()
        .expect("spawn child");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.exists(), "child acquired writer ownership");
    let fresh = SqliteAuthoritativeCandidateAdapter::new(&root).expect("fresh candidate");
    assert_eq!(
        fresh
            .open_existing(session.session_id(), SqliteOpenMode::Writable)
            .expect_err("live child writer excluded")
            .code,
        "writer-already-open"
    );
    std::fs::write(&release, b"abort").expect("release child");
    assert!(!child.wait().expect("wait child").success(), "child aborts");
    std::thread::sleep(Duration::from_millis(1_100));
    let takeover = fresh
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("expired child lease can be taken over");
    assert!(CurrentContractOracle::compare(&state, takeover.normalized_state()).passed);
    fresh.close(takeover).expect("close takeover");
}

#[test]
fn unknown_format_and_canonical_corruption_fail_closed_while_derived_cache_rebuilds() {
    let adapter = new_adapter("format-corruption-cache");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let writer = adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("writer");
    adapter
        .tamper_derived_cache_for_test(&writer, "not-a-derived-projection")
        .expect("tamper non-authoritative cache");
    adapter.close(writer).expect("close cache writer");
    let rebuilt = adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("writable reopen rebuilds cache from relational authority");
    assert!(CurrentContractOracle::compare(&state, rebuilt.normalized_state()).passed);
    adapter.close(rebuilt).expect("close rebuilt cache");

    let canonical_writer = adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("canonical writer");
    adapter
        .tamper_canonical_provenance_for_test(&canonical_writer)
        .expect("tamper canonical row");
    assert_eq!(
        adapter
            .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
            .expect_err("canonical corruption refuses authority")
            .code,
        "canonical-corruption"
    );
    adapter
        .close(canonical_writer)
        .expect("close corrupted writer");

    let format_adapter = new_adapter("unknown-format");
    let format_session = format_adapter.create(&state).expect("format session");
    let format_writer = format_adapter
        .open_existing(format_session.session_id(), SqliteOpenMode::Writable)
        .expect("format writer");
    format_adapter
        .set_format_version_for_test(&format_writer, 2)
        .expect("set newer format");
    format_adapter
        .close(format_writer)
        .expect("close format writer");
    for mode in [SqliteOpenMode::ReadOnly, SqliteOpenMode::Writable] {
        assert_eq!(
            format_adapter
                .open_existing(format_session.session_id(), mode)
                .expect_err("newer format refused")
                .code,
            "unsupported-newer-format"
        );
    }
}

#[test]
fn duplication_is_independent_and_uses_windows_safe_physical_addressing() {
    let adapter = new_adapter("duplication-addressing");
    let original = build_original_for_duplication_fixture();
    let session = adapter.create(&original).expect("create original");
    let mut source = adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("source writer");
    let duplicate_id = "session:windows:?*<>|";
    let duplicate = adapter
        .duplicate(&mut source, duplicate_id)
        .expect("duplicate latest relational authority");
    assert_eq!(duplicate.session_id(), duplicate_id);
    let leaf = duplicate
        .storage_path_for_test()
        .file_name()
        .and_then(|name| name.to_str())
        .expect("physical key");
    assert!(leaf.starts_with("vp-session-v1-"));
    assert!(!leaf.contains('?'));
    assert!(!leaf.contains('*'));

    let expected_duplicate = {
        let mut value = original.clone();
        value.duplicated_from_session_id = Some(original.session_id.clone());
        value.session_id = duplicate_id.to_owned();
        value.durable_command_tokens.evidence_writer_token =
            "writer-encoded-session-id-v1:73657373696f6e3a77696e646f77733a3f2a3c3e7c".to_owned();
        vox_proof::persistence_evidence::current_contract::finalize_derived_fields(&mut value);
        value.normalize()
    };
    let duplicate_read = adapter
        .open_existing(duplicate.session_id(), SqliteOpenMode::ReadOnly)
        .expect("duplicate read");
    assert!(
        CurrentContractOracle::compare(&expected_duplicate, duplicate_read.normalized_state())
            .passed
    );
    adapter.close(duplicate_read).expect("close duplicate read");

    let source_next = updated_state(original.clone(), "writer:source-after-duplicate");
    let source_preconditions = source.authoritative_preconditions();
    adapter
        .apply_authoritative_transition(&mut source, &source_preconditions, &source_next)
        .expect("mutate original after duplicate");
    let duplicate_after = adapter
        .open_existing(duplicate.session_id(), SqliteOpenMode::ReadOnly)
        .expect("duplicate remains independently readable");
    assert!(
        CurrentContractOracle::compare(&expected_duplicate, duplicate_after.normalized_state())
            .passed
    );
}

#[test]
fn interrupted_transitions_preserve_only_the_committed_authority_boundary() {
    let adapter = new_adapter("fault-boundaries");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let mut writer = adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("writer");
    let next = updated_state(state.clone(), "writer:before-commit-fault");
    adapter.arm_fail_before_commit_for_test();
    let before_fault_preconditions = writer.authoritative_preconditions();
    assert_eq!(
        adapter
            .apply_authoritative_transition(&mut writer, &before_fault_preconditions, &next,)
            .expect_err("pre-commit fault returns no acknowledgement")
            .code,
        "injected-interrupted-transition"
    );
    let before_commit = adapter
        .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
        .expect("read last committed state");
    assert!(CurrentContractOracle::compare(&state, before_commit.normalized_state()).passed);
    adapter.close(before_commit).expect("close reader");

    let committed_without_ack = updated_state(state.clone(), "writer:after-commit-fault");
    adapter.arm_fail_after_commit_before_ack_for_test();
    let after_fault_preconditions = writer.authoritative_preconditions();
    assert_eq!(
        adapter
            .apply_authoritative_transition(
                &mut writer,
                &after_fault_preconditions,
                &committed_without_ack,
            )
            .expect_err("after-commit fault returns no acknowledgement")
            .code,
        "injected-after-commit-before-ack"
    );
    let recovered = adapter
        .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
        .expect("committed state remains reconstructible");
    assert!(
        CurrentContractOracle::compare(&committed_without_ack, recovered.normalized_state()).passed
    );
}

#[test]
fn hostile_bounds_are_refused_without_replacing_last_committed_authority() {
    let adapter = new_adapter("hostile-bounds");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let mut writer = adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("writer");
    let oversized = updated_state(state.clone(), &"x".repeat(2 * 1024 * 1024));
    let oversized_preconditions = writer.authoritative_preconditions();
    assert_eq!(
        adapter
            .apply_authoritative_transition(&mut writer, &oversized_preconditions, &oversized,)
            .expect_err("oversized outbound authority refused")
            .code,
        "canonical-payload-too-large"
    );
    let reader = adapter
        .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
        .expect("last committed authority survives");
    assert!(CurrentContractOracle::compare(&state, reader.normalized_state()).passed);
    adapter.close(reader).expect("close reader");
    adapter
        .inject_oversized_derived_cache_for_test(&writer)
        .expect("inject hostile cache");
    adapter.close(writer).expect("close writer before reopen");
    let bounded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        adapter.open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
    }));
    assert!(bounded.is_ok(), "hostile stored payload must not panic");
    let reopened = bounded
        .expect("no panic")
        .expect("read-only reopen ignores invalid derived cache");
    assert!(CurrentContractOracle::compare(&state, reopened.normalized_state()).passed);
    adapter.close(reopened).expect("close read-only reopen");
    let rebuilt = adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("writable reopen rebuilds derived cache from canonical authority");
    assert!(CurrentContractOracle::compare(&state, rebuilt.normalized_state()).passed);
    adapter.close(rebuilt).expect("close rebuilt writer");
}

fn copy_sqlite_authority_leaves(from: &std::path::Path, to: &std::path::Path) {
    for leaf in [
        "current-contract-v3.sqlite",
        "current-contract-v3.sqlite-wal",
        "current-contract-v3.sqlite-shm",
    ] {
        let source = from.join(leaf);
        if source.exists() {
            std::fs::copy(&source, to.join(leaf)).expect("copy authority leaf");
        }
    }
}

#[test]
fn relocated_database_fails_closed_on_semantic_session_identity_mismatch() {
    let adapter = new_adapter("session-identity-binding");
    let state_a = build_golden_small_state();
    let state_b = build_candidate_rejected_state();
    let session_a = adapter.create(&state_a).expect("create session A");
    let session_b = adapter.create(&state_b).expect("create session B");
    let path_a = session_a.storage_path_for_test().to_path_buf();
    let path_b = session_b.storage_path_for_test().to_path_buf();
    let backup_b = path_b.with_extension("backup-before-copy");
    std::fs::create_dir_all(&backup_b).expect("create backup dir");
    copy_sqlite_authority_leaves(&path_b, &backup_b);
    copy_sqlite_authority_leaves(&path_a, &path_b);
    assert_eq!(
        adapter
            .open_existing(session_b.session_id(), SqliteOpenMode::ReadOnly)
            .expect_err("read-only open rejects foreign persisted identity")
            .code,
        "session-identity-mismatch"
    );
    assert_eq!(
        adapter
            .open_existing(session_b.session_id(), SqliteOpenMode::Writable)
            .expect_err("writable open rejects foreign persisted identity before writer lease")
            .code,
        "session-identity-mismatch"
    );
    assert_eq!(
        adapter
            .open_existing(session_b.session_id(), SqliteOpenMode::Writable)
            .expect_err("failed writable open must not acquire writer ownership")
            .code,
        "session-identity-mismatch"
    );
    let reopened_a = adapter
        .open_existing(session_a.session_id(), SqliteOpenMode::ReadOnly)
        .expect("source session remains intact");
    assert!(CurrentContractOracle::compare(&state_a, reopened_a.normalized_state()).passed);
    assert_ne!(
        reopened_a.normalized_state().session_id,
        session_b.session_id(),
        "foreign session state must not be exposed under the requested identity"
    );
    adapter.close(reopened_a).expect("close source session");
    copy_sqlite_authority_leaves(&backup_b, &path_b);
    let restored_b = adapter
        .open_existing(session_b.session_id(), SqliteOpenMode::ReadOnly)
        .expect("restored session B authority remains under its semantic identity");
    assert!(CurrentContractOracle::compare(&state_b, restored_b.normalized_state()).passed);
    adapter.close(restored_b).expect("close session B");
    let _ = std::fs::remove_dir_all(&backup_b);
}

#[cfg(unix)]
#[test]
fn static_database_aliases_and_hard_links_fail_closed() {
    use std::os::unix::fs::symlink;

    let adapter = new_adapter("static-alias");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let database = session
        .storage_path_for_test()
        .join("current-contract-v3.sqlite");
    let hard_link = session
        .storage_path_for_test()
        .join("current-contract-v3.sqlite.hard-link");
    std::fs::hard_link(&database, &hard_link).expect("create hard link");
    assert_eq!(
        adapter
            .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
            .expect_err("hard-linked authority leaf rejected")
            .code,
        "authority-leaf-hard-linked"
    );
    std::fs::remove_file(&hard_link).expect("remove test hard link");

    let real_database = session
        .storage_path_for_test()
        .join("current-contract-v3.sqlite.real");
    std::fs::rename(&database, &real_database).expect("move authority leaf");
    symlink(&real_database, &database).expect("create static alias");
    assert_eq!(
        adapter
            .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
            .expect_err("symlinked authority leaf rejected")
            .code,
        "aliased-sqlite-database"
    );
    std::fs::remove_file(&database).expect("remove database alias");
    std::fs::rename(&real_database, &database).expect("restore database");

    for (leaf, expected_code) in [
        ("current-contract-v3.sqlite-wal", "aliased-sqlite-wal"),
        ("current-contract-v3.sqlite-shm", "aliased-sqlite-shm"),
    ] {
        let alias = session.storage_path_for_test().join(leaf);
        let target = session
            .storage_path_for_test()
            .join(format!("{leaf}.target"));
        std::fs::write(&target, b"not-authority").expect("write alias target");
        symlink(&target, &alias).expect("create static alias");
        assert_eq!(
            adapter
                .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
                .expect_err("aliased auxiliary authority leaf rejected")
                .code,
            expected_code
        );
        std::fs::remove_file(&alias).expect("remove alias");
    }
}

#[cfg(windows)]
fn create_windows_junction(link: &std::path::Path, target: &std::path::Path) {
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .expect("start mklink junction command");
    assert!(
        output.status.success(),
        "mklink /J failed for {} -> {}: {}",
        link.display(),
        target.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(windows)]
#[test]
fn static_windows_database_aliases_and_hard_links_fail_closed() {
    let adapter = new_adapter("windows-static-alias");
    let state = build_golden_small_state();
    let session = adapter.create(&state).expect("create");
    let database = session
        .storage_path_for_test()
        .join("current-contract-v3.sqlite");
    let hard_link = session
        .storage_path_for_test()
        .join("current-contract-v3.sqlite.hard-link");
    std::fs::hard_link(&database, &hard_link).expect("create hard link");
    assert_eq!(
        adapter
            .open_existing(session.session_id(), SqliteOpenMode::ReadOnly)
            .expect_err("hard-linked authority leaf rejected")
            .code,
        "authority-leaf-hard-linked"
    );
    std::fs::remove_file(&hard_link).expect("remove test hard link");

    let session_adapter = new_adapter("windows-reparse-session");
    let junction_session = session_adapter.create(&state).expect("create session");
    let external_adapter = new_adapter("windows-reparse-external");
    let external_session = external_adapter
        .create(&state)
        .expect("create external session");
    let session_path = junction_session.storage_path_for_test().to_path_buf();
    std::fs::remove_dir_all(&session_path).expect("remove session before junction fixture");
    create_windows_junction(&session_path, external_session.storage_path_for_test());
    assert_eq!(
        session_adapter
            .open_existing(junction_session.session_id(), SqliteOpenMode::ReadOnly)
            .expect_err("junction session root rejected")
            .code,
        "aliased-session-path"
    );

    let leaf_adapter = new_adapter("windows-reparse-leaves");
    let leaf_session = leaf_adapter.create(&state).expect("create leaf session");
    let session_root = leaf_session.storage_path_for_test().to_path_buf();
    let reparse_target = session_root.join("reparse-target");
    std::fs::create_dir(&reparse_target).expect("create reparse target");
    let database = session_root.join("current-contract-v3.sqlite");
    let database_backup = session_root.join("current-contract-v3.sqlite.real");
    std::fs::rename(&database, &database_backup).expect("move authority leaf");
    create_windows_junction(&database, &reparse_target);
    assert_eq!(
        leaf_adapter
            .open_existing(leaf_session.session_id(), SqliteOpenMode::ReadOnly)
            .expect_err("junction authority database rejected")
            .code,
        "aliased-sqlite-database"
    );
    std::fs::remove_dir(&database).expect("remove database junction");
    std::fs::rename(&database_backup, &database).expect("restore database");

    for (leaf, expected_code) in [
        ("current-contract-v3.sqlite-wal", "aliased-sqlite-wal"),
        ("current-contract-v3.sqlite-shm", "aliased-sqlite-shm"),
    ] {
        let path = session_root.join(leaf);
        if !path.exists() {
            std::fs::write(&path, b"not-authority").expect("create auxiliary leaf");
        }
        let backup = session_root.join(format!("{leaf}.real"));
        std::fs::rename(&path, &backup).expect("move auxiliary leaf");
        create_windows_junction(&path, &reparse_target);
        assert_eq!(
            leaf_adapter
                .open_existing(leaf_session.session_id(), SqliteOpenMode::ReadOnly)
                .expect_err("junction auxiliary authority leaf rejected")
                .code,
            expected_code
        );
        std::fs::remove_dir(&path).expect("remove auxiliary junction");
        std::fs::rename(&backup, &path).expect("restore auxiliary leaf");
    }
}
