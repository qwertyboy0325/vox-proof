# MD-011: Human-raised Review Cases, Manual Replacements, and Append-only Correction History

Status: proposed

Date: 2026-07-19

Decision authority: Ezra

Classification: review-case origin, correction-decision semantics, review-ledger history, and reviewed-output materialization extension (not accepted)

## Context

MD-002 established `ReviewCase` as the unit of review, append-only `ReviewLedger` events, and the v0.1 detector-raised decision slice.

MD-003 established minimal v0.1 reviewed-output materialization for applicable `AcceptAlternative` decisions only.

MD-002 deferred human-raised case acceptance, replacement payload semantics, withdrawal event shape, and a richer fold rule under the working term `CustomReplacement`.

v0.2 planning requires reviewers to raise cases analysis missed, author replacements absent from detector alternatives, and correct mistakes through append-only history without rewriting prior events or silently reactivating superseded decisions.

This proposed Material Decision records the v0.2 semantic boundary. It does not authorize implementation and does not select persistence technology.

## Terminology

This decision uses the following canonical terms:

```text
DetectorRaised
HumanRaised
ManualReplacement
NeedsManualCorrection
ReviewCase
CorrectionDecision
ReviewLedger
```

`DetectorRaised` preserves the established MD-002 origin term. v0.1 and v0.2 share this origin vocabulary. v0.2 extends the allowed origin set and lifecycle semantics with `HumanRaised`. Historical compatibility does not require a terminology alias.

MD-002 and MD-003 deferred payload semantics under the working term `CustomReplacement`. That term is not a separate v0.2 action type. If accepted, this decision standardizes the resolved manual correction action as `ManualReplacement`.

## Decision

### ReviewCase origin

Every `ReviewCase` has exactly one explicit origin:

```text
DetectorRaised
HumanRaised
```

A **DetectorRaised** case is produced from an analysis result.

A **HumanRaised** case is created explicitly by a reviewer when no suitable detector-raised case exists.

Human-raised cases:

* are canonical session review entities;
* are not detector output;
* must not be inserted retroactively into an immutable `AnalysisRun` or `AnalysisSnapshot`;
* survive later reanalysis unless explicitly withdrawn;
* retain provenance identifying the human creation event and observed source revision.

Raising a human-raised case and deciding it remain separate ledger events, as established in MD-002.

### Manual case anchor

A human-raised case must anchor to:

* one observed `TranscriptRevisionId`;
* one segment or cue;
* one contiguous Unicode-safe source-text range within that segment.

Selecting the entire cue text is valid.

Explicitly deferred:

* cross-segment anchors;
* discontinuous selections;
* timing modification;
* cue split/merge editing.

### Resolved and unresolved correction states

For v0.2 fold and materialization, distinguish whether a case currently has an active terminal decision and whether a successor decision resolves the case under the applicable decision contract.

**Unresolved, non-materializable**

* no active terminal decision that resolves the case;
* `NeedsManualCorrection`;
* `Defer`;
* a withdrawn active terminal decision with no resolving successor;
* withdrawal normally leaves the case unresolved.

**Terminal decisions that resolve the case**

* `ManualReplacement` — resolved and materializable;
* `AcceptAlternative` — resolved and materializable only where valid for the case origin.

**Reject**

`Reject` retains the semantic classification established in MD-002. MD-011 does not redefine it.

**Supersession**

Supersession is a relationship between ledger events: a later decision event may supersede an earlier applicable event. Supersession is not itself a resolution classification.

At most one terminal decision may be active for a given active `ReviewCase` at a time.

### NeedsManualCorrection

`NeedsManualCorrection` remains unresolved and non-materializable.

It records that the issue appears real but the available alternatives are insufficient. It does not supply output text and cannot alter reviewed output by itself.

A later immutable decision event may supersede it. Only a successor decision that is terminal and resolved under the applicable decision contract resolves the case. Not every later event resolves the case: `Defer` does not resolve the case, and withdrawal does not resolve the case and normally leaves it unresolved.

### ManualReplacement

