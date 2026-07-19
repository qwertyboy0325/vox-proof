# Persistence Evidence Harness Foundation

Status: pre-authorization implementation draft; experimental test/evidence
infrastructure; not production persistence.

## Purpose

This module provides the shared, candidate-neutral foundation for the bounded
persistence evidence spike proposed by MD-015. MD-014 owns durability,
integrity, recovery, lifecycle, and retention requirements. MD-015 defines how
candidate mechanisms must be evaluated against those requirements.

MD-015 remains proposed. This module is not governance-authorized evidence,
does not record an official candidate run, and is not authorized for production
use. Its public Rust API is experimental and may change without compatibility
guarantees; production modules must not depend on it.

The module does not select or implement a persistence mechanism. Passing its
oracle or harness scenarios does not approve a backend, dependency, schema, or
production adapter.

## Boundary

- `fixture` builds deterministic semantic workloads.
- `model` contains spike-only normalized state and evidence result models.
- `oracle` validates logical identity, HumanRaised creation history, correction
  history, retention reachability, references, and authority.
- `adapter` defines semantic operations future candidate prototypes implement.
- `scenario` owns stable, versioned scenario identities.
- `runner` provides a minimal baseline entry point and non-authoritative
  eligibility aggregation.

Production application and domain modules do not depend on this module.
Candidate prototypes may depend on this module during the spike.

## Fixture versioning

Fixtures have stable IDs and explicit versions. A semantic workload change
requires a fixture version increment. The Small fixture is implemented;
Medium and Stress are declared scales for later deterministic expansion.
Fixture truth never uses current time, random identifiers, usernames, host
paths, or machine-dependent values.

The fixture uses accepted domain code where that code exists:

- transcripts produce real `TranscriptRevisionId` values;
- detector-raised cases are constructed through the current review pipeline;
- analysis identities are derived from current `AnalysisRun` snapshots.

Proposed v0.2 concepts not implemented by the accepted domain core remain
spike-only normalized records. They are not final product data contracts.
Spike-local identity wrappers are test-only and do not define production ID
formats.

Current explicit test/evidence versions are:

- Small fixture: `SMALL_FIXTURE_VERSION`;
- semantic oracle: `ORACLE_VERSION`;
- harness: `HARNESS_VERSION`;
- scenario catalog: `SCENARIO_CATALOG_VERSION`.

These constants identify this draft's test/evidence behavior. They do not imply
semantic-versioning stability.

## Oracle semantics

The oracle compares normalized domain meaning, not physical storage:

- canonical ledger order is preserved;
- unordered collections are sorted by stable logical identity;
- authoritative text payloads are exact and untruncated;
- HumanRaised case creation remains distinct from correction decisions;
- ledger sequence, bindings, payload, and provenance are canonical;
- withdrawal, supersession, and recovery provenance remain visible;
- retention roots, target artifacts, and relation types remain explicit;
- derived and temporary artifacts may differ without changing canonical truth;
- physical filenames, pages, offsets, locks, and transaction identifiers are
  excluded.

The diagnostic fingerprint is secondary evidence only. Structured semantic
comparison determines pass/fail.

## Candidate adapters

Future spike candidates implement `PersistenceCandidateAdapter` using
independently addressable semantic session references and open handles.
Authoritative commands carry scoped semantic preconditions, allowing stale
ReviewLedger, active-analysis, and analysis-attachment behavior to be tested
without prescribing one physical lock or transaction design. Duplication
returns a separately addressable session with explicit source lineage.

Unsupported optional compaction or destructive-GC capability is recorded as a
limitation. It is not automatically treated as correctness success or failure.

## Intentionally not implemented

- SQLite, JSONL, bundle, hybrid, or any other candidate;
- production session schemas or migration;
- OS-level crash or power-loss injection;
- benchmarks, compaction, GC, backup, encryption, telemetry, UI, or cloud sync;
- a production export or evidence-package serialization format.

No evidence spike has been executed by adding this foundation.
