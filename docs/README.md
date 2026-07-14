Status: current
Owns: Documentation navigation, canonical document ownership, and document lifecycle meanings.
Does not own: Product scope, architecture details, data contracts, quality criteria, or execution progress.
Last reviewed against code: v0.1 Track 1 code closed loop exists; real-material/product validation remains pending.

# VoxProof Documentation

This index points to the active canonical documents for VoxProof. Each durable claim should have one owning document.

## Canonical Documents

- [product/versioning.md](product/versioning.md): pre-1.0 version semantics, the version ladder, and per-version allowed claims.
- [product/v0.1.md](product/v0.1.md): current v0.1 product scope and acceptance boundary.
- [product/v0.1-execution-order.md](product/v0.1-execution-order.md): current global execution goal, parallel completion-and-validation posture, and near-term order toward a testable v0.1 loop.
- [product/hypotheses.md](product/hypotheses.md): unvalidated market, user, and future-product hypotheses.
- [architecture/overview.md](architecture/overview.md): architecture principles and fixed v0.1 processing shape.
- [architecture/data-contract.md](architecture/data-contract.md): conceptual domain entities and data ownership boundaries.
- [quality/evaluation.md](quality/evaluation.md): fixtures, ground truth, metrics, and regression expectations.

## Governance

- [Material Decisions](governance/material-decisions.md)
- [MD-001: Stable TranscriptRevisionId](governance/decisions/MD-001-transcript-revision-id.md)
- [MD-002: ReviewCase and Review Ledger Semantics](governance/decisions/MD-002-review-ledger-semantics.md)
- [MD-003: Minimal Reviewed Output Materialization Semantics](governance/decisions/MD-003-reviewed-output-materialization.md)

## Document Lifecycle

- `current`: the active source of durable understanding for the project state.
- `draft`: a proposed document or revision that is not yet canonical.
- `exploratory`: research, sketches, or option analysis that should not be treated as a commitment.
- `superseded`: historical material replaced by a newer canonical document.

Historical or superseded documents must be removed from active navigation. Issues and pull requests should eventually own execution progress; documentation owns durable product, architecture, data-contract, and quality understanding.
