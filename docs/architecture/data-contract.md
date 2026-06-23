Status: current
Owns: Conceptual domain model and data ownership boundaries.
Does not own: Final JSON schemas, storage paths, database design, UI state model, or implementation-specific type definitions.
Last reviewed against code: Rust bootstrap exists; no end-to-end VoxProof pipeline behavior has been verified yet

# Conceptual Data Contract

This document describes the conceptual domain model. It is not yet a final JSON schema.

## Core Concepts

### Transcript

`Transcript` represents the source transcript being reviewed. For v0.1, the initial transcript format is SRT. The transcript remains the source material from which review cases and reviewed output are derived.

### LanguagePack

`LanguagePack` provides reusable language knowledge for analysis. It is more than a word list. It may eventually contain canonical terms, aliases, language metadata, term type, pronunciation hints, observed ASR confusions, related terms, scope, approval status, and version.

### AnalysisRun

`AnalysisRun` represents one analysis of explicit inputs and configuration snapshots. It connects source transcript data, optional audio, Language Pack state, analyzer outputs, ranking, and review results for traceability.

### CandidateSpan / ReviewCase

`CandidateSpan` is the core review unit. It identifies a suspicious span in the source transcript and may also be presented as a `ReviewCase` for human review.

A `CandidateSpan` is not limited to one word. It is not replaced by waveform data in future versions. Future acoustic information is additional evidence associated with the same review unit.

### Evidence

`Evidence` records why a candidate was proposed. Evidence may come from glossary or alias matching, phonetic or pinyin similarity, mixed-language signals, or future capability modules.

### CorrectionDecision

`CorrectionDecision` records a human decision for a review case: acceptance, rejection, edit, or deferral. Human decisions are the source of transcript changes.

A single accepted correction does not automatically change the Language Pack. Promotion of observed corrections into a Language Pack is a future governed process, not v0.1 behavior.

## Ownership Boundaries

- Analyzer modules produce evidence.
- Ranking synthesizes evidence into review priority.
- Human review creates correction decisions.
- Materialization derives reviewed output.
- UI owns only transient interaction state such as selection, filters, and playback position.

Field-level schemas, persistence choices, and storage paths are intentionally unsettled until implementation work requires them.
