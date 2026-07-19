# MD-014: Session Durability, Recovery, and Retention Requirements

Status: accepted

Date: 2026-07-19

Accepted: 2026-07-19 per explicit owner authorization

Decision authority: Ezra

Classification: durable session authority, recovery, lifecycle, retention, and compaction requirements

## Context

MD-001 through MD-004 establish transcript revision identity, review ledger authority, materialization, and analysis identity.

Proposed MD-011 establishes append-only correction history, stale-write protection at the command boundary, and durable ledger semantics for v0.2 corrections.

Proposed MD-012 establishes immutable knowledge snapshots, promotion and revocation provenance, and knowledge retention constraints.

Proposed MD-013 establishes bounded analysis jobs, atomic attachment of immutable analysis results, source revision lifecycle, reanalysis, and reconciliation. It explicitly deferred durability, recovery, locking, and retention to MD-014.

The data contract and v0.2 C4 draft recognize a future versioned local session store but do not define durability semantics, recovery classes, garbage collection, or compaction rules.

v0.2 planning requires durable authoritative session state without selecting a persistence backend, file structure, database, log format, locking primitive, integrity algorithm, encryption mechanism, or checkpoint strategy.

This Material Decision records required semantics and safety properties. It does not authorize implementation and does not claim benchmark, threat-model, or recovery-test evidence already exists.

## Requirement versus mechanism

```text
MD-014 defines required semantics and safety properties.
A later mechanism decision must select and justify an implementation.
```

Candidate mechanisms such as SQLite, JSONL, append-only logs, directory bundles, content-addressed blobs, or hybrid storage remain unresolved.

Any mention of candidate mechanisms in this decision is non-authoritative spike input only. It does not select, prefer, or approve an implementation.

## Decision

### Canonical versus derived state

#### Canonical session state

Canonical state includes or references authoritative historical facts such as:

* imported source revisions;
* `ReviewCase` identities and origins;
* append-only `ReviewLedger` events;
* immutable analysis-result identities and active-analysis selection history;
* knowledge governance identities stored within the session boundary where applicable;
* session identity and duplication lineage;
* format and governance metadata required to interpret history.

The exact field set may defer to domain decisions and the data contract.

Canonical state must not be silently reconstructed from disposable caches when authoritative history is missing.

#### Derived state

Derived state may include:

* queue projections;
* search indexes;
* thumbnails;
* media probe cache;
* reconciliation indexes;
* materialized previews;
* performance metrics;
* component or view caches;
* rebuildable snapshot indexes.

Derived state may be deleted and rebuilt without changing domain truth.

An immutable analysis result is not disposable merely because it is inactive.

### Durable command semantics

An authoritative command succeeds only after its semantic transition is durably committed.

Required rules:

* in-memory mutation alone is not success;
* the UI or caller must not receive authoritative success before durability succeeds;
* persistence failure rejects the command;
* after a durability failure, the application must block further authoritative writes until the session returns to a validated writable state;
* the system must expose the last known durable revision, sequence, event, or equivalent boundary;
* operational logging failure does not invalidate a domain command unless that log is itself canonical audit history.

This extends MD-011 command semantics and MD-013 atomic attachment semantics.

### Crash consistency

After interruption, the session must resolve to a valid bounded state such as:

```text
last fully committed state
recoverable incomplete transition
read-only salvage state
unrecoverable corruption
```

The storage mechanism must not silently expose a partially committed authoritative transition as valid.

Recovery must distinguish canonical corruption from derived-state loss.

Not every hardware or storage failure is recoverable.

### Single-writer ownership

A session permits at most one authoritative writer at a time.

Required behavior:

* conflicting writer access defaults to read-only or refusal;
* writer ownership must be explicit and verifiable;
* stale ownership cannot be cleared solely by trusting a process identifier;
* force-takeover requires revalidation and recovery checks;
* read-only access must not mutate canonical state;
* multiple readers may be allowed where the mechanism safely supports them.

PID files, advisory locks, mandatory locks, mutexes, leases, heartbeats, and database locks are not selected here.

### Stale-write prevention

Authoritative commands must validate an expected session revision, active event identity, state token, or equivalent optimistic-concurrency precondition.

If stale:

* reject the command;
* append no speculative event;
* do not overwrite another transition;
* do not silently merge conflicting user actions.

This extends MD-011 stale-write protection and MD-013 stale attachment rejection.

The concrete token representation is not defined here.

### Session format versioning

Every durable session format must declare enough version information to interpret canonical state safely.

Required rules:

