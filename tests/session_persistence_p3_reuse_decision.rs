use std::fs;

use tempfile::TempDir;
use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewItemKind, ApplicationReviewTarget,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority, DeclaredSessionOperatorRole,
};
use vox_proof::candidate::{Evidence, SessionTermEntry};
use vox_proof::project_memory::{ProductProjectMemoryStore, ProjectMemoryOpenMode};
use vox_proof::reuse_proposal_target::{
    FROZEN_PROJECT_REUSE_ANALYSIS_IDENTITY_TAG_PREFIX, REUSE_PROPOSAL_TARGET_IDENTITY_TAG_PREFIX,
};
use vox_proof::review::{CorrectionDecision, ReviewCaseStatus};
use vox_proof::session_persistence::{
    AuthorityScope, DurableApplicationSession, OpenMode, ProductSessionStore,
    SessionPersistenceError, arm_fail_after_target_before_ledger_for_test,
    disarm_fail_after_target_before_ledger_for_test,
};
use vox_proof::srt::parse_srt;

fn material_use() -> ApplicationMaterialUseDeclaration {
    ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned)
}

fn session_authority(label: &str) -> DeclaredSessionAuthority {
    DeclaredSessionAuthority::new(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        label,
    )
    .expect("authority")
}

fn alias_entry(canonical: &str, alias: &str) -> SessionTermEntry {
    SessionTermEntry::new(canonical, vec![alias.to_string()], Vec::new())
}

fn kafak_transcript() -> vox_proof::transcript::Transcript {
    parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("transcript")
}

fn postgres_transcript() -> vox_proof::transcript::Transcript {
    parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgres").expect("transcript")
}

fn two_cue_transcript() -> vox_proof::transcript::Transcript {
    parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nPostgres\n\n2\n00:00:01,000 --> 00:00:02,000\nQafka",
    )
    .expect("transcript")
}

fn kafka_terms() -> Vec<SessionTermEntry> {
    vec![alias_entry("Kafka", "Kafak")]
}

fn postgres_terms() -> Vec<SessionTermEntry> {
    vec![alias_entry("PostgreSQL", "Postgres")]
}

fn current_srt(durable: &DurableApplicationSession) -> String {
    durable
        .session()
        .derive_current_projection()
        .expect("projection")
        .srt
}

fn freeze_reuse(durable: &mut DurableApplicationSession) {
    let (prepared, token) = durable
        .prepare_run_reuse_enabled_review()
        .expect("prepare freeze");
    assert!(
        prepared
            .expected_selection_token
            .starts_with(FROZEN_PROJECT_REUSE_ANALYSIS_IDENTITY_TAG_PREFIX)
    );
    durable
        .record_run_reuse_enabled_review(prepared, token)
        .expect("record freeze");
}

