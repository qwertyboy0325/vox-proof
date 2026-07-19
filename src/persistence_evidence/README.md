# Persistence Evidence Harness

Status: accepted MD-014 / MD-015 spike infrastructure

## Purpose

Shared, candidate-neutral foundation for the bounded persistence evidence spike under
accepted MD-014 and MD-015. This module is spike-only test/evidence infrastructure,
not production persistence.

## Candidate spike boundary

Real candidate adapters live behind the `persistence-spike` Cargo feature:

- `embedded-relational-sqlite-spike` — embedded transactional relational store
- `append-bundle-log-spike` — append-oriented bundle/log mechanism

Production modules must not depend on candidate code.

## Evidence execution

```bash
cargo test --features persistence-spike --test persistence_candidates
cargo run --features persistence-spike --bin persistence_evidence_run
```

Target declaration and candidate class design:

- `evidence/persistence/spike-v1/targets.json`
- `evidence/persistence/spike-v1/candidate-classes.json`

Generated run summaries are written under `evidence/persistence/<run-id>/`.

## Intentionally not implemented

- production persistence selection or implementation
- destructive historical GC in spike adapters
- hardware power-loss injection
- automatic mechanism selection
