use std::fs;

use tempfile::TempDir;
use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewItemKind, ApplicationReviewTarget,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority, DeclaredSessionOperatorRole,
};
use vox_proof::project_memory::{
    compute_project_memory_snapshot_identity, ProductProjectMemoryStore, ProjectMemoryOpenMode,
    ProjectMemoryRecord, PROJECT_MEMORY_FORMAT_VERSION, PROJECT_MEMORY_FORMAT_VERSION_V3,
};
use vox_proof::project_terminology::{
    PROJECT_DERIVED_TERMINOLOGY_ANALYSIS_IDENTITY_TAG_PREFIX,
    PROJECT_TERMINOLOGY_PROPOSAL_TARGET_IDENTITY_TAG_PREFIX,
};
use vox_proof::reusable_influence::{
    AllowedEffectsConsent, ReusableGovernanceEvent, ReuseAllowedEffect,
};
use vox_proof::review::{CorrectionDecision, ReviewLedgerEvent};
use vox_proof::session_persistence::{
    arm_fail_after_target_before_ledger_for_test, disarm_fail_after_target_before_ledger_for_test,
    AuthorityScope, DurableApplicationSession, OpenMode, ProductSessionStore,
    SessionPersistenceError, StaleAuthorityPrecondition,
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

fn parse(text: &str) -> vox_proof::transcript::Transcript {
    parse_srt(&format!("1\n00:00:00,000 --> 00:00:01,000\n{text}")).expect("transcript")
}

fn freeze_bound(durable: &mut DurableApplicationSession) {
    let (prepared, precondition) = durable
        .prepare_run_reuse_enabled_review()
        .expect("prepare freeze");
    durable
        .record_run_reuse_enabled_review(prepared, precondition)
        .expect("freeze");
}

fn current_srt(durable: &DurableApplicationSession) -> String {
    durable
        .session()
        .derive_current_projection()
        .expect("projection")
        .srt
}

fn session_db(root: &std::path::Path, session_id: &str) -> std::path::PathBuf {
    root.join(session_id).join("session.db")
}

fn table_exists(db_path: &std::path::Path, name: &str) -> bool {
    let connection = rusqlite::Connection::open(db_path).expect("open db");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get(0),
        )
        .expect("sqlite_master");
    count > 0
}

fn session_terms_json(db_path: &std::path::Path) -> String {
    let connection = rusqlite::Connection::open(db_path).expect("open db");
    connection
        .query_row("SELECT terms_json FROM session_terms", [], |row| row.get(0))
        .expect("terms_json")
}

fn promote_human_raised(
    durable: &mut DurableApplicationSession,
    start_byte: usize,
    end_byte: usize,
    replacement: &str,
) {
    freeze_bound(durable);
    durable
        .raise_and_manual_replace(0, start_byte, end_byte, replacement)
        .expect("raise");
    let candidate = durable
        .session()
        .reuse_candidates()
        .expect("candidates")
        .into_iter()
        .find(|candidate| candidate.key.source_locator.is_human_raised())
        .expect("human candidate");
    let prepared = durable
        .prepare_accept_reuse_candidate(&candidate.key)
        .expect("prepare accept");
    durable
        .record_accept_reuse_candidate(prepared)
        .expect("accept");
}

fn sample_promotion(effects: AllowedEffectsConsent) -> ReusableGovernanceEvent {
    let mut session = vox_proof::application_service::begin_application_review(
        parse("Kafak"),
        vec![vox_proof::candidate::SessionTermEntry::new(
            "Kafka",
            vec!["Kafak".to_string()],
            Vec::new(),
        )],
        material_use(),
        session_authority("operator"),
    )
    .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("manual replacement");
    session
        .initialize_project_scope("placeholder", "Placeholder")
        .expect("scope");
    let candidate = session.reuse_candidates().expect("candidates")[0].clone();
    let scope = session
        .reuse_state()
        .project_scope()
        .expect("scope")
        .clone();
    let mut event = vox_proof::application_reuse::build_promotion_accepted_event(
        &candidate,
        &scope,
        session.session_authority(),
    );
    let ReusableGovernanceEvent::PromotionAccepted {
        allowed_effects, ..
    } = &mut event
    else {
        panic!("expected promotion");
    };
    *allowed_effects = effects;
    event
}