fn promote_postgres_to_postgresql(
    session_store: &ProductSessionStore,
    project_store: &ProductProjectMemoryStore,
    project_id: &vox_proof::reuse_primitives::ProjectScopeId,
) {
    let mut session_a = DurableApplicationSession::create_bound_to_project(
        session_store,
        project_store,
        project_id,
        postgres_transcript(),
        postgres_terms(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    let target = session_a.session().review_items()[0].target;
    let prepared = session_a
        .prepare_manual_replacement(target, "PostgreSQL")
        .expect("prepare MR");
    session_a
        .record_manual_replacement(prepared)
        .expect("record MR");
    let candidate = session_a.session().reuse_candidates().expect("candidates")[0].clone();
    let prepared = session_a
        .prepare_accept_reuse_candidate(&candidate.key)
        .expect("prepare accept");
    session_a
        .record_accept_reuse_candidate(prepared)
        .expect("accept");
    session_a.close().expect("close a");
}

fn reuse_item(
    durable: &DurableApplicationSession,
) -> vox_proof::application_service::ApplicationReviewItem {
    durable
        .session()
        .review_items()
        .into_iter()
        .find(|item| {
            matches!(
                item.kind,
                ApplicationReviewItemKind::ProjectReuseProposal { .. }
            )
        })
        .expect("reuse item")
}

fn bind_b(
    session_store: &ProductSessionStore,
    project_store: &ProductProjectMemoryStore,
    project_id: &vox_proof::reuse_primitives::ProjectScopeId,
    transcript: vox_proof::transcript::Transcript,
    terms: Vec<SessionTermEntry>,
) -> DurableApplicationSession {
    let mut session_b = DurableApplicationSession::create_bound_to_project(
        session_store,
        project_store,
        project_id,
        transcript,
        terms,
        material_use(),
        session_authority("operator-b"),
    )
    .expect("session b");
    freeze_reuse(&mut session_b);
    session_b
}

#[test]
fn a_derivation_exposes_proposal_without_authority_or_output_change() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    promote_postgres_to_postgresql(&session_store, &project_store, &project_id);

    let session_b = bind_b(
        &session_store,
        &project_store,
        &project_id,
        postgres_transcript(),
        Vec::new(),
    );
    assert!(session_b.session().compose_project_reuse_proposals());
    let item = reuse_item(&session_b);
    assert!(matches!(
        item.target,
        ApplicationReviewTarget::ProjectReuseProposal { .. }
    ));
    assert_eq!(item.status, ReviewCaseStatus::Undecided);
    assert!(
        item.target
            .reuse_proposal_identity()
            .expect("identity")
            .to_tagged_string()
            .starts_with(REUSE_PROPOSAL_TARGET_IDENTITY_TAG_PREFIX)
    );
    assert!(
        session_b
            .session()
            .persisted_reuse_proposal_targets()
            .is_empty()
    );
    assert!(session_b.session().review_ledger().events().is_empty());
    assert!(current_srt(&session_b).contains("Postgres"));
    assert!(!current_srt(&session_b).contains("PostgreSQL"));
    assert!(
        session_b
            .session()
            .review_items()
            .iter()
            .filter(|item| matches!(
                item.target,
                ApplicationReviewTarget::CanonicalTermCase { .. }
            ))
            .all(|item| !matches!(
                item.review_case.candidate_span().evidence(),
                Evidence::ReusableExactObservedForm(_)
            ))
    );
    session_b.close().expect("close b");
}

#[test]
fn b_accept_persists_thin_target_atomically_and_changes_output_after_commit() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    promote_postgres_to_postgresql(&session_store, &project_store, &project_id);

    let mut session_b = bind_b(
        &session_store,
        &project_store,
        &project_id,
        postgres_transcript(),
        Vec::new(),
    );
    assert!(current_srt(&session_b).contains("Postgres"));
    let target = reuse_item(&session_b).target;
    let prepared = session_b
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare accept");
    assert!(prepared.reuse_proposal_target.is_some());
    session_b
        .record_human_decision(prepared)
        .expect("record accept");
    assert_eq!(
        session_b.session().persisted_reuse_proposal_targets().len(),
        1
    );
    assert!(current_srt(&session_b).contains("PostgreSQL"));
    assert!(!current_srt(&session_b).contains("Postgres"));
    session_b.close().expect("close b");
}

#[test]
fn c_reject_keeps_source_and_survives_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    promote_postgres_to_postgresql(&session_store, &project_store, &project_id);

    let mut session_b = bind_b(
        &session_store,
        &project_store,
        &project_id,
        postgres_transcript(),
        Vec::new(),
    );
    let target = reuse_item(&session_b).target;
    let identity = target.reuse_proposal_identity().expect("identity");
    let prepared = session_b
        .prepare_human_decision(target, CorrectionDecision::Reject)
        .expect("prepare reject");
    session_b
        .record_human_decision(prepared)
        .expect("record reject");
    assert!(current_srt(&session_b).contains("Postgres"));
    let session_id = session_b.session_id().to_owned();
    session_b.close().expect("close b");

    let reopened = DurableApplicationSession::open(&session_store, &session_id, OpenMode::ReadOnly)
        .expect("reopen");
    assert!(current_srt(&reopened).contains("Postgres"));
    let item = reuse_item(&reopened);
    assert_eq!(item.target.reuse_proposal_identity(), Some(identity));
    assert!(matches!(
        item.status,
        ReviewCaseStatus::Decided {
            decision: CorrectionDecision::Reject,
            ..
        }
    ));
    reopened.close().expect("close reopen");
}

