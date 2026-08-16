# MD-021: Cross-Material Governed Project Memory And Reuse Decision Boundary

Status: accepted

Date: 2026-08-16

Accepted: 2026-08-16 per explicit owner authorization

Decision authority: Ezra

Classification: Cross-material Project Memory authority store, v2 session binding, derived reuse proposals, thin decision targets, and Gate 7 evidence attribution

Owner adjudication: **MD-021 DESIGN GATE — PASS WITH REQUIRED CLARIFICATIONS**, then owner accept of the locked clarifications below.

## Context

MD-018 records Gate 3 reusable-influence semantics. Gate 3 implementation remains session-local in process and, after MD-020 / Gate 4, session-local on disk. Same typed `project_scope.stable_id` in two sessions does not share authority. That is not a cross-material flywheel.

MD-020 records product SQLite **session** format v1, three session command scopes, and fail-closed writable recovery. MD-020 explicitly does not authorize session format v2.

This decision records the durable cross-material boundary required before Project Memory implementation. It does not establish v0.3, Gate 7 measurement, desktop UX, or automatic correction.

## Relationship to prior decisions

```text
MD-018  → reusable-influence semantics (exact pair, explicit promote, revocable)
MD-019  → SQLite mechanism for product persistence
MD-020  → product session format v1, canonical/derived, three session command scopes
MD-021  → per-project SQLite Project Memory, session v2 binding, derived proposals + thin targets
```

MD-021 must not weaken MD-014, MD-018, MD-019, or MD-020. v1 sessions remain unchanged. Session-local `ReusableInfluenceSnapshotIdentity` hashing (`reusable-influence-snapshot:sha256-v2`) remains unchanged.

## Locked owner clarifications

These clarifications are part of the accepted decision. They are not implementation choices.

1. Product-generated opaque project identity. `display_name` is the user-nameable field. Existing `ProjectScopeId` is reused; no new identity ontology.
2. New Project Memory snapshot identity / domain. Do not reuse `sha256-v2` semantics or tag under a different algorithm.
3. Thin `ReuseProposalTarget` is persisted only on first human decision, atomically with `ReviewLedger` `DecisionRecorded`. Unacted derived proposals are not canonical storage.
4. Missing or unverifiable Project Memory blocks writable reuse. It must not erase readability of already committed Material-B human authority.
5. Session format v2 is new-session-only. v1 is unchanged. No automatic migration.
6. Promotion writes Project Memory only. It does not require a two-database atomic commit with the source session.
7. Gate 7 `repeated_correction_avoided` must be attributable to Project Memory, not an equivalent canonical-term proposal.

## Decision

### 1. Storage topology — A1

```yaml
project_memory:
  mechanism: SQLite
  cardinality: one_authority_store_per_project
  not: session_database
  not: one_global_application_database
```

Project Memory is a **separate store** from the product session. It is not a fourth MD-020 session command scope.

v1 sessions keep session-local `ReuseGovernance`. v2 flywheel promotion appends to the project store only.

### 2. Project identity

```yaml
project_identity:
  type: product_generated_opaque_id
  rust_type: ProjectScopeId
  mutability: immutable_after_create
  display_name:
    posture: user_nameable_metadata
    not_identity: true
```

v2 projects are created by the product. The operator does not declare the durable `project_id`. `display_name` may change without changing identity and without rewriting snapshot identity.

Do not invent a second project-id type.

### 3. Authoritative write path

```text
Project A session
 Human Manual Replacement X→Y
       ↓
 Explicit Promote
       ↓
ProjectMemory.sqlite
 append-only governed record
 source_session_id + source decision provenance
       ↓
 ProjectMemorySnapshot (new identity domain)
```

Promotion writes project authority only. The source session remains the authority for its own `ReviewLedger`. Project Memory records provenance; it does not become transcript authority.

### 4. Derived proposal and thin target — Option A

```text
Material B v2 session
 bound once to project
       ↓
 derived exact reuse analysis
       ↓
 proposal X→Y
 [NOT authority, NOT persisted yet]
       ↓
 Human Accept / Reject / Edit
       ↓
 atomic:
 thin ReuseProposalTarget persisted
 + ReviewLedger DecisionRecorded
       ↓
 reviewed projection changes
```

Locked persist timing:

```text
derived proposal → user acts → persist thin target + DecisionRecorded atomically
```

Not at analysis bind. Not as an implementation choice between bind and first decision. Unacted proposals stay derived.

Thin targets must carry enough occurrence and replacement data that **already committed** Material-B human decisions can materialize without Project Memory.

Do not persist full `ReusableExactObservedForm` review cases as session `review_cases` authority.

### 5. Project Memory snapshot identity

Current session-local identity:

```text
domain: voxproof-reusable-influence-snapshot-identity-v2
tag:    reusable-influence-snapshot:sha256-v2:
```