#[test]
fn new_sessions_are_format_v4() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let durable = DurableApplicationSession::create(
        &store,
        parse("Hello"),
        Vec::new(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    assert_eq!(durable.format_version(), 4);
    let session_id = durable.session_id().to_owned();
    let db_path = session_db(temp.path(), &session_id);
    assert!(table_exists(
        &db_path,
        "project_terminology_proposal_targets"
    ));
    durable.close().expect("close");
}

#[test]
fn historical_v3_open_does_not_add_terminology_table_and_cannot_create_terminology_decisions() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgres"),
        Vec::new(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    promote_human_raised(&mut session_a, 0, 8, "PostgreSQL");
    session_a.close().expect("close a");

    let mut session_b =
        DurableApplicationSession::create_historical_bound_format_v3_for_compatibility_test(
            &session_store,
            &project_store,
            &project_id,
            parse("Postgre sequel"),
            Vec::new(),
            material_use(),
            session_authority("operator-b"),
        )
        .expect("historical v3");
    assert_eq!(session_b.format_version(), 3);
    let session_id = session_b.session_id().to_owned();
    let db_path = session_db(temp.path(), &session_id);
    assert!(
        !table_exists(&db_path, "project_terminology_proposal_targets"),
        "v3 create must not install the terminology table"
    );
    freeze_bound(&mut session_b);
    assert!(!session_b.session().compose_project_terminology_proposals());
    assert!(session_b
        .session()
        .derived_project_terminology_proposal_targets()
        .is_empty());
    assert!(session_b.session().review_items().iter().all(|item| {
        !matches!(
            item.target,
            ApplicationReviewTarget::ProjectTerminologyProposal { .. }
        )
    }));
    session_b.close().expect("close b");

    let reopened = DurableApplicationSession::open(&session_store, &session_id, OpenMode::Writable)
        .expect("reopen v3");
    assert!(
        !table_exists(&db_path, "project_terminology_proposal_targets"),
        "opening v3 must not add the terminology table"
    );
    reopened.close().expect("close reopen");
}

#[test]
fn replacement_y_selects_derived_effect_only_when_structurally_eligible() {
    let postgres = AllowedEffectsConsent::for_confirmed_replacement("PostgreSQL");
    assert!(postgres.includes(ReuseAllowedEffect::ExactObservedFormProposalGeneration));
    assert!(postgres.includes(ReuseAllowedEffect::DerivedCanonicalTerminologyProposalGeneration));

    let pronoun = AllowedEffectsConsent::for_confirmed_replacement("她");
    assert_eq!(
        pronoun.effective(),
        vec![ReuseAllowedEffect::ExactObservedFormProposalGeneration]
    );

    let numeric = AllowedEffectsConsent::for_confirmed_replacement("15");
    assert_eq!(
        numeric.effective(),
        vec![ReuseAllowedEffect::ExactObservedFormProposalGeneration]
    );
}

#[test]
fn durable_promotions_persist_explicit_effects_from_y() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());

    let eligible = project_store.create("Eligible").expect("project");
    let eligible_id = eligible.project_id().clone();
    eligible.close().expect("close");
    let mut session = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &eligible_id,
        parse("Postgres"),
        Vec::new(),
        material_use(),
        session_authority("operator"),
    )
    .expect("session");
    promote_human_raised(&mut session, 0, 8, "PostgreSQL");
    session.close().expect("close");
    let project = project_store
        .open(&eligible_id, ProjectMemoryOpenMode::ReadOnly)
        .expect("open");
    assert_eq!(project.format_version(), PROJECT_MEMORY_FORMAT_VERSION_V3);
    match &project.records()[0].event {
        ReusableGovernanceEvent::PromotionAccepted {
            allowed_effects,
            payload,
            ..
        } => {
            assert_eq!(payload.confirmed_replacement, "PostgreSQL");
            assert!(allowed_effects.is_explicit());
            assert!(allowed_effects
                .includes(ReuseAllowedEffect::DerivedCanonicalTerminologyProposalGeneration));
        }
        other => panic!("expected promotion, got {other:?}"),
    }
    project.close().expect("close");

    let pronoun = project_store.create("Pronoun").expect("project");
    let pronoun_id = pronoun.project_id().clone();
    pronoun.close().expect("close");
    let mut session = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &pronoun_id,
        parse("他"),
        Vec::new(),
        material_use(),
        session_authority("operator"),
    )
    .expect("session");
    promote_human_raised(&mut session, 0, "他".len(), "她");
    session.close().expect("close");
    let project = project_store
        .open(&pronoun_id, ProjectMemoryOpenMode::ReadOnly)
        .expect("open");
    match &project.records()[0].event {
        ReusableGovernanceEvent::PromotionAccepted {
            allowed_effects, ..
        } => {
            assert!(allowed_effects.is_explicit());
            assert!(!allowed_effects
                .includes(ReuseAllowedEffect::DerivedCanonicalTerminologyProposalGeneration));
        }
        other => panic!("expected promotion, got {other:?}"),
    }
    project.close().expect("close");

    let numeric = project_store.create("Numeric").expect("project");
    let numeric_id = numeric.project_id().clone();
    numeric.close().expect("close");
    let mut session = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &numeric_id,
        parse("50"),
        Vec::new(),
        material_use(),
        session_authority("operator"),
    )
    .expect("session");
    promote_human_raised(&mut session, 0, 2, "15");
    session.close().expect("close");
    let project = project_store
        .open(&numeric_id, ProjectMemoryOpenMode::ReadOnly)
        .expect("open");
    match &project.records()[0].event {
        ReusableGovernanceEvent::PromotionAccepted {
            allowed_effects, ..
        } => {
            assert!(allowed_effects.is_explicit());
            assert!(!allowed_effects
                .includes(ReuseAllowedEffect::DerivedCanonicalTerminologyProposalGeneration));
        }
        other => panic!("expected promotion, got {other:?}"),
    }
    project.close().expect("close");
}