`ManualReplacement` is a terminal, resolved, materializable human decision.

Its replacement text:

* may be absent from detector alternatives;
* does not need to exist in a glossary, session-term list, or other knowledge asset;
* must be non-empty;
* must be single-line;
* must not be byte-for-byte identical to the selected source text at the case anchor under the established source-text comparison rule used for review and materialization;
* must not be silently trimmed, normalized, rewritten, or inferred by the product;
* does not mean deletion;
* does not automatically create, update, or promote reusable knowledge.

`ManualReplacement` is valid for `DetectorRaised` and `HumanRaised` cases once replacement payload semantics are accepted and implemented.

Applying `AcceptAlternative` to a HumanRaised case remains a typed error, as established in MD-002.

### Append-only ReviewLedger history

`ReviewLedger` remains append-only.

Historical events are never deleted, mutated, or re-labelled in place.

The following actions create new immutable events:

* initial terminal or non-terminal decision;
* withdrawal of the active terminal decision;
* supersession by a new terminal decision;
* reapplication after withdrawal;
* manual-case withdrawal;
* later replacement or resolution after withdrawal.

Redo or reapplication creates a new event. It does not revive or rewrite an older event.

Knowledge-related invalidation or revocation may reference historical corrections but does not modify them. Knowledge governance remains out of scope for this decision.

### Active terminal decision and v0.2 fold rule

Review status and materialization eligibility are derived from append-only ledger events using a deterministic fold.

For a given active `ReviewCase`:

* at most one terminal decision is active at a time;
* a new terminal decision explicitly supersedes the current active terminal decision;
* withdrawing the current active terminal decision leaves the case unresolved unless a later event resolves it;
* withdrawing a successor does not silently reactivate an older superseded terminal decision;
* historical event identity remains stable;
* fold order is determined by the deterministic ordered `ReviewLedger` events applicable to the case, not by mutable array position, UI order, or ranking index.

Conceptual fold behavior:

```text
start unresolved
apply the deterministic ordered ReviewLedger events applicable to the case
terminal decision -> becomes active terminal, superseding any prior active terminal
withdraw active terminal -> clear active terminal; case becomes unresolved
withdraw successor only -> remain unresolved; do not restore superseded terminal
NeedsManualCorrection -> unresolved; non-materializable; may be superseded
Defer -> does not resolve the case
Reject -> classification remains governed by MD-002
AcceptAlternative / ManualReplacement -> active terminal; materializable when applicable and resolving under the applicable contract
```

MD-002's v0.1 last-decision-wins fold remains authoritative for established v0.1 artifacts that contain only `DecisionRecorded` events with no withdrawal or supersession vocabulary. v0.2 introduces explicit withdrawal and supersession semantics without retroactively reinterpreting v0.1 records.

### Stale-write protection

Authoritative correction commands must be evaluated against an expected active event identity, case revision, session revision, or equivalent optimistic-concurrency token supplied by the caller.

If the expected state is stale:

* reject the command;
* do not append a speculative correction event;
* do not silently overwrite or merge another reviewer's action.

The exact token representation is implementation-neutral and not decided here.

### Materialization extension

If accepted, this decision extends MD-003 by making applicable `ManualReplacement` decisions materializable.

Materialization uses the exact accepted replacement payload recorded in the ledger event, subject to established source-anchor binding, observed-revision applicability, overlap validation, and fail-closed refusal semantics from MD-003.

The following do not materialize reviewed output:

* `NeedsManualCorrection`;
* withdrawn active terminal decisions;
* superseded terminal decisions;
* unresolved cases;
* `Defer`;

`Reject` materialization classification remains governed by MD-002 and MD-003.

For `DetectorRaised` cases, established v0.1 `AcceptAlternative` materialization semantics remain unchanged.

For HumanRaised cases, materialization uses the case anchor and the accepted `ManualReplacement` payload rather than a detector alternative index.

Legacy v0.1 `AcceptAlternative` records remain interpreted against their original fixed candidate ordering at the time of decision.

### v0.1 compatibility

MD-011 must not retroactively reinterpret v0.1 records.