#[test]
fn d_manual_replacement_writes_z_without_promoting_into_project() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    promote_postgres_to_postgresql(&session_store, &project_store, &project_id);

    let mut session_b = bind_b(
        &session_store,
        &project_store,
        &project_id,
        postgres_transcript(),
        Vec::new(),
    );
    let target = reuse_item(&session_b).target;
    let prepared = session_b
        .prepare_manual_replacement(target, "KRaft")
        .expect("prepare MR");
    session_b
        .record_manual_replacement(prepared)
        .expect("record MR");
    assert!(current_srt(&session_b).contains("KRaft"));
    session_b.close().expect("close b");

    let reopened_project = project_store
        .open(&project_id, ProjectMemoryOpenMode::ReadOnly)
        .expect("project reopen");
    assert_eq!(reopened_project.records().len(), 1);
    match &reopened_project.records()[0].event {
        vox_proof::reusable_influence::ReusableGovernanceEvent::PromotionAccepted {
            payload,
            ..
        } => {
            assert_eq!(payload.observed_text, "Postgres");
            assert_eq!(payload.confirmed_replacement, "PostgreSQL");
        }
        other => panic!("expected promotion, got {other:?}"),
    }
    reopened_project.close().expect("close project");
}

#[test]
fn e_reopen_reconstructs_target_from_frozen_boundary() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    promote_postgres_to_postgresql(&session_store, &project_store, &project_id);

    let mut session_b = bind_b(
        &session_store,
        &project_store,
        &project_id,
        postgres_transcript(),
        Vec::new(),
    );
    let before = reuse_item(&session_b);
    let identity = before.target.reuse_proposal_identity().expect("identity");
    let freeze = session_b
        .session()
        .frozen_project_reuse_analysis()
        .expect("freeze")
        .clone();
    let prepared = session_b
        .prepare_human_decision(
            before.target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare accept");
    session_b
        .record_human_decision(prepared)
        .expect("record accept");
    let session_id = session_b.session_id().to_owned();
    session_b.close().expect("close b");

    let reopened = DurableApplicationSession::open(&session_store, &session_id, OpenMode::ReadOnly)
        .expect("reopen");
    let after = reuse_item(&reopened);
    assert_eq!(after.target.reuse_proposal_identity(), Some(identity));
    assert_eq!(
        reopened
            .session()
            .frozen_project_reuse_analysis()
            .expect("reopened freeze")
            .freeze_identity,
        freeze.freeze_identity
    );
    assert!(current_srt(&reopened).contains("PostgreSQL"));
    reopened.close().expect("close reopen");
}