#[test]
fn historical_missing_allowed_effects_remain_exact_only_after_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductProjectMemoryStore::new(temp.path());
    let mut project = store.create("Historical").expect("create");
    let project_id = project.project_id().clone();
    project
        .append_promotion(
            "session-a",
            sample_promotion(AllowedEffectsConsent::HistoricalExactOnly),
        )
        .expect("append historical");
    assert_eq!(project.format_version(), PROJECT_MEMORY_FORMAT_VERSION);
    let json: String = {
        let connection = rusqlite::Connection::open(store.database_path(project_id.as_str()))
            .expect("open project db");
        connection
            .query_row(
                "SELECT event_json FROM governance_events WHERE event_index = 0",
                [],
                |row| row.get(0),
            )
            .expect("event json")
    };
    assert!(
        !json.contains("allowed_effects"),
        "historical records must omit allowed_effects"
    );
    project.close().expect("close");

    let reopened = store
        .open(&project_id, ProjectMemoryOpenMode::ReadOnly)
        .expect("reopen");
    match &reopened.records()[0].event {
        ReusableGovernanceEvent::PromotionAccepted {
            allowed_effects, ..
        } => {
            assert_eq!(allowed_effects, &AllowedEffectsConsent::HistoricalExactOnly);
            assert!(!allowed_effects
                .includes(ReuseAllowedEffect::DerivedCanonicalTerminologyProposalGeneration));
        }
        other => panic!("expected promotion, got {other:?}"),
    }
    reopened.close().expect("close");
}

