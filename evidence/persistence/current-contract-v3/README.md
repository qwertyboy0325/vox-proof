# Current-Contract Persistence Evidence v3

Status: evidence contract package (VP-GATE4-EVIDENCE-COMPLETION-01A)

Correction-02 (`VP-GATE4-EVIDENCE-COMPLETION-01A-REMOTE-REVIEW-CORRECTION-02`)
closes remaining remote-review blockers: production-equivalent snapshot identity,
historical reuse-enabled analysis binding, fail-closed oracle folds, Unicode-safe
anchor validation, and strengthened scenario/measurement input validators.

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
tracker_status: GATE4_EVIDENCE_COMPLETION_01A_CORRECTION_02_REMOTE_REVIEW_PENDING
```

## Selected owner path

```yaml
selected_path: A_corrected_append_authoritative_comparator
next_package: VP-GATE4-EVIDENCE-COMPLETION-01B  # not authorized until corrected 01A accepted
```

## Artifacts

See `readiness.json` and repository module
`src/persistence_evidence/current_contract/`.
