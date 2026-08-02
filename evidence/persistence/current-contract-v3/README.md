# Current-Contract Persistence Evidence v3

Status: evidence contract package (VP-GATE4-EVIDENCE-COMPLETION-01A)

Correction-05 (`VP-GATE4-EVIDENCE-COMPLETION-01A-REMOTE-REVIEW-CORRECTION-05`)
closes remote-review blockers in anchor-validated ReviewCase authority and
production/evidence lifecycle identity cross-checks. Corrected 01A was
owner-accepted by Ezra on 2026-08-02 at
`9be1e99ca45db2dcd6b2b7fbe03137e6cc28e241`. Package 01B is authorized only for
the bounded candidate append-authoritative storage adapter and its review.

Correction-01 (`VP-GATE4-EVIDENCE-COMPLETION-01A-REMOTE-REVIEW-CORRECTION-01`)
addresses remote-review blockers F1–F4. 01A is **not** owner-accepted; next gate is
ChatGPT remote review and owner acceptance of corrected 01A.

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

`current-contract-append-authoritative-candidate` version `01B-1` is available
only behind the `persistence-spike` feature. It writes deterministic
current-contract v3 state records followed by an explicit append commit
acknowledgement, rehydrates from the committed canonical prefix, and recomputes
derived fields before the existing v3 oracle validates the result. Incomplete
tails are detected but never authoritative; malformed records, stale append
preconditions, duplicate canonical identities, unsafe session paths, oversized
records, and unknown newer writable formats fail closed.

This is a bounded candidate adapter for future 01C evaluation, not a selected
mechanism, generated evidence artifact, or production session store.

## Readiness

```yaml
mechanism_comparison_readiness: not_ready
mechanism_selection_readiness: not_ready
selection_status: none
tracker_status: GATE4_EVIDENCE_COMPLETION_01B_REMOTE_REVIEW_PENDING
```

## Selected owner path

```yaml
selected_path: A_corrected_append_authoritative_comparator
next_package: VP-GATE4-EVIDENCE-COMPLETION-01B  # bounded candidate adapter only
```

01B implementation is pending fresh remote review and owner acceptance. Readiness
remains `not_ready`; 01C evidence execution is not authorized.

## Artifacts

See `readiness.json` and repository module
`src/persistence_evidence/current_contract/`.