#[test]
fn project_memory_snapshot_hash_is_stable_for_format_one_and_two_and_includes_effects_at_format_three(
) {
    let project_id =
        vox_proof::reuse_primitives::ProjectScopeId::new(uuid::Uuid::new_v4().to_string())
            .expect("project id");
    let historical = vec![ProjectMemoryRecord {
        source_session_id: "session-a".to_owned(),
        event: sample_promotion(AllowedEffectsConsent::HistoricalExactOnly),
    }];
    let explicit_derived = vec![ProjectMemoryRecord {
        source_session_id: "session-a".to_owned(),
        event: sample_promotion(AllowedEffectsConsent::for_confirmed_replacement(
            "PostgreSQL",
        )),
    }];
    let explicit_exact = vec![ProjectMemoryRecord {
        source_session_id: "session-a".to_owned(),
        event: sample_promotion(AllowedEffectsConsent::exact_only_explicit()),
    }];

    let format_one_historical = compute_project_memory_snapshot_identity(
        &project_id,
        PROJECT_MEMORY_FORMAT_VERSION,
        1,
        &historical,
    );
    let format_one_derived = compute_project_memory_snapshot_identity(
        &project_id,
        PROJECT_MEMORY_FORMAT_VERSION,
        1,
        &explicit_derived,
    );
    let format_two_historical =
        compute_project_memory_snapshot_identity(&project_id, 2, 1, &historical);
    let format_two_derived =
        compute_project_memory_snapshot_identity(&project_id, 2, 1, &explicit_derived);
    assert_eq!(format_one_historical, format_one_derived);
    assert_eq!(format_two_historical, format_two_derived);
    assert_ne!(format_one_historical, format_two_historical);

    let format_three_historical = compute_project_memory_snapshot_identity(
        &project_id,
        PROJECT_MEMORY_FORMAT_VERSION_V3,
        1,
        &historical,
    );
    let format_three_derived = compute_project_memory_snapshot_identity(
        &project_id,
        PROJECT_MEMORY_FORMAT_VERSION_V3,
        1,
        &explicit_derived,
    );
    let format_three_exact = compute_project_memory_snapshot_identity(
        &project_id,
        PROJECT_MEMORY_FORMAT_VERSION_V3,
        1,
        &explicit_exact,
    );
    assert_ne!(format_three_historical, format_three_derived);
    assert_eq!(format_three_historical, format_three_exact);
    assert_ne!(format_one_historical, format_three_historical);
}

#[test]
fn progressive_terminology_proposes_postgresql_for_postgre_sequel_without_exact_reuse() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgres"),
        Vec::new(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    promote_human_raised(&mut session_a, 0, 8, "PostgreSQL");
    session_a.close().expect("close a");

    let mut session_b = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgre sequel"),
        Vec::new(),
        material_use(),
        session_authority("operator-b"),
    )
    .expect("session b");
    assert_eq!(session_b.format_version(), 4);
    freeze_bound(&mut session_b);
    assert!(session_b.session().compose_project_terminology_proposals());
    let derived = session_b
        .session()
        .derived_project_terminology_proposal_targets();
    assert_eq!(derived.len(), 1);
    assert!(derived[0]
        .identity()
        .to_tagged_string()
        .starts_with(PROJECT_TERMINOLOGY_PROPOSAL_TARGET_IDENTITY_TAG_PREFIX));
    assert!(derived[0]
        .analysis_identity()
        .to_tagged_string()
        .starts_with(PROJECT_DERIVED_TERMINOLOGY_ANALYSIS_IDENTITY_TAG_PREFIX));
    assert_eq!(derived[0].proposed_replacement(), "PostgreSQL");
    assert_eq!(derived[0].occurrence().observed_text, "Postgre sequel");

    let items = session_b.session().review_items();
    assert!(items.iter().all(|item| {
        !matches!(
            item.target,
            ApplicationReviewTarget::ProjectReuseProposal { .. }
        )
    }));
    let terminology = items
        .iter()
        .find(|item| {
            matches!(
                item.kind,
                ApplicationReviewItemKind::ProjectTerminologyProposal { .. }
            )
        })
        .expect("terminology proposal");
    assert!(current_srt(&session_b).contains("Postgre sequel"));
    assert!(!current_srt(&session_b).contains("PostgreSQL"));

    let prepared = session_b
        .prepare_human_decision(
            terminology.target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare accept");
    session_b
        .record_human_decision(prepared)
        .expect("accept terminology");
    assert!(current_srt(&session_b).contains("PostgreSQL"));
    assert!(!current_srt(&session_b).contains("Postgre sequel"));
    assert!(matches!(
        session_b.session().review_ledger().events().last(),
        Some(ReviewLedgerEvent::TerminologyProposalDecisionRecorded { .. })
    ));

    let session_id = session_b.session_id().to_owned();
    let db_path = session_db(temp.path(), &session_id);
    assert_eq!(session_terms_json(&db_path), "[]");
    session_b.close().expect("close b");

    let reopened = DurableApplicationSession::open(&session_store, &session_id, OpenMode::ReadOnly)
        .expect("reopen");
    assert!(current_srt(&reopened).contains("PostgreSQL"));
    assert!(reopened.session().compose_project_terminology_proposals());
    reopened.close().expect("close reopen");

    fs::remove_dir_all(
        project_store
            .database_path(project_id.as_str())
            .parent()
            .expect("parent"),
    )
    .expect("remove project");

    let missing_pm =
        DurableApplicationSession::open(&session_store, &session_id, OpenMode::ReadOnly)
            .expect("reopen without project");
    assert!(!missing_pm.project_memory_available());
    assert!(!missing_pm.session().compose_project_terminology_proposals());
    assert!(missing_pm
        .session()
        .derived_project_terminology_proposal_targets()
        .is_empty());
    assert!(current_srt(&missing_pm).contains("PostgreSQL"));
    let blocked_target = missing_pm
        .session()
        .review_items()
        .into_iter()
        .find(|item| {
            matches!(
                item.target,
                ApplicationReviewTarget::ProjectTerminologyProposal { .. }
            )
        })
        .map(|item| item.target);
    missing_pm.close().expect("close missing pm");

    if let Some(target) = blocked_target {
        let writable =
            DurableApplicationSession::open(&session_store, &session_id, OpenMode::Writable)
                .expect("writable");
        let blocked = writable.prepare_human_decision(target, CorrectionDecision::Reject);
        assert_eq!(blocked, Err(SessionPersistenceError::WritableReuseBlocked));
        writable.close().expect("close writable");
    }
}

