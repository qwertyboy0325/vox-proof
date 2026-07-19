# Persistence Spike v1 — Bounded Correction Plan

Work package: `finalize-persistence-scenario-claim-contract-precision`
Review baseline: `48928b30b87c62a8edcac4ebab402cfea39ac279`
Status: **plan only — no implementation authorized by this document**

This plan follows specialist storage-systems and evidence-methodology review. Original evidence at `evidence/persistence/spike-v1-macos-6460148/` is preserved unchanged.

---

## Fail-closed readiness invariant

Until all blocking mechanism and scenario corrections have passed the declared specialist reviews and corrected evidence has been re-executed, aggregation must remain fail-closed:

```text
mechanism_comparison_readiness = not_ready
mechanism_selection_readiness = not_ready
selection_status = none
```

Metadata cleanup, schema changes, corrected names, or existing 13/13 catalog Passed results must **not** automatically restore any `Eligible*` state.

Package 1 must preserve this invariant.

---

## Evidence-strength taxonomy rules (automation-safe)

Evidence-strength levels must be assigned from the **semantic claim actually demonstrated**, not from the presence of a particular test operation.

The following do **not** automatically upgrade evidence strength:

- close and reopen
- reading from disk
- exact error classification
- persisted before/after comparison
- use of a transaction
- use of fsync
- running on another process or platform

Each level requires claim-specific justification.

`InterfaceBehavior`, `LogicalStateTransition`, `ProcessCrashRecovery`, `FilesystemDurability`, `HardwarePowerLoss`, and `CrossPlatform` are **not** a universal linear maturity ladder for every scenario. A scenario may be fully correct at `InterfaceBehavior` without needing to target a higher level.

### No-transition invariant rule

A scenario that verifies **no mutation occurred** is not automatically a `LogicalStateTransition`.

Rejection and corruption scenarios must record:

- the rejected operation
- the exact error or policy result
- the independent persisted observation
- the no-transition invariant being tested

Evidence strength must then be assigned based on the specific semantic claim demonstrated.

---

## A. Claim-only corrections

| Item | Action |
|---|---|
| Eligibility wording | Downgrade `EligibleForComparison` to **host catalog-pass only**; do not read as mechanism comparison or selection readiness |
| Candidate class declarations | Rewrite `candidate-classes.json` to match actual authority models or mark declarations aspirational until rebuilt |
| SQLite class label | Reclassify from "relational transactional authority" to **SQLite-hosted full-state JSON snapshot with logical writer lock** |
| Append class label | Reclassify from "append-authoritative log" to **checkpoint-snapshot directory bundle with metadata journal stub** |
| Material distinctness | Record declared authority-model distinctness as **not demonstrated**; do not claim total mechanism identity |
| Scenario names/descriptions | Align `derived-state-corruption`, `interrupted-*`, `semantic-duplication`, `concurrent-writer-attempt` names with demonstrated assertion strength |
| FailureModel labels | Reclassify stale-* scenarios from `ConcurrentAccess` to scoped-precondition where multi-actor workload is absent |
| Error classification recording | Populate `failure_classification` in results when specific codes are asserted; stop accepting `Err(_)` |
| Evidence-strength encoding | Add explicit strength per scenario in harness output; cap at demonstrated level |
| Unsupported states | Retain `interrupted-cleanup` as Unsupported; deferred until capability exists |

### Material distinctness (precise wording)

The originally declared authority-model distinction — relational canonical entity authority versus append-authoritative replay — is **not demonstrated** by the current implementations.

Both implementations currently persist full normalized-state JSON snapshots as their effective canonical representation.

They still differ in physical container, transaction engine, journaling, and file-operation behavior; this review does **not** conclude that they are identical in every mechanism-level property.

```text
declared_authority_model_distinctness: not_demonstrated
all_mechanism_level_distinctness: not_assessed_or_not_disproven
```

This narrower wording does not restore comparison readiness.

---

## B. Shared harness corrections (Package 4 scope)

