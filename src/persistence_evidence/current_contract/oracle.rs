use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::model::{CurrentContractCanonicalProjection, CurrentContractState};

pub const CURRENT_CONTRACT_ORACLE_VERSION: &str = "3";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleViolationCodeV3 {
    MissingCanonicalHistory,
    FabricatedAutomaticDecision,
    ChangedSessionAuthority,
    ChangedMaterialUseDeclaration,
    ChangedSessionIdentity,
    ChangedSourceRevisionIdentity,
    ChangedReviewLedgerOrder,
    ChangedReviewLedgerPayload,
    ChangedReuseGovernanceOrder,
    ChangedReuseGovernanceActor,
    ChangedProjectScopeStableId,
    ChangedReusableSnapshotIdentity,
    ReuseAnalysisSnapshotMismatch,
    MissingCommittedReviewEvent,
    UncommittedReviewEventExposed,
    CanonicalFingerprintMismatch,
    DerivedRebuildMismatch,
    SourceLocatorBoundaryViolation,
    HumanRaisedCasePresent,
    ExportTransportPresentInHydrationState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleDiagnosticV3 {
    pub code: OracleViolationCodeV3,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleResultV3 {
    pub oracle_version: String,
    pub passed: bool,
    pub violations: Vec<OracleDiagnosticV3>,
    pub warnings: Vec<OracleDiagnosticV3>,
    pub canonical_fingerprint: String,
    pub derived_fingerprint: Option<String>,
    pub claimable_evidence_strength: Vec<String>,
}

pub struct CurrentContractOracle;

impl CurrentContractOracle {
    pub fn validate(state: &CurrentContractState) -> OracleResultV3 {
        let state = state.clone().normalize();
        let mut violations = validate_invariants(&state);
        sort_diagnostics(&mut violations);
        let passed = violations.is_empty();
        OracleResultV3 {
            oracle_version: CURRENT_CONTRACT_ORACLE_VERSION.to_owned(),
            passed,
            violations,
            warnings: Vec::new(),
            canonical_fingerprint: canonical_fingerprint(&state.canonical_projection()),
            derived_fingerprint: Some(derived_fingerprint(&state)),
            claimable_evidence_strength: if passed {
                vec!["InterfaceBehavior".to_owned()]
            } else {
                Vec::new()
            },
        }
    }

    pub fn compare(
        expected: &CurrentContractState,
        actual: &CurrentContractState,
    ) -> OracleResultV3 {
        let expected = expected.clone().normalize();
        let actual = actual.clone().normalize();
        let mut violations = validate_invariants(&actual);
        compare_canonical(
            &expected.canonical_projection(),
            &actual.canonical_projection(),
            &mut violations,
        );
        compare_derived_fold(&expected, &actual, &mut violations);

        let expected_fp = canonical_fingerprint(&expected.canonical_projection());
        let actual_fp = canonical_fingerprint(&actual.canonical_projection());
        if expected_fp != actual_fp && violations.is_empty() {
            violations.push(diagnostic(
                OracleViolationCodeV3::CanonicalFingerprintMismatch,
                "canonical_projection",
                "canonical fingerprints differ without a more specific diagnostic",
            ));
        }
        sort_diagnostics(&mut violations);
        violations.dedup();

        let passed = violations.is_empty();
        OracleResultV3 {
            oracle_version: CURRENT_CONTRACT_ORACLE_VERSION.to_owned(),
            passed,
            violations,
            warnings: Vec::new(),
            canonical_fingerprint: actual_fp,
            derived_fingerprint: Some(derived_fingerprint(&actual)),
            claimable_evidence_strength: if passed {
                vec!["InterfaceBehavior".to_owned()]
            } else {
                Vec::new()
            },
        }
    }
}

fn validate_invariants(state: &CurrentContractState) -> Vec<OracleDiagnosticV3> {
    let mut violations = Vec::new();
    if state
        .review_cases
        .iter()
        .any(|case| case.origin == "human_raised")
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::HumanRaisedCasePresent,
            "review_cases",
            "HumanRaised cases are excluded from current-contract fixture v3",
        ));
    }
    if state
        .review_ledger_events
        .iter()
        .any(|event| event.provenance == "automatic")
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::FabricatedAutomaticDecision,
            "review_ledger_events",
            "automatic provenance is forbidden in current-contract evidence",
        ));
    }
    violations
}

