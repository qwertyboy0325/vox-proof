# Current-Contract Persistence Evidence v3

Status: evidence contract package (VP-GATE4-EVIDENCE-COMPLETION-01A)

Correction-05 (`VP-GATE4-EVIDENCE-COMPLETION-01A-REMOTE-REVIEW-CORRECTION-05`)
closes remote-review blockers in anchor-validated ReviewCase authority and
production/evidence lifecycle identity cross-checks. Corrected 01A was
owner-accepted by Ezra on 2026-08-02 at
`9be1e99ca45db2dcd6b2b7fbe03137e6cc28e241`. Package 01B is authorized only for
the bounded candidate append-authoritative storage adapter and its review.

This directory records the mechanism-independent persistence evidence contracts
aligned with owner-accepted Gate 1–3 application semantics.

## Scope

- fixture id: `voxproof-persistence-evidence-current-contract`
- fixture version: `3`
- oracle version: `3`
- scenario contract version: `3`
- measurement contract version: `2`

## Correction-02 highlights

- production-equivalent `reusable-influence-snapshot:sha256-v2` identity via `production_bridge`
- typed analysis snapshot inputs for exact `hash_analysis_snapshot` reproduction
- optional canonical `reuse_enabled_analysis_binding` historical provenance
- fail-closed review-ledger and governance folds with session-bound actor validation
- SRT-based Unicode-safe anchor validation (non-panicking)
- `ReadOnlyOpenPolicy` for unknown-newer-format conditional safe read
- `validate_scenario_contracts_v3` and `validate_measurement_contract_value` input validators

## Correction-04 highlights

- canonical review-ledger and reuse-governance vector order is preserved for validation
- effective review status retains the observed revision and a typed MD-017 decision
- promotion and rejection payload observed bytes must equal the locator-bound review-case bytes
- malformed review events cannot authorize reusable promotion or historical provenance
- historical binding tests cover canonical snapshot, source revision, terms, identity, and reuse-enabled configuration

## Correction-05 highlights

- governance accepts only ReviewCases whose Unicode-safe SourceAnchor resolves to their exact observed bytes
- synchronized forged ReviewCase and promotion/rejection payload bytes cannot mutate either governance fold
- production snapshot identities are compared for promoted active, retained pre-revoke, retained pre-supersession, and fresh post-supersession runs
- the bounded application retains only its latest reuse-enabled run; governance mutation clears that run, while evidence may retain one explicit historical binding

## Correction-01 highlights

- complete typed `EvidenceSourceDecisionLocator` (all MD-018 fields)
- versioned canonical serialization (no Rust `Debug` authority identity)
- independent review-ledger and reuse-governance oracle folds
- derived fields excluded from canonical fingerprint; independently recomputed
- seven named fixture variants including non-zero source anchor and duplication lineage
- precise scenario fault/recovery semantics and capability-dependent compaction
- explicit deferred measurement scales with selection-impacting rationale

## Not in scope

- persistence mechanism selection
- evidence run generation (package 01C)
- production persistence integration
- candidate-mechanism selection or a Material Decision
- real-session migration or product persistence integration

## 01B bounded candidate adapter

`current-contract-append-authoritative-candidate` version `01B-1` is the
historical Correction-02 candidate owner-accepted by Ezra on 2026-08-11 at
`2229a36ff09fa56482673151a0ae010f3b9ec099`. Its `01B-2` successor is separately
accepted at `cee1b0c7ae8e03b8ece1f9f6051b174e49ec44b0` and remains a distinct
append-authoritative candidate.
It writes deterministic
current-contract v3 state records followed by an explicit append commit
acknowledgement, rehydrates from the committed canonical prefix, and recomputes
derived fields before the existing v3 oracle validates the result. Incomplete
tails are detected but never authoritative; malformed records, stale append
preconditions, duplicate canonical identities, unsafe session paths, oversized
records, and unknown newer writable formats fail closed.

Correction-01 adds fresh-process `open_existing` from only the bounded storage
root and validated session ID. Correction-02 adds handle-based static hard-link
containment for every authority-bearing leaf. The final bounded correction
canonicalizes that root, rejects static filesystem aliases for the session and
authority leaves,
bounds manifest reads and outbound append serialization, prevents incomplete
creation from being opened, and preflights remaining record capacity. The synced
Commit record is the semantic acknowledgement boundary; a failed manifest
checkpoint is reported in the acknowledgement and repaired under writer
ownership before another append. Mutating fault hooks require the live writer,
and duplication uses the latest replayed authority with a collision-safe,
independent evidence-writer identity. OS-released exclusive ownership retains
child-abort takeover coverage. The equivalence contract names this exact
candidate ID.

Static on-disk aliases (symlinks, hard links, and Windows reparse points) are
treated as hostile input.
This candidate does not claim protection against active same-privilege namespace
replacement races.

This is a bounded candidate adapter for future 01C evaluation, not a selected
mechanism, generated evidence artifact, or production session store.

## Current SQLite-v3 candidate precondition

`current-contract-sqlite-authoritative-candidate` version `01C-SQLITE-1` is a
separate current-contract-v3 SQLite candidate. Its canonical authority is a
typed relational mapping of `CurrentContractState`; it does not route through
the historical `EvidenceFixture` / `NormalizedSemanticState` candidate. The
current implementation and review package is bounded to candidate work only:
it does not execute 01C, select a mechanism, integrate product persistence, or
introduce migration behavior.

The candidate uses a SQLite transaction as the committed authority boundary,
then independently reopens and validates the relational rows with
`CurrentContractOracle` v3 before returning acknowledgement. It keeps derived
cache data non-authoritative, uses a bounded SHA-256 physical key distinct from
the semantic session ID, rejects static filesystem aliases at authority leaves,
and does not claim protection from active same-privilege namespace replacement.
Its Windows runtime evidence remains pending; macOS behavior does not establish
Windows behavior.

## Readiness

```yaml
mechanism_comparison_readiness: not_ready
mechanism_selection_readiness: not_ready
selection_status: none
tracker_status: GATE4_01C_SQLITE_V3_CANDIDATE_REVIEW_PENDING
01B_2:
  accepted_head: cee1b0c7ae8e03b8ece1f9f6051b174e49ec44b0
  owner_accepted: true
01C:
  authorized: true
  blocked_on: current_contract_v3_sqlite_candidate
sqlite_v3_candidate:
  status: implementation_or_review_pending
```

## Selected owner path

```yaml
selected_path: A_corrected_append_authoritative_comparator
next_package: VP-GATE4-EVIDENCE-COMPLETION-01B  # bounded candidate adapter only
```

The SQLite-v3 candidate is pending Windows runtime validation and final review.
Its implementation does not make 01C authorized for equivalent execution.

## Artifacts

See `readiness.json`, [the Correction-01 reviewer package](reviewer-correction-01.md),
[the Correction-02 reviewer package](reviewer-correction-02.md),
and repository module `src/persistence_evidence/current_contract/`.
