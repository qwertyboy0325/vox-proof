Status: current
Owns: Public project introduction, high-level current focus, implementation signal, and navigation to canonical documentation.
Does not own: Installation, technical stack details, benchmarks, detailed roadmap, or full product and architecture specifications.
Last reviewed against code: the Track 1 local review loop exists and is covered by unit and CLI tests; real-material mechanism validation remains pending

# VoxProof

Local-first, evidence-backed transcript QA.

VoxProof is a local-first, evidence-backed transcript QA tool for reviewing an existing transcript, initially SRT, with optional audio and a Language Pack. It identifies high-risk transcript spans, presents bounded candidate corrections with evidence, and requires human review before producing a corrected transcript.

## Current Focus

VoxProof is currently under active development.

The current focus is completing the narrow text-first pain-point MVP while making each real run produce local calibration and evaluation artifacts:

- local-first SRT transcript inspection
- source-preserving transcript representation
- usable session terms, glossary, and observed-error-form inputs
- evidence-backed candidate review cases with provisional deterministic detectors
- reviewed SRT, decision log, correction profile, and local run metrics
- raw-ASR-versus-human-final comparison for iterative calibration
- explicit Material Decisions before durable semantic changes

Engineering completion and validation collection proceed in parallel. The first measured error distribution is expected to come from the first instrumented real-material run rather than block the remaining MVP work. This does not make synthetic tests or an engineering-complete prototype product-validation evidence.

Recent work:

- Rust project bootstrap
- strict SRT parsing and validation boundaries
- source anchors and candidate spans
- glossary-backed evidence model
- v0.1 scope, non-goals, and data-contract documentation
- Material Decision governance
- stable `TranscriptRevisionId` decision and implementation
- human review decisions and append-only review ledger
- deterministic reviewed SRT derivation
- decision log rendering
- minimal facilitated CLI review flow
- provisional session-scoped term / observed-form file input
- human-readable session summary and minimum local run metrics

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
- [x] Segment-position reporting cleanup
- [x] Human review decision log
- [x] CLI review flow
- [ ] Private mixed zh-EN evaluation fixture
- [x] Session-term / observed-form input
- [ ] Matching semantics v0
- [ ] Eval harness
- [x] Correction profile / session report artifact
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
