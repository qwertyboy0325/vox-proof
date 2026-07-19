# MD-015: Persistence Mechanism Evidence Protocol

Status: accepted

Date: 2026-07-19

Accepted: 2026-07-19 per explicit owner authorization

Decision authority: Ezra

Classification: pre-selection persistence mechanism evidence protocol and pass/fail gates

## Context

Proposed MD-014 records the durability, integrity, recovery, lifecycle, and retention requirements that any v0.2 session persistence mechanism must satisfy.

MD-014 requires a bounded security and performance spike before selecting a persistence backend, but it does not define the evidence protocol, shared workload, fault-injection matrix, measurement requirements, artifact structure, or pass/fail gates for that spike.

The data contract and v0.2 C4 draft recognize a future versioned local session store but do not define how candidate mechanisms must be compared.

v0.2 planning requires comparable, reproducible evidence before a later Material Decision may select a session persistence mechanism.

This Material Decision records the evidence protocol only. It does not authorize production persistence implementation, does not select a backend, and does not claim that spike evidence already exists.

## Relationship to MD-014

```text
MD-014 defines the required durability, integrity, recovery, lifecycle, and retention semantics.

MD-015 defines how candidate mechanisms must be tested against those requirements.

A later Material Decision selects the mechanism based on recorded evidence.
```

MD-015 must not weaken MD-014 requirements.

Passing the bounded spike makes a candidate eligible for comparison. It does not accept a backend.

## Decision

### Authoritative scope

MD-015 owns the evidence protocol required before a later decision may select a session persistence mechanism.

It owns:

1. candidate classes that must receive comparable evaluation;
2. bounded prototype requirements;
3. shared semantic workload;
4. fault-injection scenarios;
5. correctness assertions;
6. security and malformed-input cases;
7. performance and resource measurements;
8. platform coverage;
9. evidence artifact structure;
10. reproducibility requirements;
11. pass/fail gates;
12. disqualification conditions;
13. result interpretation rules;
14. the boundary between spike evidence and production authorization.

MD-015 does not own:

* production backend selection;
* production persistence implementation;
* final schema;
* final file or directory layout;
* application migration rollout;
* installer packaging;
* backup service;
* cloud synchronization;
* collaboration;
* encryption selection;
* telemetry;
* final retention durations;
* final recovery UI;
* v0.2 establishment evidence beyond the persistence mechanism question.

### Candidate mechanism classes

At least two materially distinct candidate classes must be evaluated against the same semantic workload.

Required candidate classes include at least:

```text
Embedded transactional database
Append-oriented event/log storage
Versioned session bundle or directory package
Hybrid canonical-log plus derived-index design
```

Candidate examples such as SQLite or JSONL may be named only as examples of classes, not as selected solutions.

Rules:

* at least two materially distinct mechanism classes must be evaluated;
* a candidate may be excluded before implementation only with documented evidence-based rationale;
* candidates must implement the same externally observable semantic workload;
* a deliberately weak strawman does not satisfy comparative evaluation;
* production dependencies are not approved merely because used in the spike.

All four classes need not become full prototypes if bounded pre-screening justifies narrowing. Pre-screening must record why excluded classes are not materially comparable or why they fail early disqualification gates without strawman workloads.

### Prototype boundary

Each evaluated candidate must implement only enough to test MD-014 properties.

The prototype should support a bounded subset equivalent to:

* create/open a session;
* persist an imported transcript revision;
* append representative `ReviewLedger` events;
* attach an immutable analysis result;
* perform an active-analysis transition;
* reject a stale command;
* close and reopen;
* detect an unsupported newer format;
* duplicate a session semantically;
* classify canonical versus derived loss;
* run bounded cleanup of disposable artifacts.

The prototype must not be mistaken for production architecture.

Avoid implementing unrelated UI, detector, knowledge-extraction, media, or export features.

Prototype adapters may share a common semantic test surface without prescribing production adapter structure.

### Common semantic fixture

