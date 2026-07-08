Status: current
Owns: Conceptual domain model and data ownership boundaries.
Does not own: Final JSON schemas, storage paths, database design, UI state model, or implementation-specific type definitions.
Last reviewed against code: SRT parse/validate, transcript revision identity, source anchors, `AnalysisRun`/`AnalysisSnapshot`, `CandidateSpan`/`CandidateKey`, typed glossary `Evidence`, non-binding `CandidateAlternative`, the first glossary detector, and the 1:1 `ReviewCase` wrapper exist in the Rust core; a minimal single-detector assembly function composes the glossary path end-to-end into `Vec<ReviewCase>`; ranking, review status/decisions, persistence, and materialization are not yet implemented; no product-level end-to-end pipeline exists yet

# Conceptual Data Contract

This document describes the conceptual domain model. It is not yet a final JSON schema.

## Core Concepts

### Transcript

`Transcript` represents the source transcript being reviewed. For v0.1, the initial transcript format is SRT. The transcript remains the source material from which review cases and reviewed output are derived.

A `Transcript` has a revision identity derived deterministically from its parsed segments. Source anchors and, later, correction decisions are bound to a specific transcript revision, so any change to source content yields a distinct revision. The accepted v1 revision format and hashing algorithm are governed by `docs/governance/decisions/MD-001-transcript-revision-id.md`.

### SourceAnchor

`SourceAnchor` addresses a location in source material. For v0.1 it is a non-empty byte range over one parsed segment's text, aligned to Unicode character boundaries, and bound to a specific `Transcript` revision.

The coordinate plane is the parsed `Segment` text, not raw `.srt` file bytes, because file-level details such as byte-order marks, line-ending style, and serialization layout are not stable source semantics. An anchor that does not match the current transcript revision does not resolve to source text.

For v0.1 a source anchor stays within a single segment. Cross-segment and discontinuous anchors are deferred.

### Normalization View

Normalization produces a derived analysis view of the transcript. It never rewrites source text. For v0.1 the normalization is identity-preserving, so analysis coordinates coincide with source-anchor coordinates. Any future non-identity normalization must map its analysis coordinates back to source anchors rather than becoming a second source of truth.

### LanguagePack

`LanguagePack` provides reusable language knowledge for analysis. It is more than a word list. It may eventually contain canonical terms, aliases, language metadata, term type, pronunciation hints, observed ASR confusions, related terms, scope, approval status, and version.

### CorrectionDecision

`CorrectionDecision` records a human decision for a review case: acceptance, rejection, edit, or deferral. Human decisions are the source of transcript changes.

A single accepted correction does not automatically change the Language Pack. Promotion of observed corrections into a Language Pack is a future governed process, not v0.1 behavior.

## v0.1 Review-Unit and Detection Lifecycle Contract

This section is authoritative for how detector findings and human review units are modeled in v0.1. Each decision carries a status marker. It states accepted commitments, deferred directions, and out-of-scope behavior. It is a contract, not a claim of full end-to-end implementation. As of this revision, `AnalysisRun`, `AnalysisSnapshot`, `CandidateKey`, `DetectionKind`, `DetectorProvenance`, `CandidateSpan`, typed glossary `Evidence`, `CandidateAlternative`, the first glossary detector, and the 1:1 `ReviewCase` wrapper exist in code with unit test coverage; pipeline assembly, ranking, review status/decisions, persistence, and materialization do not.

### CandidateSpan

Status: accepted for v0.1

`CandidateSpan` represents a detector-level finding against one localized source region.

For v0.1:

- A `CandidateSpan` has exactly one `SourceAnchor`.
- That `SourceAnchor` identifies exactly one contiguous, Unicode-safe range within exactly one parsed `Segment`.
- `CandidateSpan` is a local finding, not a distributed semantic issue model.

Out of scope for v0.1:

- discontinuous ranges;
- multi-anchor `CandidateSpan` values;
- cross-segment `CandidateSpan` values;
- candidate-level aggregation of multiple source regions.

Deferred: multi-location or cross-context review semantics are modeled above `CandidateSpan`, likely through `ReviewCase` or another explicit composite review model. `CandidateSpan` is not expected to become `Vec<SourceAnchor>`.

### CandidateKey

Status: accepted for v0.1

`CandidateKey` is the v0.1 semantic identity of a `CandidateSpan`. It expresses whether detector output is the same finding within its analysis context. It is deterministic and derived from:

- source revision;
- detector identity;
- detection kind;
- `SourceAnchor`.

`CandidateKey` exists for deterministic deduplication, reproducible detector output, and stable comparison of findings within one semantic context. It is semantic identity, not object-allocation, database, or random runtime identity, and must not be named as a generic `id`.

Deferred: `CandidateRecordId` or any opaque persisted-record identity; database primary-key design; public serialized ID format; hash-encoding strategy; cross-run review-state carryover; detector-version migration semantics. A future `CandidateRecordId` may identify a stored row, review-queue item, or historical record, but must not replace `CandidateKey`.

### DetectionKind

Status: accepted for v0.1

`DetectionKind` is a fixed, product-level taxonomy, not merely an inventory of implemented code. The accepted v0.1 categories are:

