# VoxProof Possibility Queue

Status: exploratory / non-binding

Owns:
Long-term product possibilities, speculative product-line directions, reusable asset hypotheses, and future expansion paths that are not yet implementation scope.

Does not own:
Current implementation scope, accepted architecture, material decisions, data contracts, privacy policy, security commitments, repository behavior, or implementation authorization.

Rule:
Nothing in this document authorizes implementation.

A possibility becomes implementation scope only after it is narrowed into a bounded, evidence-backed product slice. If it changes a durable product, data, architecture, privacy, security, identity, or review-decision boundary, it must go through the material-decision process before implementation.

## 1. Why This Document Exists

VoxProof has a narrow implementation wedge and a much larger product possibility space.

The implementation wedge exists to protect delivery:
source transcript input, source-preserving parsing, validation, anchors, evidence-backed candidates, human-facing review cases, and later human correction decisions.

The possibility space exists to preserve product imagination:
audio context, speaker behavior, semantic classification, human correction history, reusable regression assets, and evidence-aware knowledge workflows.

This document prevents those two modes from being confused.

It preserves speculative ideas without silently converting them into code, architecture, roadmap, or product commitments.

## 2. Ezra's Working Mode

Ezra's product ideation may be expansive, cross-domain, and speculative. It may use analogies from systems, physical phenomena, art, sound, language behavior, and human review workflows.

Implementation must remain conservative, narrow, and verifiable.

Working principle:

```text
Expansive product ideation searches the architecture space.
Conservative implementation slices control the blast radius.
Rejected ideas are not discarded; they are parked until prerequisites exist.
```

A large idea should usually move through this sequence:

```text
possibility
→ product hypothesis
→ reusable asset hypothesis
→ prerequisite check
→ smallest safe slice
→ material-decision gate if needed
→ implementation
```

Do not skip from possibility directly to implementation.

## 3. Product North Star

VoxProof turns audio-context-aware human review into reusable correction, speaker, and semantic knowledge assets.

More explicitly:

```text
source artifact
+ source/audio anchor
+ candidate
+ evidence
+ human decision
+ accepted correction or semantic confirmation
= reusable asset for future review, regression, retrieval, summarization, and quality improvement
```

VoxProof should not merely produce one-off transcript edits. Its long-term value is the accumulation of reviewed, traceable, reusable assets.

## 4. Current Narrow Wedge

The current implementation wedge is:

```text
local-first post-ASR transcript QA
```

The current product path is:

```text
SRT transcript
→ parse / validate
→ source-preserving Transcript
→ SourceAnchor
→ AnalysisRun / AnalysisSnapshot
→ CandidateSpan
→ Evidence
→ ReviewCase
→ later: human CorrectionDecision
→ later: reviewed output
→ later: regression asset
```

Current implementation should remain focused on this wedge unless Ezra explicitly changes the active slice.

## 5. Asset Flywheel

The long-term product flywheel is:

```text
raw source
→ detected suspicious span or semantic candidate
→ evidence-backed review case
→ human correction / confirmation / rejection
→ accepted asset
→ regression case
→ better future detection, ranking, correction, and summarization
```

The important asset is not only the corrected output.

The important asset is the reviewed decision with traceable evidence.

## 6. Candidate Asset Classes

These are possible future asset classes. They are not current implementation requirements.

### 6.1 Correction Asset

A reviewed correction asset may eventually capture:

- source transcript revision
- source anchor
- optional audio anchor
- candidate kind
- detector provenance
- evidence bundle
- suggested alternatives
- human decision
- accepted correction
- reviewer rationale
- regression expectation

Purpose:
Improve future detection, correction suggestions, and regression validation.

Current status:
Future direction. Not yet an accepted data contract.

### 6.2 Audio Context Asset

An audio context asset may eventually capture:

- audio time range
- neighboring speech context
- acoustic confidence signals
- alignment between transcript span and audio range
- evidence supporting why a transcript span is suspicious

Purpose:
Allow VoxProof to find transcript issues that are not obvious from text alone.

Current status:
Future direction. Do not implement until the text-side source/candidate/review/decision model is stable enough to receive audio evidence.

### 6.3 Speaker-Linked Correction Asset

A speaker-linked correction asset may eventually capture:

- speaker cluster identity
- recurring ASR confusions for that voice
- speech habits
- preferred terms
- domain-specific shorthand
- human-confirmed corrections associated with that speaker cluster

Purpose:
Allow VoxProof to learn how specific speakers or speaker clusters tend to speak, refer to concepts, and trigger ASR mistakes.

Current status:
High-value future direction. High sensitivity. Requires explicit material decisions before implementation.

Important semantic boundary:
A voice cluster is not the same thing as a person.