fn compare_canonical(
    expected: &CurrentContractCanonicalProjection,
    actual: &CurrentContractCanonicalProjection,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    if expected.session_id != actual.session_id {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedSessionIdentity,
            "session_id",
            "session identity changed",
        ));
    }
    if expected.session_authority != actual.session_authority {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedSessionAuthority,
            "session_authority",
            "session authority changed",
        ));
    }
    if expected.material_use_declaration != actual.material_use_declaration {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedMaterialUseDeclaration,
            "material_use_declaration",
            "material-use declaration changed",
        ));
    }
    if expected.project_scope_stable_id != actual.project_scope_stable_id {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedProjectScopeStableId,
            "project_scope.stable_id",
            "project stable scope identity changed",
        ));
    }
    if expected.review_ledger_events != actual.review_ledger_events {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedReviewLedgerOrder,
            "review_ledger_events",
            "review ledger order or payload changed",
        ));
    }
    if expected.reuse_governance_events != actual.reuse_governance_events {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedReuseGovernanceOrder,
            "reuse_governance_events",
            "reuse governance order or payload changed",
        ));
    }
    if expected.reusable_snapshot_identity != actual.reusable_snapshot_identity {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedReusableSnapshotIdentity,
            "reusable_snapshot_identity",
            "reusable snapshot identity changed",
        ));
    }
    if expected.reuse_enabled_analysis_identity != actual.reuse_enabled_analysis_identity {
        violations.push(diagnostic(
            OracleViolationCodeV3::ReuseAnalysisSnapshotMismatch,
            "reuse_enabled_analysis_identity",
            "reuse-enabled analysis identity changed",
        ));
    }
}

fn compare_derived_fold(
    expected: &CurrentContractState,
    actual: &CurrentContractState,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    if expected.effective_reusable_records != actual.effective_reusable_records {
        violations.push(diagnostic(
            OracleViolationCodeV3::DerivedRebuildMismatch,
            "effective_reusable_records",
            "derived reusable fold changed",
        ));
    }
}

pub fn canonical_fingerprint(projection: &CurrentContractCanonicalProjection) -> String {
    let json = serde_json::to_string(projection).expect("canonical projection serializes");
    format!(
        "semantic:sha256-v1:{}",
        digest_hex(Sha256::digest(json.as_bytes()))
    )
}

pub fn derived_fingerprint(state: &CurrentContractState) -> String {
    let payload = serde_json::json!({
        "effective_review_status": state.effective_review_status,
        "effective_reusable_records": state.effective_reusable_records,
        "historical_reusable_records": state.historical_reusable_records,
        "derived_queue_projection": state.derived_queue_projection,
    });
    let json = serde_json::to_string(&payload).expect("derived payload serializes");
    format!(
        "derived:sha256-v1:{}",
        digest_hex(Sha256::digest(json.as_bytes()))
    )
}

fn digest_hex(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    bytes.as_ref().iter().fold(
        String::with_capacity(bytes.as_ref().len() * 2),
        |mut out, byte| {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
            out
        },
    )
}

fn diagnostic(code: OracleViolationCodeV3, path: &str, message: &str) -> OracleDiagnosticV3 {
    OracleDiagnosticV3 {
        code,
        path: path.to_owned(),
        message: message.to_owned(),
    }
}

fn sort_diagnostics(diagnostics: &mut [OracleDiagnosticV3]) {
    diagnostics.sort_by(|left, right| {
        (left.code, &left.path, &left.message).cmp(&(right.code, &right.path, &right.message))
    });
}
