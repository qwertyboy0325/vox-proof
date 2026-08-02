use sha2::{Digest, Sha256};

use super::model::{
    EvidenceAnalysisSnapshot, EvidenceGovernanceActor, EvidenceReusableRecord,
    EvidenceSourceDecisionLocator, REUSABLE_INFLUENCE_PROJECTION_VERSION,
};
use super::violations::{OracleDiagnosticV3, OracleViolationCodeV3, diagnostic};

pub fn parse_review_case_id(case_id: &str) -> Result<usize, OracleDiagnosticV3> {
    let encoded_index = case_id.strip_prefix("review-case:").ok_or_else(|| {
        diagnostic(
            OracleViolationCodeV3::MalformedReviewCaseId,
            "review_case_id",
            "review case id must use review-case:<local_index> syntax",
        )
    })?;
    if encoded_index.is_empty()
        || encoded_index.starts_with('+')
        || (encoded_index.len() > 1 && encoded_index.starts_with('0'))
        || !encoded_index.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(diagnostic(
            OracleViolationCodeV3::MalformedReviewCaseId,
            "review_case_id",
            "review case id must use canonical review-case:<usize> syntax",
        ));
    }
    encoded_index.parse::<usize>().map_err(|_| {
        diagnostic(
            OracleViolationCodeV3::MalformedReviewCaseId,
            "review_case_id",
            "review case local index is not a valid usize",
        )
    })
}

pub fn validate_revision_id(tagged: &str) -> Result<(), OracleDiagnosticV3> {
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
    if !hex
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(diagnostic(
            OracleViolationCodeV3::ChangedSourceRevisionIdentity,
            "source_revision_id",
            "revision digest hex is malformed",
        ));
    }
    Ok(())
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

pub fn validate_analysis_snapshot(
    snapshot: &EvidenceAnalysisSnapshot,
) -> Result<(), OracleDiagnosticV3> {
    validate_revision_id(&snapshot.source_revision_id)?;
    validate_session_terms_identity(&snapshot.session_terms_identity)?;
    let _configuration = match [
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
    let expected_identity = evidence_analysis_snapshot_identity(snapshot);
    if snapshot.identity != expected_identity {
        return Err(diagnostic(
            OracleViolationCodeV3::MalformedAnalysisSnapshot,
            "analysis_snapshots.identity",
            "analysis snapshot display identity does not match typed fields",
        ));
    }
    Ok(())
}

fn validate_session_terms_identity(tagged: &str) -> Result<(), OracleDiagnosticV3> {
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
    if !hex
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(diagnostic(
            OracleViolationCodeV3::MalformedAnalysisSnapshot,
            "session_terms_identity",
            "session terms digest hex is malformed",
        ));
    }
    Ok(())
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

pub fn is_reuse_enabled_analysis_snapshot(snapshot: &EvidenceAnalysisSnapshot) -> bool {
    evidence_matches_configuration(
        snapshot,
        crate::candidate::reuse_enabled_session_term_analysis_identity(),
    )
}

pub fn is_canonical_analysis_snapshot(snapshot: &EvidenceAnalysisSnapshot) -> bool {
    evidence_matches_configuration(
        snapshot,
        crate::candidate::canonical_session_term_analysis_identity(),
    )
}

pub fn validate_source_decision_locator(
    locator: &EvidenceSourceDecisionLocator,
    snapshots: &[EvidenceAnalysisSnapshot],
) -> Result<(), OracleDiagnosticV3> {
    validate_revision_id(&locator.source_revision_id)?;
    let source_analysis_snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.identity == locator.source_analysis_snapshot_identity)
        .ok_or_else(|| {
            diagnostic(
                OracleViolationCodeV3::SourceLocatorBoundaryViolation,
                "source_locator.source_analysis_snapshot_identity",
                "referenced analysis snapshot does not exist",
            )
        })?;
    validate_analysis_snapshot(source_analysis_snapshot)?;
    if source_analysis_snapshot.source_revision_id != locator.source_revision_id {
        return Err(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.source_analysis_snapshot_identity",
            "analysis snapshot source revision does not match source locator revision",
        ));
    }
    parse_review_case_id(&locator.source_review_case_id)?;
    parse_decision_digest_hex(&locator.decision_digest)?;
    Ok(())
}

pub fn production_decision_digest_hex(
    case_id: &str,
    observed_revision_id: &str,
    replacement: &str,
    observed_source_bytes: &str,
) -> Result<String, OracleDiagnosticV3> {
    let case_index = parse_review_case_id(case_id)?;
    validate_revision_id(observed_revision_id)?;
    validate_manual_replacement(replacement, observed_source_bytes)?;
    let mut hasher = Sha256::new();
    hasher.update(b"voxproof-manual-replacement-decision-digest-v1");
    hasher.update((case_index as u64).to_le_bytes());
    hasher.update(observed_revision_id.as_bytes());
    hash_string(&mut hasher, replacement);
    Ok(digest_hex(hasher.finalize().into()))
}

