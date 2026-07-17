use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::{self, Write};

use serde::Serialize;

use crate::analysis::{AnalysisSnapshot, SessionTermsIdentity};
use crate::anchor::SourceAnchor;
use crate::calibration::{COMPATIBILITY_POLICY_ID, ComparisonRefusal, ensure_compatible};
use crate::candidate::{DetectionKind, Evidence, PhoneticTargetKind, SessionTermEntry};
use crate::pipeline::{CanonicalTermReviewRun, run_canonical_term_review};
use crate::transcript::Transcript;

pub const SCHEMA_REVISION: &str = "voxproof-calibration-correspondence-v0";
pub const EVALUATION_NOTE: &str = "Deterministic local calibration correspondence artifact only. Not canonical Evidence, certified ground truth, correctness, precision/recall, detector effectiveness, and product validation.";
pub const MAX_LCS_CELLS: u64 = 4_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CalibrationEvaluationReport {
    pub schema_revision: &'static str,
    pub status: &'static str,
    pub compatibility_policy_id: &'static str,
    pub note: &'static str,
    pub local_edit_policy: LocalEditPolicy,
    pub inputs: EvaluationInputs,
    pub analysis_snapshot: AnalysisSnapshotView,
    pub summary: EvaluationSummary,
    pub cues: Vec<EvaluationCueRecord>,
    pub local_edits: Vec<LocalEditRecord>,
    pub review_cases: Vec<ReviewCaseRecord>,
    pub correspondences: Vec<CorrespondenceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalEditPolicy {
    pub id: &'static str,
    pub version: &'static str,
    pub unit: &'static str,
    pub range_unit: &'static str,
    pub tie_break: &'static str,
    pub max_lcs_cells: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvaluationInputs {
    pub raw: TranscriptInputRecord,
    pub comparison_final: ComparisonFinalInputRecord,
    pub session_terms: SessionTermsInputRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TranscriptInputRecord {
    pub path: String,
    pub revision_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComparisonFinalInputRecord {
    pub path: String,
    pub revision_id: String,
    pub provenance_asserted_by_cli: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionTermsInputRecord {
    pub path: String,
    pub entry_count: usize,
    pub identity: String,
    pub ordered_entries: Vec<OrderedSessionTermEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrderedSessionTermEntry {
    pub entry_index: usize,
    pub canonical_term: String,
    pub aliases: Vec<String>,
    pub observed_error_forms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnalysisSnapshotView {
    pub source_revision_id: String,
    pub session_terms_identity: String,
    pub detector_set: Vec<DetectorIdentityView>,
    pub detector_config: IdentityView,
    pub algorithm: IdentityView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DetectorIdentityView {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IdentityView {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvaluationSummary {
    pub cue_count: usize,
    pub unchanged_cue_count: usize,
    pub changed_cue_count: usize,
    pub local_edit_count: usize,
    pub insertion_edit_count: usize,
    pub deletion_edit_count: usize,
    pub replacement_edit_count: usize,
    pub review_case_count: usize,
    pub candidate_on_unchanged_cue_count: usize,
    pub candidate_changed_cue_no_overlap_count: usize,
    pub candidate_edit_overlap_relation_count: usize,
    pub candidate_with_edit_overlap_count: usize,
    pub edit_with_candidate_overlap_count: usize,
    pub exact_anchor_exact_alternative_relation_count: usize,
    pub edit_without_candidate_overlap_count: usize,
    pub multi_candidate_edit_count: usize,
    pub multi_edit_candidate_count: usize,
    pub ambiguous_overlap_component_count: usize,
    pub mechanically_unclassified_edit_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvaluationCueRecord {
    pub segment_position: usize,
    pub cue_index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub change_kind: EvaluationChangeKind,
    pub raw_text: String,
    pub final_text: String,
    pub edit_ids: Vec<String>,
    pub review_case_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationChangeKind {
    Unchanged,
    TextChanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalEditRecord {
    pub edit_id: String,
    pub segment_position: usize,
    pub cue_index: u32,
    pub ordinal_in_cue: usize,
    pub kind: LocalEditKind,
    pub raw: RawEditSide,
    #[serde(rename = "final")]
    pub final_side: FinalEditSide,
    pub term_scope: TermScopeRecord,
    pub overlapping_case_ids: Vec<String>,
    pub exact_structural_correspondence_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalEditKind {
    Insertion,
    Deletion,
    Replacement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawEditSide {
    pub start_byte: usize,
    pub end_byte: usize,
    pub text: String,
    pub source_anchor: Option<SourceAnchorView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insertion_byte: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FinalEditSide {
    pub start_byte: usize,
    pub end_byte: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceAnchorView {
    pub revision_id: String,
    pub segment_position: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TermScopeRecord {
    pub full_region_matches: Vec<TermFormMatch>,
    pub contained_occurrences: Vec<ContainedTermOccurrence>,
    pub canonical_owner_count: usize,
    pub mechanically_unclassified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TermFormMatch {
    pub entry_index: usize,
    pub form_kind: TermFormKind,
    pub form_index: usize,
    pub canonical_owner: String,
    pub form: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContainedTermOccurrence {
    pub start_byte: usize,
    pub end_byte: usize,
    pub entry_index: usize,
    pub form_kind: TermFormKind,
    pub canonical_owner: String,
    pub form: String,
    pub form_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TermFormKind {
    CanonicalTerm,
    Alias,
    ObservedErrorForm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewCaseRecord {
    pub case_id: String,
    pub candidate_key_components: CandidateKeyComponents,
    pub matched_text: String,
    pub detector: DetectorIdentityView,
    pub evidence: EvidenceView,
    pub alternatives: Vec<AlternativeView>,
    pub edit_relation: EditRelationRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidateKeyComponents {
    pub detector_id: String,
    pub detection_kind: String,
    pub source_anchor: SourceAnchorView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AlternativeView {
    pub index: usize,
    pub replacement_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EditRelationRecord {
    pub state: EditRelationState,
    pub edit_ids: Vec<String>,
    pub geometry: Option<OverlapGeometry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EditRelationState {
    OnUnchangedCue,
    ChangedCueNoOverlap,
    SingleEditOverlap,
    MultipleEditOverlap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlapGeometry {
    Exact,
    CandidateContainsEdit,
    EditContainsCandidate,
    PartialOverlap,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum EvidenceView {
    GlossaryAlias {
        matched_form: String,
        canonical_term: String,
    },
    ObservedErrorForm {
        matched_form: String,
        canonical_term: String,
    },
    PhoneticSimilarity {
        observed_surface: String,
        target_surface: String,
        target_kind: String,
        canonical_owner: String,
        source_representation: PhoneticRepresentationView,
        target_representation: PhoneticRepresentationView,
        edit_distance: usize,
        ratio_numerator: usize,
        ratio_denominator: usize,
        ratio_permille: usize,
        matched_key: String,
        detector_config: IdentityView,
        algorithm: IdentityView,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PhoneticRepresentationView {
    pub normalized_letters: String,
    pub primary_key: String,
    pub alternate_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorrespondenceRecord {
    pub correspondence_id: String,
    pub case_id: String,
    pub edit_id: String,
    pub anchor_relation: AnchorRelation,
    pub alternative_relations: Vec<AlternativeRelation>,
    pub exact_anchor_exact_alternative_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorRelation {
    Exact,
    CandidateContainsEdit,
    EditContainsCandidate,
    PartialOverlap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AlternativeRelation {
    pub alternative_index: usize,
    pub replacement_text: String,
    pub final_region_relation: FinalRegionRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalRegionRelation {
    ExactFinalRegion,
    Different,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalibrationEvaluationRefusal {
    RawHasValidationIssues {
        issues: Vec<String>,
    },
    FinalHasValidationIssues {
        issues: Vec<String>,
    },
    CueCountMismatch {
        raw: usize,
        final_count: usize,
    },
    CueIndexMismatch {
        segment_position: usize,
        raw_index: u32,
        final_index: u32,
    },
    StartTimingMismatch {
        segment_position: usize,
        cue_index: u32,
    },
    EndTimingMismatch {
        segment_position: usize,
        cue_index: u32,
    },
    LocalDiffWorkBudgetExceeded {
        segment_position: usize,
        cue_index: u32,
        raw_scalar_count: usize,
        final_scalar_count: usize,
        max_lcs_cells: u64,
    },
    AnalysisFailed {
        message: String,
    },
    BindingFailure {
        message: String,
    },
}

impl CalibrationEvaluationRefusal {
    pub fn message(&self) -> String {
        match self {
            Self::RawHasValidationIssues { issues } => validation_refusal_message(
                "raw SRT has validation issues; evaluation refused",
                issues,
            ),
            Self::FinalHasValidationIssues { issues } => validation_refusal_message(
                "final SRT has validation issues; evaluation refused",
                issues,
            ),
            Self::CueCountMismatch { raw, final_count } => {
                format!("evaluation refused: cue count mismatch (raw: {raw}, final: {final_count})")
            }
            Self::CueIndexMismatch {
                segment_position,
                raw_index,
                final_index,
            } => format!(
                "evaluation refused: cue index mismatch at segment_position {segment_position} (raw: {raw_index}, final: {final_index})"
            ),
            Self::StartTimingMismatch {
                segment_position,
                cue_index,
            } => format!(
                "evaluation refused: start timing mismatch at segment_position {segment_position} cue_index {cue_index}"
            ),
            Self::EndTimingMismatch {
                segment_position,
                cue_index,
            } => format!(
                "evaluation refused: end timing mismatch at segment_position {segment_position} cue_index {cue_index}"
            ),
            Self::LocalDiffWorkBudgetExceeded {
                segment_position,
                cue_index,
                raw_scalar_count,
                final_scalar_count,
                max_lcs_cells,
            } => format!(
                "evaluation refused: local diff work budget exceeded at segment_position {segment_position} cue_index {cue_index} (raw_scalar_count: {raw_scalar_count}, final_scalar_count: {final_scalar_count}, max_lcs_cells: {max_lcs_cells})"
            ),
            Self::AnalysisFailed { message } => format!("evaluation refused: {message}"),
            Self::BindingFailure { message } => format!("evaluation refused: {message}"),
        }
    }
}

impl From<ComparisonRefusal> for CalibrationEvaluationRefusal {
    fn from(refusal: ComparisonRefusal) -> Self {
        match refusal {
            ComparisonRefusal::RawHasValidationIssues { issues } => {
                Self::RawHasValidationIssues { issues }
            }
            ComparisonRefusal::FinalHasValidationIssues { issues } => {
                Self::FinalHasValidationIssues { issues }
            }
            ComparisonRefusal::CueCountMismatch { raw, final_count } => {
                Self::CueCountMismatch { raw, final_count }
            }
            ComparisonRefusal::CueIndexMismatch {
                segment_position,
                raw_index,
                final_index,
            } => Self::CueIndexMismatch {
                segment_position,
                raw_index,
                final_index,
            },
            ComparisonRefusal::StartTimingMismatch {
                segment_position,
                cue_index,
            } => Self::StartTimingMismatch {
                segment_position,
                cue_index,
            },
            ComparisonRefusal::EndTimingMismatch {
                segment_position,
                cue_index,
            } => Self::EndTimingMismatch {
                segment_position,
                cue_index,
            },
        }
    }
}

fn validation_refusal_message(heading: &str, issues: &[String]) -> String {
    let mut message = heading.to_string();
    for line in issues {
        message.push('\n');
        message.push_str(line);
    }
    message
}

struct ScalarText {
    scalars: Vec<char>,
    byte_starts: Vec<usize>,
}

impl ScalarText {
    fn from_str(text: &str) -> Self {
        let scalars: Vec<char> = text.chars().collect();
        let mut byte_starts: Vec<usize> = text.char_indices().map(|(index, _)| index).collect();
        byte_starts.push(text.len());
        Self {
            scalars,
            byte_starts,
        }
    }

    fn len(&self) -> usize {
        self.scalars.len()
    }

    fn byte_range(&self, start_scalar: usize, end_scalar: usize) -> (usize, usize) {
        (self.byte_starts[start_scalar], self.byte_starts[end_scalar])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalEditDraft {
    segment_position: usize,
    cue_index: u32,
    ordinal_in_cue: usize,
    kind: LocalEditKind,
    raw_start_scalar: usize,
    raw_end_scalar: usize,
    final_start_scalar: usize,
    final_end_scalar: usize,
}

fn check_work_budget(
    segment_position: usize,
    cue_index: u32,
    raw_scalar_count: usize,
    final_scalar_count: usize,
) -> Result<(), CalibrationEvaluationRefusal> {
    match (raw_scalar_count as u64).checked_mul(final_scalar_count as u64) {
        Some(cells) if cells <= MAX_LCS_CELLS => Ok(()),
        Some(_) | None => Err(CalibrationEvaluationRefusal::LocalDiffWorkBudgetExceeded {
            segment_position,
            cue_index,
            raw_scalar_count,
            final_scalar_count,
            max_lcs_cells: MAX_LCS_CELLS,
        }),
    }
}

fn lcs_row(a: &[char], b: &[char]) -> Vec<usize> {
    let mut previous = vec![0usize; b.len() + 1];
    let mut current = vec![0usize; b.len() + 1];

    for &left in a {
        for (index, &right) in b.iter().enumerate() {
            current[index + 1] = if left == right {
                previous[index] + 1
            } else {
                previous[index + 1].max(current[index])
            };
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous
}

fn hirschberg_matches(a: &[char], b: &[char]) -> Vec<(usize, usize)> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }

    if a.len() == 1 {
        for (index, &right) in b.iter().enumerate() {
            if a[0] == right {
                return vec![(0, index)];
            }
        }
        return Vec::new();
    }

    let mid = a.len() / 2;
    let left_a = &a[..mid];
    let right_a = &a[mid..];

    let forward = lcs_row(left_a, b);
    let backward = {
        let reversed_left: Vec<char> = right_a.iter().rev().copied().collect();
        let reversed_b: Vec<char> = b.iter().rev().copied().collect();
        lcs_row(&reversed_left, &reversed_b)
    };

    let mut best_score = 0usize;
    let mut best_split = 0usize;
    for split in 0..=b.len() {
        let score = forward[split] + backward[b.len() - split];
        if score > best_score {
            best_score = score;
            best_split = split;
        } else if score == best_score && split < best_split {
            best_split = split;
        }
    }

    let mut left_matches = hirschberg_matches(left_a, &b[..best_split]);
    let mut right_matches = hirschberg_matches(right_a, &b[best_split..]);
    for (raw_index, final_index) in &mut right_matches {
        *raw_index += mid;
        *final_index += best_split;
    }
    left_matches.append(&mut right_matches);
    left_matches
}

fn edits_from_matches(
    raw: &ScalarText,
    final_text: &ScalarText,
    matches: &[(usize, usize)],
) -> Vec<LocalEditDraft> {
    let mut edits = Vec::new();
    let mut raw_cursor = 0usize;
    let mut final_cursor = 0usize;
    let mut ordinal = 0usize;

    let flush = |raw_start: usize,
                 raw_end: usize,
                 final_start: usize,
                 final_end: usize,
                 ordinal: &mut usize,
                 edits: &mut Vec<LocalEditDraft>| {
        if raw_start == raw_end && final_start == final_end {
            return;
        }
        let kind = match (raw_start == raw_end, final_start == final_end) {
            (true, false) => LocalEditKind::Insertion,
            (false, true) => LocalEditKind::Deletion,
            (false, false) => LocalEditKind::Replacement,
            (true, true) => return,
        };
        edits.push(LocalEditDraft {
            segment_position: 0,
            cue_index: 0,
            ordinal_in_cue: *ordinal,
            kind,
            raw_start_scalar: raw_start,
            raw_end_scalar: raw_end,
            final_start_scalar: final_start,
            final_end_scalar: final_end,
        });
        *ordinal += 1;
    };

    for &(raw_index, final_index) in matches {
        flush(
            raw_cursor,
            raw_index,
            final_cursor,
            final_index,
            &mut ordinal,
            &mut edits,
        );
        raw_cursor = raw_index + 1;
        final_cursor = final_index + 1;
    }

    flush(
        raw_cursor,
        raw.len(),
        final_cursor,
        final_text.len(),
        &mut ordinal,
        &mut edits,
    );

    edits
}

fn compute_local_edits(
    segment_position: usize,
    cue_index: u32,
    raw_text: &str,
    final_text: &str,
) -> Result<Vec<LocalEditDraft>, CalibrationEvaluationRefusal> {
    let raw = ScalarText::from_str(raw_text);
    let final_side = ScalarText::from_str(final_text);
    check_work_budget(segment_position, cue_index, raw.len(), final_side.len())?;

    if raw_text == final_text {
        return Ok(Vec::new());
    }

    let matches = hirschberg_matches(&raw.scalars, &final_side.scalars);
    let mut edits = edits_from_matches(&raw, &final_side, &matches);
    for edit in &mut edits {
        edit.segment_position = segment_position;
        edit.cue_index = cue_index;
    }
    Ok(edits)
}

fn overlap_geometry(
    candidate_start: usize,
    candidate_end: usize,
    edit_start: usize,
    edit_end: usize,
) -> OverlapGeometry {
    if candidate_start == edit_start && candidate_end == edit_end {
        OverlapGeometry::Exact
    } else if candidate_start <= edit_start && candidate_end >= edit_end {
        OverlapGeometry::CandidateContainsEdit
    } else if edit_start <= candidate_start && edit_end >= candidate_end {
        OverlapGeometry::EditContainsCandidate
    } else {
        OverlapGeometry::PartialOverlap
    }
}

fn ranges_overlap(start_a: usize, end_a: usize, start_b: usize, end_b: usize) -> bool {
    start_a < end_b && start_b < end_a
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum OverlapNode {
    Case(usize),
    Edit(usize),
}

fn form_kind_rank(kind: TermFormKind) -> u8 {
    match kind {
        TermFormKind::CanonicalTerm => 0,
        TermFormKind::Alias => 1,
        TermFormKind::ObservedErrorForm => 2,
    }
}

#[derive(Clone)]
struct TermFormRef<'a> {
    entry_index: usize,
    form_kind: TermFormKind,
    form_index: usize,
    canonical_owner: &'a str,
    form: &'a str,
}

fn collect_term_forms<'a>(entries: &'a [SessionTermEntry]) -> Vec<TermFormRef<'a>> {
    let mut forms = Vec::new();
    for (entry_index, entry) in entries.iter().enumerate() {
        if !entry.canonical_term.is_empty() {
            forms.push(TermFormRef {
                entry_index,
                form_kind: TermFormKind::CanonicalTerm,
                form_index: 0,
                canonical_owner: &entry.canonical_term,
                form: &entry.canonical_term,
            });
        }
        for (form_index, alias) in entry.aliases.iter().enumerate() {
            if !alias.is_empty() {
                forms.push(TermFormRef {
                    entry_index,
                    form_kind: TermFormKind::Alias,
                    form_index,
                    canonical_owner: &entry.canonical_term,
                    form: alias,
                });
            }
        }
        for (form_index, observed) in entry.observed_error_forms.iter().enumerate() {
            if !observed.is_empty() {
                forms.push(TermFormRef {
                    entry_index,
                    form_kind: TermFormKind::ObservedErrorForm,
                    form_index,
                    canonical_owner: &entry.canonical_term,
                    form: observed,
                });
            }
        }
    }
    forms
}

fn full_region_matches(final_text: &str, entries: &[SessionTermEntry]) -> Vec<TermFormMatch> {
    let mut matches = Vec::new();
    for form in collect_term_forms(entries) {
        if !form.form.is_empty() && final_text == form.form {
            matches.push(TermFormMatch {
                entry_index: form.entry_index,
                form_kind: form.form_kind,
                form_index: form.form_index,
                canonical_owner: form.canonical_owner.to_string(),
                form: form.form.to_string(),
            });
        }
    }
    matches.sort_by(|left, right| {
        (
            left.entry_index,
            form_kind_rank(left.form_kind),
            left.form_index,
        )
            .cmp(&(
                right.entry_index,
                form_kind_rank(right.form_kind),
                right.form_index,
            ))
    });
    matches
}

fn contained_occurrences(
    final_region_text: &str,
    final_region_start_byte: usize,
    entries: &[SessionTermEntry],
) -> Vec<ContainedTermOccurrence> {
    let mut occurrences = Vec::new();
    let char_boundaries: Vec<usize> = std::iter::once(0)
        .chain(
            final_region_text
                .char_indices()
                .skip(1)
                .map(|(index, _)| index),
        )
        .collect();

    for start_offset in char_boundaries {
        let slice = &final_region_text[start_offset..];
        for form in collect_term_forms(entries) {
            if !form.form.is_empty() && slice.starts_with(form.form) {
                let end_offset = start_offset + form.form.len();
                occurrences.push(ContainedTermOccurrence {
                    start_byte: final_region_start_byte + start_offset,
                    end_byte: final_region_start_byte + end_offset,
                    entry_index: form.entry_index,
                    form_kind: form.form_kind,
                    canonical_owner: form.canonical_owner.to_string(),
                    form: form.form.to_string(),
                    form_index: form.form_index,
                });
            }
        }
    }

    occurrences.sort_by(|left, right| {
        (
            left.start_byte,
            left.end_byte,
            left.entry_index,
            form_kind_rank(left.form_kind),
            left.form_index,
        )
            .cmp(&(
                right.start_byte,
                right.end_byte,
                right.entry_index,
                form_kind_rank(right.form_kind),
                right.form_index,
            ))
    });
    occurrences
}

fn case_id_string(local_index: usize) -> String {
    format!("local:{local_index}")
}

fn edit_id_string(segment_position: usize, ordinal_in_cue: usize) -> String {
    format!("cue:{segment_position}:edit:{ordinal_in_cue}")
}

fn correspondence_id_string(case_local_index: usize, edit_id: &str) -> String {
    format!("case:{case_local_index}/{edit_id}")
}

fn source_anchor_view(anchor: &SourceAnchor) -> SourceAnchorView {
    SourceAnchorView {
        revision_id: anchor.revision.to_tagged_string(),
        segment_position: anchor.segment_position(),
        start_byte: anchor.start_byte,
        end_byte: anchor.end_byte,
    }
}

fn detection_kind_string(kind: DetectionKind) -> &'static str {
    match kind {
        DetectionKind::GlossaryAliasMatch => "glossary_alias_match",
        DetectionKind::MixedLanguageAnomaly => "mixed_language_anomaly",
        DetectionKind::PhoneticSimilarity => "phonetic_similarity",
        DetectionKind::RepeatedPhrase => "repeated_phrase",
    }
}

fn evidence_view(evidence: &Evidence) -> EvidenceView {
    match evidence {
        Evidence::GlossaryAlias(value) => EvidenceView::GlossaryAlias {
            matched_form: value.matched_form.clone(),
            canonical_term: value.entry.canonical_term.clone(),
        },
        Evidence::ObservedErrorForm(value) => EvidenceView::ObservedErrorForm {
            matched_form: value.matched_form.clone(),
            canonical_term: value.entry.canonical_term.clone(),
        },
        Evidence::PhoneticSimilarity(value) => EvidenceView::PhoneticSimilarity {
            observed_surface: value.observed_surface.clone(),
            target_surface: value.target_surface.clone(),
            target_kind: match value.target_kind {
                PhoneticTargetKind::CanonicalTerm => "canonical_term".to_string(),
                PhoneticTargetKind::Alias => "alias".to_string(),
            },
            canonical_owner: value.canonical_term.clone(),
            source_representation: PhoneticRepresentationView {
                normalized_letters: value.source_representation.normalized_letters.clone(),
                primary_key: value.source_representation.primary_key.clone(),
                alternate_key: value.source_representation.alternate_key.clone(),
            },
            target_representation: PhoneticRepresentationView {
                normalized_letters: value.target_representation.normalized_letters.clone(),
                primary_key: value.target_representation.primary_key.clone(),
                alternate_key: value.target_representation.alternate_key.clone(),
            },
            edit_distance: value.comparison.edit_distance,
            ratio_numerator: value.comparison.ratio_numerator,
            ratio_denominator: value.comparison.ratio_denominator,
            ratio_permille: value.comparison.ratio_permille,
            matched_key: value.comparison.matched_key.clone(),
            detector_config: IdentityView {
                id: value.detector_config.id().to_string(),
                version: value.detector_config.version().to_string(),
            },
            algorithm: IdentityView {
                id: value.algorithm.id().to_string(),
                version: value.algorithm.version().to_string(),
            },
        },
    }
}

fn snapshot_view(snapshot: AnalysisSnapshot) -> AnalysisSnapshotView {
    let configuration = snapshot.configuration();
    AnalysisSnapshotView {
        source_revision_id: snapshot.source_revision().to_tagged_string(),
        session_terms_identity: snapshot.session_terms().to_tagged_string(),
        detector_set: configuration
            .detector_set()
            .detectors()
            .iter()
            .map(|detector| DetectorIdentityView {
                id: detector.id().to_string(),
                version: detector.version().to_string(),
            })
            .collect(),
        detector_config: IdentityView {
            id: configuration.detector_config().id().to_string(),
            version: configuration.detector_config().version().to_string(),
        },
        algorithm: IdentityView {
            id: configuration.algorithm().id().to_string(),
            version: configuration.algorithm().version().to_string(),
        },
    }
}

fn ordered_session_term_entries(entries: &[SessionTermEntry]) -> Vec<OrderedSessionTermEntry> {
    entries
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| OrderedSessionTermEntry {
            entry_index,
            canonical_term: entry.canonical_term.clone(),
            aliases: entry.aliases.clone(),
            observed_error_forms: entry.observed_error_forms.clone(),
        })
        .collect()
}

pub fn evaluate_calibration_report(
    raw: &Transcript,
    final_transcript: &Transcript,
    entries: &[SessionTermEntry],
    raw_path: &str,
    final_path: &str,
    terms_path: &str,
) -> Result<CalibrationEvaluationReport, CalibrationEvaluationRefusal> {
    ensure_compatible(raw, final_transcript).map_err(CalibrationEvaluationRefusal::from)?;
    let review_run = run_canonical_term_review(raw, entries).map_err(map_detection_error)?;
    build_report_from_owned_run(
        raw,
        final_transcript,
        entries,
        raw_path,
        final_path,
        terms_path,
        &review_run,
    )
}

fn verify_run_bindings(
    raw: &Transcript,
    entries: &[SessionTermEntry],
    review_run: &CanonicalTermReviewRun,
) -> Result<(), CalibrationEvaluationRefusal> {
    let snapshot = review_run.analysis_run().snapshot();
    let raw_revision = raw.revision_id();
    let terms_identity = SessionTermsIdentity::from_entries(entries);

    if snapshot.source_revision() != raw_revision {
        return Err(CalibrationEvaluationRefusal::BindingFailure {
            message: format!(
                "analysis snapshot source revision {:?} does not match raw revision {:?}",
                snapshot.source_revision(),
                raw_revision
            ),
        });
    }

    if snapshot.session_terms() != terms_identity {
        return Err(CalibrationEvaluationRefusal::BindingFailure {
            message: format!(
                "analysis snapshot session terms identity {:?} does not match effective entries identity {:?}",
                snapshot.session_terms(),
                terms_identity
            ),
        });
    }

    for review_case in review_run.review_cases() {
        let anchor = review_case.candidate_span().anchor();
        if anchor.revision != raw_revision {
            return Err(CalibrationEvaluationRefusal::BindingFailure {
                message: format!(
                    "review case {} anchor revision does not match raw revision",
                    review_case.id().local_index()
                ),
            });
        }
        raw.resolve(anchor)
            .ok_or_else(|| CalibrationEvaluationRefusal::BindingFailure {
                message: format!(
                    "review case {} anchor failed to resolve against raw transcript",
                    review_case.id().local_index()
                ),
            })?;
    }

    Ok(())
}

fn build_report_from_owned_run(
    raw: &Transcript,
    final_transcript: &Transcript,
    entries: &[SessionTermEntry],
    raw_path: &str,
    final_path: &str,
    terms_path: &str,
    review_run: &CanonicalTermReviewRun,
) -> Result<CalibrationEvaluationReport, CalibrationEvaluationRefusal> {
    verify_run_bindings(raw, entries, review_run)?;

    let session_terms_identity = SessionTermsIdentity::from_entries(entries);

    let mut cue_records = Vec::new();
    let mut local_edit_records = Vec::new();
    let mut changed_cue_positions: HashSet<usize> = HashSet::new();

    for (segment_position, (raw_segment, final_segment)) in raw
        .segments()
        .iter()
        .zip(final_transcript.segments().iter())
        .enumerate()
    {
        let change_kind = if raw_segment.text() == final_segment.text() {
            EvaluationChangeKind::Unchanged
        } else {
            changed_cue_positions.insert(segment_position);
            EvaluationChangeKind::TextChanged
        };

        let mut edit_ids = Vec::new();
        if matches!(change_kind, EvaluationChangeKind::TextChanged) {
            let drafts = compute_local_edits(
                segment_position,
                raw_segment.index(),
                raw_segment.text(),
                final_segment.text(),
            )?;
            let raw_scalar = ScalarText::from_str(raw_segment.text());
            let final_scalar = ScalarText::from_str(final_segment.text());

            for draft in drafts {
                let edit_id = edit_id_string(segment_position, draft.ordinal_in_cue);
                edit_ids.push(edit_id.clone());

                let (raw_start_byte, raw_end_byte) =
                    raw_scalar.byte_range(draft.raw_start_scalar, draft.raw_end_scalar);
                let (final_start_byte, final_end_byte) =
                    final_scalar.byte_range(draft.final_start_scalar, draft.final_end_scalar);

                let raw_text = if draft.kind == LocalEditKind::Insertion {
                    String::new()
                } else {
                    raw_segment.text()[raw_start_byte..raw_end_byte].to_string()
                };
                let final_text = if draft.kind == LocalEditKind::Deletion {
                    String::new()
                } else {
                    final_segment.text()[final_start_byte..final_end_byte].to_string()
                };

                let source_anchor = if draft.kind == LocalEditKind::Insertion {
                    None
                } else {
                    Some(source_anchor_view(
                        &raw.anchor(segment_position, raw_start_byte, raw_end_byte)
                            .expect("local edit raw range resolves to anchor"),
                    ))
                };

                let insertion_byte = if draft.kind == LocalEditKind::Insertion {
                    Some(raw_start_byte)
                } else {
                    None
                };

                let full_matches = full_region_matches(&final_text, entries);
                let contained = contained_occurrences(&final_text, final_start_byte, entries);

                let mut owners = BTreeSet::new();
                for item in &full_matches {
                    owners.insert(item.canonical_owner.as_str());
                }
                for item in &contained {
                    owners.insert(item.canonical_owner.as_str());
                }
                let canonical_owner_count = owners.len();

                local_edit_records.push(LocalEditRecord {
                    edit_id: edit_id.clone(),
                    segment_position,
                    cue_index: raw_segment.index(),
                    ordinal_in_cue: draft.ordinal_in_cue,
                    kind: draft.kind,
                    raw: RawEditSide {
                        start_byte: raw_start_byte,
                        end_byte: raw_end_byte,
                        text: raw_text,
                        source_anchor,
                        insertion_byte,
                    },
                    final_side: FinalEditSide {
                        start_byte: final_start_byte,
                        end_byte: final_end_byte,
                        text: final_text.clone(),
                    },
                    term_scope: TermScopeRecord {
                        full_region_matches: full_matches,
                        contained_occurrences: contained,
                        canonical_owner_count,
                        mechanically_unclassified: false,
                    },
                    overlapping_case_ids: Vec::new(),
                    exact_structural_correspondence_ids: Vec::new(),
                });
            }
        }

        cue_records.push(EvaluationCueRecord {
            segment_position,
            cue_index: raw_segment.index(),
            start_ms: raw_segment.start_ms(),
            end_ms: raw_segment.end_ms(),
            change_kind,
            raw_text: raw_segment.text().to_string(),
            final_text: final_segment.text().to_string(),
            edit_ids,
            review_case_ids: Vec::new(),
        });
    }

    let mut review_case_records = Vec::new();
    let mut correspondences = Vec::new();
    let mut overlap_edges: BTreeSet<(OverlapNode, OverlapNode)> = BTreeSet::new();
    let mut exact_alt_relations: BTreeSet<(String, String, usize)> = BTreeSet::new();

    for review_case in review_run.review_cases() {
        let local_index = review_case.id().local_index();
        let case_id = case_id_string(local_index);
        let candidate = review_case.candidate_span();
        let anchor = candidate.anchor();
        let segment_position = anchor.segment_position();
        let matched_text =
            raw.resolve(anchor)
                .ok_or_else(|| CalibrationEvaluationRefusal::BindingFailure {
                    message: format!(
                        "review case {local_index} anchor failed to resolve during report render"
                    ),
                })?;

        if let Some(cue) = cue_records
            .iter_mut()
            .find(|cue| cue.segment_position == segment_position)
        {
            cue.review_case_ids.push(case_id.clone());
        }

        let edits_in_cue: Vec<(usize, &LocalEditRecord)> = local_edit_records
            .iter()
            .enumerate()
            .filter(|(_, edit)| edit.segment_position == segment_position)
            .collect();

        let mut overlapping_edit_ids = Vec::new();
        let mut geometry: Option<OverlapGeometry> = None;

        for (edit_index, edit) in &edits_in_cue {
            if edit.kind == LocalEditKind::Insertion {
                continue;
            }

            if ranges_overlap(
                anchor.start_byte,
                anchor.end_byte,
                edit.raw.start_byte,
                edit.raw.end_byte,
            ) {
                overlapping_edit_ids.push(edit.edit_id.clone());
                overlap_edges.insert((
                    OverlapNode::Case(local_index),
                    OverlapNode::Edit(*edit_index),
                ));

                let current_geometry = overlap_geometry(
                    anchor.start_byte,
                    anchor.end_byte,
                    edit.raw.start_byte,
                    edit.raw.end_byte,
                );
                geometry = Some(match geometry {
                    None => current_geometry,
                    Some(existing) if existing == current_geometry => existing,
                    Some(_) => OverlapGeometry::PartialOverlap,
                });

                let anchor_relation = match current_geometry {
                    OverlapGeometry::Exact => AnchorRelation::Exact,
                    OverlapGeometry::CandidateContainsEdit => AnchorRelation::CandidateContainsEdit,
                    OverlapGeometry::EditContainsCandidate => AnchorRelation::EditContainsCandidate,
                    OverlapGeometry::PartialOverlap => AnchorRelation::PartialOverlap,
                };

                let mut exact_indices = Vec::new();
                let alternative_relations = candidate
                    .alternatives()
                    .iter()
                    .enumerate()
                    .map(|(index, alternative)| {
                        let exact = current_geometry == OverlapGeometry::Exact
                            && alternative.replacement_text() == edit.final_side.text;
                        if exact {
                            exact_indices.push(index);
                            exact_alt_relations.insert((
                                case_id.clone(),
                                edit.edit_id.clone(),
                                index,
                            ));
                        }
                        AlternativeRelation {
                            alternative_index: index,
                            replacement_text: alternative.replacement_text().to_string(),
                            final_region_relation: if alternative.replacement_text()
                                == edit.final_side.text
                            {
                                FinalRegionRelation::ExactFinalRegion
                            } else {
                                FinalRegionRelation::Different
                            },
                        }
                    })
                    .collect::<Vec<_>>();

                correspondences.push(CorrespondenceRecord {
                    correspondence_id: correspondence_id_string(local_index, &edit.edit_id),
                    case_id: case_id.clone(),
                    edit_id: edit.edit_id.clone(),
                    anchor_relation,
                    alternative_relations,
                    exact_anchor_exact_alternative_indices: exact_indices,
                });
            }
        }

        let state = if !changed_cue_positions.contains(&segment_position) {
            EditRelationState::OnUnchangedCue
        } else if overlapping_edit_ids.is_empty() {
            EditRelationState::ChangedCueNoOverlap
        } else if overlapping_edit_ids.len() == 1 {
            EditRelationState::SingleEditOverlap
        } else {
            EditRelationState::MultipleEditOverlap
        };

        review_case_records.push(ReviewCaseRecord {
            case_id: case_id.clone(),
            candidate_key_components: CandidateKeyComponents {
                detector_id: candidate.provenance().detector_id().to_string(),
                detection_kind: detection_kind_string(candidate.kind()).to_string(),
                source_anchor: source_anchor_view(anchor),
            },
            matched_text: matched_text.to_string(),
            detector: DetectorIdentityView {
                id: candidate.provenance().detector_id().to_string(),
                version: candidate.provenance().detector_version().to_string(),
            },
            evidence: evidence_view(candidate.evidence()),
            alternatives: candidate
                .alternatives()
                .iter()
                .enumerate()
                .map(|(index, alternative)| AlternativeView {
                    index,
                    replacement_text: alternative.replacement_text().to_string(),
                })
                .collect(),
            edit_relation: EditRelationRecord {
                state,
                edit_ids: overlapping_edit_ids,
                geometry,
            },
        });
    }

    for (left, right) in &overlap_edges {
        let (case_local, edit_index) = match (left, right) {
            (OverlapNode::Case(local_index), OverlapNode::Edit(edit_index)) => {
                (*local_index, *edit_index)
            }
            (OverlapNode::Edit(edit_index), OverlapNode::Case(local_index)) => {
                (*local_index, *edit_index)
            }
            _ => continue,
        };
        local_edit_records[edit_index]
            .overlapping_case_ids
            .push(case_id_string(case_local));
    }
    for edit in &mut local_edit_records {
        edit.overlapping_case_ids.sort();
        edit.overlapping_case_ids.dedup();
    }

    for edit in &mut local_edit_records {
        edit.exact_structural_correspondence_ids = correspondences
            .iter()
            .filter(|record| {
                record.edit_id == edit.edit_id
                    && !record.exact_anchor_exact_alternative_indices.is_empty()
            })
            .map(|record| record.correspondence_id.clone())
            .collect();

        let has_candidate_overlap = !edit.overlapping_case_ids.is_empty();
        let has_term_signal = !edit.term_scope.full_region_matches.is_empty()
            || !edit.term_scope.contained_occurrences.is_empty();
        edit.term_scope.mechanically_unclassified = !has_candidate_overlap && !has_term_signal;
    }

    correspondences.sort_by(|left, right| {
        let left_case = left
            .case_id
            .strip_prefix("local:")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(usize::MAX);
        let right_case = right
            .case_id
            .strip_prefix("local:")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(usize::MAX);
        (left_case, left.edit_id.as_str()).cmp(&(right_case, right.edit_id.as_str()))
    });

    let mut edit_degrees: HashMap<usize, usize> = HashMap::new();
    let mut case_degrees: HashMap<usize, usize> = HashMap::new();
    for (left, right) in &overlap_edges {
        let (case_local, edit_index) = match (left, right) {
            (OverlapNode::Case(local_index), OverlapNode::Edit(edit_index)) => {
                (*local_index, *edit_index)
            }
            (OverlapNode::Edit(edit_index), OverlapNode::Case(local_index)) => {
                (*local_index, *edit_index)
            }
            _ => continue,
        };
        *case_degrees.entry(case_local).or_default() += 1;
        *edit_degrees.entry(edit_index).or_default() += 1;
    }

    let ambiguous_component_count =
        count_ambiguous_overlap_components(&overlap_edges, &case_degrees, &edit_degrees);

    let unchanged_cue_count = cue_records
        .iter()
        .filter(|cue| cue.change_kind == EvaluationChangeKind::Unchanged)
        .count();
    let changed_cue_count = cue_records.len() - unchanged_cue_count;
    let insertion_edit_count = local_edit_records
        .iter()
        .filter(|edit| edit.kind == LocalEditKind::Insertion)
        .count();
    let deletion_edit_count = local_edit_records
        .iter()
        .filter(|edit| edit.kind == LocalEditKind::Deletion)
        .count();
    let replacement_edit_count = local_edit_records
        .iter()
        .filter(|edit| edit.kind == LocalEditKind::Replacement)
        .count();

    let candidate_on_unchanged_cue_count = review_case_records
        .iter()
        .filter(|record| record.edit_relation.state == EditRelationState::OnUnchangedCue)
        .count();
    let candidate_changed_cue_no_overlap_count = review_case_records
        .iter()
        .filter(|record| record.edit_relation.state == EditRelationState::ChangedCueNoOverlap)
        .count();

    let candidate_with_edit_overlap_count = case_degrees.len();
    let edit_with_candidate_overlap_count = edit_degrees.len();
    let edit_without_candidate_overlap_count =
        local_edit_records.len() - edit_with_candidate_overlap_count;
    let multi_candidate_edit_count = edit_degrees.values().filter(|degree| **degree > 1).count();
    let multi_edit_candidate_count = case_degrees.values().filter(|degree| **degree > 1).count();
    let mechanically_unclassified_edit_count = local_edit_records
        .iter()
        .filter(|edit| edit.term_scope.mechanically_unclassified)
        .count();

    Ok(CalibrationEvaluationReport {
        schema_revision: SCHEMA_REVISION,
        status: "complete",
        compatibility_policy_id: COMPATIBILITY_POLICY_ID,
        note: EVALUATION_NOTE,
        local_edit_policy: LocalEditPolicy {
            id: "unicode-scalar-hirschberg-lcs",
            version: "1",
            unit: "unicode_scalar_value",
            range_unit: "utf8_byte_half_open_in_parsed_cue_text",
            tie_break: "raw_midpoint_then_smallest_final_split_then_earliest_base_match",
            max_lcs_cells: MAX_LCS_CELLS,
        },
        inputs: EvaluationInputs {
            raw: TranscriptInputRecord {
                path: raw_path.to_string(),
                revision_id: raw.revision_id().to_tagged_string(),
            },
            comparison_final: ComparisonFinalInputRecord {
                path: final_path.to_string(),
                revision_id: final_transcript.revision_id().to_tagged_string(),
                provenance_asserted_by_cli: false,
            },
            session_terms: SessionTermsInputRecord {
                path: terms_path.to_string(),
                entry_count: entries.len(),
                identity: session_terms_identity.to_tagged_string(),
                ordered_entries: ordered_session_term_entries(entries),
            },
        },
        analysis_snapshot: snapshot_view(review_run.analysis_run().snapshot()),
        summary: EvaluationSummary {
            cue_count: cue_records.len(),
            unchanged_cue_count,
            changed_cue_count,
            local_edit_count: local_edit_records.len(),
            insertion_edit_count,
            deletion_edit_count,
            replacement_edit_count,
            review_case_count: review_case_records.len(),
            candidate_on_unchanged_cue_count,
            candidate_changed_cue_no_overlap_count,
            candidate_edit_overlap_relation_count: overlap_edges.len(),
            candidate_with_edit_overlap_count,
            edit_with_candidate_overlap_count,
            exact_anchor_exact_alternative_relation_count: exact_alt_relations.len(),
            edit_without_candidate_overlap_count,
            multi_candidate_edit_count,
            multi_edit_candidate_count,
            ambiguous_overlap_component_count: ambiguous_component_count,
            mechanically_unclassified_edit_count,
        },
        cues: cue_records,
        local_edits: local_edit_records,
        review_cases: review_case_records,
        correspondences,
    })
}

fn count_ambiguous_overlap_components(
    overlap_edges: &BTreeSet<(OverlapNode, OverlapNode)>,
    case_degrees: &HashMap<usize, usize>,
    edit_degrees: &HashMap<usize, usize>,
) -> usize {
    if overlap_edges.is_empty() {
        return 0;
    }

    let mut nodes: BTreeSet<OverlapNode> = BTreeSet::new();
    for (left, right) in overlap_edges {
        nodes.insert(*left);
        nodes.insert(*right);
    }

    let mut parent: HashMap<OverlapNode, OverlapNode> =
        nodes.iter().map(|node| (*node, *node)).collect();

    fn find(parent: &mut HashMap<OverlapNode, OverlapNode>, node: OverlapNode) -> OverlapNode {
        let current = parent.get(&node).copied().unwrap_or(node);
        if current != node {
            let root = find(parent, current);
            parent.insert(node, root);
            root
        } else {
            current
        }
    }

    fn union(
        parent: &mut HashMap<OverlapNode, OverlapNode>,
        left: OverlapNode,
        right: OverlapNode,
    ) {
        let left_root = find(parent, left);
        let right_root = find(parent, right);
        if left_root != right_root {
            parent.insert(right_root, left_root);
        }
    }

    for (left, right) in overlap_edges {
        union(&mut parent, *left, *right);
    }

    let mut components: HashMap<OverlapNode, Vec<OverlapNode>> = HashMap::new();
    for node in nodes {
        components
            .entry(find(&mut parent, node))
            .or_default()
            .push(node);
    }

    components
        .values()
        .filter(|members| {
            members
                .iter()
                .filter_map(|node| match node {
                    OverlapNode::Case(local_index) => case_degrees.get(local_index).copied(),
                    OverlapNode::Edit(edit_index) => edit_degrees.get(edit_index).copied(),
                })
                .max()
                .unwrap_or(0)
                > 1
        })
        .count()
}

pub fn render_evaluation_report(
    report: &CalibrationEvaluationReport,
) -> Result<String, serde_json::Error> {
    let mut json = serde_json::to_string_pretty(report)?;
    json.push('\n');
    Ok(json)
}

pub fn write_evaluation_report_exclusive(path: &str, json: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                format!("refused to write evaluation report: destination already exists: {path}")
            } else {
                format!("failed to create evaluation report: {error}")
            }
        })?;

    if let Err(error) = file.write_all(json.as_bytes()) {
        drop(file);
        if let Err(cleanup_error) = std::fs::remove_file(path) {
            return Err(format!(
                "failed to write evaluation report: {error}; failed to remove partial destination: {cleanup_error}; partial destination may remain: {path}"
            ));
        }
        return Err(format!("failed to write evaluation report: {error}"));
    }

    Ok(())
}

pub fn map_detection_error(
    error: crate::candidate::DetectionError,
) -> CalibrationEvaluationRefusal {
    CalibrationEvaluationRefusal::AnalysisFailed {
        message: format!("{error:?}"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use super::*;
    use crate::analysis::AnalysisRun;
    use crate::candidate::{
        CandidateAlternative, CandidateSpan, DetectionKind, DetectorProvenance, Evidence,
        GlossaryAliasEvidence, ObservedErrorFormEvidence, SessionTermEntry,
    };
    use crate::pipeline::{CanonicalTermReviewRun, run_canonical_term_review};
    use crate::review::ReviewCase;
    use crate::srt::parse_srt;

    fn cue(index: u32, text: &str) -> String {
        format!("{index}\n00:00:00,000 --> 00:00:01,000\n{text}")
    }

    fn parse_pair(raw: &str, final_srt: &str) -> (Transcript, Transcript) {
        (
            parse_srt(raw).expect("raw parse"),
            parse_srt(final_srt).expect("final parse"),
        )
    }

    fn entry(canonical: &str, aliases: &[&str], errors: &[&str]) -> SessionTermEntry {
        SessionTermEntry::new(
            canonical,
            aliases.iter().map(|value| (*value).to_string()).collect(),
            errors.iter().map(|value| (*value).to_string()).collect(),
        )
    }

    fn build_report(
        raw_srt: &str,
        final_srt: &str,
        entries: &[SessionTermEntry],
    ) -> CalibrationEvaluationReport {
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        evaluate_calibration_report(&raw, &final_t, entries, "raw.srt", "final.srt", "terms.txt")
            .expect("report")
    }

    fn build_report_from_run(
        raw_srt: &str,
        final_srt: &str,
        entries: &[SessionTermEntry],
        review_run: &CanonicalTermReviewRun,
    ) -> CalibrationEvaluationReport {
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        build_report_from_owned_run(
            &raw,
            &final_t,
            entries,
            "raw.srt",
            "final.srt",
            "terms.txt",
            review_run,
        )
        .expect("report")
    }

    fn byte_offset(haystack: &str, needle: &str) -> usize {
        haystack.find(needle).expect("substring present")
    }

    fn bound_report_with_spans(
        raw_srt: &str,
        final_srt: &str,
        entries: &[SessionTermEntry],
        spans: Vec<CandidateSpan>,
    ) -> CalibrationEvaluationReport {
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        let analysis_run = AnalysisRun::for_canonical_session_terms(&raw, entries);
        let review_run =
            CanonicalTermReviewRun::new(analysis_run, ReviewCase::from_detector_candidates(spans));
        build_report_from_owned_run(
            &raw,
            &final_t,
            entries,
            "raw.srt",
            "final.srt",
            "terms.txt",
            &review_run,
        )
        .expect("report")
    }

    fn observed_span(
        raw: &Transcript,
        entry: &SessionTermEntry,
        matched_form: &str,
        start: usize,
        end: usize,
    ) -> CandidateSpan {
        let anchor = raw.anchor(0, start, end).expect("anchor");
        CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            DetectorProvenance::new("test.observed", "1"),
            anchor,
            Evidence::ObservedErrorForm(ObservedErrorFormEvidence {
                entry: entry.clone(),
                matched_form: matched_form.to_string(),
            }),
            vec![CandidateAlternative::new(entry.canonical_term.clone())],
        )
    }

    fn assert_char_boundaries(text: &str, start: usize, end: usize) {
        assert!(
            text.is_char_boundary(start),
            "start {start} not char boundary"
        );
        assert!(text.is_char_boundary(end), "end {end} not char boundary");
    }

    fn geometry_from_ranges(
        candidate_start: usize,
        candidate_end: usize,
        edit_start: usize,
        edit_end: usize,
    ) -> OverlapGeometry {
        assert!(ranges_overlap(
            candidate_start,
            candidate_end,
            edit_start,
            edit_end
        ));
        overlap_geometry(candidate_start, candidate_end, edit_start, edit_end)
    }

    fn local_edits_for(raw_text: &str, final_text: &str) -> Vec<LocalEditDraft> {
        compute_local_edits(0, 1, raw_text, final_text).expect("local edits")
    }

    fn resolved_anchor_text(raw_srt: &str, anchor: &SourceAnchorView) -> String {
        let raw = parse_srt(raw_srt).expect("parse raw");
        let source_anchor = raw
            .anchor(0, anchor.start_byte, anchor.end_byte)
            .expect("anchor");
        raw.resolve(&source_anchor).expect("resolve").to_string()
    }

    fn candidate_anchor(case: &ReviewCaseRecord) -> (usize, usize) {
        let anchor = &case.candidate_key_components.source_anchor;
        (anchor.start_byte, anchor.end_byte)
    }

    fn edit_raw_range(edit: &LocalEditRecord) -> (usize, usize) {
        (edit.raw.start_byte, edit.raw.end_byte)
    }

    #[test]
    fn unchanged_cue_produces_no_local_edits() {
        let srt = &cue(1, "same");
        let report = build_report(srt, srt, &[]);
        assert_eq!(report.summary.local_edit_count, 0);
        assert_eq!(report.summary.unchanged_cue_count, 1);
    }

    #[test]
    fn replacement_edit_is_detected() {
        let raw = &cue(1, "abXcd");
        let final_srt = &cue(1, "abYcd");
        let report = build_report(raw, final_srt, &[]);
        assert_eq!(report.summary.replacement_edit_count, 1);
        assert_eq!(report.local_edits[0].raw.text, "X");
        assert_eq!(report.local_edits[0].final_side.text, "Y");
    }

    #[test]
    fn insertion_at_start_middle_and_end_are_classified() {
        let cases = [
            ("abc", "Xabc", LocalEditKind::Insertion, 0usize),
            ("abc", "aXbc", LocalEditKind::Insertion, 1),
            ("abc", "abXc", LocalEditKind::Insertion, 2),
        ];
        for (raw_text, final_text, expected_kind, expected_byte) in cases {
            let report = build_report(&cue(1, raw_text), &cue(1, final_text), &[]);
            assert_eq!(
                report.summary.insertion_edit_count, 1,
                "{raw_text}->{final_text}"
            );
            let edit = &report.local_edits[0];
            assert_eq!(edit.kind, expected_kind);
            assert_eq!(edit.raw.text, "");
            assert_eq!(edit.raw.source_anchor, None);
            assert_eq!(edit.raw.insertion_byte, Some(expected_byte));
            assert!(edit.overlapping_case_ids.is_empty());
            assert!(edit.exact_structural_correspondence_ids.is_empty());
            assert!(report.correspondences.is_empty());
        }
    }

    #[test]
    fn deletion_at_start_middle_and_end_are_classified() {
        let cases = [
            ("Xabc", "abc", "X"),
            ("aXbc", "abc", "X"),
            ("abXc", "abc", "X"),
        ];
        for (raw_text, final_text, deleted) in cases {
            let report = build_report(&cue(1, raw_text), &cue(1, final_text), &[]);
            assert_eq!(
                report.summary.deletion_edit_count, 1,
                "{raw_text}->{final_text}"
            );
            let edit = &report.local_edits[0];
            assert_eq!(edit.kind, LocalEditKind::Deletion);
            assert_eq!(edit.raw.text, deleted);
            assert_eq!(edit.final_side.text, "");
            assert!(edit.raw.source_anchor.is_some());
        }
    }

    #[test]
    fn insertion_has_zero_candidate_overlap_and_counts_as_unclassified_without_terms() {
        let report = build_report(&cue(1, "abc"), &cue(1, "Xabc"), &[]);
        let edit = &report.local_edits[0];
        assert_eq!(edit.kind, LocalEditKind::Insertion);
        assert!(edit.overlapping_case_ids.is_empty());
        assert!(edit.term_scope.mechanically_unclassified);
        assert_eq!(report.summary.edit_without_candidate_overlap_count, 1);
    }

    #[test]
    fn insertion_can_carry_exact_final_side_term_scope() {
        let entries = vec![entry("ASUS", &[], &[])];
        let report = build_report(&cue(1, "abc"), &cue(1, "ASUSabc"), &entries);
        let edit = &report.local_edits[0];
        assert_eq!(edit.kind, LocalEditKind::Insertion);
        assert!(!edit.term_scope.full_region_matches.is_empty());
        assert!(!edit.term_scope.mechanically_unclassified);
        assert!(edit.overlapping_case_ids.is_empty());
    }

    #[test]
    fn multiple_disjoint_edits_in_one_cue() {
        let report = build_report(&cue(1, "aXbYc"), &cue(1, "aZbWc"), &[]);
        assert_eq!(report.summary.local_edit_count, 2);
        assert_eq!(report.summary.replacement_edit_count, 2);
    }

    #[test]
    fn final_text_reconstructs_from_local_edits() {
        let raw_text = "abXcd";
        let final_text = "abYcd";
        let report = build_report(&cue(1, raw_text), &cue(1, final_text), &[]);
        let mut rebuilt = raw_text.to_string();
        for edit in report.local_edits.iter().rev() {
            rebuilt.replace_range(
                edit.raw.start_byte..edit.raw.end_byte,
                &edit.final_side.text,
            );
        }
        assert_eq!(rebuilt, final_text);
    }

    #[test]
    fn every_non_empty_raw_edit_anchor_resolves() {
        let report = build_report(&cue(1, "aXbYc"), &cue(1, "aZbWc"), &[]);
        for edit in &report.local_edits {
            if edit.kind != LocalEditKind::Insertion {
                let anchor = edit.raw.source_anchor.as_ref().expect("anchor");
                assert_eq!(
                    anchor.revision_id, report.inputs.raw.revision_id,
                    "source_anchor_view uses bound revision"
                );
            }
        }
    }

    #[test]
    fn work_budget_exactly_at_limit_succeeds() {
        let raw_text = "a".repeat(2000);
        let final_text = "b".repeat(2000);
        let report = build_report(&cue(1, &raw_text), &cue(1, &final_text), &[]);
        assert_eq!(report.summary.local_edit_count, 1);
        assert_eq!(report.local_edit_policy.max_lcs_cells, 4_000_000);
    }

    #[test]
    fn work_budget_one_over_limit_refuses() {
        let raw_text = "a".repeat(2001);
        let final_text = "b".repeat(2000);
        let (raw, final_t) = parse_pair(&cue(1, &raw_text), &cue(1, &final_text));
        let refusal =
            evaluate_calibration_report(&raw, &final_t, &[], "r", "f", "t").expect_err("refused");
        assert!(matches!(
            refusal,
            CalibrationEvaluationRefusal::LocalDiffWorkBudgetExceeded {
                raw_scalar_count: 2001,
                final_scalar_count: 2000,
                max_lcs_cells: 4_000_000,
                ..
            }
        ));
    }

    #[test]
    fn work_budget_overflow_refuses_without_wrapping() {
        let refusal = check_work_budget(0, 1, usize::MAX, usize::MAX).expect_err("refused");
        assert!(matches!(
            refusal,
            CalibrationEvaluationRefusal::LocalDiffWorkBudgetExceeded {
                raw_scalar_count,
                final_scalar_count,
                max_lcs_cells: 4_000_000,
                ..
            } if raw_scalar_count == usize::MAX && final_scalar_count == usize::MAX
        ));
    }

    #[test]
    fn contained_occurrences_retain_overlaps_and_ordering() {
        let entries = vec![entry("aa", &[], &[])];
        let occurrences = contained_occurrences("aaa", 0, &entries);
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].start_byte, 0);
        assert_eq!(occurrences[1].start_byte, 1);
    }

    #[test]
    fn overlapping_ascii_forms_emit_multiple_occurrences() {
        let entries = vec![entry("aba", &[], &[]), entry("a", &[], &[])];
        let occurrences = contained_occurrences("aba", 0, &entries);
        assert!(occurrences.len() >= 3);
    }

    #[test]
    fn full_region_distinct_from_contained_occurrence() {
        let entries = vec![entry("ASUS", &[], &[])];
        assert_eq!(full_region_matches("ASUS", &entries).len(), 1);
        assert!(full_region_matches("prefix ASUS suffix", &entries).is_empty());
        assert_eq!(
            contained_occurrences("prefix ASUS suffix", 0, &entries).len(),
            1
        );
    }

    #[test]
    fn empty_forms_are_excluded_from_term_scope_helpers() {
        let entries = vec![SessionTermEntry {
            canonical_term: String::new(),
            aliases: vec!["alias".to_string()],
            observed_error_forms: vec![],
        }];
        assert!(full_region_matches("", &entries).is_empty());
        assert!(contained_occurrences("", 0, &entries).is_empty());
    }

    #[test]
    fn touching_half_open_boundaries_do_not_overlap() {
        assert!(!ranges_overlap(0, 3, 3, 6));
        assert!(ranges_overlap(0, 4, 3, 6));
    }

    #[test]
    fn canonical_review_run_finds_asis_phonetic_case() {
        let raw = parse_srt(&cue(1, "ASIS")).expect("parse");
        let entries = vec![entry("ASUS", &[], &[])];
        let run = run_canonical_term_review(&raw, &entries).expect("analysis");
        assert_eq!(run.review_cases().len(), 1);
    }

    #[test]
    fn authoritative_evaluation_snapshot_and_cases_share_run() {
        let raw = parse_srt(&cue(1, "ASIS")).expect("parse");
        let final_t = parse_srt(&cue(1, "ASUS")).expect("parse");
        let entries = vec![entry("ASUS", &[], &[])];
        let report =
            evaluate_calibration_report(&raw, &final_t, &entries, "r", "f", "t").expect("report");
        assert_eq!(
            report.analysis_snapshot.source_revision_id,
            report.inputs.raw.revision_id
        );
        assert_eq!(
            report.analysis_snapshot.session_terms_identity,
            report.inputs.session_terms.identity
        );
        assert_eq!(report.summary.review_case_count, 1);
    }

    #[test]
    fn structural_refusal_precedes_work_budget_refusal() {
        let raw_text = "a".repeat(2001);
        let final_text = "b".repeat(2000);
        let raw = format!(
            "1\n00:00:00,000 --> 00:00:01,000\n{raw_text}\n\n2\n00:00:01,000 --> 00:00:02,000\nx"
        );
        let final_srt = format!("1\n00:00:00,000 --> 00:00:01,000\n{final_text}");
        let (raw_t, final_t) = parse_pair(&raw, &final_srt);
        let refusal =
            evaluate_calibration_report(&raw_t, &final_t, &[], "r", "f", "t").expect_err("refused");
        assert!(matches!(
            refusal,
            CalibrationEvaluationRefusal::CueCountMismatch { .. }
        ));
    }

    #[test]
    fn mismatched_snapshot_revision_fails_closed() {
        let (raw, final_t) = parse_pair(&cue(1, "ASIS"), &cue(1, "ASUS"));
        let entries = vec![entry("ASUS", &[], &[])];
        let other = parse_srt(&cue(1, "OTHER")).expect("parse");
        let forged_run = run_canonical_term_review(&other, &entries).expect("analysis");
        let refusal =
            build_report_from_owned_run(&raw, &final_t, &entries, "r", "f", "t", &forged_run)
                .expect_err("binding failure");
        assert!(matches!(
            refusal,
            CalibrationEvaluationRefusal::BindingFailure { .. }
        ));
    }

    #[test]
    fn ambiguous_component_one_case_two_edits() {
        let edges = BTreeSet::from([
            (OverlapNode::Case(0), OverlapNode::Edit(0)),
            (OverlapNode::Case(0), OverlapNode::Edit(1)),
        ]);
        let case_degrees = HashMap::from([(0usize, 2usize)]);
        let edit_degrees = HashMap::from([(0usize, 1usize), (1usize, 1usize)]);
        assert_eq!(
            count_ambiguous_overlap_components(&edges, &case_degrees, &edit_degrees),
            1
        );
    }

    #[test]
    fn ambiguous_component_two_cases_one_edit() {
        let edges = BTreeSet::from([
            (OverlapNode::Case(0), OverlapNode::Edit(0)),
            (OverlapNode::Case(1), OverlapNode::Edit(0)),
        ]);
        let case_degrees = HashMap::from([(0usize, 1usize), (1usize, 1usize)]);
        let edit_degrees = HashMap::from([(0usize, 2usize)]);
        assert_eq!(
            count_ambiguous_overlap_components(&edges, &case_degrees, &edit_degrees),
            1
        );
    }

    #[test]
    fn ambiguous_components_count_disconnected_separately() {
        let edges = BTreeSet::from([
            (OverlapNode::Case(0), OverlapNode::Edit(0)),
            (OverlapNode::Case(1), OverlapNode::Edit(0)),
            (OverlapNode::Case(2), OverlapNode::Edit(1)),
            (OverlapNode::Case(3), OverlapNode::Edit(1)),
        ]);
        let case_degrees = HashMap::from([(0, 1), (1, 1), (2, 1), (3, 1)]);
        let edit_degrees = HashMap::from([(0, 2), (1, 2)]);
        assert_eq!(
            count_ambiguous_overlap_components(&edges, &case_degrees, &edit_degrees),
            2
        );
    }

    #[test]
    fn isolated_nodes_are_not_ambiguous_components() {
        let edges = BTreeSet::from([(OverlapNode::Case(0), OverlapNode::Edit(0))]);
        let case_degrees = HashMap::from([(0usize, 1usize)]);
        let edit_degrees = HashMap::from([(0usize, 1usize)]);
        assert_eq!(
            count_ambiguous_overlap_components(&edges, &case_degrees, &edit_degrees),
            0
        );
    }

    #[test]
    fn exact_anchor_exact_alternative_counts_distinct_indexes() {
        let raw = parse_srt(&cue(1, "X")).expect("parse");
        let entries = vec![entry("Y", &[], &[])];
        let analysis_run = AnalysisRun::for_canonical_session_terms(&raw, &entries);
        let anchor = raw.anchor(0, 0, 1).expect("anchor");
        let provenance = DetectorProvenance::new("test.detector", "1");
        let evidence = Evidence::GlossaryAlias(GlossaryAliasEvidence {
            matched_form: "X".to_string(),
            entry: entries[0].clone(),
        });
        let span = CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            provenance,
            anchor,
            evidence,
            vec![
                CandidateAlternative::new("Y"),
                CandidateAlternative::new("Y"),
            ],
        );
        let forged_run = CanonicalTermReviewRun::new(
            analysis_run,
            ReviewCase::from_detector_candidates(vec![span]),
        );
        let report = build_report_from_run(&cue(1, "X"), &cue(1, "Y"), &entries, &forged_run);
        assert_eq!(report.summary.candidate_edit_overlap_relation_count, 1);
        assert_eq!(
            report.summary.exact_anchor_exact_alternative_relation_count,
            2
        );
        assert_eq!(report.correspondences.len(), 1);
        assert_eq!(
            report.correspondences[0].exact_anchor_exact_alternative_indices,
            vec![0, 1]
        );
        assert_eq!(report.correspondences[0].alternative_relations.len(), 2);
        assert_eq!(
            report.correspondences[0].alternative_relations[0].alternative_index,
            0
        );
        assert_eq!(
            report.correspondences[0].alternative_relations[1].alternative_index,
            1
        );
        assert_eq!(
            report.correspondences[0].alternative_relations[0].final_region_relation,
            FinalRegionRelation::ExactFinalRegion
        );
        assert_eq!(
            report.correspondences[0].alternative_relations[1].final_region_relation,
            FinalRegionRelation::ExactFinalRegion
        );
    }

    #[test]
    fn json_is_deterministic_for_identical_inputs() {
        let raw = &cue(1, "Kafka");
        let final_srt = &cue(1, "Apache Kafka");
        let entries = vec![entry("Apache Kafka", &["Kafka"], &[])];
        let first =
            render_evaluation_report(&build_report(raw, final_srt, &entries)).expect("json");
        let second =
            render_evaluation_report(&build_report(raw, final_srt, &entries)).expect("json");
        assert_eq!(first, second);
        assert!(first.ends_with('\n'));
        assert!(first.contains(EVALUATION_NOTE));
    }

    #[test]
    fn exclusive_write_refuses_existing_destination() {
        let dir = std::env::temp_dir().join(format!(
            "vox-eval-write-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("report.json");
        std::fs::write(&path, b"existing").expect("seed");
        let error = write_evaluation_report_exclusive(
            path.to_str().expect("utf8"),
            "{\"status\":\"complete\"}\n",
        )
        .expect_err("refused");
        assert!(error.contains("destination already exists"));
        assert_eq!(
            std::fs::read_to_string(&path).expect("preserved"),
            "existing"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cjk_replacement_locks_exact_utf8_byte_ranges() {
        let raw_text = "资料库";
        let final_text = "资料湖";
        let raw_srt = cue(1, raw_text);
        let report = build_report(&raw_srt, &cue(1, final_text), &[]);
        assert_eq!(report.summary.replacement_edit_count, 1);
        assert_eq!(report.summary.local_edit_count, 1);

        let edit = &report.local_edits[0];
        assert_eq!(edit.kind, LocalEditKind::Replacement);
        assert_eq!(edit.raw.text, "库");
        assert_eq!(edit.final_side.text, "湖");

        let ku_start = byte_offset(raw_text, "库");
        let hu_start = byte_offset(final_text, "湖");
        assert_eq!(ku_start, 6);
        assert_eq!(hu_start, 6);
        assert_eq!(edit.raw.start_byte, ku_start);
        assert_eq!(edit.raw.end_byte, ku_start + "库".len());
        assert_eq!(edit.final_side.start_byte, hu_start);
        assert_eq!(edit.final_side.end_byte, hu_start + "湖".len());
        assert_char_boundaries(raw_text, edit.raw.start_byte, edit.raw.end_byte);
        assert_char_boundaries(
            final_text,
            edit.final_side.start_byte,
            edit.final_side.end_byte,
        );

        let anchor = edit
            .raw
            .source_anchor
            .as_ref()
            .expect("non-insertion anchor");
        assert_eq!(anchor.start_byte, edit.raw.start_byte);
        assert_eq!(anchor.end_byte, edit.raw.end_byte);
        assert_eq!(anchor.revision_id, report.inputs.raw.revision_id);
        assert_eq!(resolved_anchor_text(&raw_srt, anchor), "库");
    }

    #[test]
    fn precomposed_and_decomposed_unicode_stay_distinct_without_normalization() {
        let raw_text = "é";
        let final_text = "e\u{301}";
        let report = build_report(&cue(1, raw_text), &cue(1, final_text), &[]);
        assert_eq!(report.summary.replacement_edit_count, 1);

        let edit = &report.local_edits[0];
        assert_eq!(edit.raw.text, raw_text);
        assert_eq!(edit.final_side.text, final_text);
        assert_eq!(edit.raw.start_byte, 0);
        assert_eq!(edit.raw.end_byte, raw_text.len());
        assert_eq!(edit.final_side.start_byte, 0);
        assert_eq!(edit.final_side.end_byte, final_text.len());
        assert_ne!(edit.raw.text, edit.final_side.text);

        let mut rebuilt = raw_text.to_string();
        rebuilt.replace_range(
            edit.raw.start_byte..edit.raw.end_byte,
            &edit.final_side.text,
        );
        assert_eq!(rebuilt, final_text);
    }

    #[test]
    fn emoji_multi_scalar_replacement_locks_component_byte_boundaries() {
        let raw_text = "x👩‍💻y";
        let final_text = "x👩‍🔬y";
        let raw_srt = cue(1, raw_text);
        let report = build_report(&raw_srt, &cue(1, final_text), &[]);
        assert_eq!(report.summary.replacement_edit_count, 1);

        let edit = &report.local_edits[0];
        let laptop_start = byte_offset(raw_text, "💻");
        let microscope_start = byte_offset(final_text, "🔬");
        assert_eq!(edit.raw.text, "💻");
        assert_eq!(edit.final_side.text, "🔬");
        assert_eq!(edit.raw.start_byte, laptop_start);
        assert_eq!(edit.raw.end_byte, laptop_start + "💻".len());
        assert_eq!(edit.final_side.start_byte, microscope_start);
        assert_eq!(edit.final_side.end_byte, microscope_start + "🔬".len());
        assert_char_boundaries(raw_text, edit.raw.start_byte, edit.raw.end_byte);
        assert_char_boundaries(
            final_text,
            edit.final_side.start_byte,
            edit.final_side.end_byte,
        );

        let anchor = edit
            .raw
            .source_anchor
            .as_ref()
            .expect("non-insertion anchor");
        assert_eq!(anchor.start_byte, edit.raw.start_byte);
        assert_eq!(anchor.end_byte, edit.raw.end_byte);
        assert_eq!(resolved_anchor_text(&raw_srt, anchor), "💻");

        let mut rebuilt = raw_text.to_string();
        rebuilt.replace_range(
            edit.raw.start_byte..edit.raw.end_byte,
            &edit.final_side.text,
        );
        assert_eq!(rebuilt, final_text);
    }

    #[test]
    fn multiline_middle_line_replacement_preserves_surrounding_newlines() {
        let raw_text = "line1\nline2\nline3";
        let final_text = "line1\nLINE2\nline3";
        let raw_srt = cue(1, raw_text);
        let report = build_report(&raw_srt, &cue(1, final_text), &[]);
        assert_eq!(report.summary.replacement_edit_count, 1);

        let edit = &report.local_edits[0];
        let line_start = byte_offset(raw_text, "line2");
        let final_line_start = byte_offset(final_text, "LINE2");
        let first_newline = byte_offset(raw_text, "\n");
        let second_newline = raw_text[first_newline + 1..]
            .find('\n')
            .map(|offset| first_newline + 1 + offset)
            .expect("second newline");
        assert_eq!(edit.raw.text, "line");
        assert_eq!(edit.final_side.text, "LINE");
        assert_eq!(edit.raw.start_byte, line_start);
        assert_eq!(edit.raw.end_byte, line_start + "line".len());
        assert_eq!(edit.final_side.start_byte, final_line_start);
        assert_eq!(edit.final_side.end_byte, final_line_start + "LINE".len());
        assert_char_boundaries(raw_text, edit.raw.start_byte, edit.raw.end_byte);
        assert_char_boundaries(
            final_text,
            edit.final_side.start_byte,
            edit.final_side.end_byte,
        );
        assert!(edit.raw.start_byte > first_newline);
        assert!(edit.raw.end_byte < second_newline);

        let anchor = edit
            .raw
            .source_anchor
            .as_ref()
            .expect("non-insertion anchor");
        assert_eq!(anchor.start_byte, edit.raw.start_byte);
        assert_eq!(anchor.end_byte, edit.raw.end_byte);
        assert_eq!(resolved_anchor_text(&raw_srt, anchor), "line");

        let mut rebuilt = raw_text.to_string();
        rebuilt.replace_range(
            edit.raw.start_byte..edit.raw.end_byte,
            &edit.final_side.text,
        );
        assert_eq!(rebuilt, final_text);
    }

    #[test]
    fn repeated_lcs_tie_produces_identical_deterministic_edits() {
        let raw_text = "aaaaa";
        let final_text = "aaa";
        let first = local_edits_for(raw_text, final_text);
        let second = local_edits_for(raw_text, final_text);
        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].kind, LocalEditKind::Deletion);
        assert_eq!(first[0].raw_start_scalar, 0);
        assert_eq!(first[0].raw_end_scalar, 2);
        assert_eq!(first[0].final_start_scalar, 0);
        assert_eq!(first[0].final_end_scalar, 0);
    }

    #[test]
    fn correspondence_phonetic_candidate_contains_edit_via_evaluate_path() {
        let raw_text = "ASIS";
        let final_text = "ASUS";
        let entries = vec![entry("ASUS", &[], &[])];
        let report = build_report(&cue(1, raw_text), &cue(1, final_text), &entries);
        assert_eq!(report.summary.review_case_count, 1);
        assert_eq!(report.summary.replacement_edit_count, 1);
        assert_eq!(report.summary.local_edit_count, 1);

        let case = &report.review_cases[0];
        let edit = &report.local_edits[0];
        let (candidate_start, candidate_end) = candidate_anchor(case);
        let (edit_start, edit_end) = edit_raw_range(edit);

        assert_eq!(
            case.candidate_key_components.detection_kind,
            "phonetic_similarity"
        );
        assert_ne!(
            case.candidate_key_components.detection_kind,
            "glossary_alias_match"
        );
        match &case.evidence {
            EvidenceView::PhoneticSimilarity {
                observed_surface,
                canonical_owner,
                edit_distance,
                ratio_numerator,
                ratio_denominator,
                ratio_permille,
                matched_key,
                ..
            } => {
                assert_eq!(observed_surface, "ASIS");
                assert_eq!(canonical_owner, "ASUS");
                assert_eq!(*edit_distance, 1);
                assert_eq!(*ratio_numerator, 3);
                assert_eq!(*ratio_denominator, 4);
                assert_eq!(*ratio_permille, 750);
                assert_eq!(matched_key, "ASS");
            }
            other => panic!("expected phonetic evidence, got {other:?}"),
        }

        assert_eq!(case.matched_text, "ASIS");
        assert_eq!(candidate_start, 0);
        assert_eq!(candidate_end, raw_text.len());
        assert_eq!(edit_start, 2);
        assert_eq!(edit_end, 3);
        assert_eq!(edit.raw.text, "I");
        assert_eq!(edit.final_side.text, "U");
        assert!(candidate_start < edit_start);
        assert!(candidate_end > edit_end);
        assert_eq!(
            geometry_from_ranges(candidate_start, candidate_end, edit_start, edit_end),
            OverlapGeometry::CandidateContainsEdit
        );

        assert_eq!(report.correspondences.len(), 1);
        assert_ne!(
            report.correspondences[0].anchor_relation,
            AnchorRelation::Exact
        );
        assert_eq!(
            report.correspondences[0].anchor_relation,
            AnchorRelation::CandidateContainsEdit
        );
        assert_eq!(
            report.correspondences[0].alternative_relations[0].replacement_text,
            "ASUS"
        );
        assert_eq!(
            report.correspondences[0].alternative_relations[0].final_region_relation,
            FinalRegionRelation::Different
        );
        assert_eq!(report.summary.candidate_edit_overlap_relation_count, 1);
        assert_eq!(
            report.summary.exact_anchor_exact_alternative_relation_count,
            0
        );
    }

    #[test]
    fn correspondence_exact_alias_geometry_via_evaluate_path() {
        let raw_text = "X";
        let final_text = "Y";
        let entries = vec![entry("Y", &["X"], &[])];
        let report = build_report(&cue(1, raw_text), &cue(1, final_text), &entries);
        assert_eq!(report.summary.review_case_count, 1);
        assert_eq!(report.summary.replacement_edit_count, 1);
        assert_eq!(report.summary.local_edit_count, 1);

        let case = &report.review_cases[0];
        let edit = &report.local_edits[0];
        let (candidate_start, candidate_end) = candidate_anchor(case);
        let (edit_start, edit_end) = edit_raw_range(edit);

        assert_eq!(
            case.candidate_key_components.detection_kind,
            "glossary_alias_match"
        );
        assert_ne!(
            case.candidate_key_components.detection_kind,
            "phonetic_similarity"
        );
        match &case.evidence {
            EvidenceView::GlossaryAlias {
                matched_form,
                canonical_term,
            } => {
                assert_eq!(matched_form, "X");
                assert_eq!(canonical_term, "Y");
            }
            other => panic!("expected glossary alias evidence, got {other:?}"),
        }

        assert_eq!(case.matched_text, "X");
        assert_eq!(candidate_start, 0);
        assert_eq!(candidate_end, raw_text.len());
        assert_eq!(edit_start, 0);
        assert_eq!(edit_end, raw_text.len());
        assert_eq!(edit.raw.text, "X");
        assert_eq!(edit.final_side.text, "Y");
        assert_eq!(edit.final_side.start_byte, 0);
        assert_eq!(edit.final_side.end_byte, final_text.len());
        assert_eq!(
            geometry_from_ranges(candidate_start, candidate_end, edit_start, edit_end),
            OverlapGeometry::Exact
        );

        assert_eq!(report.correspondences.len(), 1);
        assert_eq!(
            report.correspondences[0].anchor_relation,
            AnchorRelation::Exact
        );
        assert_eq!(report.correspondences[0].alternative_relations.len(), 1);
        assert_eq!(
            report.correspondences[0].alternative_relations[0].replacement_text,
            "Y"
        );
        assert_eq!(
            report.correspondences[0].alternative_relations[0].final_region_relation,
            FinalRegionRelation::ExactFinalRegion
        );
        assert_eq!(report.summary.candidate_edit_overlap_relation_count, 1);
        assert_eq!(
            report.summary.exact_anchor_exact_alternative_relation_count,
            1
        );
        assert_eq!(
            report.correspondences[0].exact_anchor_exact_alternative_indices,
            vec![0]
        );
    }

    #[test]
    fn correspondence_candidate_contains_edit_via_evaluate_path() {
        let raw_text = "Apache Kafka";
        let final_text = "Apache Kafna";
        let entries = vec![entry("Apache Kafka", &["Kafka"], &[])];
        let report = build_report(&cue(1, raw_text), &cue(1, final_text), &entries);
        assert_eq!(report.summary.review_case_count, 1);
        assert_eq!(report.summary.replacement_edit_count, 1);

        let case = &report.review_cases[0];
        let edit = &report.local_edits[0];
        let kafka_start = byte_offset(raw_text, "Kafka");
        let kafka_end = kafka_start + "Kafka".len();
        assert_eq!(
            case.candidate_key_components.source_anchor.start_byte,
            kafka_start
        );
        assert_eq!(
            case.candidate_key_components.source_anchor.end_byte,
            kafka_end
        );
        assert_eq!(edit.raw.text, "k");
        assert!(edit.raw.start_byte > kafka_start);
        assert!(edit.raw.end_byte < kafka_end);
        assert_eq!(
            geometry_from_ranges(
                kafka_start,
                kafka_end,
                edit.raw.start_byte,
                edit.raw.end_byte
            ),
            OverlapGeometry::CandidateContainsEdit
        );
        assert_eq!(
            report.correspondences[0].anchor_relation,
            AnchorRelation::CandidateContainsEdit
        );
    }

    #[test]
    fn correspondence_edit_contains_candidate_via_evaluate_path() {
        let raw_text = "aXYZb";
        let final_text = "aAb";
        let entries = vec![entry("AB", &[], &["XY"])];
        let report = build_report(&cue(1, raw_text), &cue(1, final_text), &entries);
        assert_eq!(report.summary.review_case_count, 1);
        assert_eq!(report.summary.replacement_edit_count, 1);

        let case = &report.review_cases[0];
        let edit = &report.local_edits[0];
        let (candidate_start, candidate_end) = candidate_anchor(case);
        assert_eq!(case.matched_text, "XY");
        assert_eq!(edit.raw.text, "XYZ");
        assert_eq!(candidate_start, edit.raw.start_byte);
        assert!(edit.raw.end_byte > candidate_end);
        assert_eq!(
            geometry_from_ranges(
                candidate_start,
                candidate_end,
                edit.raw.start_byte,
                edit.raw.end_byte
            ),
            OverlapGeometry::EditContainsCandidate
        );
        assert_eq!(
            report.correspondences[0].anchor_relation,
            AnchorRelation::EditContainsCandidate
        );
    }

    #[test]
    fn correspondence_partial_overlap_via_evaluate_path() {
        let raw_text = "aXYZb";
        let final_text = "aXb";
        let entries = vec![entry("AB", &[], &["XY"])];
        let report = build_report(&cue(1, raw_text), &cue(1, final_text), &entries);
        assert_eq!(report.summary.review_case_count, 1);
        assert_eq!(report.summary.deletion_edit_count, 1);

        let case = &report.review_cases[0];
        let edit = &report.local_edits[0];
        let (candidate_start, candidate_end) = candidate_anchor(case);
        assert_eq!(case.matched_text, "XY");
        assert_eq!(edit.raw.text, "YZ");
        assert!(edit.raw.start_byte > candidate_start);
        assert!(edit.raw.end_byte > candidate_end);
        assert!(ranges_overlap(
            candidate_start,
            candidate_end,
            edit.raw.start_byte,
            edit.raw.end_byte
        ));
        assert!(candidate_start > edit.raw.start_byte || candidate_end < edit.raw.end_byte);
        assert!(edit.raw.start_byte > candidate_start || edit.raw.end_byte < candidate_end);
        assert_eq!(
            geometry_from_ranges(
                candidate_start,
                candidate_end,
                edit.raw.start_byte,
                edit.raw.end_byte
            ),
            OverlapGeometry::PartialOverlap
        );
        assert_eq!(
            report.correspondences[0].anchor_relation,
            AnchorRelation::PartialOverlap
        );
    }

    #[test]
    fn correspondence_touching_ranges_create_no_edge_via_evaluate_path() {
        let raw_text = "pre XY post";
        let final_text = "pre XY";
        let entries = vec![entry("AB", &[], &["XY"])];
        let report = build_report(&cue(1, raw_text), &cue(1, final_text), &entries);
        assert_eq!(report.summary.review_case_count, 1);
        assert_eq!(report.summary.deletion_edit_count, 1);

        let case = &report.review_cases[0];
        let edit = &report.local_edits[0];
        let (candidate_start, candidate_end) = candidate_anchor(case);
        assert_eq!(candidate_end, edit.raw.start_byte);
        assert!(!ranges_overlap(
            candidate_start,
            candidate_end,
            edit.raw.start_byte,
            edit.raw.end_byte
        ));
        assert_eq!(report.correspondences.len(), 0);
        assert_eq!(
            case.edit_relation.state,
            EditRelationState::ChangedCueNoOverlap
        );
        assert!(case.edit_relation.edit_ids.is_empty());
        assert_eq!(report.summary.candidate_edit_overlap_relation_count, 0);
    }

    #[test]
    fn correspondence_candidate_on_unchanged_cue_via_evaluate_path() {
        let text = "Apache Kafka";
        let entries = vec![entry("Apache Kafka", &["Kafka"], &[])];
        let raw_srt = cue(1, text);
        let report = build_report(&raw_srt, &raw_srt, &entries);
        assert_eq!(report.summary.review_case_count, 1);
        assert_eq!(report.summary.local_edit_count, 0);

        let case = &report.review_cases[0];
        let (candidate_start, candidate_end) = candidate_anchor(case);
        let kafka_start = byte_offset(text, "Kafka");
        let kafka_end = kafka_start + "Kafka".len();
        assert_eq!(candidate_start, kafka_start);
        assert_eq!(candidate_end, kafka_end);
        assert_eq!(case.matched_text, "Kafka");
        assert_eq!(
            resolved_anchor_text(&raw_srt, &case.candidate_key_components.source_anchor),
            "Kafka"
        );
        assert_eq!(case.edit_relation.state, EditRelationState::OnUnchangedCue);
        assert!(case.edit_relation.edit_ids.is_empty());
        assert_eq!(report.summary.candidate_on_unchanged_cue_count, 1);
        assert_eq!(report.summary.candidate_edit_overlap_relation_count, 0);
    }

    #[test]
    fn correspondence_changed_cue_no_overlap_via_evaluate_path() {
        let raw_text = "ASIS foo bar";
        let final_text = "ASIS foo BAR";
        let entries = vec![entry("ASUS", &[], &[])];
        let report = build_report(&cue(1, raw_text), &cue(1, final_text), &entries);
        assert_eq!(report.summary.review_case_count, 1);
        assert_eq!(report.summary.replacement_edit_count, 1);

        let case = &report.review_cases[0];
        let edit = &report.local_edits[0];
        assert_eq!(case.matched_text, "ASIS");
        assert_eq!(edit.raw.text, "bar");
        assert!(!ranges_overlap(
            case.candidate_key_components.source_anchor.start_byte,
            case.candidate_key_components.source_anchor.end_byte,
            edit.raw.start_byte,
            edit.raw.end_byte
        ));
        assert_eq!(
            case.edit_relation.state,
            EditRelationState::ChangedCueNoOverlap
        );
        assert_eq!(report.summary.candidate_changed_cue_no_overlap_count, 1);
        assert_eq!(report.summary.candidate_edit_overlap_relation_count, 0);
    }

    #[test]
    fn aggregate_one_candidate_two_edits_wires_summary_counts() {
        let raw_text = "abXcYd";
        let final_text = "ab1c2d";
        let entries = vec![entry("1c2", &["XcY"], &[])];
        let report = build_report(&cue(1, raw_text), &cue(1, final_text), &entries);
        assert_eq!(report.summary.review_case_count, 1);
        assert_eq!(report.summary.local_edit_count, 2);

        let case = &report.review_cases[0];
        let xcy_start = byte_offset(raw_text, "XcY");
        let xcy_end = xcy_start + "XcY".len();
        assert_eq!(
            case.candidate_key_components.source_anchor.start_byte,
            xcy_start
        );
        assert_eq!(
            case.candidate_key_components.source_anchor.end_byte,
            xcy_end
        );
        for edit in &report.local_edits {
            assert!(ranges_overlap(
                xcy_start,
                xcy_end,
                edit.raw.start_byte,
                edit.raw.end_byte
            ));
        }

        assert_eq!(report.summary.candidate_edit_overlap_relation_count, 2);
        assert_eq!(report.summary.candidate_with_edit_overlap_count, 1);
        assert_eq!(report.summary.edit_with_candidate_overlap_count, 2);
        assert_eq!(report.summary.multi_edit_candidate_count, 1);
        assert_eq!(report.summary.multi_candidate_edit_count, 0);
        assert_eq!(report.summary.ambiguous_overlap_component_count, 1);
    }

    #[test]
    fn aggregate_two_candidates_one_edit_wires_summary_counts() {
        let raw_text = "ASIS";
        let final_text = "ASUS";
        let entries = vec![entry("ASUS", &[], &["ASIS", "IS"])];
        let raw = parse_srt(&cue(1, raw_text)).expect("parse");
        let entry = entries[0].clone();
        let asis_end = raw_text.len();
        let is_start = byte_offset(raw_text, "IS");
        let is_end = is_start + "IS".len();
        let spans = vec![
            observed_span(&raw, &entry, "ASIS", 0, asis_end),
            observed_span(&raw, &entry, "IS", is_start, is_end),
        ];
        let report =
            bound_report_with_spans(&cue(1, raw_text), &cue(1, final_text), &entries, spans);
        assert_eq!(report.summary.local_edit_count, 1);
        assert_eq!(report.summary.review_case_count, 2);

        let edit = &report.local_edits[0];
        let (edit_start, edit_end) = edit_raw_range(edit);
        assert_eq!(edit.raw.text, "I");
        assert_eq!(edit_start, 2);
        assert_eq!(edit_end, 3);
        assert_eq!(report.review_cases.len(), 2);
        for case in &report.review_cases {
            let (candidate_start, candidate_end) = candidate_anchor(case);
            assert!(ranges_overlap(
                candidate_start,
                candidate_end,
                edit_start,
                edit_end
            ));
            assert!(edit.overlapping_case_ids.contains(&case.case_id));
        }
        assert_eq!(edit.overlapping_case_ids.len(), 2);
        assert_eq!(report.correspondences.len(), 2);
        assert_eq!(
            report
                .correspondences
                .iter()
                .map(|record| record.case_id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            2
        );

        assert_eq!(report.summary.candidate_edit_overlap_relation_count, 2);
        assert_eq!(report.summary.candidate_with_edit_overlap_count, 2);
        assert_eq!(report.summary.edit_with_candidate_overlap_count, 1);
        assert_eq!(report.summary.multi_candidate_edit_count, 1);
        assert_eq!(report.summary.multi_edit_candidate_count, 0);
        assert_eq!(report.summary.ambiguous_overlap_component_count, 1);
    }

    #[test]
    fn aggregate_two_disconnected_ambiguous_components_wires_summary_count() {
        let raw_text = "aaXXbb YYcc";
        let final_text = "aa11bb 22cc";
        let entries = vec![entry("TERM", &[], &["XX", "X", "YY", "Y"])];
        let raw = parse_srt(&cue(1, raw_text)).expect("parse");
        let entry = entries[0].clone();
        let xx_start = byte_offset(raw_text, "XX");
        let xx_end = xx_start + "XX".len();
        let yy_start = byte_offset(raw_text, "YY");
        let yy_end = yy_start + "YY".len();
        let spans = vec![
            observed_span(&raw, &entry, "XX", xx_start, xx_end),
            observed_span(&raw, &entry, "X", xx_start, xx_start + 1),
            observed_span(&raw, &entry, "YY", yy_start, yy_end),
            observed_span(&raw, &entry, "Y", yy_start, yy_start + 1),
        ];
        let report =
            bound_report_with_spans(&cue(1, raw_text), &cue(1, final_text), &entries, spans);
        assert_eq!(report.summary.local_edit_count, 2);
        assert_eq!(report.summary.review_case_count, 4);
        assert_eq!(report.correspondences.len(), 4);
        let xx_case_ids = report.review_cases[0..2]
            .iter()
            .map(|case| case.case_id.clone())
            .collect::<BTreeSet<_>>();
        let yy_case_ids = report.review_cases[2..4]
            .iter()
            .map(|case| case.case_id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(xx_case_ids.len(), 2);
        assert_eq!(yy_case_ids.len(), 2);
        assert!(xx_case_ids.is_disjoint(&yy_case_ids));
        for correspondence in &report.correspondences {
            let in_xx = xx_case_ids.contains(&correspondence.case_id);
            let in_yy = yy_case_ids.contains(&correspondence.case_id);
            assert!(in_xx ^ in_yy, "correspondence must not cross components");
        }
        assert_eq!(report.summary.ambiguous_overlap_component_count, 2);
    }

    #[test]
    fn aggregate_insertion_without_term_increments_unclassified_counts() {
        let report = build_report(&cue(1, "abc"), &cue(1, "Xabc"), &[]);
        let edit = &report.local_edits[0];
        assert_eq!(edit.kind, LocalEditKind::Insertion);
        assert_eq!(edit.raw.start_byte, edit.raw.end_byte);
        assert_eq!(edit.raw.text, "");
        assert_eq!(edit.raw.source_anchor, None);
        assert!(edit.overlapping_case_ids.is_empty());
        assert!(report.correspondences.is_empty());
        assert!(edit.term_scope.full_region_matches.is_empty());
        assert!(edit.term_scope.contained_occurrences.is_empty());
        assert_eq!(report.summary.insertion_edit_count, 1);
        assert_eq!(report.summary.edit_without_candidate_overlap_count, 1);
        assert_eq!(report.summary.mechanically_unclassified_edit_count, 1);
        assert!(edit.term_scope.mechanically_unclassified);
    }

    #[test]
    fn aggregate_insertion_with_term_does_not_increment_mechanically_unclassified() {
        let entries = vec![entry("ASUS", &[], &[])];
        let report = build_report(&cue(1, "abc"), &cue(1, "ASUSabc"), &entries);
        let edit = &report.local_edits[0];
        assert_eq!(edit.kind, LocalEditKind::Insertion);
        assert_eq!(edit.raw.source_anchor, None);
        assert!(edit.overlapping_case_ids.is_empty());
        assert!(report.correspondences.is_empty());
        assert_eq!(edit.term_scope.full_region_matches.len(), 1);
        let full_match = &edit.term_scope.full_region_matches[0];
        assert_eq!(full_match.canonical_owner, "ASUS");
        assert_eq!(full_match.form, "ASUS");
        assert_eq!(full_match.form_kind, TermFormKind::CanonicalTerm);
        assert_eq!(report.summary.insertion_edit_count, 1);
        assert_eq!(report.summary.edit_without_candidate_overlap_count, 1);
        assert_eq!(report.summary.mechanically_unclassified_edit_count, 0);
        assert!(!edit.term_scope.mechanically_unclassified);
    }

    #[test]
    fn aggregate_changed_edit_without_candidate_overlap_uses_term_scope_predicate() {
        let raw_text = "ASIS foo bar";
        let final_text = "ASIS foo BAR";
        let entries = vec![entry("ASUS", &[], &[])];
        let report = build_report(&cue(1, raw_text), &cue(1, final_text), &entries);
        let case = &report.review_cases[0];
        let edit = &report.local_edits[0];
        let (candidate_start, candidate_end) = candidate_anchor(case);
        let (edit_start, edit_end) = edit_raw_range(edit);
        assert_eq!(case.matched_text, "ASIS");
        assert_eq!(edit.raw.text, "bar");
        assert!(!ranges_overlap(
            candidate_start,
            candidate_end,
            edit_start,
            edit_end
        ));
        assert!(edit.overlapping_case_ids.is_empty());
        assert!(report.correspondences.is_empty());
        assert_eq!(
            case.edit_relation.state,
            EditRelationState::ChangedCueNoOverlap
        );
        assert_eq!(report.summary.candidate_changed_cue_no_overlap_count, 1);
        assert_eq!(report.summary.edit_without_candidate_overlap_count, 1);
        assert!(edit.term_scope.mechanically_unclassified);
    }

    #[test]
    fn serialization_precedes_exclusive_create() {
        let report = build_report(&cue(1, "a"), &cue(1, "b"), &[]);
        let json = render_evaluation_report(&report).expect("json");
        assert!(json.contains("\"schema_revision\": \"voxproof-calibration-correspondence-v0\""));
    }
}
