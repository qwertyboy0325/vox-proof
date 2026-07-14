Status: current
Owns: Architecture principles and the current fixed conceptual processing shape.
Does not own: Cross-version correction-system semantics (owned by `product/correction-system-boundaries.md`), product acceptance criteria, field-level schemas, storage paths, implementation tasks, or future orchestration commitments.
Last reviewed against code: Track 1 local code loop exists. v0.1 is not established; real-material validation remains pending.

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

## Cross-Version Responsibility Map

The correction-system responsibilities are summarized conceptually as:

```text
Immutable Source -> Analysis View

Evidence + Context + Policy
-> candidate assessment and review

Human or Explicit Authorization
-> Typed Transformation
-> Requested Projection Materialization
```

This diagram expresses ownership boundaries, not a required linear runtime pipeline. Policy may affect which analysis or suggestion classes run, while evidence, context, policy, decisions, transformations, and projections retain separate responsibilities. The canonical definitions and current-versus-future boundary are owned by `product/correction-system-boundaries.md`.

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
