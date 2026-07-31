use sha2::Sha256;

use crate::anchor::TranscriptRevisionId;
use crate::reuse_primitives::{
    SnapshotIdentityRecordProvenance, SourceDecisionLocator, compute_snapshot_identity,
    decision_digest, hash_analysis_snapshot, hash_source_decision_locator,
};
use crate::review::{ManualReplacementText, ReviewCaseId};

use super::model::{
    EvidenceAnalysisSnapshot, EvidenceGovernanceActor, EvidenceReusableRecord,
    EvidenceSourceDecisionLocator, REUSABLE_INFLUENCE_PROJECTION_VERSION,
};
use super::violations::{OracleDiagnosticV3, OracleViolationCodeV3, diagnostic};

pub fn parse_review_case_id(case_id: &str) -> Result<ReviewCaseId, OracleDiagnosticV3> {
    let local_index = case_id
        .strip_prefix("review-case:")
        .ok_or_else(|| {
            diagnostic(
                OracleViolationCodeV3::MalformedReviewCaseId,
                "review_case_id",
                "review case id must use review-case:<local_index> syntax",
            )
        })?
        .parse::<usize>()
        .map_err(|_| {
            diagnostic(
                OracleViolationCodeV3::MalformedReviewCaseId,
                "review_case_id",
                "review case local index is not a valid usize",
            )
        })?;
    Ok(ReviewCaseId::local(local_index))
}

pub fn parse_revision_id(tagged: &str) -> Result<TranscriptRevisionId, OracleDiagnosticV3> {
    let hex = tagged.strip_prefix("rev:sha256-v1:").ok_or_else(|| {
        diagnostic(
            OracleViolationCodeV3::ChangedSourceRevisionIdentity,
            "source_revision_id",
            "revision id must use rev:sha256-v1:<hex> syntax",
        )
    })?;
    if hex.len() != 64 {
        return Err(diagnostic(
            OracleViolationCodeV3::ChangedSourceRevisionIdentity,
            "source_revision_id",
            "revision digest must be 64 hex characters",
        ));
    }
    let mut digest = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        if index >= 32 || chunk.len() != 2 {
            return Err(diagnostic(
                OracleViolationCodeV3::ChangedSourceRevisionIdentity,
                "source_revision_id",
                "revision digest hex is malformed",
            ));
        }
        let hi = (chunk[0] as char).to_digit(16).ok_or_else(|| {
            diagnostic(
                OracleViolationCodeV3::ChangedSourceRevisionIdentity,
                "source_revision_id",
                "revision digest hex is malformed",
            )
        })? as u8;
        let lo = (chunk[1] as char).to_digit(16).ok_or_else(|| {
            diagnostic(
                OracleViolationCodeV3::ChangedSourceRevisionIdentity,
                "source_revision_id",
                "revision digest hex is malformed",
            )
        })? as u8;
        digest[index] = (hi << 4) | lo;
    }
    Ok(TranscriptRevisionId::from_sha256_digest(digest))
}

pub fn parse_decision_digest_hex(hex: &str) -> Result<[u8; 32], OracleDiagnosticV3> {
    if hex.len() != 64 {
        return Err(diagnostic(
            OracleViolationCodeV3::MalformedDecisionDigest,
            "decision_digest",
            "decision digest must be 64 hex characters",
        ));
    }
    let mut out = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        if index >= 32 || chunk.len() != 2 {
            return Err(diagnostic(
                OracleViolationCodeV3::MalformedDecisionDigest,
                "decision_digest",
                "decision digest hex is malformed",
            ));
        }
        let hi = (chunk[0] as char).to_digit(16).ok_or_else(|| {
            diagnostic(
                OracleViolationCodeV3::MalformedDecisionDigest,
                "decision_digest",
                "decision digest hex is malformed",
            )
        })? as u8;
        let lo = (chunk[1] as char).to_digit(16).ok_or_else(|| {
            diagnostic(
                OracleViolationCodeV3::MalformedDecisionDigest,
                "decision_digest",
                "decision digest hex is malformed",
            )
        })? as u8;
        out[index] = (hi << 4) | lo;
    }
    Ok(out)
}

