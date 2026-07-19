# Persistence Spike v1 — Bounded Correction Plan

Work package: `reclassify-existing-persistence-evidence`
Review baseline: `c4887e32867e0c6d610ba128ef60d2fdd527084d`
Status: **plan only — no implementation authorized by this document**

This plan follows specialist storage-systems and evidence-methodology review. Original evidence at `evidence/persistence/spike-v1-macos-6460148/` is preserved unchanged.

---

## A. Claim-only corrections

| Item | Action |
|---|---|
| Eligibility wording | Downgrade `EligibleForComparison` to **host catalog-pass only**; do not read as mechanism comparison or selection readiness |
| Candidate class declarations | Rewrite `candidate-classes.json` to match actual authority models or mark declarations aspirational until rebuilt |
| SQLite class label | Reclassify from "relational transactional authority" to **SQLite-hosted full-state JSON snapshot with logical writer lock** |
| Append class label | Reclassify from "append-authoritative log" to **checkpoint-snapshot directory bundle with metadata journal stub** |
| Material distinctness | Record that current spikes are **not materially distinct** at the authority-model level |
| Scenario names/descriptions | Align `derived-state-corruption`, `interrupted-*`, `semantic-duplication`, `concurrent-writer-attempt` names with demonstrated assertion strength |
| FailureModel labels | Reclassify stale-* scenarios from `ConcurrentAccess` to scoped-precondition where multi-actor workload is absent |
| Error classification recording | Populate `failure_classification` in results when specific codes are asserted; stop accepting `Err(_)` |
| Evidence-strength encoding | Add explicit strength per scenario in harness output; cap at demonstrated level |
| Unsupported states | Retain `interrupted-cleanup` as Unsupported; do not upgrade to pass |

---

## B. Harness and scenario corrections

### Required for all persistence claims

- [ ] **Persisted reopen** after every authoritative command, duplication, compaction, and corruption scenario
- [ ] **Independent expected values** — expected oracle must not share `semantic_ops::apply_command` blind spot with adapters for high-risk claims
- [ ] **Evidence-strength encoding** in scenario results JSON

### Per-scenario

| Scenario | Correction |
|---|---|
| `baseline-create-open-close` | Add close → reopen → read cycle |
| `append-correction-event`, `attach-analysis-result` | Close handle, reopen, verify persisted state |
| `stale-*` | Optional: rename failure model; retain current assertion with corrected classification |
| `concurrent-writer-attempt` | Replace `compare(before, before)`; add crash-held-writer and orphan-lock recovery |
| `unknown-newer-format` | Remove fixture self-oracle; optionally test read-only salvage path |
| `derived-state-corruption` | Assert derived detect/rebuild on open; or downgrade claim to "canonical readable despite derived trash" |
| `canonical-reference-corruption` | Require specific error code; test read-only salvage if applicable |
| `semantic-duplication` | Reopen source and duplicate independently; verify source unchanged; divergent mutation test |
| `interrupted-authoritative-transition` | Add fault points: after partial write, at commit boundary, after ack; classify pre-write abort separately |
| `interrupted-compaction` | Reopen after interrupt; validate state and oracle; fix append seq bug first |
| `interrupted-cleanup` | Implement only when destructive GC capability exists |

### Fault model

- [ ] Add `FaultLayer::Filesystem` injection path
- [ ] Fault points **after meaningful writes**, not only before durability begins
- [ ] Commit-durable-before-response verification per MD-015 durable acknowledgement test

### Scenario claim contract encoding

- [ ] Embed or reference claim contract fields in harness manifest
- [ ] Encode `forbidden_shortcuts` checks in test helpers

---

## C. SQLite candidate corrections

| Area | Correction |
|---|---|
| Backup/duplication | Replace `fs::copy(session.db)` with **SQLite backup API** or **WAL checkpoint + full file set copy** |
| Duplication while writer open | Test and implement safe protocol while source handle is open |
| Writer lock recovery | Add stale-writer detection, integrity reload, and validated takeover — not PID-only |
| Connection/transaction semantics | Document ack boundary as COMMIT + synchronous=FULL; do not claim filesystem durability without evidence |
| Relational vs JSON authority | Either decompose canonical state into relational tables or remove relational authority claim |
| Integrity | Add `PRAGMA integrity_check` or equivalent if claimed |
| WAL companions | Include `-wal`/`-shm` in any file-copy control tests as negative evidence |

---

## D. Append-bundle candidate corrections

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

---

## E. Deferred evidence

The following remain **deferred** until corrections pass specialist review:

1. macOS evidence re-execution
2. Windows evidence execution
3. Comparative performance measurements (declared in `targets.json` but `not_executed`)
4. Mechanism-selection Material Decision

No macOS regeneration, Windows CI run, or benchmark execution is authorized until blocking corrections in sections B–D are implemented and re-reviewed.

---

## F. Package decomposition

Recommended bounded implementation packages (sequential):

| # | Package | Scope | Stop gate |
|---|---|---|---|
| 1 | Scenario contract and claim-classification correction | Harness metadata, scenario descriptions, failure models, evidence-strength encoding, eligibility wording | Specialist evidence review |
| 2 | SQLite duplication and writer-recovery correction | `embedded_relational.rs`, duplication scenario, writer takeover | Storage systems review |
| 3 | Append authoritative replay and recovery-ordering correction | `append_bundle.rs`, log payload, replay, compaction seq fix, fsync | Storage systems review |
| 4 | Fault model and scenario assertion correction | `fault.rs`, `scenario_runner.rs`, reopen assertions, error codes | Evidence methodology review |
| 5 | macOS evidence re-execution | Regenerate `spike-v1-macos-*` at corrected harness commit | Owner gate before publish |
| 6 | Windows evidence execution | CI workflow run, record Windows-specific semantics | Owner gate |
| 7 | Comparative measurement execution | Multi-sample metrics per `targets.json` | Owner gate |
| 8 | Mechanism-selection readiness review | Compare eligible candidates; prepare selection MD draft | Owner Material Decision gate |

Each package must declare its own work-package fields, reviewer roles, and stop gate. This correction plan does **not** authorize any package automatically.

---

## Blocking summary

| Gate | Blocking corrections |
|---|---|
| Before macOS re-execution | F-001–F-011, F-013–F-015 (mechanism defects + scenario assertion fixes) |
| Before Windows execution | All macOS-blocking items plus cross-platform harness parity review |
| Before performance comparison | Correctness gates pass; comparative measurement harness implemented |
| Before mechanism selection | All above plus Windows evidence; explicit eligibility reclassification; owner Material Decision |

---

## Related artifacts

- `findings.json` — consolidated finding matrix
- `scenario-claim-contracts.json` — per-scenario claim contracts
- `reclassification.json` — formal downgrade record