pub fn compute_production_snapshot_identity(
    project_scope_stable_id: &str,
    governance_event_boundary: usize,
    active_records: &[EvidenceReusableRecord],
    analysis_snapshots: &[EvidenceAnalysisSnapshot],
) -> Result<String, OracleDiagnosticV3> {
    if !valid_scope_text(project_scope_stable_id) {
        return Err(diagnostic(
            OracleViolationCodeV3::ChangedProjectScopeStableId,
            "project_scope.stable_id",
            "project scope stable id is invalid",
        ));
    }
    let mut hasher = Sha256::new();
    hasher.update(b"voxproof-reusable-influence-snapshot-identity-v2");
    hash_string(&mut hasher, project_scope_stable_id);
    hasher.update((governance_event_boundary as u64).to_le_bytes());
    hash_string(&mut hasher, REUSABLE_INFLUENCE_PROJECTION_VERSION);
    hasher.update((active_records.len() as u64).to_le_bytes());
    for record in active_records {
        validate_source_decision_locator(&record.source_locator, analysis_snapshots)?;
        hasher.update((record.record_id as u64).to_le_bytes());
        hash_string(&mut hasher, &record.observed_text);
        hash_string(&mut hasher, &record.confirmed_replacement);
        hash_evidence_source_decision_locator(
            &mut hasher,
            &record.source_locator,
            analysis_snapshots,
        )?;
        hash_string(&mut hasher, &record.promotion_actor.role_label);
        hash_string(&mut hasher, &record.promotion_actor.display_label);
    }
    Ok(format!(
        "reusable-influence-snapshot:sha256-v2:{}",
        digest_hex(hasher.finalize().into())
    ))
}

pub fn hash_evidence_analysis_snapshot(
    hasher: &mut Sha256,
    snapshot: &EvidenceAnalysisSnapshot,
) -> Result<(), OracleDiagnosticV3> {
    validate_analysis_snapshot(snapshot)?;
    hash_string(hasher, &snapshot.source_revision_id);
    hash_string(hasher, &snapshot.session_terms_identity);
    hasher.update((snapshot.detectors.len() as u64).to_le_bytes());
    for detector in &snapshot.detectors {
        hash_string(hasher, &detector.id);
        hash_string(hasher, &detector.version);
    }
    hash_string(hasher, &snapshot.detector_config_id);
    hash_string(hasher, &snapshot.detector_config_version);
    hash_string(hasher, &snapshot.algorithm_id);
    hash_string(hasher, &snapshot.algorithm_version);
    Ok(())
}

pub fn hash_evidence_source_decision_locator(
    hasher: &mut Sha256,
    locator: &EvidenceSourceDecisionLocator,
    snapshots: &[EvidenceAnalysisSnapshot],
) -> Result<(), OracleDiagnosticV3> {
    validate_source_decision_locator(locator, snapshots)?;
    hash_string(hasher, &locator.source_revision_id);
    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.identity == locator.source_analysis_snapshot_identity)
        .expect("validated snapshot reference");
    hash_evidence_analysis_snapshot(hasher, snapshot)?;
    hasher.update((parse_review_case_id(&locator.source_review_case_id)? as u64).to_le_bytes());
    hasher.update((locator.review_ledger_position as u64).to_le_bytes());
    hasher.update(parse_decision_digest_hex(&locator.decision_digest)?);
    hasher.update((locator.effective_at_ledger_length as u64).to_le_bytes());
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

fn hash_string(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn evidence_analysis_snapshot_identity(snapshot: &EvidenceAnalysisSnapshot) -> String {
    let detector_ids = snapshot
        .detectors
        .iter()
        .map(|detector| format!("{}@{}", detector.id, detector.version))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "analysis-snapshot:{}:{}:{}:{}@{}:{}@{}",
        snapshot.source_revision_id,
        snapshot.session_terms_identity,
        detector_ids,
        snapshot.detector_config_id,
        snapshot.detector_config_version,
        snapshot.algorithm_id,
        snapshot.algorithm_version
    )
}

fn validate_manual_replacement(
    replacement: &str,
    observed_source_bytes: &str,
) -> Result<(), OracleDiagnosticV3> {
    if replacement.is_empty()
        || replacement.chars().all(char::is_whitespace)
        || replacement.len() > crate::review::MAX_MANUAL_REPLACEMENT_UTF8_BYTES
        || replacement.as_bytes() == observed_source_bytes.as_bytes()
        || replacement
            .chars()
            .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
    {
        return Err(diagnostic(
            OracleViolationCodeV3::MalformedReviewLedgerEvent,
            "manual_replacement_bytes",
            "manual replacement bytes are invalid for observed source bytes",
        ));
    }
    Ok(())
}

