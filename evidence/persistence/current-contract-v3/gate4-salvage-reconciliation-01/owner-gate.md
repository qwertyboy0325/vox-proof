# VP-GATE4-SALVAGE-REQUIREMENT-RECONCILIATION-01

**Status:** OPEN  
**Opened:** 2026-08-14 (Ezra)  
**Type:** semantic reconciliation owner gate

## Blocking fact

Accepted candidates **Append 01B-2** and **SQLite 01C-SQLITE-2** do not satisfy frozen **scenario contract v3** for two **Required** scenarios:

| Scenario | Contract expectation |
|----------|-------------------|
| `canonical-reference-corruption` | `ReadOnlySalvage`, read-only **allowed**, writable **forbidden** |
| `source-locator-corruption` | same |

Recorded in `0f6cd55` / `capability-audit-correction-04.json`. Harness `6adb731` evidence preserved as **invalid_for_gate**.

## Question

> For `canonical-reference-corruption` and `source-locator-corruption`, is **ReadOnlySalvage** actually a product requirement?

## What candidates do today

```text
corruption detected → fail closed → no authority exposed
```

Both candidates implement this consistently (01B-2 / 01C-SQLITE-2 tests). Append salvage exists only for **uncommitted-tail / committed-prefix**, not these corruption classes.

## What contract v3 requires

```text
corruption detected → writable forbidden → read-only salvage still available
```

This is a **different product requirement**, not “more safe” vs “less safe.”

## Owner options

### A — Yes, salvage is required

- Keep scenario contract v3 salvage semantics
- Evolve **both** candidates (new versions)
- **Owner preference rank: 3**

### B — No, fail-closed is sufficient (preferred)

- Revise scenario contract (new version; v3 superseded for active use)
- Align Required corruption scenarios with fail-closed refusal
- Rerun bounded affected evidence only
- **Owner preference rank: 1**

### C — Split by corruption class

- New contract version with explicit per-class semantics
- Some classes salvage, some fail-closed
- **Owner preference rank: 2**

## Current readiness (unchanged)

```text
mechanism_comparison_readiness:  not_ready
mechanism_selection_readiness:   not_ready
selection_status:                none
```

## Prohibited until this gate closes

- Candidate semantic changes
- Scenario contract v3 edits without owner decision
- PRE_FINAL / Sol / mechanism selection
- Correction-04 harness work
- Required → Unsupported downgrade without contract revision

## Requested action

**Select A, B, or C** (or a refined B/C variant) and authorize one downstream work package.
