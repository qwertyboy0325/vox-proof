Status: current
Owns: Conceptual domain model and data ownership boundaries.
Does not own: Cross-version correction-system product semantics (owned by `product/correction-system-boundaries.md`), final JSON schemas, storage paths, database design, UI state model, or implementation-specific type definitions.
Last reviewed against code: Track 1 local code loop exists. SRT parse/validate, transcript revision identity, effective session-term/detector/config/algorithm analysis identity, native canonical-only session-term entries, source anchors, `AnalysisRun`/`AnalysisSnapshot`, `CandidateSpan`/`CandidateKey`, typed glossary, observed-error-form, and bounded ASCII-Latin phonetic-similarity `Evidence`, non-binding `CandidateAlternative`, exact alias and observed-error-form detectors, the ASCII-Latin phonetic similarity detector, the 1:1 `ReviewCase` wrapper, review decisions, minimal reviewed-output materialization, and session export artifacts exist. The v0.1 core mechanism is established by MD-008. The accepted establishment evidence covers the bounded mechanism and owner-operated real-material review. Product effectiveness, external-user validation, broad detector effectiveness, and durable persistence remain unestablished.

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

### Reusable Knowledge Inputs

The provisional product terminology distinguishes:

- a Domain Collection as a reusable domain/context knowledge unit;
- an Active Domain Collection Selection as a session-scoped selection of one immutable collection revision for analysis;
- a Language Pack as reusable language-specific resources;
- a Knowledge Pack as a packaging, import/export, and distribution bundle rather than an active runtime authority.

Multiple Domain Collections are allowed by the conceptual product model, although a first runtime may support fewer. Imported, available, selected, and active-for-analysis remain distinct states. The current session-term file is a provisional session-scoped adapter and is not a schema or import format for any of these concepts.

The provisional adapter natively represents a canonical-only entry as a canonical term with empty alias and observed-error-form collections. A line may therefore contain a canonical term alone. `alias:` and `error:` prefixes remain required for non-canonical source forms. A canonical-only entry is an effective analysis input, not shorthand for a self-referential alias; those two structures remain distinct in session-term identity. This representation does not establish a public serialization or persistence contract.

Future session-term, collection, and language-resource adapters may produce a shared resolved terminology input for evidence producers. That intermediate boundary, the refined terminology, and activation/version semantics remain provisional. Product authority, recommendation, and authorization rules are canonical in `product/correction-system-boundaries.md`.

### Cross-Version Correction-System Placeholders

The following conceptual boundaries prevent future capabilities from being collapsed into the current v0.1 types. They do not define accepted runtime types, fields, schemas, persistence, precedence, or orchestration:

- **Evidence** records inspectable reasons that a source span may deserve review. It does not establish truth or authorize an edit.
- **Context** describes immutable resolved circumstances relevant to analysis, such as scenario, expected language mix, active collection selections, surrounding transcript availability, or optional ASR evidence references. A future `SessionContext` does not own transcript state, decisions, policy, projection requests, authorization, UI state, persistence, or orchestration.
- **Policy** represents peer inputs for user-controlled matching, suggestion, cleanup, presentation, or automation behavior. Recommendations, resolved active policy, and explicit authorization remain distinct. Policy does not invent source facts.
- **Transformation** records the semantic intent of an accepted operation. Replacement text alone is not a complete future contract for normalization, cleanup, editorial work, or deletion.
- **Projection** is a derived output view materialized from immutable source plus applicable accepted or explicitly authorized transformations.

Matching and other policies, projection requests, and automation authorization are peer inputs rather than fields of a catch-all `SessionContext`. Responsibility boundaries and current-versus-future scope are canonical in `product/correction-system-boundaries.md`.

### CorrectionDecision

`CorrectionDecision` currently records rejection, deferral, acceptance of a detector alternative, or a need for manual correction. Human decisions are the only implemented source of transcript changes.

Human decisions are recorded as append-only `ReviewLedger` events, keyed to the corresponding `ReviewCase` identity. `ReviewLedger` events are authoritative for decision state. Rendered decision logs are deterministic exports, not a second decision authority. Normative event semantics are governed by `docs/governance/decisions/MD-002-review-ledger-semantics.md`.

A single accepted correction does not automatically change reusable domain or language resources or become active policy. Promotion remains a future governed process, not v0.1 behavior.

## v0.1 Review-Unit and Detection Lifecycle Contract

