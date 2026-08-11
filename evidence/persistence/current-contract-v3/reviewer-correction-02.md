# 01B Remote-Review Correction-02 Package

```yaml
base: d9cd9e3719ba834893b3de7fcc23710c41efacd7
status: remote_review_pending
primary_blocker: FR-01-hard-link
candidate_id: current-contract-append-authoritative-candidate
candidate_version: 01B-1
selection_status: none
01C_authorized: false
owner_acceptance: pending
```

Correction: `VP-GATE4-01B-REMOTE-REVIEW-CORRECTION-02-HARD-LINK-CONTAINMENT`

## Bounded change

The candidate remains `current-contract-append-authoritative-candidate` (`01B-1`)
behind `persistence-spike`. This correction closes FR-01 static hard-link
containment only. It adds no evidence run, generated evidence artifact,
candidate selection, Material Decision, production persistence integration,
migration, or Gate 7 work.

## Hard-link containment contract

- authority validation uses metadata from the exact opened `File` handle used for
  the operation; path-only prechecks remain for symlink refusal but are not
  sufficient alone
- opened authority leaves must be regular files with exactly one hard link
  (`nlink == 1` on Unix; `number_of_links == Some(1)` on Windows when available)
- when link count cannot be established (`None` on Windows metadata, or
  unsupported targets), the adapter fails closed with
  `authority-link-count-unavailable`
- stable error codes: `authority-leaf-not-regular`, `authority-leaf-hard-linked`,
  `authority-link-count-unavailable`
- validation applies before manifest parse/replace, canonical-log
  replay/append/truncate, and writer-lock acquisition
- manifest checkpoint temporaries are created with `create_new`, validated on the
  opened handle, synced, and the existing destination is validated before rename

## Threat-model boundary

This correction rejects static filesystem aliases already present when an
authority operation begins. It does not claim protection against active
same-privilege namespace replacement races after handle validation.

On Windows stable std, handle-backed link count is not yet available without
the `windows_by_handle` feature gate; authority operations fail closed with
`authority-link-count-unavailable` until stable API exposure or authorized
runtime evidence exists. macOS/Unix validation does not establish Windows
behavior.

## Regression coverage

`tests/persistence_append_authoritative_01b.rs` adds:

- shared canonical log with distinct writer locks across two session roots
  (read-only and writable fail closed; source log unchanged)
- hard-linked manifest reopen fail-closed before parse/repair
- hard-linked writer-lock writable reopen fail-closed before lock acceptance
- existing Correction-01 regressions remain (fresh reopen, child-abort takeover,
  committed-prefix recovery, duplication, symlink alias refusal, and related F1–F7
  coverage)

## Provenance

Correction-01 historical package remains at `d9cd9e3` in
`reviewer-correction-01.md`. Post-`d9cd9e3` hard-link containment claims belong
only in this Correction-02 package.
