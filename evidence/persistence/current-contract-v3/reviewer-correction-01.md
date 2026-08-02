# 01B Remote-Review Correction-01 Package

Status: remote review pending

Correction: `VP-GATE4-EVIDENCE-COMPLETION-01B-REMOTE-REVIEW-CORRECTION-01`

Base: `2459ffec41ef24e4813cf06a0723c190a85ad3b7`

## Bounded change

The candidate remains `current-contract-append-authoritative-candidate`
(`01B-1`) behind `persistence-spike`. This correction adds no evidence run,
generated evidence artifact, candidate selection, Material Decision, production
persistence integration, migration, or Gate 7 work.

## Corrected authority boundaries

- `open_existing` reconstructs the session address from only the candidate root
  and a validated session ID.
- writable opens use an OS-released exclusive file lock; a second live writer is
  rejected, while an aborted child process releases ownership for a fresh
  adapter takeover.
- replay records the last committed byte boundary. Writable opens and appends
  recover only the incomplete final tail by truncating to that boundary.
- every commit records the canonical fingerprint of its preceding state; replay
  refuses any mismatch before exposing state.
- writer handles are non-clonable and authoritative append/duplication require
  `&mut` access to the live writer guard.
- duplication replays the current committed source state before copying and
  derives a new independent evidence writer token.
- the equivalence contract names the 01B candidate directly.

## Regression coverage

`tests/persistence_append_authoritative_01b.rs` covers fresh adapter reopen,
live-writer refusal, child-abort takeover, committed-prefix recovery for state
and partial-record tails, retry after recovery, commit-binding tamper rejection,
mutable writer command serialization, current-contract duplicated-lineage oracle
comparison, read-only observation during an uncommitted tail, and concurrent
same-ID creation.

## Validation

```text
cargo test --workspace
cargo test --features persistence-spike --test persistence_append_authoritative_01b
cargo clippy --workspace --all-targets --all-features -- -D warnings
git diff --check
```

All commands passed. Global `cargo fmt --all -- --check` continues to report the
pre-existing unrelated formatting difference in
`tests/join_metric_aggregation_contract.rs`; changed Rust files were formatted
with scoped Rust 2024 formatting.

## Requested review decision

Confirm only whether Correction-01 closes the listed candidate-adapter defects.
Readiness remains `not_ready`, and owner acceptance remains pending.