Define one shared, deterministic, versioned test fixture used across all evaluated candidates.

The fixture must contain representative canonical state:

* one or more `TranscriptRevisionId` values;
* detector-raised and `HumanRaised` `ReviewCase` values;
* `ReviewLedger` events including approval, `ManualReplacement`, withdrawal, and supersession where applicable;
* at least two immutable analysis results;
* one active-analysis transition;
* one knowledge-informed analysis reference;
* one conflict or ambiguous lineage case;
* derived indexes or caches;
* temporary failed or cancelled job artifacts;
* retained and unreferenced historical artifacts.

The fixture must be deterministic and versioned.

Do not use sensitive production transcript data.

Fixture scale variants may include at least representative small, medium, and stress sizes for startup and replay measurements.

### Correctness oracle

Define one mechanism-independent semantic oracle used across all candidates.

The oracle may operate over an exported normalized semantic representation. MD-015 does not define the production export contract.

The oracle must verify at least:

* canonical identities preserved;
* event ordering preserved;
* no missing committed event;
* no uncommitted event exposed as committed;
* source revision identity preserved;
* `ReviewLedger` authority preserved;
* analysis-result identities preserved;
* active-analysis selection preserved;
* referenced historical artifacts remain reachable;
* derived artifacts may be removed and rebuilt without changing canonical truth;
* no automatic decision migration;
* no fabricated recovery history.

Oracle results must be recorded separately from raw measurements.

### Crash-consistency experiments

Require deterministic fault injection around authoritative transitions.

Test interruption at bounded points conceptually equivalent to:

* before persistence begins;
* after partial internal write;
* after canonical payload write but before commit boundary;
* during commit boundary;
* immediately after durable acknowledgement;
* during close;
* during migration;
* during compaction;
* during duplication;
* during destructive cleanup preparation and execution.

After each interruption, classify the session as:

```text
last committed state
safe automatic recovery
manual review required
read-only salvage
unrecoverable
```

Verify no partial authoritative transition becomes silently valid.

Do not prescribe how fault injection is implemented.

### Durable acknowledgement test

For every authoritative command in the prototype scope:

* record when the application would report success;
* force interruption immediately before and after that point;
* reopen and evaluate the semantic oracle;
* confirm reported success always corresponds to recoverable durable state;
* confirm failed durability never appears as successful after restart.

Candidates that acknowledge before durability are disqualified.

### Single-writer experiments

Require tests for:

* two concurrent writer attempts;
* writer plus reader;
* abandoned writer ownership;
* process crash while owning the session;
* stale ownership metadata;
* force-takeover attempt;
* simultaneous takeover attempts;
* read-only opening during active writer ownership.

Verify:

* at most one authoritative writer;
* no last-write-wins corruption;
* takeover does not rely only on PID absence;
* takeover performs integrity and recovery validation;
* read-only access does not mutate canonical state.

Do not select the lock implementation.

### Stale-write experiments

Use explicit competing commands with stale expected-state preconditions.

Test at least:

* stale `ReviewLedger` command;
* stale active-analysis transition;
* stale analysis attachment;
* stale retention or cleanup plan;
* unrelated scoped state change where the command should remain valid;
* conflicting state change where the command must fail.

Verify appropriately scoped concurrency semantics rather than invalidating every command after any session change.

### Format-version experiments

Require:

* current supported format;
* compatible older format;
* malformed version field;
* unknown newer version;
* interrupted migration;
* migration with missing required canonical record;
* migration with derived-state corruption only.

Verify:

* unknown newer does not open writable;
* failed migration preserves original input;
* migration never silently drops canonical history;
* derived damage does not become canonical loss;
* product version is not treated as session format version.

### Integrity and corruption experiments

Inject representative damage into:

* canonical event ordering;
* duplicate identities;
* missing referenced source revision;
* invalid ledger reference;
* broken analysis linkage;
* invalid active-analysis reference;
* damaged derived index;
* truncated temporary artifact;
* malformed retention metadata;
* incompatible governance or format metadata.

