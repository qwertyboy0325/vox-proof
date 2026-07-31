use serde::{Deserialize, Serialize};

/// Canonical versus derived classification for current-contract persistence evidence.
///
/// Evidence-only taxonomy. Not a production persistence schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldClassification {
    CanonicalAuthority,
    CanonicalReferencedPayload,
    CanonicalHistoricalProvenance,
    DerivedRebuildable,
    PresentationOnlyExcluded,
    TransportOnlyExcluded,
    FutureSemanticExcluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifiedField {
    pub field_path: &'static str,
    pub classification: FieldClassification,
    pub reconstruction_requirement: Option<&'static str>,
}

/// Static registry documenting fixture v3 field classes.
pub const FIELD_CLASSIFICATION_REGISTRY: &[ClassifiedField] = &[
    ClassifiedField {
        field_path: "session_id",
        classification: FieldClassification::CanonicalAuthority,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "duplicated_from_session_id",
        classification: FieldClassification::CanonicalHistoricalProvenance,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "session_authority",
        classification: FieldClassification::CanonicalAuthority,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "material_use_declaration",
        classification: FieldClassification::CanonicalAuthority,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "source_revisions",
        classification: FieldClassification::CanonicalReferencedPayload,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "session_terms_identity",
        classification: FieldClassification::CanonicalAuthority,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "analysis_snapshots",
        classification: FieldClassification::CanonicalAuthority,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "review_cases",
        classification: FieldClassification::CanonicalAuthority,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "review_ledger_events",
        classification: FieldClassification::CanonicalAuthority,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "effective_review_status",
        classification: FieldClassification::DerivedRebuildable,
        reconstruction_requirement: Some(
            "Fold append-only ReviewLedger events with last-decision-wins semantics.",
        ),
    },
    ClassifiedField {
        field_path: "project_scope.stable_id",
        classification: FieldClassification::CanonicalAuthority,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "project_scope.display_name",
        classification: FieldClassification::DerivedRebuildable,
        reconstruction_requirement: Some(
            "Mutable non-semantic display label; excluded from canonical fingerprint.",
        ),
    },
    ClassifiedField {
        field_path: "reuse_governance_events",
        classification: FieldClassification::CanonicalAuthority,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "rejected_candidate_identities",
        classification: FieldClassification::CanonicalHistoricalProvenance,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "effective_reusable_records",
        classification: FieldClassification::DerivedRebuildable,
        reconstruction_requirement: Some(
            "Deterministic fold of reuse-governance history against review ledger and canonical run.",
        ),
    },
    ClassifiedField {
        field_path: "historical_reusable_records",
        classification: FieldClassification::DerivedRebuildable,
        reconstruction_requirement: Some("Folded governance history excluding active records."),
    },
    ClassifiedField {
        field_path: "reusable_snapshot_identity",
        classification: FieldClassification::DerivedRebuildable,
        reconstruction_requirement: Some(
            "Derived from project scope, governance boundary, and active record provenance per MD-018.",
        ),
    },
    ClassifiedField {
        field_path: "reuse_enabled_analysis_identity",
        classification: FieldClassification::DerivedRebuildable,
        reconstruction_requirement: Some(
            "Reuse-enabled analysis identity must bind to the derived reusable snapshot identity.",
        ),
    },
    ClassifiedField {
        field_path: "derived_queue_projection",
        classification: FieldClassification::DerivedRebuildable,
        reconstruction_requirement: Some(
            "Rebuildable review queue projection for evidence-only corruption tests.",
        ),
    },
    ClassifiedField {
        field_path: "desktop_selection_state",
        classification: FieldClassification::PresentationOnlyExcluded,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "application_export_v2",
        classification: FieldClassification::TransportOnlyExcluded,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "application_export_v3",
        classification: FieldClassification::TransportOnlyExcluded,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "human_raised_review_cases",
        classification: FieldClassification::FutureSemanticExcluded,
        reconstruction_requirement: None,
    },
    ClassifiedField {
        field_path: "asr_observations",
        classification: FieldClassification::FutureSemanticExcluded,
        reconstruction_requirement: None,
    },
];

pub fn canonical_classes() -> Vec<FieldClassification> {
    vec![
        FieldClassification::CanonicalAuthority,
        FieldClassification::CanonicalReferencedPayload,
        FieldClassification::CanonicalHistoricalProvenance,
    ]
}

pub fn derived_classes() -> Vec<FieldClassification> {
    vec![FieldClassification::DerivedRebuildable]
}

pub fn excluded_classes() -> Vec<FieldClassification> {
    vec![
        FieldClassification::PresentationOnlyExcluded,
        FieldClassification::TransportOnlyExcluded,
        FieldClassification::FutureSemanticExcluded,
    ]
}