These corrections belong to **Package 4** (shared fault model and official scenario assertion correction), not to mechanism packages.

### Required for all persistence claims

- [ ] **Persisted reopen** after every authoritative command, duplication, compaction, and corruption scenario
- [ ] **Independent expected values** — expected oracle must not share `semantic_ops::apply_command` blind spot with adapters for high-risk claims
- [ ] **Evidence-strength encoding** in scenario results JSON
- [ ] **Separate per-scenario targets from higher-level claims** — use `target_evidence_strength_for_this_scenario` and `related_higher_level_claims_requiring_separate_scenarios`

### Per-scenario (Package 4)

| Scenario | Correction |
|---|---|
| `baseline-create-open-close` | Optional close → reopen → read as separate observation; primary target remains InterfaceBehavior |
| `append-correction-event`, `attach-analysis-result` | Close handle, reopen, verify persisted state |
| `stale-*` | Optional: rename failure model; retain current assertion with corrected classification |
| `concurrent-writer-attempt` | Replace `compare(before, before)`; orphan-lock recovery as **separate scenario** |
| `unknown-newer-format` | Remove fixture self-oracle; exact `unsupported-newer-format`; read-only salvage as **separate claim** |
| `derived-state-corruption` | Assert derived detect/rebuild on open; or downgrade claim |
| `canonical-reference-corruption` | Require specific error code; read-only salvage as separate claim |
| `semantic-duplication` | Reopen source and duplicate independently; verify source unchanged |
| `interrupted-authoritative-transition` | Mid-write/post-ack faults as **separate scenarios**; current handler is logical pre-commit abort |
| `interrupted-compaction` | Post-interrupt reopen as **separate scenario**; current fault is before_compaction_mutation only |
| `interrupted-cleanup` | **Deferred** — not actionable until destructive GC capability exists |

### Fault model (Package 4)

- [ ] Add `FaultLayer::Filesystem` injection path
- [ ] Fault points **after meaningful writes**, not only before durability begins
- [ ] Commit-durable-before-response verification per MD-015 durable acknowledgement test

---

## C. SQLite candidate corrections (Package 2 scope only)

Package 2 owns mechanism implementation only. It does **not** own official shared evidence-scenario semantics.

| Area | Correction |
|---|---|
| Backup/duplication | Replace `fs::copy(session.db)` with **SQLite backup API** or **WAL checkpoint + full file set copy** |
| Duplication while writer open | Test and implement safe protocol while source handle is open |
| Writer lock recovery | Add stale-writer detection, integrity reload, and validated takeover — not PID-only |
| Connection/transaction semantics | Document ack boundary as COMMIT + synchronous=FULL; do not claim filesystem durability without evidence |
| Relational vs JSON authority | Either decompose canonical state into relational tables or remove relational authority claim |
| Integrity | Add `PRAGMA integrity_check` or equivalent if claimed |
| WAL companions | Include `-wal`/`-shm` in mechanism-specific conformance tests as negative evidence |
| Mechanism-specific tests | Unit/conformance tests for backup protocol, writer recovery, authority-model alignment |

---

## D. Append-bundle candidate corrections (Package 3 scope only)

Package 3 owns mechanism implementation only. It does **not** own official shared evidence-scenario semantics.

| Area | Correction |
|---|---|
| Log payload | Store sufficient command payload in append records for deterministic reconstruction |
| Replay | Implement real replay applying commands to checkpoint base state |
| Authority model | Choose append-authoritative or checkpoint-authoritative explicitly; align manifest/generation semantics |
| Checkpoint/log/manifest ordering | Define crash-consistent ordering; add orphan artifact recovery |
| Durable sync | `sync_all` on checkpoint and manifest temp files; parent-directory fsync where supported |
| Generation activation | Crash-safe activation of new checkpoint generation |
| Writer recovery | Implement takeover with integrity validation |
| Compaction | Reset or preserve `committed_seq` consistently with truncated log; verify recovery path after compact |
| Duplication | Copy full consistent bundle snapshot; reopen both sessions independently |
| Mechanism-specific tests | Unit/conformance tests for replay, ordering, compaction seq, fsync paths |