Verify classification distinguishes:

* rebuildable derived corruption;
* canonical corruption;
* unsupported interpretation;
* salvageable partial history;
* unrecoverable authority loss.

Do not require a particular checksum or signature algorithm.

### Malformed-input and security experiments

Treat session storage as hostile input.

Include:

* oversized declared counts;
* deeply nested structures;
* cyclic or impossible references;
* path traversal attempts;
* external path references;
* decompression or expansion bomb where relevant;
* invalid string or binary lengths;
* duplicate key or identity ambiguity;
* extreme event history;
* malformed migration metadata.

Measure whether parsing and validation remain bounded.

No candidate passes if malformed input can cause uncontrolled memory growth, unbounded recursion, silent authority fabrication, or unintended external-file access.

### Startup and replay measurements

Measure at multiple fixture scales.

Include at least representative small, medium, and stress fixtures.

Record:

* open time;
* canonical validation time;
* derived rebuild time;
* peak memory;
* bytes read;
* number of replayed or validated records where measurable;
* cancellation responsiveness;
* error behavior when configured resource bounds are exceeded.

Do not establish universal performance thresholds without evidence.

Thresholds should be tied to explicit v0.2 workflow targets or comparative evidence.

### Write-path measurements

Measure representative operations:

* append correction event;
* attach analysis result;
* active-analysis transition;
* session duplication;
* close/reopen;
* derived-index rebuild;
* bounded cleanup planning.

Record latency distributions, not only averages.

At minimum report:

```text
count
median
p95
maximum
failure count
```

Do not claim benchmark precision beyond the test environment.

### Storage measurements

Record:

* base session size;
* size growth per representative event;
* analysis-result storage cost;
* derived-index overhead;
* temporary artifact overhead;
* duplication cost;
* compaction result;
* retained-history cost.

Distinguish logical data size from physical storage amplification where possible.

### Compaction experiments

Where compaction is supported by a candidate, verify:

* event identities unchanged;
* event order unchanged;
* `ReviewLedger` distinctions preserved;
* knowledge versions and snapshot identities preserved;
* analysis identities preserved;
* retained history remains interpretable;
* interrupted compaction does not expose a mixed canonical state;
* pre- and post-compaction semantic-oracle results match.

A candidate is disqualified if compaction preserves only current state while losing required history.

Candidates that do not support compaction must document that limitation and still pass retention and history-preservation gates through other tested behavior.

### Retention and GC experiments

Use a deterministic reachability graph containing:

* permanent canonical roots;
* referenced historical artifacts;
* user-pinned artifacts;
* rebuildable derived artifacts;
* temporary artifacts;
* unreferenced garbage candidates;
* incomplete or corrupted references.

Verify:

* protected roots are retained;
* incomplete reachability prevents destructive GC;
* low-risk derived or temporary cleanup remains possible;
* no retained event becomes uninterpretable;
* cleanup plan is inspectable;
* interrupted cleanup does not silently corrupt canonical state.

### Session duplication experiments

Verify:

* new session identity;
* lineage to source session;
* independent writer ownership;
* expected copied canonical history;
* no mutation of source session;
* no mutable shared ownership state;
* reopen and semantic-oracle validation of both sessions.

Raw filesystem copying may be tested as a negative or mechanism-specific control but must not automatically count as semantic duplication.

### Backup and restore boundary

The spike may test copy or backup behavior needed to evaluate crash and duplication safety.

It must not define or promise an automatic backup product feature.

Where backup-like copies are tested, verify:

* point-in-time semantic consistency;
* source session remains unchanged;
* restore does not reuse writer ownership;
* restored session identity behavior is documented;
* incomplete copies fail closed.

Final backup policy remains outside MD-015.

### Cross-platform experiments

Run relevant tests on:

```text
Windows
macOS
```

Record:

* filesystem semantics affecting ownership and replacement;
* crash or interruption behavior;
* path handling;
* case sensitivity where relevant;
* file-sharing behavior;
* unsupported assumptions;
* performance differences significant to correctness or v0.2 usability.