This section is authoritative for how detector findings and human review units are modeled in v0.1. Each decision carries a status marker. It states accepted commitments, deferred directions, and out-of-scope behavior. It is a contract, not a claim of product validation. As of this revision, `AnalysisRun`, `AnalysisSnapshot`, `CandidateKey`, `DetectionKind`, `DetectorProvenance`, `CandidateSpan`, typed glossary, observed-error-form, and bounded ASCII-Latin phonetic-similarity `Evidence`, `CandidateAlternative`, exact alias and observed-error-form detectors, the ASCII-Latin phonetic similarity detector, the 1:1 `ReviewCase` wrapper, review decisions, minimal reviewed-output materialization, and session export artifacts exist in code with test coverage. Decision logs and session summaries are deterministic export artifacts. They do not constitute a durable restore/session persistence contract. Durable persistence remains deferred.

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

Some taxonomy values do not yet have implemented producers; reserved categories must not be read as shipped detector behavior.

New categories may be added later only when a real uncovered review case and its localized review semantics are understood. Placeholder or catch-all categories, such as a generic "unexpected token pattern", "semantic inconsistency", or "low-confidence transcript", are not accepted without a real localized review contract. Exact detection algorithms, ranking policies, and detailed evidence payloads remain detector-specific and evolve with implementation.

### Detector Provenance and AnalysisSnapshot

Status: accepted for v0.1

Detector provenance explains where a `CandidateSpan` came from. The v0.1 minimum provenance fields are:

- `detector_id`;
- `detector_version`.

`DetectionKind` remains a first-class field of `CandidateSpan` and `CandidateKey`; it is not duplicated as unstructured provenance metadata. Provenance must not be modeled as a generic metadata bag such as `HashMap<String, String>` or arbitrary JSON.

The current `AnalysisSnapshot` is an accepted first-class concept that records source revision, deterministic effective session-term identity, active canonical detector-set identity, detector/configuration identity, and algorithm/version identity for one analysis run. Session-term identity includes canonical terms, aliases, and observed error forms and binds their parsed order in v0 because direct exact-detector `CandidateSpan` ordering is observable. The canonical review pipeline still sorts findings before review-case identity is assigned, so term order does not change review-case IDs or downstream output.

MD-004 owns this effective identity boundary and the minimum future canonical phonetic-evidence boundary. MD-005 authorizes the bounded ASCII-Latin phonetic evidence producer. Additional behavior-affecting context, active knowledge-asset revisions, optional ASR evidence, and candidate-affecting suggestion behavior remain future identity inputs. The current type is part of the future reproducibility boundary, not a commitment that it will be the sole owner. Effective run-level data must not be duplicated into every `CandidateSpan`; final public IDs, encodings, schemas, serialization, and persistence remain deferred.

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

`Evidence` must not degrade into a free-form `reason: String`. The exact alias detector produces typed glossary-alias evidence; the observed-error-form detector produces a distinct typed evidence variant; the ASCII-Latin phonetic similarity detector produces typed phonetic-similarity evidence with inspectable source/target representations, comparison facts, matched phonetic key, and bound detector-config and algorithm identities. Glossary and observed-error variants identify the matched source form and canonical term. Both use `DetectionKind::GlossaryAliasMatch`, whose accepted meaning includes aliases and other known non-canonical forms; separate detector provenance and typed evidence preserve the distinction without encoding input origin as a detection category. Phonetic similarity uses `DetectionKind::PhoneticSimilarity`. The observed-error classification states only how the user supplied the session input: it is not machine-generated ground truth, automatic learning, or replacement authority. Detector-specific evidence variants are added alongside the corresponding detector implementation rather than designed exhaustively in advance.

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

Minimal v0.1 review-decision semantics are governed by MD-002. Minimal reviewed-output materialization semantics are governed by MD-003, which establishes the minimal v0.1 refusal semantics:

- overlapping accepted edits refuse materialization;
- transcript-revision mismatch refuses materialization;
- invalid or incompatible materialization input fails closed.

Same-revision materialization refusal is established. Cross-revision carryover and migration remain deferred.

The following remain intentionally deferred by this review-unit and detection lifecycle contract:

- durable persisted correction-decision records;
- restore/session snapshot schema;
- cross-revision migration;
- historical decision applicability across reruns;
- detector-version migration;
- automatic application policies;
- non-minimal reviewed-output materialization formats;
- `AcceptedEditPayload` and `ResolvedEdit` as public contract types.

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
- Canonical v0.1 review ordering is deterministic and non-authoritative. Experimental retrieval/ranking remains sidecar tooling and cannot write accepted decisions or reviewed output.
- Human review creates correction decisions.
- Materialization derives reviewed output.
- Source anchors and revision identity bind evidence and decisions to a specific immutable transcript revision.
- UI owns only transient interaction state such as selection and filters; playback position is a proposed future UI state, not current v0.1 behavior.

