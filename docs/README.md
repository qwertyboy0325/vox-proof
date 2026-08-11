Status: current
Owns: Documentation navigation, canonical document ownership, and document lifecycle meanings.
Does not own: Product scope, architecture details, data contracts, quality criteria, or execution progress.
Last reviewed against code: v0.1 is established by MD-008; v0.2 is in progress; MD-016 accepts the native desktop foundation; the bounded Gate 1 desktop implementation is owner-accepted at `0104ea2519cfd18b3ec639e5ec1566b734a6655e`; the bounded Gate 2 Manual Replacement implementation under MD-017 is owner-accepted at `6e395bdf9cc0a5b4333876f165280e4cc4bf506f`; the bounded Gate 3 reusable-influence implementation under MD-018 is owner-accepted at `225e1b3fc6cc0403d202d1d72fe7325a97dc3096`; external-user validation remains pending.

# VoxProof Documentation

This index points to the active canonical documents for VoxProof. Each durable claim should have one owning document.

## Start Here

- [VoxProof for Everyone / 給每個人的 VoxProof](product/voxproof-for-everyone.md): bilingual, non-technical explanation of what the product does today and the long-term goal of becoming a governance and evidence layer between probabilistic AI and authoritative records.

## Canonical Documents

- [product/correction-system-boundaries.md](product/correction-system-boundaries.md): cross-version correction taxonomy and evidence, context, policy, transformation, projection, authorization, and Domain Collection boundaries.
- [product/strategic-direction.md](product/strategic-direction.md): current product thesis, strategic horizons, governed reusable-learning framing, and the boundary between direction, Material Decisions, and implementation authority.
- [product/versioning.md](product/versioning.md): pre-1.0 version semantics, the version ladder, and per-version allowed claims.
- [product/v0.1.md](product/v0.1.md): current v0.1 product scope and acceptance boundary.
- [product/v0.1.0-release-preparation.md](product/v0.1.0-release-preparation.md): v0.1.0 gate matrix, release-notes draft, validation records, retag note, and remaining release actions (v0.1 established; local annotated tag pending recreation).
- [product/v0.2-execution-order.md](product/v0.2-execution-order.md): current global execution goal, delivery profiles, and ordered product gates during v0.2.
- [product/v0.1-execution-order.md](product/v0.1-execution-order.md): retained v0.1 historical execution, implementation, calibration, and establishment-evidence context.
- [product/hypotheses.md](product/hypotheses.md): unvalidated market, user, and future-product hypotheses.
- [architecture/overview.md](architecture/overview.md): architecture principles and fixed v0.1 processing shape.
- [architecture/data-contract.md](architecture/data-contract.md): conceptual domain entities and data ownership boundaries.
- [architecture/v0.2-c4-architecture.md](architecture/v0.2-c4-architecture.md): **draft / proposed** v0.2 C4 architecture views, trust boundaries, the MD-016 accepted native-desktop foundation, and remaining open decisions (companion DSL: `architecture/v0.2-c4.dsl`).
- [quality/evaluation.md](quality/evaluation.md): fixtures, ground truth, metrics, and regression expectations.

## Active Research

- [Research register](research/README.md): mandatory discovery point for active non-authoritative research items and their lifecycle.
- [VP-ARCH-001: Loose Inference / Strict Commitment](research/VP-ARCH-001-loose-inference-strict-commitment.md): **exploratory / non-canonical** investigation of high-recall probabilistic synthesis behind a strict governed commitment boundary. Tracking issue: [#1](https://github.com/qwertyboy0325/vox-proof/issues/1).
- [VP-ARCH-002: Authority-Transition Graph](research/VP-ARCH-002-authority-transition-graph.md): **exploratory / non-canonical** architecture reasoning model for distinguishing observation, proposal, commitment, projection, validation, and continuity transitions. Tracking issue: [#14](https://github.com/qwertyboy0325/vox-proof/issues/14).

Active research is not accepted architecture. It must not be implemented or treated as a Material Decision without a separate owner decision.

## Governance

- [Material Decisions](governance/material-decisions.md)
- [MD-001: Stable TranscriptRevisionId](governance/decisions/MD-001-transcript-revision-id.md)
- [MD-002: ReviewCase and Review Ledger Semantics](governance/decisions/MD-002-review-ledger-semantics.md)
- [MD-003: Minimal Reviewed Output Materialization Semantics](governance/decisions/MD-003-reviewed-output-materialization.md)
- [MD-004: Effective Analysis Identity and Phonetic Evidence Boundary](governance/decisions/MD-004-effective-analysis-identity-and-phonetic-evidence-boundary.md)
- [MD-005: Bounded ASCII-Latin Phonetic Evidence v0](governance/decisions/MD-005-bounded-ascii-latin-phonetic-evidence-v0.md)
- [MD-006: Strict Skeleton Calibration Correspondence v0](governance/decisions/MD-006-strict-skeleton-calibration-correspondence-v0.md)
- [MD-007: v0.1 Establishment Evidence and Release Gates](governance/decisions/MD-007-v0.1-establishment-evidence-and-release-gates.md)
- [MD-008: v0.1 Core Mechanism Establishment](governance/decisions/MD-008-v0.1-core-mechanism-establishment.md)
- [MD-016: Native Desktop Framework and In-Process Integration](governance/decisions/MD-016-native-desktop-framework-and-in-process-integration.md)
- [MD-017: Governed Manual Replacement on an Existing ReviewCase](governance/decisions/MD-017-governed-manual-replacement-on-existing-review-case.md)
- [MD-018: Minimal Governed Reusable Influence Semantics](governance/decisions/MD-018-proposed-minimal-governed-reusable-influence.md)

## Repository Knowledge Authority

When repository sources disagree, use this hierarchy unless a newer explicit
owner decision says otherwise:

1. Latest explicit owner decision.
2. Accepted Material Decisions.
3. Current canonical product vision and product contracts.
4. Current canonical architecture.
5. Current execution-order documents.
6. Exact implementation, tests, and empirical evidence.
7. GitHub Issues and milestones.
8. Strategic research notes and product hypotheses.
9. Agent summaries and implementation summaries.

Implementation, tests, and empirical evidence establish what is implemented or
observed. They do not silently override accepted product semantics or a
Material Decision. Likewise, a stale issue or milestone cannot override newer
canonical documentation.

Issues and milestones are execution trackers, not semantic authority. Where
tracker ownership is separately authorized, they may record current work,
blockers, required evidence, and the next gate. They do not define what
VoxProof is, change binding product invariants, accept architecture, or make a
semantic decision binding.

## Document Lifecycle

- `current`: the active source of durable understanding for the project state.
- `draft`: a proposed document or revision that is not yet canonical.
- `exploratory`: research, sketches, or option analysis that should not be treated as a commitment.
- `superseded`: historical material replaced by a newer canonical document.

Historical or superseded documents must be removed from active navigation. Documentation owns durable product, architecture, data-contract, quality, and execution-order understanding; authorized trackers may own live task status without becoming semantic authority.
