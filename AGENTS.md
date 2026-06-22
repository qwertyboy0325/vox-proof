# AGENTS.md

Cross-agent entry point for VoxProof. This file orients an agent; it does not duplicate the canonical documents.

## What VoxProof Is

- VoxProof is a local-first, evidence-backed transcript QA tool.
- Current scope is post-ASR review, not ASR generation.
- No transcript text may be silently rewritten.
- Human decisions are canonical.

## Learning-First Implementation Posture

VoxProof is pre-Cargo-init. Do not assume a Rust package exists, and do not treat `cargo init` as an implied next step.

Default to one primary learning concept per task. When appropriate, prefer: explain the small concept -> agree the narrow exercise -> user attempts or requests a minimal starter -> review/debug -> stop.

Keep generated code small enough for the user to inspect and understand. Distinguish clearly between explanation, suggested exercise, minimal example, user-authored implementation review, and an explicit full implementation request.

## Required Reading Order

1. `README.md`
2. `docs/README.md`
3. The relevant canonical document under `docs/product/`, `docs/architecture/`, or `docs/quality/`
4. The applicable rules under `.cursor/rules/`
5. The applicable skills under `.cursor/skills/`

## Canonical Topic Map

- v0.1 scope: `docs/product/v0.1.md`
- Hypotheses: `docs/product/hypotheses.md`
- Architecture: `docs/architecture/overview.md`
- Conceptual data contracts: `docs/architecture/data-contract.md`
- Quality and regression expectations: `docs/quality/evaluation.md`

## Proof Of Behavior

Documentation describes intent. Once implementation exists, code and tests become the proof of executable behavior. Do not claim runtime behavior that has not been verified.
