Status: current
Owns: Public project introduction, high-level current focus, implementation signal, and navigation to canonical documentation.
Does not own: Installation, technical stack details, benchmarks, detailed roadmap, or full product and architecture specifications.
Last reviewed against code: stable transcript revision identity exists; no end-to-end VoxProof review flow has been verified yet

# VoxProof

Local-first, evidence-backed transcript QA.

VoxProof is a local-first, evidence-backed transcript QA tool for reviewing an existing transcript, initially SRT, with optional audio and a Language Pack. It identifies high-risk transcript spans, presents bounded candidate corrections with evidence, and requires human review before producing a corrected transcript.

## Current Focus

VoxProof is currently under active development.

The current focus is the narrow text-first review wedge:

- local-first SRT transcript inspection
- source-preserving transcript representation
- stable transcript revision identity
- evidence-backed candidate review cases
- explicit Material Decisions before durable semantic changes

Recent work:

- Rust project bootstrap
- strict SRT parsing and validation boundaries
- source anchors and candidate spans
- glossary-backed evidence model
- v0.1 scope, non-goals, and data-contract documentation
- Material Decision governance
- stable `TranscriptRevisionId` decision and implementation

## Current Implementation Status

- [x] Project scaffold
- [x] Documentation contract
- [x] Material Decision governance
- [x] SRT parser
- [x] Source-preserving transcript model
- [x] Source anchors
- [x] Stable transcript revision identity
- [x] Initial glossary-based candidate detector
- [x] Initial typed evidence model
- [x] ReviewCase wrapper for detector-raised candidates
- [ ] Private mixed zh-EN evaluation fixture
- [ ] Segment-position reporting cleanup
- [ ] Matching semantics v0
- [ ] Eval harness
- [ ] Report artifact
- [ ] Human review decision log
- [ ] CLI review flow
- [ ] Persistence

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