Linux-only evidence is insufficient for v0.2 desktop mechanism selection unless product scope changes.

Do not require identical performance across platforms.

### Reproducibility

Every evidence run must record:

* repository commit;
* prototype candidate and version;
* fixture version;
* test harness version;
* operating system and version;
* filesystem type where known;
* hardware summary;
* runtime or compiler versions;
* configuration;
* fault-injection seed or scenario identity;
* start and end timestamps;
* raw results;
* normalized semantic-oracle result;
* known limitations.

Evidence must be reproducible from tracked instructions and non-sensitive fixtures.

Do not commit large generated artifacts unless repository policy explicitly permits them.

### Evidence artifact structure

Define a conceptual evidence package containing:

```text
manifest
candidate description
test matrix
environment metadata
raw measurements
semantic-oracle results
fault-injection results
failures and limitations
comparison summary
recommendation
```

Recommendation remains non-authoritative until a later Material Decision.

Avoid selecting a final serialization format here.

### Pass gates

A candidate cannot proceed to mechanism selection unless it demonstrates:

1. no acknowledged-but-lost authoritative transition in tested scenarios;
2. no partial authoritative transition exposed as valid;
3. single-writer enforcement;
4. stale-write rejection;
5. unknown-newer-format writable rejection;
6. canonical versus derived corruption distinction;
7. bounded malformed-input handling;
8. preserved canonical identities and ordering;
9. safe interrupted compaction and cleanup behavior where supported;
10. preserved referenced history;
11. valid semantic duplication behavior;
12. adequate startup, memory, and write behavior for declared v0.2 targets;
13. Windows and macOS viability;
14. reproducible evidence.

Passing means eligible for comparison, not automatically selected.

### Disqualification conditions

Explicitly disqualify a candidate that:

* acknowledges success before durable safety;
* exposes partial canonical transitions;
* permits conflicting writers without safe rejection;
* silently last-write-wins stale commands;
* opens unknown newer formats writable;
* rebuilds missing canonical history from caches;
* cannot distinguish canonical from derived corruption;
* silently drops canonical history during migration or compaction;
* allows malformed input to fabricate authority;
* requires unbounded startup or memory for expected v0.2 scale;
* cannot preserve logical identities during deduplication;
* relies on undocumented platform behavior;
* cannot produce reproducible evidence.

A candidate may be redesigned and re-evaluated. Disqualification is evidence-run-specific unless the limitation is fundamental.

### Comparative interpretation

The later mechanism decision must compare eligible candidates across:

* semantic correctness;
* failure behavior;
* implementation complexity;
* operational inspectability;
* portability;
* performance;
* storage amplification;
* migration complexity;
* testability;
* dependency and maintenance risk.

Performance must not override failed correctness gates.

A candidate with lower throughput may be preferred if it provides materially stronger correctness and simpler recovery within acceptable v0.2 bounds.

Do not assign final weights in MD-015 unless justified by accepted product requirements.

### Negative results

Require preservation of material negative results.

Do not discard:

* failed fault-injection cases;
* unsupported platform behavior;
* malformed-input failures;
* unexplained nondeterminism;
* benchmark outliers caused by mechanism behavior;
* recovery classifications that required manual intervention.

Negative evidence is part of the later decision record.

### Production authorization boundary

* MD-015 acceptance authorizes running the bounded spike;
* it does not authorize production persistence selection;
* it does not authorize migrating real sessions;
* it does not establish v0.2;
* it does not accept any candidate mechanism;
* a later Material Decision must cite the completed evidence package and select or reject mechanisms explicitly.

## Invariants

