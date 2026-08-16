# MD-019: SQLite Product-Session Persistence Mechanism Selection

Status: accepted

Date: 2026-08-16

Accepted: 2026-08-16 per explicit owner authorization

Decision authority: Ezra

Classification: Gate 4 product-session durability mechanism selection

## Context

MD-014 records the durability, recovery, lifecycle, and retention requirements that any v0.2 session persistence mechanism must satisfy.

MD-015 records the bounded comparative evidence protocol and pass/fail gates required before mechanism selection.

Gate 4 comparative evidence is complete. The frozen comparison baseline `gate4-01c-91e258c` and the final dual-platform FCR-03 record `gate4-01c-86f5b8f-dual-scoped-fcr03` at `FINAL_FROZEN_EVIDENCE_SHA` `86f5b8f2856155bff9a5216593c1659fef707add` show correctness parity between the Append candidate `01B-3` and the SQLite candidate `01C-SQLITE-3`: zero required failures per candidate per platform, selection blockers cleared, and cross-platform semantic parity verified.

FINAL_GATE_GOVERNANCE_CHECK passed at `a881e2ac52497d412d225948d832e2841057fc38`. This decision is not a correctness verdict that SQLite is more correct than Append. Both candidates met the required evidence bar. The selection is a production-engineering decision.

## Decision

VoxProof selects **SQLite** as the product-session durability / persistence production mechanism, with **`01C-SQLITE-3` scoped-authority semantics** as the selection basis.

**`01B-3` Append is not selected for production integration.** It remains a verified rejected alternative and regression/reference candidate.

```yaml
status: accepted
decision_authority: Ezra
accepted_on: 2026-08-16

selected:
  mechanism: SQLite
  candidate_basis: 01C-SQLITE-3

rejected_for_product_integration:
  - 01B-3 Append

evidence_basis:
  frozen_comparison: gate4-01c-91e258c
  final_dual_platform: gate4-01c-86f5b8f-dual-scoped-fcr03
  final_frozen_evidence_sha: 86f5b8f2856155bff9a5216593c1659fef707add

selection_status: selected_sqlite
```

## Accepted durable boundary

This Material Decision accepts the following durable semantics for product-session persistence:

```text
SQLite physical authoritative persistence

+ transactional durable acknowledgement
+ scoped optimistic-concurrency preconditions
+ apply command onto latest canonical authority
+ append-only logical ReviewLedger / reuse-governance history
+ canonical vs derived state separation
+ fail-closed validation/recovery
+ explicit session format/schema versioning
```

`01C-SQLITE-3` limits physical `committed_generation` to transactional ordering. Semantic validity is determined by command write-set scoped preconditions, not by generation alone.

## Selection rationale

Both candidates reached correctness parity in the final evidence package. SQLite is selected primarily for production engineering:

```text
mature transactional storage/recovery
+ natural structured inspection/query
+ schema evolution path
+ less bespoke storage-engine responsibility
+ better fit for growing durable ReviewLedger /
  analysis / reusable-governance state
```

The Append candidate `01B-3` writes a full `CurrentContractState` snapshot on each transition and owns bespoke manifest, replay, tail recovery, and checkpoint repair machinery. That burden is rejected for production integration even though the candidate passed the comparative evidence bar.

## Explicitly not accepted or frozen

This decision does **not** accept or freeze:

```text
✗ spike exact SQLite schema
✗ sqlite_authoritative.rs as direct production architecture
✗ evidence adapter API
✗ evidence fixture / oracle / runner
✗ test-only lease duration
✗ exact table/index layout
✗ final migration design
✗ backup policy
✗ encryption
✗ cloud sync
✗ collaboration
✗ compaction implementation
✗ GC UI/policy details
```

```text
Selecting SQLite does not mean productizing the evidence spike unchanged.
```

This decision locks the durable semantic boundary. It does not lock spike implementation accidents as architecture.

## Relationship to prior decisions

```text
MD-014 defines durability/recovery requirements.
MD-015 defines the pre-selection evidence protocol.
MD-019 selects SQLite as the production mechanism basis.
```

MD-019 does not weaken MD-014 or MD-015. A separately authorized bounded work package is required before product-session SQLite integration into `ApplicationReviewSession`.

## Consequences

- Product-session persistence integration must implement the accepted `01C-SQLITE-3` scoped-authority semantics, not the evidence spike verbatim.
- The Append candidate `01B-3` remains in the repository as a verified rejected alternative and regression/reference candidate. It must not be deleted as part of this decision.
- Frozen Gate 4 evidence records remain immutable historical inputs to this decision.
- Backend selection for other concerns remains subject to later independent Material Decisions where required.

## Rejected for production integration

- `01B-3` Append as the product-session persistence mechanism.

Reconsidering the mechanism selection requires a new or superseding Material Decision.

## Explicitly deferred

- Production SQLite schema and migration design.
- Integration of selected semantics into `ApplicationReviewSession`.
- Backup, encryption, compaction, GC policy, and operational tooling.
- Cross-platform packaging implications beyond the accepted evidence scope.
- Any claim of FilesystemDurability, HardwarePowerLoss, or CrossPlatform evidence strength beyond what Gate 4 evidence actually demonstrated.

## Implementation rule

This acceptance records the mechanism selection only. Implementation requires a separately owner-authorized bounded work package: **Gate 4 Product-Session SQLite Integration**.