Field-level schemas, persistence choices, and storage paths are intentionally unsettled until implementation work requires them.

## v0.2 Run Envelope (Contract Foundation)

`RunEnvelope` is a typed, serializable contract foundation for the proposed v0.2 real-transcript authoritative loop. It carries non-path-based run and input identities, explicit evaluation-mode separation, input qualification metadata, lifecycle state, and expected artifact roles.

This contract does **not** make the v0.2 loop operational. Reference validity, per-cue coverage, artifact joining, metric computation, transcript execution, CLI orchestration, and product persistence remain deferred to later slices.

## v0.2 Blind Reference Seal (Contract Foundation)

`ReferenceSeal` is a typed, serializable protocol record for blind human reference ordering attestations and derived calibration-validity classification. It references `RunId` and `InputIdentityReference` from the run envelope but remains a separate contract.

`ReferenceSealState::Sealed` means protocol record finalization only. It does not imply cryptographic signing, tamper-proof storage, or durable persistence. Blind eligibility is derived fail-closed from explicit independent attestations; coverage, joins, metrics, execution, and persistence remain deferred.

## v0.2 Per-Cue Reference Coverage (Contract Foundation)

`ReferenceCoverage` is a typed, serializable contract for the expected cue universe and one explicit reference disposition per cue. Absence of a record is not equivalent to `NoTranscriptionError`. Inventory completeness and resolved reference status are structural, purpose-neutral concepts. Diagnostic and synthetic references may be complete and resolved while remaining categorically ineligible for primary blind calibration. Primary-calibration eligibility is a separate seal/envelope/purpose posture; completeness does not imply join completion or metrics eligibility. Primary blind calibration coverage requires a sealed blind-reference-eligible seal and an envelope lifecycle of `ReferenceSealed`. The contract carries no transcript text, cue text, detector output, join keys, or metrics.

## v0.2 Artifact Bundle (Contract Foundation)

`ArtifactBundle` is a typed manifest for artifact descriptors bound to one run and input revision through shared context metadata and typed SHA-256 content references. Bundle manifests contain metadata only; they do not embed payload bytes, filenames, paths, transcript text, cue text, session terms, or detector output. Existing compare/evaluate artifacts remain provisional payloads outside this contract.

Bundle completeness is structural inventory completeness and context consistency only. Context binding is not detector/reference semantic joining. A content digest binds bytes but does not prove correctness, privacy, semantic compatibility, provenance truth, payload validity, or join success. Semantic joining and metrics remain deferred.

## v0.2 Human-Final Reference (Contract Foundation)

`HumanFinalReference` is a typed, serializable **private** contract for human-final reference-error records bound to one sealed reference revision. Seal attestation and individual error records are separate contracts. Per-cue coverage and individual error records are separate: one cue may contain multiple distinct reference errors; no-error cues require coverage completion records but not fabricated error records.

Reference surfaces (`original_surface`, `human_final_surface`) are private content-bearing data. Authorization to create or execute a reference does not authorize committing its surfaces. Join fields (`detector_case_id`, `match_disposition`, adjudication, metric contribution) are forbidden in sealed reference records. Correction of a sealed reference requires a new `ReferenceRevisionId`; this slice does not implement revision history.

Recall eligibility is derived from `ReferenceClass::TranscriptionError` plus an accepted verification basis (`audio_listened` or `mixed_sources`). Transcript-context-only references are excluded from recall denominators. Original-surface equality is integrity/anchor material only and does not imply correction correctness. Byte anchors are not resolved against real transcript bytes in this contract. Join, adjudication, metrics, execution, and persistence remain deferred.

## v0.2 Detector Proposal Snapshot (Contract Foundation)

`DetectorProposalSnapshot` is a typed, serializable **private** contract for detector proposal records bound to one run, analysis identity, and detector-output artifact reference. Proposal surfaces, evidence payloads, and alternatives are private content-bearing data. Authorization to define this contract does not authorize committing real detector surfaces or executing detectors.

Semantic identity derives from detector id, `DetectionKind`, and source anchor; detector version is excluded. Duplicate proposal ids and duplicate semantic keys are forbidden in `Frozen` state. Join fields (`match_disposition`, review disposition, adjudication, metric contribution) are forbidden on proposal records and evidence. Byte anchors are not resolved against real transcript bytes in this contract. Detector execution, transcript parsing, join, adjudication, and persistence remain deferred.