fn valid_scope_text(value: &str) -> bool {
    !value.is_empty()
        && !value.chars().all(char::is_whitespace)
        && !value
            .chars()
            .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
}

#[cfg(test)]
mod tests {
    use sha2::Digest;

    use super::{
        evidence_analysis_snapshot_identity, hash_evidence_analysis_snapshot,
        hash_evidence_source_decision_locator, validate_analysis_snapshot, validate_revision_id,
    };
    use crate::application_service::{
        ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
        DeclaredSessionAuthority, DeclaredSessionOperatorRole, begin_application_review,
    };
    use crate::candidate::SessionTermEntry;
    use crate::persistence_evidence::current_contract::GOLDEN_SMALL_TRANSCRIPT;
    use crate::persistence_evidence::current_contract::projection::{
        map_analysis_snapshot_for_export, map_locator,
    };
    use crate::srt::parse_srt;

    #[test]
    fn evidence_hashes_are_byte_equivalent_to_production_hashes() {
        let transcript = parse_srt(GOLDEN_SMALL_TRANSCRIPT).expect("transcript");
        let terms = vec![SessionTermEntry::new(
            "Kafka",
            vec!["Kafak".to_owned()],
            Vec::new(),
        )];
        let authority = DeclaredSessionAuthority::new(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Ezra",
        )
        .expect("authority");
        let mut session = begin_application_review(
            transcript,
            terms,
            ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned),
            authority,
        )
        .expect("session");
        let target = session.review_items()[0].target;
        session
            .record_manual_replacement(target, "Kafka")
            .expect("replacement");
        session
            .initialize_project_scope("proj-a", "Project A")
            .expect("scope");
        let production_locator = &session.reuse_candidates().expect("candidate")[0]
            .key
            .source_locator;
        let snapshot = session
            .reuse_parts()
            .canonical_run
            .analysis_run()
            .snapshot();
        let evidence_snapshot = map_analysis_snapshot_for_export(
            snapshot,
            session.source().revision_id().to_tagged_string(),
        );
        let mut production_analysis = sha2::Sha256::new();
        crate::reuse_primitives::hash_analysis_snapshot(&mut production_analysis, &snapshot);
        let mut evidence_analysis = sha2::Sha256::new();
        hash_evidence_analysis_snapshot(&mut evidence_analysis, &evidence_snapshot)
            .expect("valid evidence snapshot");
        assert_eq!(production_analysis.finalize(), evidence_analysis.finalize());

        let mapped = map_locator(production_locator);
        let mut production_locator_hash = sha2::Sha256::new();
        crate::reuse_primitives::hash_source_decision_locator(
            &mut production_locator_hash,
            production_locator,
        );
        let mut evidence_locator_hash = sha2::Sha256::new();
        hash_evidence_source_decision_locator(
            &mut evidence_locator_hash,
            &mapped,
            &[evidence_snapshot],
        )
        .expect("valid evidence locator");
        assert_eq!(
            production_locator_hash.finalize(),
            evidence_locator_hash.finalize()
        );
    }

    #[test]
    fn noncanonical_uppercase_identity_hex_is_rejected_before_hashing() {
        assert!(
            validate_revision_id(
                "rev:sha256-v1:ABCDEF0000000000000000000000000000000000000000000000000000000000"
            )
            .is_err()
        );

        let transcript = parse_srt(GOLDEN_SMALL_TRANSCRIPT).expect("transcript");
        let terms = vec![SessionTermEntry::new(
            "Kafka",
            vec!["Kafak".to_owned()],
            Vec::new(),
        )];
        let authority = DeclaredSessionAuthority::new(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Ezra",
        )
        .expect("authority");
        let session = begin_application_review(
            transcript,
            terms,
            ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned),
            authority,
        )
        .expect("session");
        let mut snapshot = map_analysis_snapshot_for_export(
            session
                .reuse_parts()
                .canonical_run
                .analysis_run()
                .snapshot(),
            session.source().revision_id().to_tagged_string(),
        );
        let prefix = "session-terms:sha256-v1:";
        let digest = snapshot
            .session_terms_identity
            .strip_prefix(prefix)
            .expect("session terms identity prefix");
        snapshot.session_terms_identity = format!("{prefix}{}", digest.to_uppercase());
        snapshot.identity = evidence_analysis_snapshot_identity(&snapshot);

        assert!(validate_analysis_snapshot(&snapshot).is_err());
    }
}