- `GlossaryAliasMatch`: the source text matches a known glossary alias or other non-canonical form of a canonical term.
- `MixedLanguageAnomaly`: a span mixes scripts or languages in a way that suggests a transcription error rather than intended usage.
- `PhoneticSimilarity`: the source text is phonetically close to a known term, suggesting a possible mishearing.
- `RepeatedPhrase`: a phrase repeats in a way that suggests an ASR duplication artifact.

New categories may be added later only when a real uncovered review case and its localized review semantics are understood. Placeholder or catch-all categories, such as a generic "unexpected token pattern", "semantic inconsistency", or "low-confidence transcript", are not accepted without a real localized review contract. Exact detection algorithms, ranking policies, and detailed evidence payloads remain detector-specific and evolve with implementation.

### Detector Provenance and AnalysisSnapshot

Status: accepted for v0.1

Detector provenance explains where a `CandidateSpan` came from. The v0.1 minimum provenance fields are:

- `detector_id`;
- `detector_version`.

`DetectionKind` remains a first-class field of `CandidateSpan` and `CandidateKey`; it is not duplicated as unstructured provenance metadata. Provenance must not be modeled as a generic metadata bag such as `HashMap<String, String>` or arbitrary JSON.

`AnalysisSnapshot` is an accepted first-class concept representing the effective inputs and configuration under which analysis was performed. Its scope, as applicable, includes source revision, detector identity and version, language-pack revision, normalization profile, detector configuration, and other effective analysis settings. Snapshot data belongs at `AnalysisRun` scope and is not duplicated into every `CandidateSpan`. For v0.1, only snapshot fields that genuinely exist in the implementation are modeled; language-pack persistence, configuration blobs, timestamps, and storage identifiers are not invented to fill out the type.

### AnalysisRun

Status: accepted for v0.1

`AnalysisRun` represents one bounded analysis execution over one transcript revision under one effective `AnalysisSnapshot`. Its responsibilities are the provenance boundary, the reproducibility boundary, and the produced-candidate scope.

`AnalysisRun` is not a scheduler, a background-job abstraction, a persistence engine, a review-queue manager, a UI session, or a workflow engine. `CandidateKey` must not depend on a random `AnalysisRunId`: `AnalysisRun` identifies one concrete execution, while `CandidateKey` identifies semantic sameness of a finding.

### ReviewCase

Status: accepted for v0.1

`CandidateSpan` and `ReviewCase` are separate domain concepts:

- `CandidateSpan` is the detector-level finding.
- `ReviewCase` is the human-facing review unit.

For v0.1, one `ReviewCase` corresponds to exactly one `CandidateSpan`. This 1:1 relationship is intentional and must not collapse the two concepts into one type.

Deferred: future `ReviewCase` aggregation may group multiple `CandidateSpan` values into one human review unit. That aggregation is not implemented in v0.1.

### Evidence

Status: accepted for v0.1

A `CandidateSpan` must have structured `Evidence`. `Evidence` answers why a finding exists and is distinct from:

- `SourceAnchor`, which says where the finding is;
- detector provenance, which says where it came from;
- `CandidateAlternative`, which may suggest a possible replacement;
- an accepted edit, which represents an approved change.

`Evidence` must not degrade into a free-form `reason: String`. The first glossary detector must produce a typed glossary evidence shape that can identify at least the glossary entry, the matched source form, and the canonical glossary term. Detector-specific evidence variants are added alongside the corresponding detector implementation rather than designed exhaustively in advance.

### CandidateAlternative

Status: accepted for v0.1

`CandidateAlternative` is a non-binding suggested replacement candidate. It is not an edit decision and must not automatically modify source text. The distinct semantics are:

- `CandidateAlternative`: a suggested replacement candidate;
- accepted edit: a human- or explicit-policy-approved edit;
- materialized edit: an edit actually applied to reviewed output.

The canonical glossary term carried in `Evidence` is factual supporting information; it does not by itself constitute an accepted replacement. Future automation may evaluate alternatives through explicit policy:

```text
CandidateAlternative -> policy evaluation -> accepted edit -> materialized edit
```

Detector output alone never constitutes an accepted edit.

### Out of Scope for This Contract Gate

The following are intentionally not decided by this review-unit and detection lifecycle contract, and remain governed by the materialization decision gate in `docs/architecture/pending-data-contracts.md`:

- correction-decision application;
- reviewed-output materialization;
- overlap and conflict resolution;
- stale-source handling;
- historical decision applicability after reruns;
- automatic application policies;
- `AcceptedEditPayload` and `ResolvedEdit` details.

### Contract Boundary Summary

- `CandidateSpan` remains local and single-anchor.
- `ReviewCase` is the human-facing aggregation boundary.
- `CandidateKey` is semantic identity, not storage identity.
- `AnalysisRun` is a provenance boundary, not a workflow framework.
- `Evidence` is mandatory and typed.
- `CandidateAlternative` values are non-binding and separate from accepted edits.

## Ownership Boundaries

- Detectors produce `CandidateSpan` findings under an `AnalysisRun`; `ReviewCase` is the human-facing review unit, one-to-one with a `CandidateSpan` in v0.1.
- Analyzer modules produce evidence.
- Ranking synthesizes evidence into review priority.
- Human review creates correction decisions.
- Materialization derives reviewed output.
- Source anchors and revision identity bind evidence and decisions to a specific immutable transcript revision.
- UI owns only transient interaction state such as selection, filters, and playback position.

Field-level schemas, persistence choices, and storage paths are intentionally unsettled until implementation work requires them.