1. Every candidate is evaluated against the same semantic oracle.
2. Authoritative correctness gates precede performance comparison.
3. Acknowledged success must survive tested interruption.
4. Partial canonical state is never counted as valid recovery.
5. Single-writer and stale-write behavior are tested, not assumed.
6. Unknown newer formats are tested for fail-closed writable behavior.
7. Canonical and derived corruption are tested separately.
8. Malformed-input behavior is resource-bounded.
9. Compaction and GC are validated against preserved semantic history.
10. Candidate examples are not selected mechanisms.
11. Spike prototypes are not production authorization.
12. Evidence includes negative results and limitations.
13. Results are reproducible from versioned fixtures and harnesses.
14. Windows and macOS evidence is required for desktop mechanism selection.
15. Performance cannot compensate for failed durability or integrity semantics.
16. A later Material Decision is required for mechanism selection.

## Banned designs

The following designs are rejected:

1. Selecting SQLite solely because it is familiar.
2. Selecting JSONL solely because it is inspectable.
3. Selecting a bundle solely because it is easy to copy.
4. Benchmarking candidates with different semantic workloads.
5. Measuring only happy-path throughput.
6. Testing crash behavior only through graceful shutdown.
7. Accepting averages without tail latency or failure counts.
8. Omitting malformed-input tests.
9. Using sensitive production data as fixtures.
10. Treating prototype code as production-ready.
11. Hiding negative results.
12. Selecting a backend directly in MD-015.
13. Allowing performance wins to override correctness failures.
14. Using Linux-only evidence for a Windows and macOS desktop product.
15. Declaring a winner before reproducibility review.
16. Waiving an MD-014 requirement because a candidate passes unrelated benchmarks.

## Explicitly deferred

The following remain deferred and are not owned by this decision:

* production persistence backend selection;
* final schema, file layout, and adapter contracts;
* production migration rollout;
* installer packaging and updater behavior;
* automatic backup service;
* cloud synchronization and collaboration;
* encryption and key management selection;
* telemetry;
* final retention durations;
* final recovery UI;
* production export contract;
* mechanism-selection Material Decision content and identifier;
* implementation language and testing framework choice.

## Consequences

Acceptance authorizes the bounded spike only. It does not authorize production persistence implementation or backend selection.

* a bounded prototype and harness must be created;
* candidate adapters must share a common semantic test surface;
* deterministic fixtures and a semantic oracle are required;
* fault injection becomes part of mechanism evaluation;
* evidence artifacts become review inputs for a later selection decision;
* negative evidence must be retained;
* Windows and macOS testing becomes mandatory;
* persistence implementation remains unselected;
* a later mechanism-selection Material Decision is required;
* data-contract changes remain deferred until mechanism and domain impacts are reviewed.

## Compatibility with existing decisions

### MD-014

* MD-014 requirements remain authoritative;
* MD-015 operationalizes evidence collection only;
* no MD-014 requirement may be waived through benchmark success;
* candidate-specific limitations must be surfaced in evidence.

### MD-013

* analysis attachment semantics remain unchanged;
* the fixture may exercise attachment durability but does not redefine `AnalysisJob` or `AnalysisSnapshot`;
* partial analysis results remain non-authoritative.

### MD-011 and MD-012

* append-only correction and knowledge provenance are semantic-oracle requirements;
* the spike does not redefine correction or knowledge governance;
* withdrawal, supersession, and revocation history remains preserved.

### MD-001 through MD-004

* established identity and materialization semantics remain unchanged;
* candidates must preserve their historical interpretation in oracle results.

## Relationship to prior decisions

* MD-001 through MD-004 remain authoritative for their established v0.1 semantics.
* Proposed MD-011, MD-012, and MD-013 remain authoritative for their respective domains.
* Proposed MD-014 remains authoritative for durability, recovery, and retention requirements.
* MD-015 defines how candidate mechanisms must be tested before a later decision may select one.

## Related decisions

MD-015 owns the pre-selection evidence protocol for v0.2 session persistence.

Proposed MD-014 remains authoritative for durability, integrity, recovery, lifecycle, and retention requirements.

MD-015 does not select a persistence backend and does not weaken MD-014.

A later mechanism-selection Material Decision is required to select or reject mechanisms based on completed evidence.