Established v0.1 meaning is preserved:

* v0.1 detector-raised cases retain their established meaning under MD-002 `DetectorRaised`;
* v0.1 accepted decisions and materialized artifacts remain historically interpretable;
* legacy alternative-index decisions remain bound to the candidate ordering present when the decision was recorded;
* MD-002 remains authoritative for its established v0.1 slice;
* MD-003 remains authoritative for established v0.1 materialization of applicable `AcceptAlternative` decisions.

If accepted, MD-011 extends the event vocabulary and v0.2 fold semantics for new work. Any reader handling both generations must apply the record or version semantics applicable to the historical artifact.

No concrete migration format is chosen here.

## Invariants

1. Analysis output cannot be mutated to fabricate a human-raised detector result.
2. Every materialized manual replacement has a human-authored immutable ledger event.
3. No history-changing user action deletes or rewrites an older ledger event.
4. No case has more than one active terminal decision.
5. Withdrawal never implicitly revives an older superseded terminal decision.
6. A stale command cannot become authoritative.
7. `ManualReplacement` payload is preserved exactly as accepted.
8. `NeedsManualCorrection` alone cannot alter reviewed output.
9. v0.1 historical artifacts remain interpretable under their established semantics.
10. Manual correction does not automatically mutate reusable knowledge.

## Banned designs

The following designs are rejected:

1. Editing detector output in place to add missed cases.
2. Forcing every manual replacement into the detector alternative list as if it were machine evidence.
3. Treating `NeedsManualCorrection` as resolved or materializable.
4. Deleting or mutating a ledger event to implement undo.
5. Automatically reactivating a superseded terminal decision when its successor is withdrawn.
6. Treating empty replacement text as deletion.
7. Silently trimming or normalizing manual replacement text on acceptance.
8. Using current UI position, queue rank, or alternative ranking index as durable decision identity.
9. Retroactively applying v0.2 withdrawal or supersession fold rules to v0.1 records.
10. Modeling `CustomReplacement` and `ManualReplacement` as separate canonical v0.2 action types.

## Explicitly deferred

The following remain deferred and are not owned by this decision:

* storage format, database, or event-log mechanism;
* UI design and workflow presentation;
* knowledge promotion, experience extraction, and reusable-knowledge governance;
* source-revision reconciliation and cross-revision decision migration;
* analysis scheduling and background-job mechanics;
* cross-segment editing, timing editing, and deletion semantics;
* exact withdrawal, supersession, and reapplication event payloads;
* reviewer identity and attribution model;
* concrete optimistic-concurrency token encoding;
* export manifest and completeness contract beyond MD-003's established v0.1 reviewed-SRT path.

## Consequences

If accepted:

* `ReviewCase` and `CorrectionDecision` become semantic supersets of the established v0.1 model;
* exhaustive Rust matches and validation will require updates before implementation;
* the materializer gains an explicit `ManualReplacement` path in addition to applicable `AcceptAlternative`;
* session storage will eventually need versioned event representation and generation-aware fold logic;
* UI must expose unresolved, manual, withdrawn, and superseded states distinctly from active terminal decisions;
* reanalysis must not absorb human-raised cases into detector snapshots;
* future experience extraction may reference corrections but cannot rewrite them.

This proposed decision does not authorize persistence implementation, desktop UI work, knowledge extraction, or source-revision reconciliation.

## Relationship to prior decisions

* MD-002 remains authoritative for the established v0.1 ReviewCase and ReviewLedger model.
* MD-003 remains authoritative for established v0.1 materialization of applicable `AcceptAlternative` decisions.
* If accepted, MD-011 extends the deferred human-raised, manual-replacement, withdrawal, supersession, and v0.2 fold slice without rewriting MD-002 or MD-003 historical semantics.

## Related proposed decisions

MD-011 remains authoritative for correction-history semantics.

If accepted, `decisions/MD-014-proposed-session-durability-recovery-and-retention-requirements.md` would require durable acknowledgement, stale-write rejection, and preservation through recovery and compaction. MD-014 would not redefine correction actions.
