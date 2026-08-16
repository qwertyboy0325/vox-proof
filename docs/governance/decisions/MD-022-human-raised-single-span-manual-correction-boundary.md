# MD-022: Human-Raised Single-Span Manual Correction Boundary

Status: accepted

Date: 2026-08-16

Accepted: 2026-08-16 per explicit owner authorization

Decision authority: Ezra

Classification: HumanRaised ReviewCase origin, CaseRaised + ManualReplacement atomic authority, session format v3, and Project Memory promotion locator extension

## Context

MD-002 reserved `HumanRaised` as a future ReviewCase origin and banned modeling human findings as detector output, Evidence, or automatic corrections. MD-003 deferred HumanRaised materialization. MD-017 accepted bounded Manual Replacement only for existing detector-raised ReviewCases and explicitly deferred HumanRaised. MD-021 accepted the cross-material Project Memory flywheel for detector Manual Replacement promotion and derived reuse proposals.

Proposed MD-011 sketched a broader HumanRaised + withdrawal + supersession history model. It remains unaccepted. This decision accepts only the narrow HumanRaised + ManualReplacement slice required for knowledge acquisition when detectors and Project Memory miss an occurrence.

After P4, a desktop reviewer can reuse a previously promoted correction on Material B, but still cannot correct unflagged text X inside an open review unless a detector or reuse proposal already targets that span.

## Decision

### 1. Origin and target

`HumanRaised` is a true ReviewCase family origin.

```text
ReviewCase
  id: ReviewCaseId { family: DetectorRaised | HumanRaised, local_index }
  origin:
    DetectorRaised(CandidateSpan)
    HumanRaised(HumanSelectedSpan)

HumanSelectedSpan
  anchor: SourceAnchor
  observed_text: exact source bytes at creation
  creation: human-origin provenance (not Evidence)
```

HumanRaised is not:

- detector output
- a P3-style thin derived inference target
- Evidence
- a SessionTermEntry
- a fabricated AnalysisRun / AnalysisSnapshot entry

`AcceptAlternative` on a HumanRaised case is a typed error.

### 2. Anchor

A HumanRaised case must bind:

- the current source `TranscriptRevisionId`
- exactly one cue / segment
- exactly one contiguous Unicode-safe byte span within that segment

Whole-cue selection is allowed. Empty, inverted, cross-cue, discontinuous, and non-character-boundary spans are refused.

### 3. Decision action

The only HumanRaised decision action in this slice is:

```text
ManualReplacement
```

Payload rules are the accepted MD-017 rules evaluated against the selected source text:

- non-empty
- not whitespace-only
- single line (no controls / U+2028 / U+2029)
- not byte-identical to the selected source text
- no silent trim, normalization, rewrite, or inference
- not deletion

### 4. Creation and decision atomicity

Raising and deciding remain separate ledger events, recorded in one user gesture and one SQLite transaction:

```text
CaseRaised { human_raised_id, revision, anchor, observed_text }
DecisionRecorded { case_id, revision, ManualReplacement { Y } }
```

Selection is transient and non-authoritative until durable commit. This slice does not persist undecided HumanRaised issues. “Flag for later” is deferred.

Reviewed output may change only after durable commit succeeds.

### 5. Materialization and collision

HumanRaised ManualReplacement materializes through the existing MD-003 / MD-017 path:

- original source coordinate plane
- observed revision must match current source revision
- exact persisted anchor and replacement
- overlapping effective materializing edits fail closed

Creation-time refusal (before commit):

- empty / inverted / cross-cue / non-char-boundary / revision-stale spans
- overlap with any existing HumanRaised anchor
- overlap with any effective materializing detector or reuse-target range
- overlap with an undecided detector or reuse card on the same range

Non-overlapping substrings in the same cue are allowed. This slice refuses duplicate overlapping HumanRaised creates; that narrows MD-002’s duplicate-allowed v0 note for HumanRaised create only.

Last-decision-wins on the same HumanRaised ReviewCaseId remains available for a later ManualReplacement. Withdrawal / supersession UX is deferred.

### 6. Project Memory promotion

A committed HumanRaised ManualReplacement is eligible for the same explicit promotion action as a detector ManualReplacement:

```text
Use this correction in related reviews
```

No auto-promotion. Exact pair X→Y only. Material B derivation remains exact reuse.

`SourceDecisionLocator` gains an origin discriminant:

