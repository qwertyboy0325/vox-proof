# Material Decisions

Status: active governance

Owns:
The process for recording accepted durable decisions that affect VoxProof product semantics, data contracts, privacy/security posture, identity model, persistence/serialization, review-decision semantics, or major architecture boundaries.

Does not own:
Exploratory ideas, discussion notes, implementation status, ordinary refactors, test-only changes, or daily development progress.

## Rule

A Material Decision is required before implementing a durable change to any of these boundaries:

- source identity or fingerprinting
- source anchoring semantics
- evidence semantics
- review-case identity or lifecycle
- correction-decision semantics
- reviewed-output derivation
- persistence or serialization formats
- project bundle layout
- privacy, data rights, consent, retention, or local-first guarantees
- speaker identity, voice clusters, or biometric-adjacent data
- audio/source coordinate systems
- major architecture boundaries

## Authority

Ezra is the decision authority.

AI tools, agents, reviewers, discussion documents, and possibility-queue entries may propose decisions, identify risks, and draft tradeoffs.

They do not approve Material Decisions.

## Non-authoritative inputs

The following documents may inform decisions but do not authorize implementation by themselves:

- discussion notes
- review notes
- possibility queue entries
- pending contracts
- speculative architecture sketches
- implementation suggestions from AI tools

In particular:

```text
Nothing in the Possibility Queue authorizes implementation.
Nothing in discussion notes authorizes implementation.
```

## Decision record location

Accepted Material Decisions are recorded under:

```text
docs/governance/decisions/
```

Each decision should use a stable identifier:

```text
MD-001
MD-002
MD-003
MD-008
```

Accepted v0.1 establishment decision: `decisions/MD-008-v0.1-core-mechanism-establishment.md`.

## Decision record shape

Each Material Decision should include:

* title
* status
* date
* amended date, if the accepted record was clarified after acceptance
* decision authority
* context
* decision
* consequences
* explicitly deferred questions
* banned or rejected designs, if any
* implementation notes, if useful

## Decision record amendments

Accepted Material Decisions should be stable records.

If a clarification is needed before implementation or durable artifacts exist, the accepted decision may be amended in place when the change preserves the same approved decision and records the amendment date.

If implementation, persisted data, regression assets, or other durable artifacts already depend on the accepted decision, a semantic change must be recorded as a new Material Decision or as an explicit superseding decision. It must not silently rewrite the accepted record.

## Implementation rule

Implementation may proceed only after the relevant Material Decision is recorded.

If a change touches a durable boundary and no accepted Material Decision exists, stop and record the decision first.

## Scope control

Material Decisions should be narrow.

They should decide the durable semantic boundary needed for upcoming implementation, not design the entire future system.

A good Material Decision prevents silent semantic drift without becoming a roadmap.
