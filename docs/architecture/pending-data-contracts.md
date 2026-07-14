Status: exploratory
Owns: Pending data-contract questions, currently converged design direction, explicit constraints, and decision gates before future implementation work.
Does not own: Accepted architecture, final schemas, implementation tasks, storage design, current implementation state, or material decisions.
Last reviewed against code: v0.1 Track 1 code closed loop, prefixed session-scoped term input with distinct alias and observed-error-form evidence paths, and human-readable session summary exist; real-material/product validation remains pending.

# Pending Data Contract Decisions

## 1. Purpose and Authority Boundary

This document preserves current design convergence before irreversible implementation work begins. It exists so future sessions can recover the reasoning without treating exploratory notes as accepted architecture.

This document is not canonical architecture. It is not an accepted Material Decision record. It does not override `docs/architecture/data-contract.md`, and it does not authorize implementation that changes durable product or data semantics.

Its contents become binding only after explicit approval and promotion into canonical repository documentation and, where appropriate, Material Decisions. Until then, this note should prevent future work from conflating source identity, analysis findings, review units, human decisions, and materialized edits.

## 1a. Promoted Decisions (Now Canonical)

The following have been approved and promoted to `docs/architecture/data-contract.md` or accepted Material Decisions. They are no longer pending, and the canonical document or decision record is authoritative for their active definition:

- Stable `TranscriptRevisionId` via a tagged SHA-256 content fingerprint over canonical parsed transcript bytes, recorded in `docs/governance/decisions/MD-001-transcript-revision-id.md` (previously open question 1).
- The `SourceAnchor` coordinate model of section 4: a non-empty, character-boundary-aligned byte range over one parsed segment's text, bound to a specific transcript revision. The single-segment v0.1 constraint is retained.
- The v0.1 normalization decision from section 5: normalization is identity-preserving, so analysis coordinates coincide with source-anchor coordinates.
- The v0.1 review-unit and detection lifecycle: single-anchor `CandidateSpan`; `CandidateKey` as semantic identity; the fixed `DetectionKind` taxonomy; detector provenance and `AnalysisSnapshot`; `AnalysisRun` as a provenance boundary; `ReviewCase` as a human-facing unit that is one-to-one with a `CandidateSpan` in v0.1; mandatory typed `Evidence`; and non-binding `CandidateAlternative`. The canonical contract supersedes the `CandidateSpan`, `Evidence`, and `ReviewCase` wording in section 4 and the `AnalysisRun` wording in section 6.
- ReviewCase and review-ledger semantics are accepted in `docs/governance/decisions/MD-002-review-ledger-semantics.md`.
- Minimal reviewed-output materialization semantics are accepted in `docs/governance/decisions/MD-003-reviewed-output-materialization.md` and implemented for detector-raised `AcceptAlternative` decisions.

The remaining sections below stay exploratory. Where a section overlaps a promoted decision, the canonical document wins.

## 1b. Current Minimal Implementation Notes

The v0.1 Track 1 code closed loop now exists as a local facilitated CLI path:

```text
vox-proof review <input.srt> <session-terms.txt> <reviewed-output.srt> <decision-log.txt> <session-summary.txt>
```

The current loop reads an existing SRT file and a local session-term file, parses and validates them, runs exact alias and observed-error-form matching, presents the resulting ReviewCases for human decisions, records decisions in an in-memory ReviewLedger, derives reviewed SRT, renders a session decision log, derives a human-readable session summary from completed run facts, and writes all three output files.

This does not promote a persistence architecture.

Minimal session decision log rendering exists as a non-persistence session artifact. Durable persistence schema remains deferred.

Minimal facilitated CLI review flow exists. Product CLI design remains deferred.

The session summary contains session-scoped run identity, candidate/provenance counts, effective decision counts, accepted replacement outcomes, simple local timing, and output display paths. It is export-only plain text, not a stable machine schema, reusable knowledge asset, validation result, or durable persistence record.