---

## E. Deferred evidence

The following remain **deferred** until corrections pass specialist review:

1. macOS evidence re-execution
2. Windows evidence execution
3. Comparative performance measurements (declared in `targets.json` but `not_executed`)
4. Mechanism-selection Material Decision
5. `interrupted-cleanup` scenario execution (until destructive GC capability exists)

No macOS regeneration, Windows CI run, or benchmark execution is authorized until blocking corrections in Packages 2–4 are implemented and re-reviewed.

---

## F. Package decomposition

Required sequence. Each package must declare its own work-package fields, reviewer roles, and stop gate. **This plan does not authorize any package automatically.**

| # | Package | Scope | Owns official scenario semantics? | Stop gate |
|---|---|---|---|---|
| 1 | Scenario contract metadata, classification, fail-closed aggregation, and result schema only | Harness metadata, scenario descriptions, failure models, evidence-strength encoding, eligibility wording, fail-closed invariant, intended/current claim separation | No — metadata only | Specialist evidence review + owner gate |
| 2 | SQLite mechanism correction only | `embedded_relational.rs`; backup/duplication protocol; writer recovery; SQLite integrity and authority-model alignment; mechanism-specific unit/conformance tests | **No** | Storage systems review |
| 3 | Append-bundle mechanism correction only | `append_bundle.rs`; authority model; log payload/replay; ordering/fsync; compaction sequence; writer recovery; mechanism-specific unit/conformance tests | **No** | Storage systems review |
| 4 | Shared fault model and official scenario assertion correction | `fault.rs`, `scenario_runner.rs`; reopen assertions; exact error classification; cross-candidate shared evidence scenarios | **Yes** | Evidence methodology review; depends on Packages 2–3 where scenario behavior requires mechanism fixes |
| 5 | macOS evidence re-execution | Regenerate `spike-v1-macos-*` at corrected harness commit | — | Owner gate before publish |
| 6 | Windows evidence execution | CI workflow run; record Windows-specific semantics | — | Owner gate |
| 7 | Comparative measurement execution | Multi-sample metrics per `targets.json` | — | Owner gate |
| 8 | Mechanism-selection readiness review | Compare eligible candidates; prepare selection MD draft | — | Owner Material Decision gate |

**Boundary rule:** Packages 2 and 3 must not own official shared evidence-scenario semantics. Package 4 depends on relevant mechanism corrections being complete where scenario behavior requires them.

### Package 1 explicit requirements (not authorized by this plan)

Package 1 remains **metadata, classification, fail-closed aggregation, and result schema only**. It is **not authorized** by this document.

When separately authorized, Package 1 must:

- aggregate readiness using `current_demonstrated_claim` and `current_evidence_strength`, never `intended_claim` or `intended_claims`
- not infer evidence level solely from operation type (close/reopen, fsync, transaction, etc.)
- support multiple independently identified subclaims per scenario contract
- exclude unsupported and deferred claims from readiness aggregation
- require explicit no-transition invariant metadata for rejection and corruption scenarios
- remain fail-closed and **cannot restore** any `Eligible*` state

---

## Blocking summary

| Gate | Blocking corrections |
|---|---|
| Before macOS re-execution | Packages 2–4 complete and specialist-reviewed |
| Before Windows execution | All macOS-blocking items plus cross-platform harness parity review |
| Before performance comparison | Correctness gates pass; comparative measurement harness implemented |
| Before mechanism selection | All above plus Windows evidence; fail-closed invariant satisfied; owner Material Decision |

---

## Related artifacts

- `findings.json` — consolidated finding matrix with attribution discipline
- `scenario-claim-contracts.json` — per-scenario claim contracts (v3)
- `reclassification.json` — formal downgrade record (v2)
