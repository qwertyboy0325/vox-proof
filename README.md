Status: current
Owns: Public project introduction, high-level current focus, implementation signal, and navigation to canonical documentation.
Does not own: Installation, technical stack details, benchmarks, detailed roadmap, or full product and architecture specifications.
Last reviewed against code/evidence: the canonical exact and bounded ASCII-Latin phonetic evidence review loop now includes effective transcript/session-term/detector/config/algorithm analysis identity, nearby source context in review prompts, raw-versus-final comparison/change inventory for strict skeleton-compatible inputs, strict skeleton-compatible calibration correspondence evaluation (`vox-proof evaluate`, committed at `e21be2e`), and bounded ASCII-Latin phonetic similarity evidence (`ascii-latin-phonetic-similarity` v0.1.0); qualifying owner-operated FLEURS real-speech human review completed at repository HEAD `7efe8ba` with all ten MD-007 mechanism gates passing; v0.1 is established by MD-008 as a bounded core mechanism only; local annotated tag pending recreation; the experiment-only contextual retrieval/ranking sidecar exists; all implemented paths are covered by unit and CLI tests; product and external-user validation remain deferred beyond v0.1

# VoxProof

![Local-first](https://img.shields.io/badge/runtime-local--first-informational)
![v0.2 architecture: draft](https://img.shields.io/badge/v0.2%20architecture-draft-yellow)

Local-first, evidence-backed transcript QA.

VoxProof is a local-first, evidence-backed transcript QA tool for reviewing an existing transcript, initially SRT, with provisional session-scoped term input. It identifies high-risk transcript spans, presents bounded candidate corrections with evidence, and requires human review before producing a reviewed transcript.

The authoritative review path remains deterministic and human-governed. An experiment-only sidecar can also retrieve bounded non-exact candidates from the current session terms and optionally rank them through a strict external-command interface. Experimental results are not canonical Evidence, ReviewCases, ReviewLedger decisions, or materialized edits.

Optional audio, Domain Collections, policies, automation, and formal model-dependent product behavior remain future directions rather than current authoritative runtime contracts.

## Current Focus

VoxProof is currently under active development.

The current focus is completing the narrow text-first pain-point MVP while making each real run produce local calibration and evaluation artifacts:

- local-first SRT transcript inspection
- source-preserving transcript representation
- usable session terms, glossary, and observed-error-form inputs
- evidence-backed candidate review cases with provisional deterministic detectors
- reviewed SRT, decision log, correction profile, and local run metrics
- raw-ASR-versus-human-final comparison for iterative calibration
- real-material evaluation of bounded non-exact retrieval
- measurement of candidate recall, contextual-ranking uplift, and false-positive burden
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
- nearby source context in authoritative review prompts
- raw-versus-final comparison/change inventory for strict skeleton-compatible inputs (`vox-proof compare`)
- strict skeleton-compatible calibration correspondence evaluation for strict skeleton-compatible inputs (`vox-proof evaluate`, committed at `e21be2e`)
- provisional session-scoped term / observed-form file input
- human-readable session summary and minimum local run metrics
- distinct exact alias and observed-error-form evidence paths
- effective analysis identity for the canonical session-term detector set
- bounded ASCII-Latin phonetic similarity evidence in the authoritative review pipeline
- exploratory real-speech zero-candidate and reference-supported emitted-candidate mechanism paths for the bounded phonetic producer
- exploratory calibration correspondence evaluate path on frozen public FLEURS material at `e21be2e`
- phonetic-representation characterization covering Latin, Han, acronyms, symbols, and mixed-script limitations
- experiment-only bounded Latin and Han-pinyin candidate retrieval
- rules-only, deterministic-fake, and strict external-command contextual-ranking modes
- versioned contextual-resolution sidecar with request-local candidate IDs
- authority-preserving manual-correction markers with explicit alias-and-rerun guidance

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
- [ ] Persistence

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