Use identity hypotheses, not identity facts.

Wrong model:

```text
voice_cluster == person_name
```

Safer model:

```text
voice cluster
→ possible speaker identity hypothesis
→ evidence
→ human confirmation
→ confidence
→ scope
→ revocation path
```

### 6.4 Semantic Knowledge Asset

A semantic knowledge asset may eventually capture:

- entities
- terms
- definitions
- claims
- decisions
- events
- relationships
- unresolved semantic candidates
- human confirmations or corrections

Purpose:
Turn reviewed transcript/document content into a structured, evidence-backed knowledge base.

Current status:
Long-term possibility. Do not implement before reviewed correction assets and regression cases exist.

### 6.5 Evidence-Aware Summary Asset

An evidence-aware summary asset may eventually be generated from:

- source transcript
- accepted corrections
- confirmed entities
- confirmed relationships
- speaker context
- unresolved candidates
- evidence bundles
- human review history

Purpose:
Produce summaries that distinguish confirmed knowledge from unresolved inference.

Current status:
Long-term possibility. VoxProof is not currently a meeting-summary product.

## 7. Future Product Lines

These are product-line possibilities, not roadmap commitments.

### 7.1 Transcript QA

Initial wedge.

Goal:
Find suspicious transcript spans, show evidence, support human review, and produce reviewed transcript output.

Why it matters:
Transcript QA has concrete source artifacts, anchors, validation rules, candidate spans, human decisions, and derived output.

### 7.2 Correction Asset Library

Goal:
Accumulate reviewed corrections as reusable assets.

Possible uses:

- regression fixtures
- glossary improvement
- detector evaluation
- ranking improvement
- reviewer training
- quality reports

### 7.3 Speaker-Aware Correction Memory

Goal:
Use recurring voice, phrase, and correction patterns to improve future transcript QA.

Possible uses:

- speaker-specific ASR confusion patterns
- speaker-specific shorthand resolution
- voice-cluster-aware candidate ranking
- human-confirmed speaker identity hypotheses

Sensitivity:
High. This is biometric-adjacent and identity-adjacent. Do not implement without explicit material decisions.

### 7.4 Semantic Knowledge Asset Builder

Goal:
Extract semantic candidates from transcripts and documents, then let humans confirm, reject, merge, split, or correct them.

Possible uses:

- domain glossary growth
- product knowledge extraction
- decision tracking
- organizational memory
- technical documentation QA
- support knowledge improvement

### 7.5 Evidence-Aware Summarization

Goal:
Generate summaries from reviewed and evidence-backed assets rather than raw text alone.

Possible uses:

- meeting summaries
- decision summaries
- unresolved issue summaries
- glossary change summaries
- knowledge-base update summaries

Boundary:
This does not make VoxProof a generic summarization product by default.

## 8. Possibility Queue

Use this section to park ideas without authorizing implementation.

Each item should use:

```text
Idea:
Why it matters:
Current blocker:
Prerequisites:
Smallest future slice:
Material-decision risk:
Privacy/security sensitivity:
Status:
```

### PQ-001 — Audio-context-aware suspicious span detection

Idea:
Use audio context to identify transcript spans that are suspicious even when the text looks superficially valid.

Why it matters:
This is the core differentiator from text-only transcript QA.

Current blocker:
No audio source model, audio anchor model, alignment model, or audio evidence type exists.

Prerequisites:
Stable source transcript model, SourceAnchor, CandidateSpan, ReviewCase, CorrectionDecision, reviewed output, and regression case model.

Smallest future slice:
Represent an AudioAnchor for an already-known transcript span without running audio analysis.

Material-decision risk:
Medium to high. Audio evidence affects evidence semantics and source-traceability boundaries.

Privacy/security sensitivity:
Medium initially, higher once speaker or identity inference is introduced.

Status:
Parked.

### PQ-002 — Reviewed correction regression assets

Idea:
Turn accepted human corrections into regression cases.

Why it matters:
This creates the quality flywheel. Future detector or ranking changes can be checked against previously reviewed cases.

Current blocker:
No CorrectionDecision, AcceptedEdit, reviewed output, or RegressionCase model exists.

Prerequisites:
Accepted decision semantics and reviewed-output derivation boundary.

Smallest future slice:
After CorrectionDecision exists, create one regression fixture from one accepted correction.

Material-decision risk:
High. Human correction decisions are canonical and affect output derivation.

Privacy/security sensitivity:
Low to medium for local-only text; higher if audio or speaker identity is included.

Status:
Near-future candidate, but not current scope.

### PQ-003 — Speaker-linked correction memory

Idea:
Use voice clusters, speech habits, and human corrections to learn recurring speaker-specific transcript correction patterns.