pub fn try_build_analysis_snapshot(
    snapshot: &EvidenceAnalysisSnapshot,
) -> Result<AnalysisSnapshot, OracleDiagnosticV3> {
    let source_revision = parse_revision_id(&snapshot.source_revision_id)?;
    let session_terms = parse_session_terms_identity(&snapshot.session_terms_identity)?;
    let configuration = match [
        crate::candidate::canonical_session_term_analysis_identity(),
        crate::candidate::reuse_enabled_session_term_analysis_identity(),
    ]
    .into_iter()
    .find(|configuration| evidence_matches_configuration(snapshot, *configuration))
    {
        Some(configuration) => configuration,
        None => {
            return Err(diagnostic(
                OracleViolationCodeV3::MalformedAnalysisSnapshot,
                "analysis_snapshots",
                "analysis snapshot fields do not match a known production configuration",
            ));
        }
    };
    Ok(AnalysisSnapshot::from_identity_parts(
        source_revision,
        session_terms,
        configuration,
    ))
}

fn parse_session_terms_identity(
    tagged: &str,
) -> Result<crate::analysis::SessionTermsIdentity, OracleDiagnosticV3> {
    let hex = tagged
        .strip_prefix("session-terms:sha256-v1:")
        .ok_or_else(|| {
            diagnostic(
                OracleViolationCodeV3::MalformedAnalysisSnapshot,
                "session_terms_identity",
                "session terms identity must use session-terms:sha256-v1:<hex> syntax",
            )
        })?;
    if hex.len() != 64 {
        return Err(diagnostic(
            OracleViolationCodeV3::MalformedAnalysisSnapshot,
            "session_terms_identity",
            "session terms digest must be 64 hex characters",
        ));
    }
    let mut digest = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16).ok_or_else(|| {
            diagnostic(
                OracleViolationCodeV3::MalformedAnalysisSnapshot,
                "session_terms_identity",
                "session terms digest hex is malformed",
            )
        })? as u8;
        let lo = (chunk[1] as char).to_digit(16).ok_or_else(|| {
            diagnostic(
                OracleViolationCodeV3::MalformedAnalysisSnapshot,
                "session_terms_identity",
                "session terms digest hex is malformed",
            )
        })? as u8;
        digest[index] = (hi << 4) | lo;
    }
    Ok(crate::analysis::SessionTermsIdentity::from_digest_for_evidence(digest))
}

use crate::analysis::AnalysisConfigurationIdentity;

fn evidence_matches_configuration(
    snapshot: &EvidenceAnalysisSnapshot,
    configuration: AnalysisConfigurationIdentity,
) -> bool {
    let detectors = configuration.detector_set().detectors();
    if snapshot.detectors.len() != detectors.len() {
        return false;
    }
    for (left, right) in snapshot.detectors.iter().zip(detectors.iter()) {
        if left.id != right.id() || left.version != right.version() {
            return false;
        }
    }
    snapshot.detector_config_id == configuration.detector_config().id()
        && snapshot.detector_config_version == configuration.detector_config().version()
        && snapshot.algorithm_id == configuration.algorithm().id()
        && snapshot.algorithm_version == configuration.algorithm().version()
}

use crate::analysis::AnalysisSnapshot;

pub fn try_build_source_decision_locator(
    locator: &EvidenceSourceDecisionLocator,
    snapshots: &[EvidenceAnalysisSnapshot],
) -> Result<SourceDecisionLocator, OracleDiagnosticV3> {
    let source_revision = parse_revision_id(&locator.source_revision_id)?;
    let source_analysis_snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.identity == locator.source_analysis_snapshot_identity)
        .ok_or_else(|| {
            diagnostic(
                OracleViolationCodeV3::SourceLocatorBoundaryViolation,
                "source_locator.source_analysis_snapshot_identity",
                "referenced analysis snapshot does not exist",
            )
        })
        .and_then(try_build_analysis_snapshot)?;
    let source_review_case_id = parse_review_case_id(&locator.source_review_case_id)?;
    let decision_digest = parse_decision_digest_hex(&locator.decision_digest)?;
    Ok(SourceDecisionLocator {
        source_revision,
        source_analysis_snapshot,
        source_review_case_id,
        review_ledger_position: locator.review_ledger_position,
        decision_digest,
        effective_at_ledger_length: locator.effective_at_ledger_length,
    })
}

