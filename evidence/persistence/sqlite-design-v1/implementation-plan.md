# SQLite Package 2B — Implementation Plan

Produced by work package: `persistence-package-2a-sqlite-executable-design-contract` (Package 2A design contract)

Target implementation work package: `persistence-package-2b-sqlite-implementation` (authorized by owner; **not executed in 2A**)

Design authority: `evidence/persistence/sqlite-design-v1/`

Baseline after 2A: commit containing this design contract only.

Readiness: `mechanism_comparison_readiness = not_ready`, `selection_status = none` until 2C/2D evidence passes.

---

## Scope boundary

| Package | Owns |
|---|---|
| **2B** | `embedded_relational.rs` mechanism correction; writer ownership; backup duplication; relational schema; integrity_check; fault hook placement; mechanism-specific unit/integration tests |
| **2C** | Shared scenario handlers, process crash, multi-process, persisted reopen oracle (Package 4 coordination) |
| **2D** | Windows, filesystem durability, cross-platform evidence |

2B tests **must not** impersonate 2C ProcessCrashRecovery or 2D FilesystemDurability claims.

---

## Ordered implementation tasks

### 1. Relational canonical schema (SQL-AUTH-001)

**Invariants:** SQL-AUTH-001, SQL-AUTH-002

**Files:** `src/persistence_evidence/candidates/embedded_relational.rs`

**Dependencies:** none

**Work:**
- Replace `canonical_state.state_json` blob authority with relational tables covering full `NormalizedSemanticState`: `source_revisions`, `review_cases`, `review_case_raised_events`, `review_ledger_events`, `analysis_results`, `active_analysis_selection`, `knowledge_snapshot_references`, `lineage_conflicts`, `artifacts`, `retention_references`
- Implement load/save via SQL SELECT/INSERT/UPDATE assembling `NormalizedSemanticState`
- Remove monolithic blob as authority carrier
- Do not enable `session.export.json` in 2B

**Acceptance test:** create fixture session; close; new adapter reopen read-only; oracle matches fixture via table load

**Forbidden shortcuts:** keep blob as hidden authority; read external JSON on open

---

### 2. Writer connection lifecycle (SQL-ACK-001)

**Invariants:** SQL-ACK-001

**Files:** `embedded_relational.rs`

**Dependencies:** task 1

**Work:**
- Hold one `Connection` per writable handle for handle lifetime
- Per-command BEGIN/COMMIT on held connection
- Post-commit authoritative read verify before returning Ok

**Acceptance test:** append event returns Ok only if new connection read shows event row

**Forbidden shortcuts:** new Connection per operation without verify; return Ok before COMMIT

---

### 3. Writer ownership with lease and takeover (SQL-WRITER-001, SQL-WRITER-002)

**Invariants:** SQL-WRITER-001, SQL-WRITER-002

**Files:** `embedded_relational.rs`

**Dependencies:** task 2

**Work:**
- Replace `writer_lock` with `writer_ownership` table per recovery-protocol.json
- Implement acquisition, renewal, release, stale_takeover with epoch bump
- Never use PID absence as stale test

**Acceptance test:** simulate expired lease via test API clock advance; stale_takeover succeeds; second concurrent writer fails while lease valid. **This is a lease-API unit test only (InterfaceBehavior); it is not ProcessCrashRecovery or orphan-writer catalog evidence.**

**Forbidden shortcuts:** permanent orphan token; force unlock by PID check only; label lease clock simulation as 2C recovery evidence

---

### 4. Open and recovery classification (SQL-RECOVERY-001, SQL-SCHEMA-001)

**Invariants:** SQL-RECOVERY-001, SQL-SCHEMA-001

**Files:** `embedded_relational.rs`

**Dependencies:** tasks 1, 3

**Work:**
- PRAGMA integrity_check on every open
- Return explicit recovery outcomes via AdapterError.code mapping per authority-contract recovery_classification_surface
- Retain unsupported-newer-format writable rejection
- Fail closed on older format without migration

**Acceptance test:** unknown newer format writable rejected with exact code; integrity failure blocks writable open

**Forbidden shortcuts:** open writable without integrity_check; silent migration

---

### 5. WAL-safe duplication (SQL-DUP-001, SQL-DUP-002)

**Invariants:** SQL-DUP-001, SQL-DUP-002

**Files:** `embedded_relational.rs`

**Dependencies:** tasks 2, 3

**Work:**
- Replace `fs::copy` with `rusqlite::backup::Backup`
- Implement temp backup, integrity_check, identity transaction, independent semantic validation on staged temp db, destination wal_checkpoint(TRUNCATE), close destination Connection, atomic publish of closed files, post-publish reopen verification, failure cleanup
- Validate destination with new adapter reopen

**Acceptance test:** duplicate while source writable handle open; destination distinct session_id; semantic oracle on staged temp db before publish; source reopen oracle unchanged. **Mechanism tests are non-catalog and do not credit readiness; catalog semantic-duplication evidence remains 2C.**

