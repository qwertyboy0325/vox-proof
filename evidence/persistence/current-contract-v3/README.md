# Current-Contract Persistence Evidence v3

Status: evidence contract package (VP-GATE4-EVIDENCE-COMPLETION-01A)

Correction-04 (`VP-GATE4-EVIDENCE-COMPLETION-01A-REMOTE-REVIEW-CORRECTION-04`)
closes remote-review blockers in retained-index canonical event ordering,
validated typed review decisions, exact observed-text binding, and historical
reuse-enabled analysis-binding coverage. 01A is **not** owner-accepted; next
gate is fresh remote review and owner acceptance of corrected 01A.

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
- candidate storage implementation
- evidence run generation (package 01C)
- production persistence integration
- package 01B (not authorized)

## Readiness

```yaml
mechanism_comparison_readiness: not_ready
mechanism_selection_readiness: not_ready
selection_status: none
tracker_status: GATE4_EVIDENCE_COMPLETION_01A_CORRECTION_04_REMOTE_REVIEW_PENDING
```

## Selected owner path

```yaml
selected_path: A_corrected_append_authoritative_comparator
next_package: VP-GATE4-EVIDENCE-COMPLETION-01B  # not authorized until corrected 01A accepted
```

## Artifacts

See `readiness.json` and repository module
`src/persistence_evidence/current_contract/`.
