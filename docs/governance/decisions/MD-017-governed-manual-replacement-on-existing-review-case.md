# MD-017: Governed Manual Replacement on an Existing ReviewCase

Status: accepted

Date: 2026-07-29

Accepted: 2026-07-29 per explicit owner authorization

Decision authority: Ezra

Classification: bounded v0.2 correction-decision and reviewed-output extension

## Context

MD-002 establishes `ReviewCase` as the unit of review, append-only
`ReviewLedger` events, and per-case last-decision-wins effective status.
MD-003 establishes immutable source text, single-anchor replacement against the
original transcript coordinate plane, deterministic reviewed-output
materialization, and fail-closed overlap handling.

The Gate 1 native desktop foundation can record the existing decision actions
but cannot yet record exact human-authored replacement text when detector
alternatives are insufficient. The broader proposed MD-011 also discusses
human-raised cases, withdrawal, and richer supersession semantics. Those broader
semantics are not accepted by this decision.

## Decision

### Authority shape

Manual Replacement is a correction decision:

```rust
CorrectionDecision::ManualReplacement {
    replacement: ManualReplacementText,
}
```

It is recorded inside the existing
`ReviewLedgerEvent::DecisionRecorded`. There is no parallel Manual Replacement
authority event.

Gate 2 applies only to an existing detector-raised `ReviewCase` produced by the
canonical analysis run. The application command continues to target the
existing `ApplicationReviewTarget`, which binds the canonical
`AnalysisSnapshot` and run-local `ReviewCaseId`. The ledger event continues to
bind the `ReviewCaseId` and observed `TranscriptRevisionId`.

### Replacement payload

`ManualReplacementText` preserves the accepted UTF-8 bytes exactly. The product
must not trim, normalize, rewrite, infer, or otherwise mutate the payload.

Validation rejects:

- an empty string;
- a string for which every character satisfies `char::is_whitespace`;
- any Unicode control character;
- U+2028 LINE SEPARATOR;
- U+2029 PARAGRAPH SEPARATOR;
- a value byte-identical to the source text selected by the existing
  `ReviewCase` anchor;
- a payload longer than 4096 UTF-8 bytes.

Replacement payloads are single-line. Manual Replacement never means deletion.

The existing canonical `ReviewCase` anchor and MD-003 materialization validity
rules remain authoritative. This decision does not add a second
manual-specific anchor policy, arbitrary range selection, zero-width anchors,
or cue-editing semantics.

### Append-only revision and effective status

Every correction revision appends another existing `DecisionRecorded` event.
For a given `ReviewCase`, effective status remains MD-002 last-decision-wins.
Historical events remain append-only and are not changed or deleted.

This decision does not add withdrawal, explicit supersession event identity,
undo-to-undecided, reactivation, or optimistic-concurrency semantics.

`NeedsManualCorrection` remains unresolved, signal-only, and
non-materializable. A later `ManualReplacement` is a separate immutable
decision and becomes effective through the ordinary last-decision-wins fold.

### Materialization and conflicts

An effective `ManualReplacement` is resolved and materializable. It uses the
same canonical materializer, immutable source coordinate plane, source-revision
applicability rule, anchor resolution, and overlap rule as
`AcceptAlternative`.

There is no decision-type precedence. If two effective materializing decisions
overlap, reviewed-output and export derivation fail closed. A later decision on
the same case replaces the earlier decision in effective status before overlap
evaluation.

### Replay and export

Same-process in-memory replay must reconstruct canonical analysis, revalidate
every exact Manual Replacement payload against its selected source text, replay
the append-only events, and compare:

- ledger events;
- effective statuses;
- progress;
- decision and session summaries;
- current projection;
- reviewed output;
- the authority-complete export bundle.

The three-file export topology remains:

```text
reviewed SRT
application decision log
application session summary
```

The application decision-log and session-summary renderer contracts advance to
frozen v2 identities. v2 records exact escaped Manual Replacement payloads and
separates effective Manual Replacement counts from accepted-alternative counts.
Historical v1 output retains its prior meaning and is not silently
reinterpreted. These human-readable outputs remain non-persistence,
non-re-import artifacts.

### Correction Memory boundary

A Manual Replacement may be considered as a possible future input to a
separately governed Correction Memory workflow. It does not create, promote,
update, or authorize Correction Memory or cross-material reuse.

## Consequences

- `CorrectionDecision`, ledger events, and derived status values now own a
  string-bearing variant and are no longer `Copy`.
- The application service owns validated Manual Replacement command intake.
- The canonical materializer gains one additional materializing decision path.
- Decision summaries and session summaries distinguish effective Manual
  Replacements from accepted detector alternatives.
- The egui draft remains non-authoritative until the application command
  succeeds.
- Changing a decision after export continues to invalidate the GUI's
  export-complete state.

## Explicitly deferred

- human-raised review cases and `CaseRaised` events;
- arbitrary or user-authored source ranges;
- deletion and zero-width replacement;
- multiline replacement, cue reflow, cue split/merge, and timing edits;
- withdrawal, explicit supersession identities, redo, and undo-to-undecided;
- persistence, serialization, restart recovery, and cross-revision migration;
- ASR and media playback;
- Correction Memory, promotion, and cross-material reuse.

## Rejected designs

1. A parallel Manual Replacement authority event.
2. Treating empty or whitespace-only text as deletion.
3. Silent trimming or Unicode normalization.
4. GUI-owned accepted replacement state.
5. Decision-type precedence for overlapping edits.
6. Materializing `NeedsManualCorrection`.
7. Treating a Manual Replacement as reusable knowledge.
8. Accepting the broader proposed MD-011 through this bounded decision.

## Relationship to prior decisions

- MD-002 remains authoritative for `ReviewCase`, append-only
  `DecisionRecorded`, and last-decision-wins effective status.
- MD-003 remains authoritative for source immutability, canonical
  materialization, anchor applicability, and overlap refusal.
- MD-016 remains authoritative for the non-authoritative egui boundary and
  direct in-process `ApplicationReviewSession` integration.
- MD-011 remains proposed and unaccepted outside the bounded semantics
  explicitly recorded here.