Why it matters:
This could make VoxProof learn how a specific organization and speaker population actually talks.

Current blocker:
No audio model, speaker clustering model, identity hypothesis model, or privacy boundary exists.

Prerequisites:
AudioAnchor, AudioContextEvidence, reviewed correction assets, explicit identity-hypothesis semantics, and privacy/security decisions.

Smallest future slice:
Represent an anonymous speaker cluster associated with a correction pattern, without attaching a real person name.

Material-decision risk:
Very high. Voice-to-person association is identity-sensitive and must not be modeled casually.

Privacy/security sensitivity:
Very high.

Status:
Dangerous / parked.

### PQ-004 — Human-confirmed speaker identity hypotheses

Idea:
Allow a human to confirm that a speaker cluster may correspond to a person or role within a bounded context.

Why it matters:
Speaker-aware transcript QA and summaries become much more useful when speaker references are reliable.

Current blocker:
No speaker cluster model, consent model, revocation model, scope model, or identity hypothesis model exists.

Prerequisites:
SpeakerCluster, SpeakerIdentityHypothesis, explicit human confirmation state, confidence scope, revocation semantics, and privacy/security decisions.

Smallest future slice:
A local-only identity hypothesis attached to one speaker cluster in one project, with no cross-project sharing.

Material-decision risk:
Very high.

Privacy/security sensitivity:
Very high.

Status:
Dangerous / parked.

### PQ-005 — Semantic candidate classification

Idea:
Classify important semantic units such as terms, entities, decisions, claims, events, risks, requirements, and relationships.

Why it matters:
This is the bridge from transcript QA to evidence-backed knowledge assets.

Current blocker:
The current project has not yet proven the correction-decision and regression-asset loop.

Prerequisites:
Reviewed correction assets, evidence bundles, and stable human review semantics.

Smallest future slice:
Extract one simple semantic candidate type from already-reviewed transcript text, then require human confirmation.

Material-decision risk:
Medium to high. Semantic assets may become durable knowledge records.

Privacy/security sensitivity:
Medium.

Status:
Parked.

### PQ-006 — Evidence-aware summarization

Idea:
Generate summaries from confirmed assets and unresolved candidates, not raw transcript text alone.

Why it matters:
This could make summaries more accurate, traceable, and honest about uncertainty.

Current blocker:
No semantic asset model, no reviewed knowledge asset model, and no summarization boundary exists.

Prerequisites:
Correction assets, semantic assets, evidence bundles, unresolved candidate tracking, and summary provenance.

Smallest future slice:
Generate a deterministic summary-like report from already-reviewed correction assets, without using an LLM.

Material-decision risk:
High if summaries are treated as product output.

Privacy/security sensitivity:
Medium to high depending on source material.

Status:
Parked.

## 9. Classification: Now / Later / Maybe / Dangerous

### Now

Current implementation should focus on:

- SRT source contract
- parse error versus validation issue boundary
- source-preserving Transcript
- SourceAnchor
- AnalysisRun / AnalysisSnapshot
- CandidateSpan
- Evidence
- CandidateAlternative
- ReviewCase
- later: CorrectionDecision
- later: AcceptedEdit
- later: ReviewedOutput
- later: RegressionCase

### Later

Possible after the text-side review and decision model is stable:

- AudioAnchor
- AudioContextEvidence
- audio-backed suspicious span evidence
- reviewed correction regression assets

### Maybe

Possible after correction assets prove useful:

- semantic candidate classification
- semantic knowledge assets
- evidence-aware summarization
- organizational knowledge workflows

### Dangerous

Do not implement without explicit material decisions:

- speaker identity hypotheses
- voice-to-person association
- cross-project speaker memory
- cloud sync of audio or speaker-derived assets
- model fine-tuning from reviewed human decisions
- biometric-adjacent speaker profiles
- organization-wide speech knowledge base

## 10. Anti-Drift Rules

Do not convert this document into a roadmap.

Do not use this document to justify broad abstractions in code.

Do not introduce generic asset graphs, semantic engines, speaker knowledge bases, summarization runtimes, plugin systems, async orchestration, or model runtime infrastructure from this document alone.

Prefer this rule:

```text
Future compatibility in language is allowed.
Premature abstraction in code is not.
```

The current implementation should continue to move through small, verifiable slices.

## 11. Revisit Triggers

Revisit this document when:

- a correction-decision model is accepted;
- reviewed output is materialized from source transcript plus accepted decisions;
- regression cases are introduced;
- audio evidence becomes the active implementation slice;
- speaker-related data becomes a serious product candidate;
- semantic asset classification becomes a serious product candidate;
- summarization becomes a serious product candidate;
- Ezra explicitly changes the product wedge or long-term product framing.

Revisiting this document does not itself authorize implementation.