* compatible older versions may be migrated only through explicit, version-aware logic;
* migration never silently drops canonical history;
* migration failure leaves the original data intact;
* unknown newer versions must not open writable;
* unknown newer versions may open read-only only if interpretation is demonstrably safe;
* parser or migration code must fail closed on ambiguous authority;
* format version is distinct from product version.

A concrete version-number scheme is not defined here.

### Integrity validation

The system must be able to detect relevant corruption or inconsistency in canonical state.

Validation may need to cover conceptually:

* record identity;
* ordering or sequence relationships;
* references;
* source revision identity;
* event lineage;
* snapshot linkage;
* format consistency;
* canonical and derived separation.

Integrity checks are not necessarily authenticity guarantees.

Checksums do not equal signatures.

Corruption detection does not prove malicious tampering.

Cryptographic authenticity is a separate future requirement if needed.

No integrity algorithm is selected here.

### Untrusted input and resource bounds

Session files and imported state are untrusted input.

Required behavior:

* validate sizes, counts, nesting, references, and version fields;
* reject malformed or impossible state transitions;
* avoid unbounded memory allocation;
* avoid unbounded recursion or replay;
* bound decompression and expansion where applicable;
* avoid path traversal and unintended external-file access;
* fail closed rather than guessing missing authority.

Parser libraries and language-specific mechanisms are not selected here.

### Bounded startup and replay

Opening a session must have bounded behavior.

Required rules:

* no unbounded full-history replay without checkpoints, indexes, or an equivalent bounded strategy once scale requires it;
* derived indexes may be rebuilt;
* canonical history must remain interpretable;
* startup failure must not mutate the session;
* progress or diagnostics during open are non-authoritative;
* large sessions must remain cancellable or fail with an explicit bounded resource error where possible.

Checkpoints are not mandated as a specific mechanism here.

### Session lifecycle

Conceptual application-level states may include:

```text
Closed
Opening
OpenWritable
OpenReadOnly
RecoveryRequired
Recovering
Corrupted
UnsupportedVersion
Closing
```

Final enum shapes remain implementation-defined.

Required rules:

* only a validated writable state accepts authoritative commands;
* opening validates version, integrity, ownership, and canonical reconstruction;
* read-only state permits inspection and safe non-mutating operations;
* writable state must not be entered through silent fallback;
* lifecycle state is application authority, not presentation-only UI state;
* full implementation belongs to the application layer and storage adapter.

Detailed UI transitions are not defined here.

### Recovery classifications

Recovery must distinguish conceptually:

```text
SafeAutomaticRecovery
ManualReviewRequired
ReadOnlySalvageOnly
Unrecoverable
```

#### Safe automatic recovery

May include:

* rebuildable index loss;
* cache corruption;
* abandoned temporary output;
* incomplete uncommitted analysis artifacts;
* checkpoint behind canonical history where replay is valid.

#### Manual review required

May include:

* uncertain ownership;
* ambiguous final transition;
* sequence or reference inconsistency;
* interrupted migration;
* source-integrity mismatch.

#### Read-only salvage

Canonical state is partially interpretable but cannot safely accept new writes.

#### Unrecoverable

Required authority cannot be reconstructed or validated.

Exact repair algorithms are not defined here.

### Canonical versus derived corruption

* derived corruption may be discarded and rebuilt;
* canonical corruption blocks normal writable open;
* canonical corruption must not be hidden by rebuilding derived views;
* a partially readable session may still support bounded inspection or export of salvageable information;
* salvage must not fabricate missing canonical history.

### Close semantics

Authoritative transitions are already durable before success, so normal close must not depend on a traditional “save all pending changes” model.

Required close behavior:

* stop accepting new authoritative commands;
* allow or terminate the current transition safely;
* cancel incomplete analysis jobs without attaching partial results;
* release writer ownership;
* record or preserve enough state to distinguish clean close from interrupted operation where useful;
* close failure must not silently discard a committed transition.

Flush mechanisms are not selected here.

### Session duplication

Duplicating a session creates:

* a new session identity;
* explicit lineage to the source session;
* independent writer ownership;
* preserved copied canonical history as defined by the duplication operation.

It must not:

* continue using the original session identity;
* share mutable writer state;
* silently mutate the original;
* claim that a simple filesystem copy is always a valid semantic duplication.

Final identifier remapping rules are not defined here.

### Retention classes

Conceptual retention categories may include:

```text
PermanentCanonical
ReferencedHistorical
UserPinned
RebuildableDerived
Temporary
GarbageCandidate
```

Final enum shapes remain implementation-defined.

