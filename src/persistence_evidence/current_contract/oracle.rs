use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::derivation::{compare_supplied_derived, derive_contract_projection};
use super::model::{CurrentContractCanonicalProjection, CurrentContractState};
use super::violations::diagnostic;
pub use super::violations::{OracleDiagnosticV3, OracleViolationCodeV3};

pub const CURRENT_CONTRACT_ORACLE_VERSION: &str = "3";

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
        let (derived, mut violations) = derive_contract_projection(&state);
        compare_supplied_derived(&derived, &state, &mut violations);
        sort_diagnostics(&mut violations);
        let passed = violations.is_empty();
        OracleResultV3 {
            oracle_version: CURRENT_CONTRACT_ORACLE_VERSION.to_owned(),
            passed,
            violations,
            warnings: Vec::new(),
            canonical_fingerprint: canonical_fingerprint(&state.canonical_projection()),
            derived_fingerprint: Some(derived_fingerprint(&derived)),
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
        let (derived, mut violations) = derive_contract_projection(&actual);
        compare_canonical(
            &expected.canonical_projection(),
            &actual.canonical_projection(),
            &mut violations,
        );
        compare_supplied_derived(&derived, &actual, &mut violations);

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
            derived_fingerprint: Some(derived_fingerprint(&derived)),
            claimable_evidence_strength: if passed {
                vec!["InterfaceBehavior".to_owned()]
            } else {
                Vec::new()
            },
        }
    }
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
    if expected.duplicated_from_session_id != actual.duplicated_from_session_id {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedSessionIdentity,
            "duplicated_from_session_id",
            "duplication lineage changed",
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
}

pub fn canonical_fingerprint(projection: &CurrentContractCanonicalProjection) -> String {
    let json = serde_json::to_string(projection).expect("canonical projection serializes");
    format!(
        "semantic:sha256-v1:{}",
        digest_hex(Sha256::digest(json.as_bytes()))
    )
}

pub fn derived_fingerprint(derived: &super::model::DerivedContractProjection) -> String {
    let json = serde_json::to_string(derived).expect("derived projection serializes");
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

fn sort_diagnostics(diagnostics: &mut [OracleDiagnosticV3]) {
    diagnostics.sort_by(|left, right| {
        (left.code, &left.path, &left.message).cmp(&(right.code, &right.path, &right.message))
    });
}
