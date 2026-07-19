# MD-004: Effective Analysis Identity and Phonetic Evidence Boundary

Status: accepted

Date: 2026-07-17

Decision authority: Ezra

## Context

VoxProof's current `AnalysisSnapshot` identifies only the source transcript
revision. That is sufficient for the implemented fixed exact-analysis path, but
it cannot distinguish canonical runs whose effective session terms, active
detectors, detector configuration, or algorithms differ.

The next planned detector is a bounded phonetic evidence producer. Its findings
may enter canonical `Evidence` and `ReviewCase` values, but detector output must
not become correction authority or bypass human review.

Session-term source order is observable at the detector boundary. The exact
detectors visit entries and their alias or observed-error forms in input order,
so their returned canonical `CandidateSpan` ordering can change when valid
session terms are reordered. The canonical review pipeline subsequently sorts
findings by source anchor and detector identity before assigning
`ReviewCaseId` values, so review-case identity, decision applicability, logs,
summaries, and reviewed-output bytes do not change. Both behaviors are part of
the current implementation boundary.

## Decision

### Effective analysis identity

Every canonical analysis run must bind:

* the source `TranscriptRevisionId`;
* a deterministic identity of the effective session terms;
* the active canonical detector-set identity;
* the detector/configuration identity;
* the algorithm/version identity.

These are immutable, typed semantic inputs to `AnalysisSnapshot` and
`AnalysisRun`. They are not generic metadata and do not create a detector
registry, scheduler, persistence system, or runtime configuration framework.

A detector must fail closed when the supplied run does not identify the
transcript, session terms, detector set, detector configuration, and algorithm
that the detector actually uses.

### Session-term identity

The session-term identity includes every parsed:

* canonical term;
* alias and its canonical-term ownership;
* observed error form and its canonical-term ownership.

Observed error forms are included because exact observed-error detection is
part of the same canonical run, even though a future phonetic detector must not
use them as phonetic targets.

For v0.1, session-term identity preserves and binds parsed entry order and the
alias and observed-error-form order within each entry. Alias and observed error
classifications remain distinct. This is required because detector-level
canonical candidate ordering is currently observable.

Order binding in this v0 identity does not declare source-file order to be
permanent product semantics. A future change may make detector output
order-independent and version the session-term identity rule accordingly.

### Canonical phonetic evidence boundary

`DetectionKind::PhoneticSimilarity` remains the product-level detection kind.
A future canonical phonetic evidence payload must contain inspectable typed
facts sufficient to reproduce and review the finding:

* the observed source surface;
* the matched target form;
* whether the target form is a canonical term or alias;
* the owning canonical term;
* the source and target phonetic representations;
* the comparison facts, such as distance or score components;
* the detector/configuration identity;
* the phonetic algorithm/version identity.

The source location remains owned by `SourceAnchor`; detector identity remains
owned by `DetectorProvenance`. Evidence must not become a free-form rationale.

For the planned v0 producer, phonetic targets are session-term canonical terms
and aliases only. Observed `error:` forms remain inputs only to exact
observed-error evidence. Search is cue-local and bounded.

### Ambiguous-target suppression

If multiple canonical terms qualify for the same source anchor, v0 must emit no
canonical phonetic `CandidateSpan` for that anchor. It must not choose by
session-term order, incidental iteration order, or hidden ranking.

A later contract may introduce an inspectable ambiguity artifact or explicit
multi-target review semantics. Suppression in v0 does not establish either.

### Human authority

Phonetic detector output may enter canonical `Evidence`, `CandidateSpan`, and
the one-to-one detector-raised `ReviewCase` flow. It remains non-binding.

It must never directly modify canonical transcript state, create an accepted
decision, auto-apply an alternative, or bypass `ReviewLedger`. Reviewed output
continues to follow MD-003: only an applicable human
`AcceptAlternative` decision may materialize a replacement.

## Consequences

The effective identity prerequisite must land before a canonical phonetic
producer.

Changing effective session terms, active detector set, detector configuration,
or algorithm version creates a distinct analysis snapshot even when the source
transcript is unchanged.

The existing exact alias and observed-error detectors remain behaviorally
unchanged after migration to the expanded run identity.

The future phonetic detector can expand its target policy, source-span policy,
or language/script support only through explicit versioned configuration and,
when durable semantics change, the applicable decision process.

## Explicitly deferred

This decision does not decide:

* persistence or serialization schemas;
* a public hash or textual encoding for analysis or session-term identity;
* UI presentation;
* CJK or pinyin eligibility and comparison policy;
* cross-cue matching;
* a long-term detector registry or plugin system;
* durable storage of suppressed ambiguities;
* multi-target `CandidateSpan` or `ReviewCase` semantics;
* final phonetic thresholds, window sizes, or algorithms;
* expansion to Domain Collections, policies, or other target sources.

## Banned designs

1. Resolving multiple qualifying canonical terms by input order.
2. Resolving phonetic ambiguity through an unrecorded ranking or tie-break.
3. Treating observed `error:` forms as v0 phonetic targets.
4. Calling the experimental retrieval or ranking sidecar from the canonical
   detector path.
5. Allowing detector output to edit transcript state or bypass human review.
6. Representing effective analysis identity as an untyped metadata map.
7. Silently changing or ignoring source-form order while detector-level
   canonical candidate ordering remains order-sensitive.

## Implementation consequences

The prerequisite implementation expands `AnalysisSnapshot` and `AnalysisRun`
with typed identities for effective session terms, active detector set,
detector configuration, and algorithm version. The exact canonical pipeline
must construct and validate those identities.

This decision does not authorize implementation of phonetic matching itself.

## Related proposed decisions

MD-004 remains authoritative for established v0.1 effective analysis identity.

If accepted, `decisions/MD-012-proposed-experience-derivation-and-scoped-knowledge-governance.md` would add immutable knowledge snapshot identity and ranking-policy identity as explicit v0.2 analysis inputs where applicable. MD-012 would not reinterpret historical v0.1 analysis artifacts recorded here.