#[test]
fn seeded_canonical_same_y_collapses_derived_terminology() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgres"),
        Vec::new(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    promote_human_raised(&mut session_a, 0, 8, "PostgreSQL");
    session_a.close().expect("close a");

    let mut session_b = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgre sequel"),
        vec![vox_proof::candidate::SessionTermEntry::new(
            "PostgreSQL",
            Vec::new(),
            Vec::new(),
        )],
        material_use(),
        session_authority("operator-b"),
    )
    .expect("session b");
    freeze_bound(&mut session_b);
    let items = session_b.session().review_items();
    assert!(items.iter().any(|item| {
        matches!(
            item.target,
            ApplicationReviewTarget::CanonicalTermCase { .. }
        )
    }));
    assert!(items.iter().all(|item| {
        !matches!(
            item.target,
            ApplicationReviewTarget::ProjectTerminologyProposal { .. }
        )
    }));
    session_b.close().expect("close b");
}

fn terminology_item(
    durable: &DurableApplicationSession,
) -> vox_proof::application_service::ApplicationReviewItem {
    durable
        .session()
        .review_items()
        .into_iter()
        .find(|item| {
            matches!(
                item.target,
                ApplicationReviewTarget::ProjectTerminologyProposal { .. }
            )
        })
        .expect("terminology item")
}

fn historical_postgres_exact_only_promotion() -> ReusableGovernanceEvent {
    let mut event = sample_promotion(AllowedEffectsConsent::HistoricalExactOnly);
    let ReusableGovernanceEvent::PromotionAccepted { payload, .. } = &mut event else {
        panic!("expected promotion");
    };
    payload.observed_text = "Postgres".to_owned();
    payload.confirmed_replacement = "PostgreSQL".to_owned();
    event
}

#[test]
fn historical_exact_only_does_not_derive_terminology_even_when_y_is_eligible() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let mut project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project
        .append_promotion("session-a", historical_postgres_exact_only_promotion())
        .expect("append historical");
    project.close().expect("close project");

    let mut session_b = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgre sequel"),
        Vec::new(),
        material_use(),
        session_authority("operator-b"),
    )
    .expect("session b");
    freeze_bound(&mut session_b);
    assert!(session_b
        .session()
        .derived_project_terminology_proposal_targets()
        .is_empty());
    assert!(session_b.session().review_items().iter().all(|item| {
        !matches!(
            item.target,
            ApplicationReviewTarget::ProjectTerminologyProposal { .. }
        )
    }));
    session_b.close().expect("close b");
}