`SourceDecisionLocator` remains session-local and has no `source_session_id`. That hasher and tag must not gain new inputs under the same tag.

Project Memory uses a **new** identity type and domain, for example:

```text
type:   ProjectMemorySnapshotIdentity
domain: voxproof-project-memory-snapshot-identity-v1
tag:    project-memory-snapshot:sha256-v1:
```

Required hash inputs include at least:

```text
project format version
project_id
governance event boundary
projection version
per active record:
  source_session_id
  session-local source decision locator fields
  observed / replacement pair
  promotion actor role and label
```

Same tag with a different algorithm is forbidden.

### 6. Session format v2

```yaml
product_session_format_version:
  v1:
    status: unchanged
    supported: true
    auto_migration_to_v2: false
  v2:
    status: new_session_only
    required_for: project_bound_flywheel
    auto_migration_from_v1: false
```

A v1 file must keep opening as v1. Unknown versions remain fail-closed. Binding a session to a project happens at v2 create, once.

v2 session command scopes remain exactly:

```text
ReviewLedger | ReuseGovernance | ActiveAnalysis
```

Project Memory commands are project-store commands, not a fourth session scope.

v1 `ReuseGovernance` continues to mean session-local reuse history. v2 flywheel reuse history lives in Project Memory. v2 sessions must not treat session-local reuse-governance rows as the cross-material authority.

### 7. Missing or unverifiable Project Memory

MD-020 recovery forbids entering **partial writable authority**. It does not require historical canonical output to become unreadable.

```text
Project store available + verified
→ writable / new reuse analysis / new reuse decisions allowed

Project store missing or unverifiable
→ writable reuse operations blocked
→ no new proposal reconstruction
→ existing committed Material-B human decisions may open read-only
→ reviewed projection/export may be reconstructed from persisted thin targets + ReviewLedger
→ provenance shown as unavailable/unverified
```

Writable reuse includes promotion, revocation, supersession, new reuse-enabled analysis, and new reuse decisions. Canonical `ReviewLedger` readability of already committed decisions must survive.

### 8. Gate 7 evidence attribution

`repeated_correction_avoided` must not count cases where canonical session terms already propose the same replacement.

```text
same occurrence
canonical already proposes same Y
+
project memory proposes same Y

→ canonical case remains the decision target
→ reuse contribution is redundant supporting evidence
→ NOT eligible for Gate 7 repeated_correction_avoided
```

Eligible sample:

```text
Without project memory:
B has no equivalent Y proposal

With project memory:
B gets Y proposal

Human accepts
→ avoided one manual replacement
```

Implementation of P1–P4 does not by itself establish Gate 7 or v0.3.

## Implementation sequencing

Owner-accepted sequencing, with no extra architecture-research stage:

```text
MD-021 owner accept
        ↓
P1 Project Memory
        ↓
P2 v2 binding + snapshot
        ↓
P3 proposal → human decision
        ↓
complete core flywheel
        ↓
P4 Desktop product UX
        ↓
P5 real A→B evidence
```

P1 and P2 may share one implementation authorization with two checkpoints. P3 is a separate authorization because decision identity and hydrate risk are highest.

P1/P2 must not implement P3 decision targeting, thin-target persistence, or Gate 7 measurement.

## Explicitly deferred

- fuzzy match, embeddings, LLM ranking, phonetic reusable influence beyond the accepted exact pair
- automatic correction
- automatic promotion
- ASR, OCR, VLM
- HumanRaised cases
- public pack / Experience / Language Pack formats
- v1 session → project auto-migration
- ontology beyond `ProjectScopeId`
- persisting `ReusableExactObservedForm` as `PersistedEvidenceV1` review-case authority
- changing v1 session-local snapshot hashing
- Gate 7 measurement and v0.3 establishment

## Banned or rejected designs

- one global application SQLite for all projects
- storing Project Memory inside the session database as shared cross-session authority
- owner-declared durable project identity for v2 creates
- `reusable-influence-snapshot:sha256-v2` with new hash inputs
- persisting unacted derived proposals as session authority
- persisting full reuse review cases into `review_cases` as the Material-B decision target
- fail-closed **read** of already committed Material-B human authority solely because Project Memory is missing
- two-database atomic promotion commit as a v1 requirement
- automatic v1 → v2 migration
- counting equivalent canonical-term proposals as Gate 7 avoided corrections

## Consequences

- Cross-material reuse has an accepted durable boundary.
- Gate 4 v1 sessions remain valid and unmodified.
- Desktop copy about project memory remains non-authoritative until P3/P4 wire decision targeting.
- Real A→B evidence remains blocked until eligible repeated-error material exists and P3 exists.

## Implementation notes

Table families and Rust type names in a later work package are implementation, not ontology. This decision freezes semantic and durable boundaries only.

Thin-target persist and composition rules are P3. P1 must not mutate session v1 schema. P2 may add v2-only tables on new files and must keep opening existing v1 files.
