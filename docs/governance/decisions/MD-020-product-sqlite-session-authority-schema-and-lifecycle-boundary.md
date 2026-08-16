# MD-020: Product SQLite Session Authority Schema and Lifecycle Boundary

Status: accepted

Date: 2026-08-16

Accepted: 2026-08-16 per explicit owner authorization

Decision authority: Ezra

Classification: Gate 4 product-session SQLite authority schema, scoped command contract, and lifecycle boundary

## Context

MD-014 records durable session authority, recovery, lifecycle, and canonical-versus-derived separation requirements.

MD-019 selects SQLite with `01C-SQLITE-3` scoped-authority semantics as the production persistence mechanism. MD-019 explicitly does **not** freeze production schema, migration design, spike adapter APIs, or evidence implementation details.

Gate 4 Product SQLite Integration Preflight (`AUTHORIZE_GATE4_PRODUCT_SQLITE_INTEGRATION_PREFLIGHT_01`, base `bcddc11464e6b8b63665487e1bcafa6554daf8ab`) recommended a `DurableApplicationSession` coordinator wrapping `ApplicationReviewSession` and identified that a narrow Material Decision is required before implementation.

Owner adjudication: **PREFLIGHT PASS_WITH_REQUIRED_MD020_CLARIFICATIONS**.

This Material Decision records the production SQLite session authority boundary only. It does not authorize implementation by itself. A separately owner-authorized bounded work package is required after acceptance.

## Relationship to prior decisions

```text
MD-014  → durability / recovery requirements
MD-019  → SQLite mechanism + 01C-SQLITE-3 scoped semantics
MD-020  → product schema families, canonical/derived boundary, scoped commands, lifecycle
```

MD-020 must not weaken MD-014 or MD-019.

## Decision

### Format version and migration posture

```yaml
product_session_format_version: 1

migration:
  supported_versions: [1]
  unknown_newer: fail_closed_writable
  older_unsupported: explicit_unsupported_error
  automatic_migration: none
```

A future format version requires a new or superseding Material Decision. MD-020 does not authorize v2 design.

### Storage mechanism

```yaml
storage:
  mechanism: SQLite
  source_transcript:
    posture: embedded_canonical_parsed_payload
    external_source_path: non_authoritative_metadata_only
```

The authoritative transcript is the canonical parsed payload stored in the session store. An external file path, if retained, is display or import provenance only and must not be treated as session authority.

### Session identity

```yaml
session_identity:
  type: opaque_generated_id
  duplication_creates_new_identity: true
```

Session duplication, if supported later, creates a new session identity. It does not silently alias authority.

### Canonical authority

The following are **canonical persisted authority**. They survive restart and define what a reopened session must interpret.

```text
session identity and lineage metadata
material-use declaration
declared session authority
transcript / source revision + embedded canonical parsed payload
parsed session terms + stable terms identity
immutable analysis snapshots
immutable ReviewCase snapshots
append-only ReviewLedger events
project_scope.stable_id (set once)
append-only reuse-governance events
immutable reuse-enabled analysis bindings (historical provenance)
```

#### ReviewCase snapshots are canonical authority

**Stored `ReviewCase` snapshots are authority.** `ReviewCase` values are the formal human review unit in the data contract. `ReviewLedger` events reference `ReviewCase` identity. Gate 4 evidence taxonomy classifies `review_cases` as canonical authority.

Therefore:

```text
analysis snapshot        → canonical immutable
ReviewCase snapshot      → canonical immutable (authority)
ReviewLedger events      → canonical append-only
queue / status / projection → derived
```

On reopen, the implementation **may** rerun detectors for **consistency verification** only. **Fresh detector output must never replace persisted immutable `ReviewCase` authority.** A newer detector or algorithm version must not silently become the durable decision target. A mismatch between verification output and stored canonical `ReviewCase` snapshots is a recovery or integrity failure, not authority replacement.

### Durable non-semantic metadata

The following may be persisted but are **not** semantic authority, **not** authority commands, and are excluded from authority fingerprinting and from the three semantic authority scopes' stale-validity checks:

```text
project_scope.display_name
external source / display paths, if retained
```

`project_scope.stable_id` remains canonical and is set exactly once. `display_name` may be durable and mutable, but changing it must **not** be implemented as a `ReuseGovernance` authority transition and must **not** require a fabricated governance event or scoped stale precondition beyond ordinary metadata update rules.

### Derived state

The following are **derived rebuildable** and must not be persisted as authority:

```text
effective ReviewCase statuses
review queue
progress and summaries
reviewed projection
effective reusable state
reusable snapshot identity (current fold)
current reuse_enabled_run (runtime state)
export bundles
```

`reuse_enabled_run` is derived runtime state. Historical reuse-enabled analysis bindings are canonical provenance records, not a claim that the current session still has an active reusable run.

### Physical ordering versus semantic concurrency

```text
committed_generation = physical transactional ordering and durable acknowledgement identity only
```

Semantic stale-write detection uses **scoped preconditions** per command scope, not a global generation counter exposed to the application or UI.

The product must **not** use `committed_generation` as the primary semantic stale guard. Application-visible stale refusal uses scoped authority preconditions:

```text
StaleAuthorityPrecondition {
  scope: ReviewLedger | ReuseGovernance | ActiveAnalysis
}
```

A legacy `StaleGeneration` controller variant, if present, must not be revived as the semantic concurrency contract. Physical generation may remain an internal diagnostic only.

## Scoped command contract

MD-020 adopts exactly three durable command scopes aligned with `01C-SQLITE-3`. No fourth scope is introduced.

### ReviewLedger

```text
append review decisions (DecisionRecorded events)
```

Precondition: `review_ledger_head` matches current append-only event count.

### ReuseGovernance

```text
initialize project scope (set stable_id exactly once)
promotion accept / reject
revoke reusable influence
supersede reusable influence
```

**`initialize_project_scope`**

```yaml
scope: ReuseGovernance
preconditions:
  project_scope_stable_id: none
  reuse_governance_head: matches current event count
effect:
  set project_scope.stable_id exactly once
```

This operation does **not** require a fabricated governance event.

**Governance mutations and reuse-enabled runtime state**

When reuse governance changes, canonical historical reuse-enabled analysis bindings are **not deleted**. Bindings remain immutable provenance. The current reusable run becomes inactive when the binding's reusable snapshot identity no longer matches the current derived reusable snapshot fold. `ApplicationReviewSession.reuse_enabled_run` is derived runtime state set to none by projection rules, not by cross-scope canonical deletion.

### ActiveAnalysis

```text
attach immutable analysis snapshot
attach immutable reusable-analysis binding
```

**`run_reuse_enabled_review`**

When the reuse-enabled analysis result becomes the currently effective reuse-enabled analysis, the ActiveAnalysis command must be **one atomic transaction** that together:

```text
append immutable AnalysisSnapshot
+ append immutable reuse-enabled binding
  (analysis snapshot identity ↔ reusable snapshot identity ↔ governance boundary)
+ update active/current analysis selection token
```

Historical bindings are never deleted. Later reuse-governance changes do not rewrite history; they only cause current projection to determine that an older binding is no longer current.

```yaml
scope: ActiveAnalysis
precondition: active_analysis_snapshot_identity matches current selection token
effect: as atomic write-set above
```

Future non-reuse analysis attachment (for example ASR observation attachment in later gates) also uses ActiveAnalysis scope.

## Transactional command commit contract

An authoritative product command succeeds only after durable commit. The required sequence is:

```text
prepare command against current product authority
→ begin SQLite transaction
→ load latest relevant canonical scope
→ validate scoped precondition
→ append or set only the authorized canonical write-set
→ validate resulting authority
→ COMMIT
→ durable acknowledgement
→ refresh in-memory ApplicationReviewSession from persisted canonical authority
```

### Prohibited patterns

```text
✗ mutate ApplicationReviewSession first, then sqlite.save()
✗ persist whole ApplicationReviewSession blob as authority
✗ treat committed_generation as semantic stale guard for UI or command admission
✗ silently replace canonical ReviewCase snapshots with newly detected cases on reopen
✗ delete canonical reuse-enabled bindings when governance changes
```

