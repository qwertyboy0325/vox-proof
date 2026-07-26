# AGENTS.md

Cross-agent entry point for VoxProof. This file orients an agent; it does not duplicate the canonical documents.

## What VoxProof Is

- VoxProof is a local-first, evidence-backed transcript QA tool.
- Current scope is post-ASR review, not ASR generation.
- No transcript text may be silently rewritten.
- Human decisions are canonical.

## Learning-First Implementation Posture

VoxProof has a Rust bootstrap, but it is not yet an end-to-end VoxProof product pipeline.

Default to bounded, verifiable VoxProof product slices. Do not artificially decompose a coherent feature into one-concept micro-exercises.

Treat ordinary Rust syntax and common engineering concerns as implementation details unless the user asks for instruction. Slow down and explain carefully at Rust-specific semantic or architectural boundaries such as ownership, borrowing, aliasing, lifetimes, iterator consume/borrow/mutate semantics, trait or generic abstraction boundaries, async and shared-state choices, or `unsafe` and performance-sensitive code.

Keep generated changes inspectable and bounded, but do not force a beginner tutorial pace. Distinguish clearly between explanation, suggested exercise, minimal example, user-authored implementation review, and an explicit full implementation request. The implementation goal is VoxProof progress; Rust learning supports that goal.

## Required Reading Order

1. `README.md`
2. `docs/README.md`
3. The relevant canonical document under `docs/product/`, `docs/architecture/`, or `docs/quality/`
4. The applicable rules under `.cursor/rules/`
5. The applicable skills under `.cursor/skills/`

## Open Research Gate

Before proposing or implementing changes involving architecture, inference, candidate generation, review semantics, decisions, canonical projection, temporal state, conflict handling, supersession, revocation, expiry, or persistence:

1. read `docs/research/README.md`;
2. inspect every active research item relevant to the proposed change;
3. preserve each item's stated non-authoritative boundary.

Research items are not accepted architecture and are not implementation authorization.

If a proposed change implicitly resolves, assumes, or bypasses an open research question, stop and request an explicit owner decision. Do not silently choose an answer through code, schema changes, renaming, documentation wording, or persistence design.

## Canonical Topic Map

- v0.1 scope: `docs/product/v0.1.md`
- Current global execution goal and order: `docs/product/v0.1-execution-order.md`
- Hypotheses: `docs/product/hypotheses.md`
- Architecture: `docs/architecture/overview.md`
- Conceptual data contracts: `docs/architecture/data-contract.md`
- Quality and regression expectations: `docs/quality/evaluation.md`
- Active non-authoritative research: `docs/research/README.md`

## Proof Of Behavior

Documentation describes intent. Once implementation exists, code and tests become the proof of executable behavior. Do not claim runtime behavior that has not been verified.