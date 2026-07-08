# MD-001: Stable TranscriptRevisionId

Status: accepted

Date: 2026-07-08

Decision authority: Ezra

## Context

`TranscriptRevisionId` is the identity used by source anchors, candidate keys, analysis snapshots, future review cases, future decisions, and future regression assets to refer to the reviewed transcript revision.

The current implementation uses `std::hash::DefaultHasher`, which is not a stable durable fingerprint across process, build, compiler version, or machine.

That is acceptable only while nothing durable is persisted.

It is not acceptable once VoxProof records review decisions, regression assets, project bundles, or any persisted review state.

## Decision

`TranscriptRevisionId` must be a stable, tagged content fingerprint over canonical parsed transcript content.

The v1 revision identity is:

```text
rev:sha256-v1:<hex-digest>
```

The digest is SHA-256 over canonical transcript bytes.

The algorithm tag is part of the identity format so future hash revisions can coexist with old records.

## Canonical transcript bytes

The canonical byte stream must be deterministic and independent of platform, process, compiler version, memory layout, or Rust `Hash` implementation.

The v1 canonical byte stream is:

```text
domain separator:
  voxproof-transcript-rev-v1

for each segment, in source order:
  cue index as u32 little-endian
  start_ms as u64 little-endian
  end_ms as u64 little-endian
  text byte length as u64 little-endian
  UTF-8 text bytes
```

Segment boundaries must be explicit.

Text must be encoded as UTF-8 bytes.

Length-prefixing is required so different field groupings cannot collide by concatenation.

## Included in the revision identity

The transcript revision includes:

* parsed cue index
* parsed start time in milliseconds
* parsed end time in milliseconds
* parsed text content
* segment order

Cue index is included because it is parsed source content, even though it may be duplicated or non-consecutive and separately reported as a validation issue.

## Excluded from the revision identity

The transcript revision excludes:

* parser version
* validation issues
* detector configuration
* analysis snapshot data
* source file path
* raw source file bytes
* operating-system file metadata
* line-ending differences already normalized by parsing

Parser version is excluded because `TranscriptRevisionId` identifies parsed transcript content, not parser implementation. If a parser change alters parsed content, the revision id changes because the content changes.

Validation issues are excluded because they are deterministic derivations of parsed transcript content under validation rules. If validation rules change but parsed content does not, the transcript revision identity should not change.

Raw file identity is a separate future concern.

## Related future identities

The following are explicitly deferred:

* `SourceFileFingerprint`
* `AnalysisSnapshotId`
* project bundle identity
* persisted run identity
* display-shortened revision ids

`TranscriptRevisionId` identifies parsed transcript content only.

`SourceFileFingerprint` may later identify imported raw file bytes when project bundles or import provenance require it.

`AnalysisSnapshotId` may later identify a source revision plus analyzer configuration, detector versions, language pack snapshots, and other analysis inputs.

## Dependency decision

A narrow dependency exception is accepted for a mature SHA-256 implementation.

The zero-dependency preference must not force VoxProof to hand-roll cryptographic hashing or keep a non-durable identity scheme.

## Implementation consequences

The implementation should replace the current `DefaultHasher`-based `TranscriptRevisionId`.

`TranscriptRevisionId` should no longer be a `u64`.

The revision id should be precomputed when a `Transcript` is constructed, rather than recomputed for every anchor creation.

This implementation should remain limited to transcript revision identity. It must not introduce persistence, project bundles, source-file fingerprints, analysis snapshot ids, or reviewed-output materialization.

## Banned designs

The following designs are rejected:

* using `std::hash::DefaultHasher` for durable transcript identity
* using compiler- or process-dependent hashing for durable records
* deriving durable review decisions directly from non-stable hash values
* hand-rolling SHA-256
* including parser version in transcript content identity
* including validation issues in transcript content identity
* using source file path as transcript identity
