# MD-006: Strict Skeleton Calibration Correspondence v0

Status: current
Classification: provisional v0 local calibration mechanism policy
Revisable from user/problem evidence

## Authorizes

This decision narrowly authorizes the following local calibration mechanism for v0:

- CLI command: `vox-proof evaluate`
- Report schema revision: `voxproof-calibration-correspondence-v0`
- Strict identical cue count, cue index, `start_ms`, and `end_ms` compatibility between raw and final SRT inputs at every `segment_position`
- Unicode-scalar Hirschberg LCS v1 local edit decomposition with published deterministic tie-break: raw midpoint, then smallest final split, then earliest base match
- Per changed cue work budget: `MAX_LCS_CELLS = 4_000_000`
- Neutral local-edit inventories and candidate/edit correspondence facts only
- Overlapping exact session-term occurrence inventory on final-side edit regions
- Precisely defined structural summary counts
- Tagged local provenance encoding for effective session terms: `session-terms:sha256-v1:<lowercase hex>`

## Does Not Authorize or Establish

This decision explicitly does **not** authorize or establish:

- canonical Evidence
- correction correctness taxonomy
- ground-truth certification
- precision, recall, unsafe-edit, or effectiveness claims
- numeric v0.1 gates
- persistence or re-import of calibration artifacts
- cross-version schema stability
- durable cross-run ReviewCase identity
- mismatched segmentation alignment
- decision reuse
- automatic decisions or materialization
- experimental retrieval/ranking promotion

## Session Terms Identity Encoding

Effective session-term entries are identified locally by deterministic SHA-256 over a fixed domain-separated byte sequence. The digest input is order-significant and uses the existing length-prefixed UTF-8 encoding implemented in `SessionTermsIdentity::from_entries`:

- domain separator `voxproof-session-terms-identity-v1`
- ordered entry count as a little-endian `u64` length prefix
- for each entry in order:
  - canonical term as length-prefixed UTF-8 bytes
  - alias count as a little-endian `u64` length prefix, then each alias as length-prefixed UTF-8 bytes in order
  - observed-error-form count as a little-endian `u64` length prefix, then each observed-error form as length-prefixed UTF-8 bytes in order

The tagged form renders the existing digest; it does not define JSON canonicalization:

```text
session-terms:sha256-v1:<lowercase hex>
```

This encoding is for local provenance within calibration reports only.

## Insertion Correspondence Boundary

Insertions retain zero-width raw byte positions, carry no `SourceAnchor`, create no correspondence record, create no overlap-graph edge, expose no `overlapping_case_ids`, and have overlap degree zero. Insertions may still carry exact final-side session-term scope facts.

## Policy Posture

This is provisional v0 local calibration mechanism policy. It may be revised when user or problem evidence requires a different bounded mechanism.