#[test]
fn missing_project_before_terminology_decision_blocks_and_does_not_persist_target() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgres"),
        Vec::new(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    promote_human_raised(&mut session_a, 0, 8, "PostgreSQL");
    session_a.close().expect("close a");

    let mut session_b = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgre sequel"),
        Vec::new(),
        material_use(),
        session_authority("operator-b"),
    )
    .expect("session b");
    freeze_bound(&mut session_b);
    let target = terminology_item(&session_b).target;
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
    assert!(reopened
        .session()
        .persisted_project_terminology_proposal_targets()
        .is_empty());
    assert!(current_srt(&reopened).contains("Postgre sequel"));
    reopened.close().expect("close");
}

#[test]
fn project_growth_keeps_committed_terminology_and_refuses_stale_uncommitted_action() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgres"),
        Vec::new(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    promote_human_raised(&mut session_a, 0, 8, "PostgreSQL");
    session_a.close().expect("close a");

    let mut session_b = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgre sequel"),
        Vec::new(),
        material_use(),
        session_authority("operator-b"),
    )
    .expect("session b");
    freeze_bound(&mut session_b);
    let identity_before_growth = session_b
        .session()
        .derived_project_terminology_proposal_targets()[0]
        .identity();
    let target = terminology_item(&session_b).target;
    let prepared = session_b
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare accept");
    session_b.record_human_decision(prepared).expect("accept");
    let session_id = session_b.session_id().to_owned();
    session_b.close().expect("close b");

    let mut session_c = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Zookeeper"),
        Vec::new(),
        material_use(),
        session_authority("operator-c"),
    )
    .expect("session c");
    promote_human_raised(&mut session_c, 0, 9, "ZooKeeper");
    session_c.close().expect("close c");

    let reopened = DurableApplicationSession::open(&session_store, &session_id, OpenMode::Writable)
        .expect("reopen b");
    assert!(current_srt(&reopened).contains("PostgreSQL"));
    assert_eq!(
        reopened
            .session()
            .persisted_project_terminology_proposal_targets()[0]
            .identity(),
        identity_before_growth
    );
    let stale_target = terminology_item(&reopened).target;
    let stale = reopened.prepare_human_decision(stale_target, CorrectionDecision::Reject);
    assert!(matches!(
        stale,
        Err(SessionPersistenceError::StaleAuthorityPrecondition(
            StaleAuthorityPrecondition {
                scope: AuthorityScope::ActiveAnalysis,
            }
        ))
    ));
    reopened.close().expect("close reopened");

    let mut uncommitted = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgre sequel"),
        Vec::new(),
        material_use(),
        session_authority("operator-d"),
    )
    .expect("uncommitted");
    freeze_bound(&mut uncommitted);
    let uncommitted_target = terminology_item(&uncommitted).target;
    let uncommitted_id = uncommitted.session_id().to_owned();
    uncommitted.close().expect("close uncommitted");

    let mut session_e = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Reddis"),
        Vec::new(),
        material_use(),
        session_authority("operator-e"),
    )
    .expect("session e");
    promote_human_raised(&mut session_e, 0, 6, "Redis");
    session_e.close().expect("close e");

    let grown =
        DurableApplicationSession::open(&session_store, &uncommitted_id, OpenMode::Writable)
            .expect("reopen uncommitted");
    let stale = grown.prepare_human_decision(
        uncommitted_target,
        CorrectionDecision::AcceptAlternative {
            alternative_index: 0,
        },
    );
    assert!(matches!(
        stale,
        Err(SessionPersistenceError::StaleAuthorityPrecondition(
            StaleAuthorityPrecondition {
                scope: AuthorityScope::ActiveAnalysis,
            }
        ))
    ));
    assert!(current_srt(&grown).contains("Postgre sequel"));
    grown.close().expect("close grown");
}

#[test]
fn same_y_exact_reuse_collapses_to_terminology_target() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Kafak"),
        Vec::new(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    promote_human_raised(&mut session_a, 0, 5, "Kafka");
    session_a.close().expect("close a");

    let mut session_b = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Kafak"),
        Vec::new(),
        material_use(),
        session_authority("operator-b"),
    )
    .expect("session b");
    freeze_bound(&mut session_b);
    let items = session_b.session().review_items();
    assert!(items.iter().any(|item| {
        matches!(
            item.target,
            ApplicationReviewTarget::ProjectTerminologyProposal { .. }
        )
    }));
    assert!(items.iter().all(|item| {
        !matches!(
            item.target,
            ApplicationReviewTarget::ProjectReuseProposal { .. }
        )
    }));
    session_b.close().expect("close b");
}

