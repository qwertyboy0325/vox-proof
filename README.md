Status: current
Owns: Public project introduction and navigation to canonical documentation.
Does not own: Installation, implementation status, technical stack, benchmarks, detailed roadmap, or full product and architecture specifications.
Last reviewed against code: Rust bootstrap exists; no end-to-end VoxProof pipeline behavior has been verified yet

# VoxProof

Local-first, evidence-backed transcript QA.

VoxProof is a local-first, evidence-backed transcript QA tool for reviewing an existing transcript, initially SRT, with optional audio and a Language Pack. It identifies high-risk transcript spans, presents bounded candidate corrections with evidence, and requires human review before producing a corrected transcript.

## What VoxProof Does

`existing SRT + optional audio + Language Pack -> evidence-backed review -> reviewed SRT + decision records`

## Direction Beyond v0.1

v0.1 deliberately establishes an evidence-backed human-review foundation before introducing model-dependent behavior. Future exploration may use multiple local models for semantic and contextual interpretation, with scoped adaptation to recurring speaker, project, team, or domain language patterns and review preferences.

These are research directions rather than implemented capabilities or delivery commitments. See [Product Hypotheses](docs/product/hypotheses.md) for the assumptions that would need validation.

## Non-Goals

- Not an ASR engine.
- Not an automatic rewriting tool.
- Not a meeting-summary app.
- Not cloud-first.
- Not a medical or clinical decision system.

See the [documentation index](docs/README.md) for the canonical project documentation.