# VP-GATE4-SALVAGE-REQUIREMENT-RECONCILIATION-01

**Status:** CLOSED  
**Opened:** 2026-08-14 (Ezra)  
**Closed:** 2026-08-14  
**Owner decision:** B_tight  
**Anchor commit:** `e03b15e`

## Decision summary

Authoritative canonical/provenance corruption → **fail closed** (no read-only salvage, no trusted read exposure).  
Non-authoritative / reconstructible corruption → salvage/rebuild may remain supported.  
Append incomplete-tail committed-prefix salvage preserved.

See `owner-decision.json` for full record.

## Authorized downstream

**VP-GATE4-SCENARIO-CONTRACT-V4-RECONCILIATION-01** — scenario contract v4 for `canonical-reference-corruption` and `source-locator-corruption`.

## Current readiness (unchanged until valid v4 evidence)

```text
mechanism_comparison_readiness:  not_ready
mechanism_selection_readiness:   not_ready
selection_status:                none
```
