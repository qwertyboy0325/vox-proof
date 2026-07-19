# MD-013: Analysis Jobs, Source Revisions, and Reconciliation

Status: proposed

Date: 2026-07-19

Decision authority: Ezra

Classification: bounded analysis job execution, immutable analysis snapshots, source revision lifecycle, reanalysis, and case reconciliation (not accepted)

## Context

MD-001 establishes stable `TranscriptRevisionId` for parsed transcript content.

MD-002 and proposed MD-011 establish `ReviewCase`, append-only `ReviewLedger` events, and correction authority.

MD-003 establishes reviewed-output materialization from source transcript plus applicable human correction decisions.

MD-004 establishes effective v0.1 analysis identity inputs for `AnalysisRun` and `AnalysisSnapshot`.

Proposed MD-012 establishes immutable knowledge snapshots, ranking-policy identity, and the distinction between historical reproduction and current-knowledge reanalysis.

The data contract treats `AnalysisRun` as one bounded analysis execution and `AnalysisSnapshot` as the immutable identity of effective analysis inputs and attached analytical result context. It does not yet define background jobs, atomic attachment, stale-result rejection, source re-import, or cross-revision reconciliation.

v0.2 planning requires bounded background analysis, immutable attached results, explicit source revision lifecycle, preserved human-raised cases across reanalysis, and derived reconciliation without automatic decision migration.

This proposed Material Decision records the v0.2 semantic boundary. It does not authorize implementation, does not select persistence technology, and does not claim benchmark or pilot evidence for job execution or reconciliation.

## Terminology

This decision uses the following conceptual entities:

```text
AnalysisJob
AnalysisRun
AnalysisSnapshot
TranscriptRevisionId
SourceRevision
KnowledgeSnapshot
ReviewCase
ReviewLedger
ReconciliationResult
CaseLineage
```

**Relationship to established terms**

* `AnalysisJob` is transient application execution. It is not canonical session analysis authority.
* `AnalysisRun` and `AnalysisSnapshot` remain the established immutable completed analytical result identities. In v0.2, an attached completed result is represented by an immutable `AnalysisSnapshot` and its associated `AnalysisRun` provenance boundary.
* `SourceRevision` means immutable imported source content authority for a session workflow. The established v1 identifier is `TranscriptRevisionId` per MD-001. This decision uses `SourceRevision` only as conceptual prose where lifecycle semantics apply; it does not replace `TranscriptRevisionId`.
* `KnowledgeSnapshot` remains owned by proposed MD-012. It is a declared analysis input where reusable knowledge affects output. It must not be conflated with `AnalysisSnapshot`.

## Decision

### Analysis job execution

Analysis runs as a bounded background **analysis job**.

Required rules:

* the UI or caller remains responsive;
* execution is cancellable through cooperative cancellation;
* cancellation is checked at bounded checkpoints;
* progress is a non-authoritative projection;
* progress may be delayed, coalesced, or lost without changing correctness;
* job resource use and concurrency are bounded;
* one authoritative analysis job per session is allowed at a time in v0.2;
* global concurrency must also be bounded;
* a job is bound to declared immutable inputs before execution begins.

An `AnalysisJob` must not become a durable substitute for an attached `AnalysisSnapshot`.

### Declared analysis inputs

Before execution begins, a job must bind conceptually to:

* source revision identity (`TranscriptRevisionId`);
* analyzer and detector identities and versions;
* analyzer configuration;
* applicable `KnowledgeSnapshot` identity, if reusable knowledge is used;
* ranking-policy identity, where ranking affects output;
* any other declared deterministic input already required by MD-004.

Analyses that do not use reusable knowledge do not require a knowledge snapshot input.

A live mutable source, configuration, detector registry, or knowledge store must not become an undeclared hidden dependency.

MD-004 remains authoritative for established v0.1 effective analysis identity.

MD-012 remains authoritative for knowledge snapshot and ranking-policy semantics.

### Partial result semantics

Partial findings are not canonical session analysis results.

During execution:

* temporary findings may exist;
* progress may expose counts or stages;
* cancellation or failure discards the uncommitted result;
* partial output must not be attached as an authoritative snapshot;
* partial findings must not create canonical `ReviewCase` values;
* partial findings must not affect materialization or knowledge promotion.