### Post-commit memory refresh failure

If the database commit succeeds but post-commit in-memory refresh fails:

```text
durable authority remains committed
session enters RecoveryRequired
rehydrate from SQLite before accepting new commands
```

The implementation must not roll back semantic truth or pretend the commit never happened.

## Integration architecture boundary

MD-020 accepts the preflight-recommended integration shape:

```text
DurableApplicationSession (coordinator)
  → ProductSessionStore (new product module; not persistence_evidence)
  → ApplicationReviewSession (in-memory product authority, refreshed after commit)
```

Reads may use the in-memory session. Mutations flow through the coordinator and store using the transactional contract above.

MD-020 does **not** accept:

```text
✗ ApplicationReviewSession owning SQLite directly
✗ importing persistence_evidence spike adapters as production dependencies
✗ productizing evidence fixture / oracle / runner / measurement harness
✗ copying spike table layout verbatim as permanent product schema
```

## Reopen and reconstruction contract

```text
open session
→ load canonical authority from SQLite
→ reconstruct ApplicationReviewSession
→ rebuild derived state
→ verify consistency (including verify_in_memory_replay-class checks where applicable)
→ resume writable or read-only
```

Reopen proof requirements:

- persisted ReviewCase snapshots and ReviewLedger events remain the decision reference target;
- canonical analysis snapshots remain immutable inputs to verification;
- detector rerun is verification only unless owner accepts a future migration decision;
- projection and export materializers must reproduce the same outputs from reopened canonical authority;
- corruption, unsupported format version, or failed reconstruction returns fail-closed errors to the application layer without partial writable authority.

## Canonical table families (semantic, not spike layout)

MD-020 locks **table families**, not the evidence spike's exact DDL:

```text
session_metadata          (identity, format version, lineage)
source_payload            (revision + embedded parsed transcript)
session_terms             (parsed terms + identity)
declarations              (material use, session authority)
analysis_snapshots        (immutable snapshots)
review_cases              (immutable ReviewCase snapshots)
review_ledger_events      (append-only)
project_scope             (stable_id canonical; display_name metadata)
reuse_governance_events   (append-only)
reuse_enabled_bindings    (immutable historical provenance)
command_tokens            (scoped precondition heads)
authority_transitions     (physical generation / ack only)
writer_ownership        (single-writer lease for writable mode)
```

Exact column and index design remains implementation detail within these families, provided the semantic boundary above is preserved.

## Consequences

- Product-session SQLite integration may proceed only after MD-020 acceptance and a bounded implementation work package.
- Package 1 may implement `create` / `open` and durable `ReviewLedger` commands under this contract.
- Evidence namespace (`persistence_evidence`, frozen Gate 4 records, `01B-3` reference candidate) remains separate and unchanged.
- UI and controller layers must surface scoped stale preconditions, not generation mismatch, as the primary concurrency refusal.

## Rejected designs

- Whole-session blob persistence as authority.
- Memory-first mutation with deferred save.
- Global generation as semantic stale guard.
- Reopen that replaces canonical ReviewCases with newly detected cases.
- Cross-scope deletion of reuse-enabled bindings on governance mutation.
- A fourth durable command scope.
- Automatic migration in format v1.
- Product dependency on `persistence_evidence` spike modules.

## Explicitly deferred

- Exact SQLite DDL, indexes, and on-disk path conventions beyond semantic families.
- Backup, encryption, compaction, GC policy, and cloud sync.
- ASR observation attachment semantics (reserved via ActiveAnalysis for later gates).
- Session duplication UX and copy semantics beyond identity rule.
- Format v2 and migration machinery.
- Desktop packaging and multi-platform session portability beyond stored canonical payload.

## Implementation rule

Acceptance records schema and lifecycle boundary only. Implementation requires owner authorization for **Gate 4 Product-Session SQLite Integration Package 1: create/open + durable ReviewLedger**.
