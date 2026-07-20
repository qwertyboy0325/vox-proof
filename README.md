<!--
Status: current
Owns: Public project introduction, high-level current focus, implementation signal, navigation to canonical documentation.
Does not own: Installation, technical stack details, benchmarks, detailed roadmap, or full product and architecture specifications.
Last reviewed against code/evidence:
- v0.1 bounded core: exact/phonetic review loop, compare/evaluate calibration, FLEURS human-review gates (MD-007)
- Persistence Packages 2A–2D merged to main; mechanism selection remains none
- v0.2 authoritative loop branch ready; implementation not started
- Product and external-user validation remain deferred
-->

# VoxProof

[![Persistence CI](https://github.com/qwertyboy0325/vox-proof/actions/workflows/persistence-sqlite-windows.yml/badge.svg?branch=main)](https://github.com/qwertyboy0325/vox-proof/actions/workflows/persistence-sqlite-windows.yml)
![Version](https://img.shields.io/badge/version-0.1.0-blue)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Runtime](https://img.shields.io/badge/runtime-local--first-informational)

**Local-first, evidence-backed transcript QA with deterministic review boundaries and explicit human authority.**

VoxProof reviews an existing transcript—initially SRT—with provisional session-scoped terminology. It surfaces bounded, evidence-backed candidate corrections, requires human decisions before any reviewed output, and keeps experimental sidecar results outside canonical authority.

> VoxProof is under active development. Current evidence establishes bounded mechanisms and engineering behavior—not production readiness, product validation, filesystem durability, or hardware power-loss resilience.

## Project Status

| Area | Status |
|---|---|
| Bounded v0.1 core | Established |
| Authoritative review path | Deterministic and human-governed |
| Main persistence CI | Passing on Windows GitHub Actions |
| Persistence evidence | Packages 2A–2D merged; mechanism selection remains `none` |
| v0.2 authoritative loop | Branch ready; implementation not started |
| Product validation | Deferred |

## Current Focus

VoxProof is completing the narrow text-first pain-point MVP while making each real run produce local calibration and evaluation artifacts:

- local-first SRT transcript inspection with source preservation
- provisional session-scoped terms, glossary, and observed-error-form inputs
- evidence-backed candidate review cases with deterministic detectors
- reviewed SRT, decision log, correction profile, and local run metrics
- raw-ASR-versus-human-final comparison for iterative calibration
- explicit Material Decisions before durable semantic changes

Engineering completion and validation collection proceed in parallel. An engineering-complete prototype is not product validation.

Recent work (capability groups):

- **Deterministic SRT parsing and source preservation** — strict parsing, anchors, stable revision identity, source-preserving transcript model
- **Evidence-backed candidate detection** — glossary, observed-error-form, and bounded ASCII-Latin phonetic paths; effective analysis identity
- **Human review ledger and reviewed transcript derivation** — interactive CLI review, decision log, reviewed SRT, nearby source context in prompts
- **Calibration comparison and evaluation** — `vox-proof compare` and `vox-proof evaluate` for strict skeleton-compatible inputs
- **Bounded phonetic and contextual-retrieval experiments** — exploratory real-speech probes; experiment-only sidecar with non-authoritative ranking
- **Persistence authority, recovery, and cross-platform evidence** — SQLite spike contracts, durability harness, bounded macOS / Windows-GHA evidence (selection `none`)

See [documentation index](docs/README.md) and [v0.1 execution order](docs/product/v0.1-execution-order.md) for canonical detail.

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
- [x] Nearby source context in authoritative review prompts
- [x] Raw-versus-final comparison/change inventory for strict skeleton-compatible inputs
- [x] Strict skeleton-compatible calibration correspondence evaluation (`vox-proof evaluate`)
- [x] Mixed Traditional-Chinese / ASCII-Latin synthetic fixture (MD-007 D10 mechanism tests; not real-speech or product validation)
- [x] Session-term / observed-form input
- [x] Explicit observed-error-form mapping
- [x] Effective canonical analysis identity prerequisite
- [x] Bounded ASCII-Latin phonetic similarity evidence
- [x] Phonetic representation characterization
- [x] Experimental bounded non-exact candidate retrieval
- [x] Experimental contextual-ranking subprocess boundary
- [x] Experiment-only contextual-resolution JSON sidecar
- [ ] Authorized real-material contextual-resolution study
- [ ] Measured retrieval and ranking baseline
- [ ] Matching semantics v0
- [ ] Eval harness
- [x] Correction profile / session report artifact
- [x] Persistence authority and recovery contracts
- [x] SQLite persistence mechanism and durability evidence harness
- [x] Bounded macOS / Windows-GHA persistence evidence
- [ ] Persistence mechanism selection
- [ ] Product persistence integration

## What VoxProof Does

Authoritative path:

`existing SRT + provisional session terms -> bounded exact and phonetic evidence -> human review -> reviewed SRT + decision records`

Experimental sidecar:

`existing SRT + provisional session terms + session description -> bounded non-exact retrieval -> optional contextual ranking -> experiment sidecar`

Experimental candidates do not become canonical Evidence, ReviewCases, ReviewLedger decisions, or materialized edits.

## Direction Beyond v0.1

v0.1 deliberately establishes an evidence-backed human-review foundation before formalizing model-dependent behavior. Current experimentation may use bounded candidate retrieval and external contextual ranking, but it does not establish production provider, policy, automation, or non-exact Evidence semantics.

Future exploration may use multiple local models for semantic and contextual interpretation, with scoped adaptation to recurring speaker, project, team, or domain language patterns and review preferences.

These remain research directions rather than delivery commitments. See [Product Hypotheses](docs/product/hypotheses.md) for the assumptions that would need validation.

## Non-Goals

- Not an ASR engine.
- Not an automatic rewriting tool.
- Not a meeting-summary app.
- Not cloud-first.
- Not a medical or clinical decision system.

See the [documentation index](docs/README.md) for the canonical project documentation.