#[test]
fn different_y_collision_preserves_canonical_and_terminology() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgres"),
        Vec::new(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    promote_human_raised(&mut session_a, 0, 8, "PostgreSQL");
    session_a.close().expect("close a");

    let mut session_b = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgre sequel"),
        vec![vox_proof::candidate::SessionTermEntry::new(
            "Cafka",
            vec!["Postgre sequel".to_string()],
            Vec::new(),
        )],
        material_use(),
        session_authority("operator-b"),
    )
    .expect("session b");
    freeze_bound(&mut session_b);
    let items = session_b.session().review_items();
    let canonical = items.iter().find(|item| {
        matches!(
            item.target,
            ApplicationReviewTarget::CanonicalTermCase { .. }
        )
    });
    let terminology = items.iter().find(|item| {
        matches!(
            item.kind,
            ApplicationReviewItemKind::ProjectTerminologyProposal { .. }
        )
    });
    assert!(canonical.is_some());
    let terminology = terminology.expect("conflict terminology item");
    match terminology.kind {
        ApplicationReviewItemKind::ProjectTerminologyProposal {
            conflict_with_canonical,
        } => assert!(conflict_with_canonical),
        other => panic!("expected terminology kind, got {other:?}"),
    }
    session_b.close().expect("close b");
}

#[test]
fn terminology_target_without_decision_rolls_back() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgres"),
        Vec::new(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    promote_human_raised(&mut session_a, 0, 8, "PostgreSQL");
    session_a.close().expect("close a");

    let mut session_b = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse("Postgre sequel"),
        Vec::new(),
        material_use(),
        session_authority("operator-b"),
    )
    .expect("session b");
    freeze_bound(&mut session_b);
    let target = terminology_item(&session_b).target;
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
    assert!(reopened
        .session()
        .persisted_project_terminology_proposal_targets()
        .is_empty());
    assert!(reopened.session().review_ledger().events().is_empty());
    assert!(current_srt(&reopened).contains("Postgre sequel"));
    reopened.close().expect("close reopen");
}

#[test]
fn historical_v1_and_v2_remain_readable_without_terminology_writes() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let v1 = DurableApplicationSession::create_historical_unbound_format_v1_for_compatibility_test(
        &session_store,
        parse("Hello"),
        Vec::new(),
        material_use(),
        session_authority("operator-v1"),
    )
    .expect("v1");
    assert_eq!(v1.format_version(), 1);
    let v1_id = v1.session_id().to_owned();
    let v1_db = session_db(temp.path(), &v1_id);
    assert!(!table_exists(
        &v1_db,
        "project_terminology_proposal_targets"
    ));
    assert!(!v1.session().compose_project_terminology_proposals());
    v1.close().expect("close v1");

    let mut v2 =
        DurableApplicationSession::create_historical_bound_format_v2_for_compatibility_test(
            &session_store,
            &project_store,
            &project_id,
            parse("Postgre sequel"),
            Vec::new(),
            material_use(),
            session_authority("operator-v2"),
        )
        .expect("v2");
    assert_eq!(v2.format_version(), 2);
    let v2_id = v2.session_id().to_owned();
    let v2_db = session_db(temp.path(), &v2_id);
    assert!(!table_exists(
        &v2_db,
        "project_terminology_proposal_targets"
    ));
    freeze_bound(&mut v2);
    assert!(!v2.session().compose_project_terminology_proposals());
    assert!(v2
        .session()
        .derived_project_terminology_proposal_targets()
        .is_empty());
    v2.close().expect("close v2");

    let reopened_v1 = DurableApplicationSession::open(&session_store, &v1_id, OpenMode::ReadOnly)
        .expect("reopen v1");
    assert!(!table_exists(
        &v1_db,
        "project_terminology_proposal_targets"
    ));
    reopened_v1.close().expect("close reopen v1");
    let reopened_v2 = DurableApplicationSession::open(&session_store, &v2_id, OpenMode::ReadOnly)
        .expect("reopen v2");
    assert!(!table_exists(
        &v2_db,
        "project_terminology_proposal_targets"
    ));
    reopened_v2.close().expect("close reopen v2");
}