Internal temporary storage is permitted. Temporary storage is not canonical authority.

### Completion and atomic attachment

A result may become authoritative only after:

1. analysis completes successfully;
2. all required result validation passes;
3. input identities still match the intended session and source context;
4. the complete immutable result is attached atomically or with equivalent all-or-nothing semantics.

The caller must not receive authoritative success before attachment succeeds.

If attachment fails, the job result remains non-authoritative.

Attachment creates or records an immutable `AnalysisSnapshot` and its associated `AnalysisRun` provenance boundary. It does not mutate earlier snapshots.

### Stale-result rejection

A completed job must be rejected if its declared inputs are stale relative to the command or session context under which attachment was requested.

Examples include:

* active source revision changed;
* session revision advanced incompatibly;
* requested knowledge snapshot or configuration no longer matches the attachment target;
* another authoritative analysis completed first;
* the target session is no longer writable.

Stale rejection:

* does not mutate the session;
* does not overwrite the newer result;
* may preserve bounded diagnostics;
* must not silently rebind the result to newer inputs.

Deleting every stale temporary artifact is not required here. Retention belongs to MD-014.

### Immutable analysis snapshots

Every attached completed result is immutable.

Reanalysis creates a new snapshot identity.

It must not:

* overwrite an earlier result;
* mutate earlier cases;
* rewrite earlier provenance;
* update old decisions in place;
* replace the knowledge snapshot identity attached to an old result.

An attached `AnalysisSnapshot` conceptually records or references:

* snapshot identity;
* source revision (`TranscriptRevisionId`);
* declared analyzer and detector versions;
* configuration identity;
* knowledge snapshot identity where applicable;
* ranking policy identity where applicable;
* generated detector-raised `ReviewCase` or finding identities;
* creation and attachment provenance.

Final serialization remains implementation-defined.

### Source revisions

Source content is represented through immutable source revision identities.

Required rules:

* paths are not source identity;
* a mutable external file is not canonical historical authority;
* imported authoritative content must remain reconstructable independently of later external-file changes;
* decisions and analysis snapshots remain bound to the revision they used;
* parser or interpretation identity must be preserved where it can affect semantic interpretation;
* parser changes must not silently reinterpret historical revisions.

MD-001 remains authoritative for `TranscriptRevisionId` and path independence.

Identical canonical imported content may resolve to the existing semantic revision rather than create a duplicate. Changed text, timing, structure, parser interpretation, or other semantically relevant content may create a new revision.

### Re-import

Re-importing source content must not overwrite an existing source revision.

Required behavior:

* identical canonical imported content may resolve to the existing semantic revision;
* changed content may create a new revision;
* a new revision may retain lineage to the source or session context where useful;
* existing decisions remain bound to the old revision;
* switching active revision does not migrate decisions automatically.

The canonicalization algorithm is not defined here. Byte identity alone is not sufficient if the established source contract distinguishes semantic interpretation.

### External file changes

External source-file changes may be detected and reported.

They must not be silently imported.

Required behavior:

* the session remains bound to its imported authoritative revision;
* the user may explicitly choose to re-import;
* a missing or modified external file does not erase historical source content;
* automatic filesystem watching is not required for v0.2;
* polling, watcher implementation, and path permissions are implementation details.

### Reanalysis

Reanalysis:

* creates a new immutable `AnalysisSnapshot`;
* keeps old snapshots;
* keeps old detector-raised cases;
* keeps old decisions and provenance;
* binds to explicit source and knowledge inputs;
* does not overwrite the currently active or historical result merely because it is newer;
* may become the selected active analysis snapshot only through an explicit application transition.

Reanalysis job execution semantics are owned here. Reanalysis knowledge-input semantics remain owned by MD-012.

### Human-raised cases

Human-raised cases are session-level canonical review entities, not analysis output.

Required rules:

* they survive detector reanalysis;
* they remain bound to their source revision and anchor;
* they are not copied into new analysis snapshots;
* they are not deleted because no detector finding matches them;
* they may later require source-revision reconciliation when the source changes;
* withdrawal remains governed by MD-011.

MD-013 does not redefine human-raised creation or correction semantics.

