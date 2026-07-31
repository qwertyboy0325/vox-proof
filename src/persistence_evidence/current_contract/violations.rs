use serde::{Deserialize, Serialize};

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
    ReviewLedgerIndexGap,
    ReviewLedgerDuplicateIndex,
    MissingReviewCaseReference,
    ManualReplacementByteMismatch,
    DecisionDigestMismatch,
    PromotionLocatorMismatch,
    ReuseGovernanceIndexGap,
    ReuseGovernanceDuplicateIndex,
    DuplicateAcceptedRecordId,
    PromotionAfterRejectedCandidate,
    RevokeUnknownRecord,
    RevokeInactiveRecord,
    SupersedeUnknownPredecessor,
    SupersedeUnknownSuccessor,
    InvalidSupersessionLineage,
    CrossProjectRecordReference,
    SourceAnchorRevisionMismatch,
    SourceAnchorSegmentMismatch,
    SourceAnchorOutOfBounds,
    SourceAnchorResolvedBytesMismatch,
    MalformedReviewCaseId,
    MalformedDecisionDigest,
    MalformedAnalysisSnapshot,
    MalformedReviewLedgerEvent,
    ReviewLedgerEventRevisionMismatch,
    DuplicateReviewCaseId,
    UnknownDecisionProvenance,
    GovernanceActorRoleMismatch,
    GovernanceActorLabelMismatch,
    GovernanceActorLabelNonCanonical,
    PromotionPayloadMismatch,
    DuplicateRejectionIdentity,
    InvalidGovernanceTransition,
    Utf8AnchorBoundaryViolation,
    MalformedTranscriptRepresentation,
    HistoricalBindingSnapshotMismatch,
    HistoricalBindingAnalysisMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleDiagnosticV3 {
    pub code: OracleViolationCodeV3,
    pub path: String,
    pub message: String,
}

pub fn diagnostic(code: OracleViolationCodeV3, path: &str, message: &str) -> OracleDiagnosticV3 {
    OracleDiagnosticV3 {
        code,
        path: path.to_owned(),
        message: message.to_owned(),
    }
}