#[test]
fn f_project_growth_keeps_committed_decision_and_refuses_stale_reuse() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    promote_postgres_to_postgresql(&session_store, &project_store, &project_id);

    let mut session_b = bind_b(
        &session_store,
        &project_store,
        &project_id,
        two_cue_transcript(),
        vec![alias_entry("Quark", "Qafka")],
    );
    let reuse_target = reuse_item(&session_b).target;
    let prepared = session_b
        .prepare_human_decision(
            reuse_target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare accept");
    session_b
        .record_human_decision(prepared)
        .expect("record accept");
    let canonical_target = session_b
        .session()
        .review_items()
        .into_iter()
        .find(|item| {
            matches!(
                item.target,
                ApplicationReviewTarget::CanonicalTermCase { .. }
            )
        })
        .expect("canonical")
        .target;
    let session_id = session_b.session_id().to_owned();
    session_b.close().expect("close b");

    let mut session_c = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nZookeeper").expect("c transcript"),
        vec![alias_entry("ZooKeeper", "Zookeeper")],
        material_use(),
        session_authority("operator-c"),
    )
    .expect("session c");
    let target = session_c.session().review_items()[0].target;
    let prepared = session_c
        .prepare_manual_replacement(target, "ZooKeeper")
        .expect("prepare MR");
    session_c
        .record_manual_replacement(prepared)
        .expect("record MR");
    let candidate = session_c.session().reuse_candidates().expect("candidates")[0].clone();
    let prepared = session_c
        .prepare_accept_reuse_candidate(&candidate.key)
        .expect("prepare promote");
    session_c
        .record_accept_reuse_candidate(prepared)
        .expect("promote");
    session_c.close().expect("close c");

    let mut reopened =
        DurableApplicationSession::open(&session_store, &session_id, OpenMode::Writable)
            .expect("reopen b");
    assert!(current_srt(&reopened).contains("PostgreSQL"));
    let stale_target = reuse_item(&reopened).target;
    let stale = reopened.prepare_human_decision(stale_target, CorrectionDecision::Reject);
    assert!(matches!(
        stale,
        Err(SessionPersistenceError::StaleAuthorityPrecondition(
            vox_proof::session_persistence::StaleAuthorityPrecondition {
                scope: AuthorityScope::ActiveAnalysis,
            }
        ))
    ));
    let prepared = reopened
        .prepare_human_decision(canonical_target, CorrectionDecision::Reject)
        .expect("canonical still valid");
    reopened
        .record_human_decision(prepared)
        .expect("record canonical");
    reopened.close().expect("close reopened");
}

#[test]
fn g_missing_project_after_commit_keeps_output_and_blocks_new_reuse() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    promote_postgres_to_postgresql(&session_store, &project_store, &project_id);

    let mut session_b = bind_b(
        &session_store,
        &project_store,
        &project_id,
        postgres_transcript(),
        Vec::new(),
    );
    let target = reuse_item(&session_b).target;
    let prepared = session_b
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare accept");
    session_b
        .record_human_decision(prepared)
        .expect("record accept");
    let session_id = session_b.session_id().to_owned();
    session_b.close().expect("close b");

    fs::remove_dir_all(
        project_store
            .database_path(project_id.as_str())
            .parent()
            .expect("parent"),
    )
    .expect("remove project");

    let reopened = DurableApplicationSession::open(&session_store, &session_id, OpenMode::ReadOnly)
        .expect("reopen");
    assert!(!reopened.project_memory_available());
    assert!(current_srt(&reopened).contains("PostgreSQL"));
    assert!(!reopened.session().compose_project_reuse_proposals());
    let target = reuse_item(&reopened).target;
    reopened.close().expect("close readonly");

    let writable = DurableApplicationSession::open(&session_store, &session_id, OpenMode::Writable)
        .expect("writable reopen");
    let blocked = writable.prepare_human_decision(target, CorrectionDecision::Reject);
    assert_eq!(blocked, Err(SessionPersistenceError::WritableReuseBlocked));
    writable.close().expect("close writable");
}

#[test]
fn h_missing_project_before_decision_cannot_authorize_proposal() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    promote_postgres_to_postgresql(&session_store, &project_store, &project_id);

    let session_b = bind_b(
        &session_store,
        &project_store,
        &project_id,
        postgres_transcript(),
        Vec::new(),
    );
    let target = reuse_item(&session_b).target;
    let session_id = session_b.session_id().to_owned();
    session_b.close().expect("close b");
    fs::remove_dir_all(
        project_store
            .database_path(project_id.as_str())
            .parent()
            .expect("parent"),
    )
    .expect("remove project");

    let reopened = DurableApplicationSession::open(&session_store, &session_id, OpenMode::Writable)
        .expect("reopen");
    assert_eq!(
        reopened.prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0
            }
        ),
        Err(SessionPersistenceError::WritableReuseBlocked)
    );
    assert!(
        reopened
            .session()
            .persisted_reuse_proposal_targets()
            .is_empty()
    );
    reopened.close().expect("close");
}