### Reconciliation

**Reconciliation** is a derived comparison between cases or findings across source revisions or analysis snapshots.

It is not mutation or migration.

A **reconciliation result** is a derived, rebuildable classification. Conceptual outcomes may include:

```text
Exact
Probable
Conflict
NoSuccessor
New
Stale
```

Final enum shapes remain implementation-defined.

Required rules:

* reconciliation results are derived and rebuildable;
* exact correspondence may establish lineage;
* probable correspondence remains non-authoritative;
* conflicts stay visible;
* no successor does not delete the historical case;
* new cases have no required predecessor;
* stale or invalid anchors fail closed;
* reconciliation does not change existing decision identity.

Reconciliation is evidence for review. It is not correction authority.

### Case lineage

**Case lineage** represents a derived relationship between historical and newer cases.

It may support:

* audit navigation;
* comparison;
* reapply suggestions;
* conflict detection;
* source revision transition review.

It must not:

* silently copy decisions;
* silently change the active decision;
* mutate an `AnalysisSnapshot`;
* claim semantic equivalence when mapping is ambiguous.

Exact lineage is evidence of correspondence, not authorization to migrate a decision.

### Decision reapplication

A historical decision may be suggested for reuse when reconciliation provides sufficient evidence.

Reapplication must:

* require explicit reviewer confirmation;
* create a new `ReviewLedger` event;
* preserve provenance to the historical decision;
* bind to the new case and source revision;
* pass current anchor, overlap, and materialization validation;
* not revive or mutate the historical decision event.

Manual replacements may be suggested for reuse but remain non-authoritative until confirmed.

### Detector finding disappearance

If a detector-raised case does not appear in a later analysis:

* the historical case and decisions remain;
* disappearance does not revoke an approved correction;
* disappearance does not prove the prior case was invalid;
* reconciliation may classify it as `NoSuccessor`, stale, or equivalent;
* active output behavior remains governed by the applicable source revision and active decisions.

Corrections must not be auto-withdrawn.

### Split, merge, and ambiguity

When source revisions split or merge cues or segments, or anchors cannot map unambiguously:

* reconciliation must not guess a unique authoritative successor;
* the condition must remain visible;
* reviewer intervention is required before reapplication;
* cross-segment correction is not introduced by this decision;
* historical anchors remain unchanged.

### Active analysis selection

A session may contain multiple immutable analysis snapshots.

Exactly one may be selected as the active analysis context for the current review workflow, or none where the session has not completed analysis.

Changing active analysis:

* is an explicit application transition;
* does not delete old snapshots;
* does not migrate decisions;
* does not absorb human-raised cases into detector results;
* may require conflict or reconciliation review before materialization or export.

The complete session state machine, durability, recovery, locking, and retention requirements belong to MD-014.

## Invariants

1. Partial analysis results never become canonical.
2. Cancellation or failure cannot attach a partial snapshot.
3. Every attached analysis result is immutable.
4. Reanalysis creates a new identity.
5. Historical snapshots and decisions are never overwritten by reanalysis.
6. Every analysis declares the source revision it used.
7. Knowledge-informed analysis declares the immutable knowledge snapshot it used.
8. Stale results cannot be silently rebound or attached.
9. Human-raised cases are not analysis output.
10. Human-raised cases survive detector reanalysis.
11. Reconciliation is derived and does not migrate decisions.
12. Exact lineage does not authorize automatic decision copying.
13. Decision reapplication requires a new ledger event.
14. Detector disappearance does not revoke a correction.
15. External file changes do not silently replace the imported source revision.
16. Ambiguous split or merge mapping does not fabricate a unique successor.
17. Old source revisions remain historically reconstructable.
18. Active analysis selection does not erase history.

## Banned designs

The following designs are rejected:

1. Running authoritative analysis synchronously on the UI thread.
2. Attaching partial findings incrementally as canonical cases.
3. Overwriting the previous analysis result.
4. Reading mutable live source or knowledge as undeclared inputs.
5. Rebinding a stale completed result to newer inputs.
6. Treating file path as source identity.
7. Silently re-importing changed external files.
8. Mutating old cases to represent new detector output.
9. Automatically copying decisions across exact lineage.
10. Deleting cases with no successor.
11. Invalidating approved corrections because a detector no longer reports them.
12. Guessing through split or merge ambiguity.
13. Inserting human-raised cases into `AnalysisRun` output.
14. Using reconciliation as authority rather than evidence.
15. Conflating `AnalysisSnapshot` with `KnowledgeSnapshot`.
16. Treating progress projection as canonical analysis state.