The current CLI term input is a provisional, session-scoped UTF-8 line format:

```text
canonical term | alias:alternate form | error:observed ASR form | ...
```

Every source-form field requires an `alias:` or `error:` prefix. Values are trimmed around the prefix boundary while exact Unicode text and case are otherwise preserved. Duplicate or cross-classified source forms are rejected before review. Blank lines and lines beginning with `#` after optional whitespace are ignored. The ASCII `|` is always a delimiter; quoting, escaping, and terms containing a literal `|` are unsupported. Observed forms are user-supplied or human-confirmed session evidence, not automatically learned truth. This is not a Domain Collection or pack format. Reusable knowledge schemas and persistence remain deferred.

## 2. Current Product Invariants

This note refines pending data semantics within established VoxProof invariants. It does not change the product direction.

- VoxProof is local-first post-ASR transcript QA.
- Source transcript data remains traceable.
- Analyzers produce evidence, not silent edits.
- Human correction decisions are canonical.
- Reviewed output is derived from original source data plus accepted decisions.
- UI does not own domain truth.
- Fixed pipeline comes before dynamic orchestration.
- Do not introduce plugin systems, generic workflow engines, event buses, actor systems, distributed workers, or model supervisors without a concrete requirement.

## 3. Current Conceptual Processing Shape

The current conceptual flow is:

```text
SRT input
→ Parse / Validate
→ immutable Transcript
→ derived normalization view
→ CandidateSpan detection
→ analyzer evidence
→ ReviewCase aggregation and ranking
→ human CorrectionDecision
→ AcceptedEditPayload
→ deterministic materialization
→ reviewed SRT or explicit conflict report
```

This remains a modular monolith direction, not a plugin platform.

Note: the minimal v0.1 code loop now implements this shape only for the detector-raised exact alias and observed-error-form paths and `AcceptAlternative` materialization authorized by MD-003. The broader conceptual model in this document remains exploratory.

## 4. Proposed Domain Separation

The concepts in this section are proposed and non-binding. They describe a possible semantic model for future implementation work.

Note: the `CandidateSpan`, `Evidence`, and `ReviewCase` concepts here are superseded by the canonical v0.1 Review-Unit and Detection Lifecycle Contract in `docs/architecture/data-contract.md`. In particular, v0.1 `CandidateSpan` uses exactly one `SourceAnchor` (not a `SourceSelection`), and one `ReviewCase` maps to exactly one `CandidateSpan`. Where this section differs, the canonical document wins.

### Transcript and Segment

`Transcript` is immutable source material. v0.1 initially uses parsed SRT input.

`Segment` represents parsed subtitle content and associated timing/order metadata. Source text must not be overwritten by normalization, analyzers, review state, or output materialization.

### SourceAnchor

Proposed semantic identity:

```text
SourceAnchor
    = immutable transcript revision identity
      + segment identity
      + UTF-8 half-open byte range over canonical parsed Segment.text
```

Proposed constraints:

- The coordinate plane is parsed immutable `Segment.text`, not raw `.srt` file bytes.
- Raw file offsets are not canonical because BOMs, CRLF/LF differences, whitespace formatting, and subtitle serialization layout are not stable source semantics.
- The range form is `[start_byte, end_byte)`.
- Both endpoints must fall on valid Unicode character boundaries.
- v0.1 permits non-empty ranges only.
- A `SourceAnchor` belongs to one immutable `Transcript` revision.
- Transcript revision identity is part of the anchor itself, not merely an optional later validation check.
- UI may later expose grapheme-aware selection, but presentation coordinates must not replace canonical domain identity.

### SourceSelection

Proposed definition:

```text
SourceSelection
    = non-empty ordered collection of SourceAnchors
```

Proposed invariants:

- All anchors belong to the same immutable `Transcript` revision.
- Anchors use deterministic ordering: source segment order, then `start_byte`.
- Anchors must be pairwise non-overlapping.
- Equivalent selections should have one canonical representation.
- Adjacent anchors within the same segment may later be normalized into one anchor, unless a future edit-script model requires them to remain distinct.
- `SourceSelection` is for analysis findings and evidence association, not a directly materializable v0.1 edit target.

Example:

```text
foo [middle text] bar
```

If a discontinuous selection contains `foo` and `bar`, replacement text `qux` has no unique materialization meaning. Therefore detection scope and edit scope must remain separate.

### CandidateSpan

Proposed definition:

```text
CandidateSpan
    = analysis-run-scoped suspicious finding
      + SourceSelection
      + detector provenance
      + detection kind
```

`CandidateSpan` is not merely a source location. It is not an edit, and it is not proof that the source is wrong. It records that a detector found a review-worthy target under a specific `AnalysisRun`.

Detector provenance should identify the detector or rule source, relevant configuration snapshot, and finding category. `CandidateSpan` may reference a discontinuous `SourceSelection` in future semantics. v0.1 implementation may initially constrain `CandidateSpan` to one `SourceAnchor` in one `Segment`.

### Evidence

`Evidence` is distinct from `CandidateSpan` provenance.

- `CandidateSpan` says what was found and by which detector.
- `Evidence` records inspectable support for why it should be reviewed or ranked.

Examples may include:

- Glossary or alias match.
- Canonical term match.
- Phonetic or pinyin similarity.
- Mixed-language signal.
- Nearby transcript context.
- Rule score.
- Analyzer configuration or version.

`Evidence` must remain explainable and should not degrade into an unstructured `reason: String` escape hatch.

### ReviewCase

Proposed definition:

```text
ReviewCase
    = analysis-run-scoped human review aggregate
      + one or more CandidateSpans
      + aggregated Evidence
      + candidate alternatives
      + rank
```

`ReviewCase` is the unit presented to human review. v0.1 may initially constrain one `ReviewCase` to one `CandidateSpan`. The semantic model should allow future aggregation of multiple findings that refer to one practical review problem.

`ReviewCase` does not itself own authoritative mutable decision state. Review status is a derived projection from applicable decision history.

### CorrectionDecision

Proposed definition:

```text
CorrectionDecision
    = append-only human decision event
      against one ReviewCase
      under a specific observed source snapshot
```

Human decisions are canonical. Decision history must be deterministically ordered per `ReviewCase`. Ordering must use a monotonic per-`ReviewCase` sequence or immutable append index, not wall-clock timestamps alone.

`ReviewStatus` is derived from the latest applicable decision under that deterministic ordering.

Initial decision categories:

```text
Reject
Defer
Accept generated candidate
Edit manually
```

Reject and Defer carry no edit payload. Accept generated candidate creates an accepted edit payload and may retain a chosen candidate reference. Edit manually creates an accepted edit payload with human-authored replacement text. Accept and Edit use one materialization path; only replacement provenance differs.

### AcceptedEditPayload and ResolvedEdit

Proposed v0.1 boundary:

```text
AcceptedEditPayload
    = exactly one contiguous SourceAnchor
      within one immutable Segment
      + observed source fingerprint
      + replacement text
      + replacement provenance
```

Constraints:

- v0.1 does not support cross-segment edits.
- v0.1 does not support discontinuous edits.
- v0.1 does not use `SourceSelection` directly as an edit target.
- Replacement provenance distinguishes generated-candidate acceptance from human-authored editing.
- `ResolvedEdit` is a deterministic projection of an applicable `AcceptedEditPayload`.
- `ResolvedEdit` must not be independently edited or persisted as a separate source of truth.

If future requirements need multi-anchor or cross-segment modifications, introduce an explicit edit model such as:

```text
EditScript
    = ordered, non-overlapping Vec<AtomicEdit>
```

`EditScript` is not a current v0.1 commitment.

## 5. Normalization Traceability

Proposed normalization contract:

```text
Normalization never replaces source text.

Normalized coordinates are temporary analysis coordinates.

Every detector result must resolve back to one or more SourceAnchors.

Normalized coordinates alone are not durable identity.
```

Source mapping matters because:

- Evidence must point to original observed text.
- Human review must show what was actually flagged.
- Decisions must be safely materializable.
- Rule or normalizer changes must not silently alter old span semantics.

Proposed v0.1 constraints:

```text
- Candidate findings are confined to one Segment.
- Normalization is initially identity-preserving or explicitly mapped.
- Cross-segment anchors are deferred.
- No persisted normalized offset is canonical identity.
- Normalizer version and configuration belong to the AnalysisRun snapshot.
```

This does not prescribe a complete token model, grapheme abstraction, cross-segment mapper, or Unicode-edit engine.

## 6. AnalysisRun Snapshot Semantics

Note: the accepted v0.1 shape of `AnalysisRun` and `AnalysisSnapshot` is now canonical in `docs/architecture/data-contract.md` (a provenance and reproducibility boundary, not a workflow engine, modeling only snapshot fields that genuinely exist). The eventual snapshot fields listed below remain exploratory targets, not v0.1 commitments.

Future analysis-run provenance is the boundary that makes findings reproducible and traceable. The canonical direction deliberately leaves unresolved whether it extends `AnalysisSnapshot`, adds a higher-level concept, or composes multiple records.

That boundary should eventually distinguish, at minimum:

```text
- Transcript revision identity
- active Domain Collection and language-resource revisions
- resolved matching behavior
- normalizer version and configuration
- enabled analyzers
- analyzer rule versions and configuration
- ranking configuration
- optional ASR evidence source or revision
- optional audio identity, if audio is used
```

`CandidateSpan`, `Evidence`, `ReviewCase`, rank, and alternatives are analysis-run-scoped. A run with different source, active knowledge revisions, matching behavior, normalizer, analyzer, or ranking configuration is not silently equivalent to a prior run. Old decisions must not be blindly reapplied across changed source or changed analysis conditions.

## 7. Conservative Materialization Semantics

Note: minimal v0.1 reviewed-output derivation is now governed by MD-003 and implemented for single-anchor detector-raised `AcceptAlternative` decisions. This section remains exploratory for broader future materialization concerns.

Proposed materializer behavior:

```text
immutable source Transcript
+ applicable AcceptedEditPayloads
= reviewed output
```

Materialization must be deterministic and conservative.

Hard rules:

```text
A decision may apply only when its observed source fingerprint still matches.

Accepted edits must not overlap.

Conflicting replacements for the same applicable target are not auto-resolved.

A stale source revision, fingerprint mismatch, overlap, or replacement conflict produces an explicit conflict result.

The materializer does not infer, merge, silently drop, silently rewrite, or guess.
```

Proposed safety default:

```text
If unresolved conflicts exist, VoxProof must not claim to have produced a reviewed SRT.

It preserves the original source unchanged and emits an explicit conflict report.
```

Partial mutation followed by a claimed successful reviewed output would violate the product's trust and traceability model.

## 8. LanguagePack Boundary

Status: superseded exploratory proposal.

The earlier proposal for a minimal v0.1 `LanguagePack` snapshot is no longer an active design direction. Current v0.1 uses a provisional session-term adapter. The refined provisional distinctions among Domain Collection, Active Domain Collection Selection, Language Pack, Knowledge Pack, and resolved terminology input are owned by `product/correction-system-boundaries.md` and summarized in `architecture/data-contract.md`.

No pack schema, activation contract, persistence design, or policy authority is accepted here.

## 9. Implementation Gates

Work that can proceed without final material decisions:

