# VoxProof

![Local-first](https://img.shields.io/badge/runtime-local--first-informational)

**Local-first Rust transcript QA for reviewable subtitle corrections.**

VoxProof accepts an existing SRT file and session terminology, identifies
bounded term-related candidates, preserves the source and evidence for each
candidate, and produces a reviewed SRT only after explicit human decisions.

**Status: active prototype.** The narrow v0.1 review mechanism is established
as a bounded core mechanism. External-user validation, built-in ASR or media
handling, product-session persistence, and production durability are unfinished.

## Engineering highlights

- Deterministic SRT parsing, source anchors, and revision identity.
- Explicit human decisions with reproducible reviewed-output derivation and
  decision records.
- Rules-only evidence paths for exact aliases, observed error forms, and bounded
  ASCII-Latin phonetic similarity.
- Experimental retrieval and ranking isolated from the authoritative review
  path, so they cannot silently rewrite a transcript.

## What exists now

`existing SRT + session terms -> bounded evidence -> human review -> reviewed SRT + decision records`

- A Rust CLI for parsing and reviewing existing SRT files.
- Session terms, glossary aliases, and observed-error-form inputs.
- Candidate evidence, nearby source context, human review decisions, reviewed
  output, a decision log, and a session summary.
- Comparison and calibration utilities for strict skeleton-compatible inputs.

## Persistence research

A separate persistence-spike evidence harness exercises SQLite and append-bundle
candidates against selected process-crash/recovery, ownership, stale-write, and
corruption scenarios. It is **not** product persistence: no mechanism is
selected, and filesystem or hardware-power-loss durability is not claimed.

## Scope and limitations

- VoxProof reviews existing transcripts; it is not an ASR engine, subtitle
  editor, meeting-summary app, or automatic rewriting tool.
- No transcript text changes before a human accepts, rejects, edits, or defers a
  review item.
- Optional audio, local model runtimes, cloud sync, collaboration, and
  medical-specific workflows are outside the current runtime scope.
- The bounded mechanism does not establish general detector effectiveness,
  product usefulness, or production reliability.

## Experimental work

An experiment-only sidecar can retrieve bounded non-exact candidates from the
current session terms and optionally rank them through a strict external-command
interface. Its output does not become authoritative evidence, review decisions,
or materialized edits.

## Deep technical record

- [Documentation index](docs/README.md) — canonical product, architecture,
  quality, and Material Decision documents.
- [v0.1 scope](docs/product/v0.1.md) — accepted inputs, pipeline, evidence
  sources, acceptance boundary, and non-goals.
- [Persistence evidence harness](src/persistence_evidence/README.md) —
  candidate-neutral spike boundary and intentionally unimplemented guarantees.

The project uses substantial AI assistance where appropriate. Requirements,
boundaries, code, tests, diffs, and artifacts remain reviewable; model output
does not automatically become accepted truth.