## Explicitly deferred

The following remain deferred and are not owned by this decision:

* detector algorithms and ranking rules;
* knowledge governance beyond declared inputs;
* persistence backend, transaction primitive, and file format;
* session lock implementation;
* UI progress presentation;
* media synchronization;
* correction-decision semantics;
* reviewed-output materialization;
* automatic decision migration;
* storage retention and stale-artifact deletion policy;
* export contract;
* complete session lifecycle and recovery state machine;
* exact reconciliation enum payloads;
* canonicalization algorithm for re-import;
* worker pool, async runtime, process model, and thread counts.

MD-014 will define durability, recovery, locking, and retention requirements. This decision does not pre-empt MD-014.

## Consequences

If accepted:

* the application layer needs a transient `AnalysisJob` model;
* the session model supports multiple immutable analysis snapshots;
* active-analysis selection becomes explicit;
* analysis input identity expands where v0.2 uses knowledge and ranking;
* reanalysis requires reconciliation projections;
* source revisions and parser interpretation need durable provenance later;
* UI must separate progress from authoritative results;
* cancellation and failure do not create canonical findings;
* historical decisions remain tied to their original cases and revisions;
* persistence must eventually support atomic attachment and immutable history;
* MD-014 will define durability, recovery, locking, and retention requirements.

This proposed decision does not authorize persistence implementation, UI design, storage technology selection, or runtime architecture selection.

## Compatibility with existing decisions

### MD-001

* MD-001 remains authoritative for transcript revision identity;
* paths remain non-authoritative;
* MD-013 extends revision lifecycle, re-import, and reconciliation semantics for v0.2;
* historical MD-001 artifacts are not reinterpreted.

### MD-002 and MD-011

* `ReviewLedger` remains correction authority;
* detector-raised and human-raised case semantics remain unchanged;
* MD-013 does not redefine decision actions;
* decision reapplication creates a new ledger event;
* human-raised cases remain outside analysis output.

### MD-003

* materialization remains based on active valid decisions;
* reconciliation and analysis lineage cannot materialize output by themselves;
* stale or ambiguous mappings fail closed.

### MD-004

* MD-004 remains authoritative for v0.1 effective analysis identity;
* MD-013 adds execution and immutable attached-result semantics for v0.2;
* declared knowledge snapshot and ranking inputs come from MD-012;
* historical v0.1 analysis artifacts are not reinterpreted.

### MD-012

* MD-012 owns knowledge governance, snapshots, conflict, ranking, and reproduction intent;
* MD-013 consumes immutable knowledge snapshots as declared inputs;
* MD-013 owns job execution, attachment, reanalysis, source revision, and reconciliation;
* neither decision may mutate the other's historical identities.

## Relationship to prior decisions

* MD-001 remains authoritative for established transcript revision identity.
* MD-002 and proposed MD-011 remain authoritative for correction and ledger semantics.
* MD-003 remains authoritative for established v0.1 materialization.
* MD-004 remains authoritative for established v0.1 effective analysis identity.
* Proposed MD-012 remains authoritative for knowledge snapshot and ranking-policy semantics.
* If accepted, MD-013 extends analysis execution and reconciliation without rewriting historical semantics recorded in those decisions.

## Related proposed decisions

If accepted, this decision would own bounded analysis job execution, immutable attached-result semantics, source re-import, reanalysis, and reconciliation for v0.2.

Proposed MD-012 would remain authoritative for immutable knowledge snapshots and reproduction intent. Neither proposed decision would rewrite historical identities recorded in the other.

If accepted, `decisions/MD-014-proposed-session-durability-recovery-and-retention-requirements.md` would own durability, lifecycle, locking, recovery, and retention requirements. MD-013 would remain authoritative for analysis execution, attachment, reanalysis, and reconciliation semantics. Neither proposed decision selects persistence technology.