```text
SourceDecisionLocator.origin =
  DetectorRaised { analysis_snapshot, review_case_id, ...existing fields }
  HumanRaised    { human_raised_case_id, revision, ledger_position,
                   decision_digest, effective_at_ledger_length }
```

Do not invent a fake AnalysisSnapshot for HumanRaised promotions. Do not place HumanRaised cases into detector `canonical_run` vectors. Candidate generation walks the HumanRaised family and resolves observed text from `HumanSelectedSpan.anchor`.

### 7. Format posture

```yaml
product_session_format_version:
  v1: unchanged, readable, no HumanRaised
  v2: unchanged, readable, project-bound flywheel only, no HumanRaised
  v3: new-session-only successor
      HumanRaised ReviewCase family
      CaseRaised + DecisionRecorded
      v2 capabilities when project-bound
      unbound v3 allowed (promotion N/A)
  auto_migration: false
```

There is no authorized silent extensibility path inside v2. New HumanRaised-capable sessions use format v3. Existing v1/v2 sessions remain inspectable; HumanRaised is unavailable on those files. No automatic session migration.

Project Memory:

- detector-only locators and `project-memory-snapshot:sha256-v1` hashing remain byte-identical when no HumanRaised promotions exist
- HumanRaised promotions require additive Project Memory columns and `PROJECT_MEMORY_FORMAT_VERSION` 1 → 2
- first HumanRaised promotion may bump a live project 1→2 additively without rewriting historical detector events
- old software fails closed on format 2

### 8. Completeness and Session Terms separation

Atomically decided HumanRaised corrections participate in reviewed projection, decision log, export, and reopen. Unflagged cues remain ordinary source text and need not be raised.

“Add this term and re-run term analysis” is a different future UX (analysis-input change). This decision does not authorize that path.

## Consequences

- `ReviewCase` becomes an origin-discriminated family rather than a detector-only wrapper.
- ReviewLedger gains `CaseRaised` for HumanRaised creation provenance.
- Product session format v3 is required for HumanRaised-capable sessions.
- Application review targets gain a HumanRaised path.
- Promotion and Project Memory gain an origin-aware locator and format 2 additive storage.
- Desktop must expose bounded cue-span selection and “Correct selected text” without becoming a general subtitle editor.

## Explicitly deferred

- withdrawal and supersession UX
- unresolved human flags / NeedsManualCorrection on HumanRaised
- cross-cue, timing, split/merge, deletion, multiline editing
- general transcript editor
- dynamic Session Terms editing and reanalysis
- detector redesign, fuzzy matching, LLM detection
- auto-promotion and automatic correction
- Gate 7 real-material evidence

## Banned designs

1. Injecting a fake ReviewCase into AnalysisRun or AnalysisSnapshot.
2. Fabricating Evidence or SessionTermEntry to manufacture a case.
3. Rerunning a detector solely to create a review target.
4. Pretending Project Memory raised the issue.
5. Choosing a thin human-selected target merely to avoid a format decision.
6. Omitting CaseRaised while claiming HumanRaised recall provenance.
7. Treating creation as a decision variant.
8. Silent v2 schema/meaning mutation or automatic session migration.
9. Empty replacement as deletion, or silent trim/normalization.
10. Accepting broader MD-011 withdrawal/supersession through this decision.

## Relationship to prior decisions

- MD-002 remains authoritative for ReviewCase / ReviewLedger fundamentals; this decision realizes the reserved HumanRaised origin for the ManualReplacement slice.
- MD-003 remains authoritative for materialization and overlap refusal; this decision extends materializable ManualReplacement to HumanRaised anchors.
- MD-017 remains authoritative for detector ManualReplacement payload and last-decision-wins; this decision does not broaden MD-017 itself.
- MD-018 / MD-021 remain authoritative for exact-pair explicit promotion; this decision extends eligible source decisions to HumanRaised ManualReplacement with an origin-aware locator.
- MD-020 / MD-021 remain authoritative for session formats v1 and v2; this decision adds new-session-only v3.
- Proposed MD-011 remains unaccepted except for the narrow HumanRaised + ManualReplacement portion recorded here.

## Implementation notes

Preferred packages after acceptance:

1. H1+H2 — core HumanRaised ReviewCase, CaseRaised, session v3, materialization, promotion locator, Project Memory format 2
2. H3 — desktop cue-span selection, Correct selected text, synthetic dogfood

H1+H2 should not ship without promotion eligibility; a HumanRaised correction that cannot enter Project Memory leaves the flywheel incomplete.
