# 01B-2 Windows-Readiness Pre-Final Evidence Package

```yaml
work_package: VP-GATE4-01C-READINESS-WINDOWS-CLOSURE-01
accepted_base: 2229a36ff09fa56482673151a0ae010f3b9ec099
accepted_base_owner: Ezra
accepted_base_date: 2026-08-11
candidate_id: current-contract-append-authoritative-candidate
candidate_version: 01B-2
format_version: 1
status: windows_runtime_and_final_review_pending
01C_authorized: false
selection_status: none
product_persistence_integration: false
real_session_migration: false
```

## Bounded successor change

This successor preserves semantic `session_id`, the manifest and append-log
meaning, and persisted `format_version: 1`. It derives a deterministic,
cross-platform physical directory key from SHA-256 of the semantic ID. Existing
manifest identity validation fails closed if a physical location does not match
the requested semantic ID.

On Windows, authority-leaf hard-link count is obtained from the exact already
opened `File` handle through `GetFileInformationByHandle`; failure to query the
handle or a count other than one fails closed. Static alias checks reject
symlinks on every supported platform and Windows reparse points for session,
authority-leaf, temporary, and manifest-checkpoint paths before their relevant
authority operations. Active same-privilege namespace replacement remains out
of scope.

The existing writer-lock and checkpoint semantics are unchanged. The focused
test binary includes active Windows hard-link and junction fixtures, alongside
fresh-process reopen, abort takeover, committed-prefix recovery, checkpoint
repair, acknowledged append, and current-contract fixture round trips.

## Evidence boundary

This is a pre-final-review implementation package, not a Windows runtime result
or a Sol review. It records no 01C evidence execution, mechanism selection,
Material Decision, production persistence integration, migration, filesystem
durability, hardware power-loss resilience, or owner acceptance of `01B-2`.
