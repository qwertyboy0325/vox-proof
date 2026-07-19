# MD-003: Minimal Reviewed Output Materialization Semantics

Status: accepted

Date: 2026-07-09

Decision authority: Ezra

## Context

VoxProof's source transcript must remain unchanged while human review produces accepted corrections.

MD-002 accepts `CorrectionDecision` and an append-only review ledger, but it does not define how accepted decisions become reviewed output.

`docs/architecture/pending-data-contracts.md` treats reviewed-output materialization as a decision gate.

`docs/product/v0.1-execution-order.md` requires this Material Decision before reviewed-output implementation.

The current implementation can record detector-raised decisions in memory, but it cannot yet derive reviewed SRT.

This decision authorizes only the minimal v0.1 reviewed-output derivation for detector-raised `AcceptAlternative` decisions.

## Decision

Reviewed SRT is a derived artifact.

It is produced from:

```text
source parsed transcript
+ review cases
+ append-only review ledger events
+ accepted applicable AcceptAlternative decisions
```

The source transcript remains unchanged.

Analyzer output does not directly edit transcript text.

## Materializing decision actions in v0.1

For v0.1:

* `AcceptAlternative` may produce a text replacement.
* `Reject` produces no text change.
* `Defer` produces no text change.
* `NeedsManualCorrection` produces no text change for now.

`NeedsManualCorrection` is a review signal only until a later Material Decision defines replacement payload semantics.

Only `AcceptAlternative` may materialize text changes in v0.1.

## Applicability rule

A decision can be materialized only when:

```text
decision.observed_revision == source_transcript.revision_id()
```

If the observed revision does not match the source transcript revision, the decision is not applicable.

v0.1 must refuse reviewed-output derivation if any selected accepted materializing decision is revision-mismatched.

Do not silently skip mismatched accepted decisions while claiming success.

## Materialized edit shape

A materialized edit in v0.1 is:

```text
replace source anchor byte range with selected alternative text
```

The source anchor is the detector-raised candidate span anchor.

The replacement text is the selected `CandidateAlternative` text referenced by the accepted `AcceptAlternative` decision.

v0.1 supports only single-anchor text replacement.

Out of scope for v0.1:

* multi-anchor edits
* insertion-only zero-width anchors
* deletion-only decisions
* timing edits
* segment split/merge
* cue reflow

## Coordinate plane

All edits are interpreted against the original parsed transcript segment text for the observed transcript revision.

Edits must not be interpreted against already-mutated text.

This prevents byte ranges from shifting during derivation.

For a segment with multiple applicable accepted edits, every anchor range refers to the same original segment text.

## Conflict rule

If two applicable accepted edits overlap in the same segment, reviewed-output derivation must refuse output with an explicit conflict report.

Do not choose one edit silently.

Do not partially apply non-conflicting edits while ignoring conflicts.

For v0.1, the exact conflict report shape may be implementation-defined, but it must identify the conflicting review cases or anchors.

## Non-overlapping edits

Multiple non-overlapping accepted edits may be applied to the same segment.

Derivation semantics:

* all ranges refer to the original segment text
* the final reviewed segment text is equivalent to applying all replacements without range shifting

An implementation may apply replacements in descending byte-position order, but that is an implementation convenience, not a product semantic requirement.

## SRT serialization

v0.1 reviewed SRT output may use canonical serialization from parsed transcript data.

It does not need to preserve original raw SRT formatting.

Canonical serialization must preserve:

* cue index
* start timestamp
* end timestamp
* reviewed cue text
* segment order

Raw formatting preservation is deferred.

## Failure semantics

Reviewed-output derivation must refuse output on:

* revision mismatch for an accepted materializing decision
* overlapping materializing edits
* invalid alternative index
* anchor resolution failure

Do not emit partial reviewed SRT and call it success.

## Explicitly deferred

The following are deferred:

* `CustomReplacement`
* HumanRaised materialization
* timing correction
* audio alignment
* multi-anchor edits
* insertion-only anchors
* deletion-only decisions
* segment split/merge/reflow
* format-preserving SRT output
* cross-revision decision migration
* persistence architecture
* GUI workflow

## Banned designs

The following designs are rejected:

1. Analyzer or model directly rewriting transcript text.
2. Applying decisions without matching observed revision.
3. Silently skipping failed accepted edits while claiming success.
4. Materializing `NeedsManualCorrection` without replacement semantics.
5. Using `CandidateKey` as the materialization identity root.
6. Treating reviewed output as mutable stored truth rather than a derived artifact.

## Implementation consequences

The first reviewed-output implementation must derive reviewed SRT from source transcript plus accepted applicable `AcceptAlternative` decisions only.

It must preserve source transcript immutability.

It must refuse output on revision mismatch, overlap, invalid alternative index, or anchor resolution failure.

This decision does not authorize persistence, decision-log file output, CLI review flow, HumanRaised materialization, timing edits, or format-preserving SRT output.

## Related proposed decisions

MD-003 remains authoritative for established v0.1 materialization of applicable `AcceptAlternative` decisions.

If accepted, `decisions/MD-011-proposed-human-raised-manual-replacement-correction-history.md` would add `ManualReplacement` as an explicit materializing decision for v0.2. Unresolved, withdrawn, and superseded decisions would remain non-materializing. MD-011 would not reinterpret established v0.1 materialization artifacts or historical semantics recorded here.