pub fn production_decision_digest_hex(
    case_id: &str,
    observed_revision_id: &str,
    replacement: &str,
    observed_source_bytes: &str,
) -> Result<String, OracleDiagnosticV3> {
    let case_id = parse_review_case_id(case_id)?;
    let revision = parse_revision_id(observed_revision_id)?;
    let replacement =
        ManualReplacementText::new(replacement, observed_source_bytes).map_err(|_| {
            diagnostic(
                OracleViolationCodeV3::MalformedReviewLedgerEvent,
                "manual_replacement_bytes",
                "manual replacement bytes are invalid for observed source bytes",
            )
        })?;
    Ok(digest_hex(decision_digest(case_id, revision, &replacement)))
}

pub fn compute_production_snapshot_identity(
    project_scope_stable_id: &str,
    governance_event_boundary: usize,
    active_records: &[EvidenceReusableRecord],
    analysis_snapshots: &[EvidenceAnalysisSnapshot],
) -> Result<String, OracleDiagnosticV3> {
    let project_scope_id = crate::reuse_primitives::ProjectScopeId::new(project_scope_stable_id)
        .map_err(|_| {
            diagnostic(
                OracleViolationCodeV3::ChangedProjectScopeStableId,
                "project_scope.stable_id",
                "project scope stable id is invalid",
            )
        })?;
    let mut locators = Vec::with_capacity(active_records.len());
    for record in active_records {
        locators.push(try_build_source_decision_locator(
            &record.source_locator,
            analysis_snapshots,
        )?);
    }
    let mut provenance = Vec::with_capacity(active_records.len());
    for (record, source_locator) in active_records.iter().zip(locators.iter()) {
        provenance.push(SnapshotIdentityRecordProvenance {
            record_id:
                crate::reuse_primitives::ReusableInfluenceRecordId::from_promotion_event_index(
                    record.record_id,
                ),
            observed_text: record.observed_text.as_str(),
            confirmed_replacement: record.confirmed_replacement.as_str(),
            source_locator,
            promotion_actor_role: record.promotion_actor.role_label.as_str(),
            promotion_actor_label: record.promotion_actor.display_label.as_str(),
        });
    }
    let identity = compute_snapshot_identity(
        &project_scope_id,
        governance_event_boundary,
        REUSABLE_INFLUENCE_PROJECTION_VERSION,
        &provenance,
    );
    Ok(identity.to_tagged_string())
}

pub fn hash_evidence_analysis_snapshot(
    hasher: &mut Sha256,
    snapshot: &EvidenceAnalysisSnapshot,
) -> Result<(), OracleDiagnosticV3> {
    let production = try_build_analysis_snapshot(snapshot)?;
    hash_analysis_snapshot(hasher, &production);
    Ok(())
}

pub fn hash_evidence_source_decision_locator(
    hasher: &mut Sha256,
    locator: &EvidenceSourceDecisionLocator,
    snapshots: &[EvidenceAnalysisSnapshot],
) -> Result<(), OracleDiagnosticV3> {
    let production = try_build_source_decision_locator(locator, snapshots)?;
    hash_source_decision_locator(hasher, &production);
    Ok(())
}

pub fn validate_governance_actor_session_bound(
    actor: &EvidenceGovernanceActor,
    session_authority: &super::model::EvidenceSessionAuthority,
    path: &str,
    violations: &mut Vec<OracleDiagnosticV3>,
) -> bool {
    if crate::reusable_influence::validate_canonical_stored_actor_display_label(
        &actor.display_label,
    )
    .is_err()
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::GovernanceActorLabelNonCanonical,
            path,
            "governance actor display label is not canonical",
        ));
        return false;
    }
    if actor.role_label != session_authority.role_label {
        violations.push(diagnostic(
            OracleViolationCodeV3::GovernanceActorRoleMismatch,
            path,
            "governance actor role does not match session authority",
        ));
        return false;
    }
    if actor.display_label != session_authority.display_label {
        violations.push(diagnostic(
            OracleViolationCodeV3::GovernanceActorLabelMismatch,
            path,
            "governance actor display label does not match session authority",
        ));
        return false;
    }
    if crate::reusable_influence::validate_governance_actor(
        &crate::reusable_influence::GovernanceActorContext {
            role_label: actor.role_label.clone(),
            display_label: actor.display_label.clone(),
        },
    )
    .is_err()
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedReuseGovernanceActor,
            path,
            "governance actor context is invalid",
        ));
        return false;
    }
    true
}

fn digest_hex(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
