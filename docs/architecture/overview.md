Status: current
Owns: Architecture principles and the current fixed conceptual processing shape.
Does not own: Product acceptance criteria, field-level schemas, storage paths, implementation tasks, or future orchestration commitments.
Last reviewed against code: Rust bootstrap exists; no end-to-end VoxProof pipeline behavior has been verified yet

# Architecture Overview

## Principles

- Local-first.
- Analyzers produce evidence, not edits.
- Human decisions are canonical.
- Output is materialized from source transcript plus accepted decisions.
- An Analysis Run uses explicit input and configuration snapshots.
- Fixed pipeline before dynamic orchestration.
- Capability modules should add evidence without replacing the core review unit.
- Source data, review decisions, and evidence should remain traceable.
- UI is a presentation and interaction layer, not the owner of domain truth.

## Current Conceptual Processing Shape

`Transcript -> Normalize -> CandidateSpan detection -> analyzer modules -> evidence aggregation -> ranking -> review -> materialized output`

Analyzer modules are capability modules. They inspect transcript data, configuration, and available inputs to produce evidence associated with review units. They are not autonomous services and do not own transcript edits.

## Not Required for v0.1

VoxProof v0.1 does not require:

- A generic event bus.
- A plugin registry.
- A dynamic DAG engine.
- An actor system.
- A distributed worker system.
- A model runtime supervisor.

## Non-Committed Extension Directions

Future work may explore these directions without making them current commitments:

- Speaker prior.
- Context reranking.
- Acoustic evidence.
- ASR confidence or N-best hypotheses.
- Forced alignment.
- Local model runtimes.