```text
- narrow SRT parser
- immutable source-preserving Transcript and Segment representation
- timestamp validation
- structural validation
- parser errors distinct from validation issues
- stable transcript revision identity (MD-001)
- source anchor coordinate model within a single segment
- identity-preserving normalized transcript view
- single-anchor CandidateSpan findings with CandidateKey semantic identity
- fixed DetectionKind taxonomy
- detector provenance, AnalysisSnapshot, and AnalysisRun as a provenance boundary
- ReviewCase as a one-to-one human-facing unit over a CandidateSpan
- mandatory typed Evidence and non-binding CandidateAlternative
- detector-raised CorrectionDecision and ReviewLedger event recording (MD-002)
- minimal reviewed-output derivation for accepted `AcceptAlternative` decisions (MD-003)
- minimal session decision log rendering as a non-persistence artifact
- minimal facilitated CLI review flow
```

Decision gates:

```text
The review-unit and detection lifecycle is resolved and promoted to
docs/architecture/data-contract.md; single-anchor detectors may proceed.

Before implementing non-identity normalization transformations:
resolve normalization-to-source traceability beyond the current
identity-preserving view.

Before expanding beyond the current minimal reviewed-output derivation:
record the relevant Material Decision for replacement payloads, HumanRaised
materialization, cross-revision migration, persistence schema, or broader
serialization commitments.
```

Parser work should not be blocked by these pending decisions, because parser and structural validation are prerequisites for source-preserving identity.

## 10. Explicit Non-Goals and Deferred Complexity

The following are intentionally deferred:

```text
- cross-segment edits
- discontinuous editable selections
- generic edit-script engine
- automatic conflict resolution
- partial reviewed-output claims
- database schema
- persistence architecture
- Tauri or desktop shell
- waveform UI
- audio decoding pipeline
- local ASR or LLM runtime
- generic analyzer plugin system
- dynamic workflow engine
- async orchestration infrastructure
```

This is not a rejection of those directions forever. It is a v0.1 anti-drift boundary.

## 11. Open Questions Requiring Explicit Approval

These decisions remain unresolved and require explicit approval before promotion into canonical architecture:

1. Exact `Transcript` revision identity strategy. Resolved: stable tagged SHA-256 content fingerprint over canonical parsed transcript bytes, recorded in `docs/governance/decisions/MD-001-transcript-revision-id.md` and reflected in `docs/architecture/data-contract.md`.
2. Exact raw source-file fingerprint contents and algorithm.
3. `CandidateSpan` and `ReviewCase` identifiers and deduplication rules. Resolved for semantic identity: `CandidateKey` is derived from source revision, detector identity, detection kind, and `SourceAnchor`, and is promoted to `docs/architecture/data-contract.md`. Persisted-record identity (`CandidateRecordId`) and cross-run carryover remain deferred.
4. The precise meaning of "applicable" for a historical `CorrectionDecision` after re-analysis. Resolved only for v0.1 materialization against the observed transcript revision by MD-003; cross-revision decision migration remains deferred.
5. Whether unresolved conflicts always block all reviewed output, or whether a future explicitly labelled partial-output mode is allowed. Resolved only for v0.1 overlapping materializing edits by MD-003; broader partial-output modes remain deferred.
6. The exact representation of alternatives and generated-candidate provenance. Resolved: `CandidateAlternative` is a non-binding suggested replacement, and the v0.1 minimum detector provenance is `detector_id` and `detector_version`; promoted to `docs/architecture/data-contract.md`.
7. Whether adjacent `SourceAnchor`s are always canonicalized into one anchor.
8. The conditions under which future `EditScript` support would be justified.
9. How a segment-scoped finding (for example a subtitle timing issue affecting a whole cue) is anchored, given that `SourceAnchor` requires a non-empty text range. Raised by a concrete subtitle-QA lead; see `docs/discussion/2026-07-09.md`.

## 12. Promotion Rule

```text
This document is exploratory.

No section becomes binding merely because it is documented here.

A durable boundary becomes active only after explicit approval and promotion into:
- the relevant canonical repository documentation; and
- Material Decisions, only when cross-session preservation beyond canonical documentation is necessary.
```