#### Permanent canonical

Examples include source revisions, `ReviewLedger` events, governance events, session identity and lineage, and required canonical provenance.

#### Referenced historical

Examples include immutable analysis results referenced by decisions, reconciliation, audit, accepted knowledge, or retained exports; knowledge snapshots referenced by historical analyses; extraction or proposal artifacts referenced by accepted knowledge.

#### User pinned

Explicitly preserved by user or product workflow.

#### Rebuildable derived

Indexes, caches, previews, thumbnails, temporary metrics.

#### Temporary

Incomplete jobs, failed or cancelled uncommitted output, abandoned update or download artifacts where applicable.

#### Garbage candidate

Only artifacts proven unreferenced and non-authoritative.

### Reachability and garbage collection

Deletion of anything beyond low-risk temporary or rebuildable derived data requires reachability analysis from canonical roots.

Conceptual roots include:

* canonical history;
* active and historical decisions;
* accepted knowledge provenance;
* retained immutable analysis identities;
* user pins;
* retained export manifests where they reference session artifacts;
* latest validated canonical state.

Required rules:

* referenced artifacts are not deleted;
* unknown or incomplete reference graphs fail closed;
* automatic GC is limited to low-risk temporary or cache data;
* higher-value historical cleanup requires explicit user confirmation;
* deletion must not silently break provenance;
* cleanup should expose affected artifact classes and reclaimable size where feasible.

The traversal implementation is not defined here.

### Analysis-result retention

* active analysis results are retained;
* historical results referenced by decisions, lineage, knowledge, audit, or export are retained;
* unreferenced historical results may become explicit garbage candidates under policy;
* failed or cancelled uncommitted analysis output may be deleted automatically;
* deletion of a historical result must not make a retained canonical event uninterpretable;
* a minimal disposal or tombstone record may be retained when needed for audit, but this decision does not define its schema.

Not every inactive snapshot is disposable.

### Knowledge-snapshot retention

Knowledge snapshots referenced by historical analyses must remain available or reconstructable with equivalent immutable content and identity semantics.

Content deduplication may allow multiple logical snapshot identities to reference shared immutable content.

Deduplication:

* must not alter logical identity;
* must not change historical provenance;
* must not merge conflicting governance histories;
* is an implementation option, not a requirement.

### Compaction

Compaction may improve performance or space use but must not:

* change event identity;
* change event order;
* erase provenance;
* collapse distinct correction decisions into one;
* rewrite historical knowledge versions;
* invalidate old analysis identities;
* make canonical history uninterpretable.

Compaction may use verified checkpoints, archived immutable segments, content deduplication, or compressed historical storage only if semantics remain equivalent.

No compaction strategy is selected here.

### GC safety and failure behavior

Before destructive cleanup:

1. construct a cleanup plan;
2. determine affected artifacts and references;
3. validate no protected canonical root depends on them;
4. perform deletion with recoverable or all-or-nothing semantics where required;
5. preserve audit or disposal information where required;
6. update derived indexes consistently.

If validation is incomplete or storage is corrupted:

```text
do not perform destructive GC
```

Transaction primitives are not selected here.

### Privacy and sensitive content

Session storage may contain sensitive transcript, correction, media-reference, project, and knowledge data.

Required behavior:

* do not place content in operational logs by default;
* avoid absolute paths and host or user identity where not required;
* preserve minimum necessary provenance;
* diagnostic export must remain separately governed;
* deletion and retention policies must be transparent;
* remote telemetry is outside this decision and remains disabled unless separately accepted;
* optional encryption remains a separate mechanism or security decision.

At-rest encryption is not claimed to exist.

### Security and performance spike

Before selecting a persistence mechanism, a bounded spike must evaluate:

* crash consistency;
* single-writer behavior;
* stale-write handling;
* format migration;
* integrity detection;
* malformed-input resistance;
* startup and replay performance;
* memory bounds;
* compaction feasibility;
* backup and duplication behavior;
* platform behavior on Windows and macOS.

The spike must produce evidence and a separate mechanism decision.

MD-014 acceptance alone does not authorize selecting or implementing a backend without that review.

## Invariants