#[test]
fn i_same_replacement_collision_keeps_canonical_target_and_is_not_gate7() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    promote_postgres_to_postgresql(&session_store, &project_store, &project_id);

    let session_b = bind_b(
        &session_store,
        &project_store,
        &project_id,
        postgres_transcript(),
        postgres_terms(),
    );
    let items = session_b.session().review_items();
    let canonical: Vec<_> = items
        .iter()
        .filter(|item| {
            matches!(
                item.target,
                ApplicationReviewTarget::CanonicalTermCase { .. }
            )
        })
        .collect();
    let reuse: Vec<_> = items
        .iter()
        .filter(|item| {
            matches!(
                item.kind,
                ApplicationReviewItemKind::ProjectReuseProposal { .. }
            )
        })
        .collect();
    assert_eq!(canonical.len(), 1);
    assert!(reuse.is_empty());
    assert_eq!(
        canonical[0].kind,
        ApplicationReviewItemKind::CanonicalTermCase {
            gate7_repeated_correction_avoided_eligible: false,
        }
    );
    session_b.close().expect("close");
}

#[test]
fn j_different_replacement_collision_does_not_choose_automatically() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    promote_postgres_to_postgresql(&session_store, &project_store, &project_id);

    let session_b = bind_b(
        &session_store,
        &project_store,
        &project_id,
        postgres_transcript(),
        vec![alias_entry("Cafka", "Postgres")],
    );
    let items = session_b.session().review_items();
    let canonical = items.iter().find(|item| {
        matches!(
            item.target,
            ApplicationReviewTarget::CanonicalTermCase { .. }
        )
    });
    let reuse = items.iter().find(|item| {
        matches!(
            item.kind,
            ApplicationReviewItemKind::ProjectReuseProposal { .. }
        )
    });
    assert!(canonical.is_some());
    let reuse = reuse.expect("conflict reuse item");
    match reuse.kind {
        ApplicationReviewItemKind::ProjectReuseProposal {
            conflict_with_canonical,
            gate7_repeated_correction_avoided_eligible,
        } => {
            assert!(conflict_with_canonical);
            assert!(!gate7_repeated_correction_avoided_eligible);
        }
        other => panic!("expected reuse kind, got {other:?}"),
    }
    assert!(current_srt(&session_b).contains("Postgres"));
    session_b.close().expect("close");
}

#[test]
fn k_target_without_decision_rolls_back() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    promote_postgres_to_postgresql(&session_store, &project_store, &project_id);

    let mut session_b = bind_b(
        &session_store,
        &project_store,
        &project_id,
        postgres_transcript(),
        Vec::new(),
    );
    let target = reuse_item(&session_b).target;
    let prepared = session_b
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare accept");
    arm_fail_after_target_before_ledger_for_test();
    let failed = session_b.record_human_decision(prepared);
    disarm_fail_after_target_before_ledger_for_test();
    assert!(failed.is_err());
    let session_id = session_b.session_id().to_owned();
    session_b.close().expect("close b");

    let reopened = DurableApplicationSession::open(&session_store, &session_id, OpenMode::ReadOnly)
        .expect("reopen");
    assert!(
        reopened
            .session()
            .persisted_reuse_proposal_targets()
            .is_empty()
    );
    assert!(reopened.session().review_ledger().events().is_empty());
    assert!(current_srt(&reopened).contains("Postgres"));
    reopened.close().expect("close reopen");
}

#[test]
fn l_v1_create_and_canonical_decision_remain_available() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        kafak_transcript(),
        kafka_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("v1 create");
    assert_eq!(durable.format_version(), 4);
    assert!(!durable.session().compose_project_reuse_proposals());
    let target = durable.session().review_items()[0].target;
    assert!(matches!(
        target,
        ApplicationReviewTarget::CanonicalTermCase { .. }
    ));
    let prepared = durable
        .prepare_manual_replacement(target, "Kafka")
        .expect("prepare");
    durable.record_manual_replacement(prepared).expect("record");
    assert!(current_srt(&durable).contains("Kafka"));
    durable.close().expect("close");
}