**Forbidden shortcuts:** fs::copy(session.db); publish before integrity_check; publish before independent semantic validation; WAL finalize/close; self-oracle without reopen; treat BT-005/006 as catalog evidence

---

### 6. Derived cache detect and rebuild

**Invariants:** SQL-AUTH-001 (derived non-authoritative)

**Files:** `embedded_relational.rs`

**Dependencies:** task 1

**Work:**
- On open, recompute `derive_queue_index_v1(canonical_tables)` and compare `schema_version` + `content_hash` to stored derived_cache row
- Rebuild derived row when mismatch detected
- Canonical reads never depend on derived_cache

**Acceptance test:** corrupt derived_cache; open succeeds; canonical oracle valid; derived row rebuilt

**Forbidden shortcuts:** ignore derived corruption; treat canonical read as rebuild proof without derived observation

---

### 7. Canonical corruption error classification

**Invariants:** SQL-RECOVERY-001

**Files:** `embedded_relational.rs`

**Dependencies:** task 1

**Work:**
- Map deserialization/FK/integrity failures to specific `canonical-corruption` or `sqlite-load-state` codes
- Fail closed writable open

**Acceptance test:** malformed canonical row injection yields specific error code, not generic Err

**Forbidden shortcuts:** accept any Err; compare(fixture, fixture)

---

### 8. Fault hook extension for 2C (SQL-FAULT-001)

**Invariants:** SQL-FAULT-001

**Files:** `src/persistence_evidence/candidates/fault.rs`, `embedded_relational.rs`

**Dependencies:** tasks 2, 5

**Work:**
- Extend `FaultPoint` enum with design fault_ids and before/after authority metadata
- Place hooks at: before_sqlite_commit, after_sqlite_commit_before_ack, during_backup_copy, during_checkpoint
- 2B tests only logical injection paths documented in recovery-protocol.json

**Acceptance test:** before_sqlite_commit hook prevents COMMIT and leaves persisted state unchanged on reopen

**Forbidden shortcuts:** all faults before any mutation while naming recovery tests

---

### 9. Mechanism-specific tests (2B only)

**Invariants:** SQL-EVIDENCE-001

**Files:** `tests/persistence_candidates.rs` or new `tests/persistence_sqlite_mechanism.rs`

**Dependencies:** tasks 1–8

**Work:**
- backup round-trip with source handle open
- stale takeover after simulated orphan lease expiry
- unknown newer writable reject
- ack fails closed on commit error
- post-commit read verify

**Acceptance test:** `cargo test --features persistence-spike` passes mechanism tests

**Forbidden shortcuts:** error-return injection labeled ProcessCrashRecovery; compare(expected via apply_command, actual) for high-risk claims

---

## 2B acceptance test plan

| Test ID | Validates | Evidence strength cap |
|---|---|---|
| BT-001 | Relational reload after reopen | InterfaceBehavior |
| BT-002 | Command ack after COMMIT + verify | LogicalStateTransition |
| BT-003 | Second writer rejected | InterfaceBehavior |
| BT-004 | Stale takeover after lease expiry (lease-API unit test only) | InterfaceBehavior |
| BT-005 | Backup duplication with distinct identity (non-catalog mechanism test) | InterfaceBehavior |
| BT-006 | Source unchanged after duplication reopen (non-catalog mechanism test) | InterfaceBehavior |
| BT-007 | Unknown newer writable reject | InterfaceBehavior |
| BT-008 | Derived rebuild after corruption | InterfaceBehavior / LogicalStateTransition subclaims |
| BT-009 | Pre-commit fault leaves state unchanged | InterfaceBehavior |

---

## 2C evidence (not 2B)

- Process kill during writable hold → orphan recovery
- Post-commit before ack crash → mutation survives reopen
- Multi-process simultaneous writer/takeover
- Shared scenario_runner persisted reopen for append/attach/baseline
- Independent semantic oracle not sharing adapter apply path

---

## 2D evidence (not 2B)

- Windows full scenario matrix
- FilesystemDurability fault injection with explicit config
- Cross-platform ownership and locking semantics
- HardwarePowerLoss only if separately authorized

---

## File scope (2B)

**In scope:**
- `Cargo.toml` (enable `rusqlite` `backup` feature for persistence-spike)
- `src/persistence_evidence/candidates/embedded_relational.rs`
- `src/persistence_evidence/candidates/fault.rs` (hook enum extension only)
- `tests/persistence_candidates.rs` or `tests/persistence_sqlite_mechanism.rs`

**Out of scope:**
- `scenario_runner.rs` official scenario semantics (2C/Package 4)
- `append_bundle.rs`
- `scenario-claim-contracts.json`
- evidence regeneration

---

## Completion criteria for 2B stop gate

- All SQL-* invariants implemented and covered by BT-* tests
- Storage systems review PASS on implementation
- No readiness aggregation change until 2C evidence executed
- Owner gate before 2C execution