1. An authoritative command is not successful before its transition is durable.
2. A persistence failure cannot leave the UI believing an uncommitted transition succeeded.
3. At most one authoritative writer owns a session.
4. A stale command cannot overwrite newer state.
5. Unknown newer formats do not open writable.
6. Derived corruption cannot be mistaken for canonical corruption, or vice versa.
7. Canonical corruption blocks normal writable open.
8. Rebuildable derived data may be discarded without changing domain truth.
9. Canonical history is not destructively garbage-collected.
10. Referenced historical artifacts are retained.
11. Withdrawal, supersession, revocation, and reanalysis provenance remains interpretable after compaction.
12. GC fails closed when reachability is incomplete.
13. Session duplication creates a new identity.
14. Read-only mode does not mutate canonical state.
15. Incomplete analysis output is not committed during close or recovery.
16. Sensitive content is not written to ordinary operational logs by default.
17. A later persistence mechanism must satisfy these requirements without rewriting domain semantics.

## Banned designs

The following designs are rejected:

1. Treating in-memory state as successfully saved.
2. Permitting multiple writers with last-write-wins.
3. Opening unknown newer formats writable.
4. Rebuilding missing canonical history from caches.
5. Treating any inactive analysis result as disposable.
6. Deleting revoked or superseded history.
7. GC without reachability analysis.
8. Compaction that rewrites event identities.
9. Force-unlocking solely because a PID is absent.
10. Silent fallback from recovery into writable without validation.
11. Interpreting operational logs as canonical audit history.
12. Treating checksums as authenticity proof.
13. Selecting SQLite, JSONL, bundle, or hybrid storage in this requirements decision.
14. Claiming encryption without a separate mechanism decision.
15. Silently re-importing or reinterpreting source content during recovery.
16. Conflating format version with product version as authority.
17. Destructive GC when reference reachability is incomplete.

## Explicitly deferred

The following remain deferred and are not owned by this decision:

* persistence backend, database, append-log format, and directory layout;
* transaction mechanism, file lock, and OS lock implementation;
* checksum, MAC, signature, and encryption algorithms;
* key management and checkpoint format;
* storage compression algorithm;
* concrete retention durations;
* UI styling and installer behavior;
* cloud sync, collaboration, and automatic backup service;
* export contract and updater implementation;
* exact lifecycle enum payloads and token encodings;
* tombstone or disposal record schema;
* mechanism-selection Material Decision (future, not MD-015 in this task scope).

## Consequences

Acceptance records durable session authority requirements. It does not authorize persistence implementation, encryption deployment, or mechanism selection.

* a separate persistence-mechanism decision is required before backend selection;
* the application layer needs explicit lifecycle states;
* adapters must distinguish writable, read-only, and recovery modes;
* commands require durable acknowledgement and stale-write checks;
* storage must preserve canonical and derived separation;
* opening and recovery need bounded validation;
* historical analysis and knowledge references constrain GC;
* session duplication is semantic, not merely a file copy;
* security and performance testing becomes release-blocking before persistence selection;
* UI must expose recovery and read-only conditions without becoming authority;
* storage implementation remains open.

## Compatibility with existing decisions

### MD-011

* append-only correction history remains canonical;
* stale-write protection is reinforced;
* withdrawal and supersession events must remain durable and interpretable;
* MD-014 does not redefine correction actions.

### MD-012

* immutable correction, proposal, knowledge, and snapshot provenance constrain retention;
* revocation does not permit historical deletion;
* knowledge-snapshot identity survives deduplication and compaction;
* MD-014 does not redefine promotion or conflict semantics.

### MD-013

* analysis attachment must become durable before authoritative success;
* incomplete or stale results remain non-authoritative;
* immutable analysis results constrain retention;
* MD-014 owns lifecycle, writable and read-only state, recovery, locking, and GC;
* MD-013 execution and reconciliation semantics remain unchanged.

### MD-001 through MD-004

* transcript revision identity, `ReviewLedger` authority, materialization, and v0.1 analysis identity remain unchanged;
* persistence must preserve their established historical interpretation.

## Relationship to prior decisions

* MD-001 through MD-004 remain authoritative for their established v0.1 semantics.
* Proposed MD-011, MD-012, and MD-013 remain authoritative for their respective domains.
* MD-014 extends durability, recovery, and retention requirements without rewriting domain semantics recorded in those decisions.

## Related decisions

MD-014 owns durable session authority, lifecycle, recovery, locking, retention, and compaction requirements for v0.2.

Proposed MD-011 would remain authoritative for correction-history semantics.

Proposed MD-013 would remain authoritative for analysis execution, attachment, reanalysis, and reconciliation semantics.

Neither proposed decision selects persistence technology.

Accepted `decisions/MD-015-proposed-persistence-mechanism-evidence-protocol.md` defines the evidence protocol required before any persistence mechanism may be selected. MD-015 does not select a backend or weaken MD-014 requirements.
