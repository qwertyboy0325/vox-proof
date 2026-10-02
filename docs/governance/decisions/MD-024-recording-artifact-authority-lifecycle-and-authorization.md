# MD-024: RecordingArtifact Authority, Lifecycle, and Capture Authorization

Status: accepted

Date: 2026-09-07

Accepted: 2026-09-07 per explicit owner authorization at
`VP-RECORDING-R1-DESIGN-GATE-01`

Decision authority: Ezra

Classification: pre-session recording-source authority, lifecycle, authorization,
recovery, retention, and downstream binding boundary

## Context

The owner-approved recording-first planning direction begins with a local
microphone recording rather than an existing transcript. The current product
authority begins later:

```text
imported transcript
→ ApplicationReviewSession
→ ReviewLedger
→ reviewed projection
```

MD-019 selects SQLite for product-session persistence. MD-020 defines the
product-session authority schema and lifecycle. MD-021 defines cross-material
Project Memory and project-bound session creation.

None of those decisions defines authority for media bytes that exist before a
review session. Gate 4 session evidence does not establish microphone capture,
incremental media writes, recording finalization, process-crash salvage,
filesystem-fault behavior, or recording deletion.

The desktop currently provides macOS AVPlayer playback only. It has no
microphone capture API, `RecordingArtifact`, capture state machine, recording
store, recovery manifest, or recording evidence.

This decision proposes the minimum semantic boundary required before a
contract-only `CaptureBackend` implementation may be considered. It does not
select a capture API, codec, container, storage backend, physical layout,
locking primitive, or ASR runtime.

## Relationship to existing authority

This decision must not weaken or reinterpret:

- MD-001 transcript revision identity;
- MD-002 ReviewCase and ReviewLedger authority;
- MD-003 source-preserving reviewed-output materialization;
- MD-014 durability, recovery, retention, and sensitive-content principles;
- MD-015 evidence-before-selection discipline;
- MD-016 eframe/egui single-process desktop boundary;
- MD-019 and MD-020 product-session SQLite authority;
- MD-021 immutable project binding and missing-Project-Memory behavior;
- MD-022 HumanRaised correction authority;
- MD-023 governed project terminology as derived analysis input.

Proposed MD-011, MD-012, and MD-013 remain non-authoritative. In particular,
this decision does not accept a generic analysis-job, belief, reconciliation,
or automatic-policy architecture.

## Decision

### 1. RecordingArtifact is a distinct pre-session source authority

```text
RecordingArtifact
  = authoritative local recording source payload
  + authoritative recording lifecycle and recovery metadata

RecordingArtifact != filesystem path
RecordingArtifact != review session
RecordingArtifact != TranscriptObservation
RecordingArtifact != ReviewLedger
RecordingArtifact != Project Memory
```

A recording may exist before any project or review session exists. Recording
capture therefore must not require or borrow `session_id`.

The recording source payload and the metadata needed to interpret its identity,
committed boundary, lifecycle, and recovery class form one conceptual
`RecordingRoot`. A later mechanism may place that authority in one physical
container or multiple coordinated physical objects. Neither payload bytes nor a
catalog entry alone may establish `Finalized`; open and recovery validate the
complete selected topology.

UI state, operational logs, ASR scratch data, and
`desktop-presentation.json` are not RecordingArtifact authority.

### 2. Identity is allocated before source bytes

Each recording has one product-generated opaque `recording_id`.

Required ordering:

```text
validate caller-declared recording authorization shape and basis
→ allocate recording_id
→ initialize a non-capturing RecordingRoot and durably bind authorization to recording_id
→ request OS permission and open the selected input device
→ initialize one exclusive writer within that RecordingRoot
→ begin capture
→ write first durable source byte
```

The identifier is allocated after the declaration is validated and before the
declaration becomes durable recording authorization. The authorization binding
must become durable before OS device access, live-writer creation, sample
admission, or source-byte writes.

OS permission denial or device-open failure creates no live writer and writes
no source bytes. The authorization-bound identity may remain only as a
non-capturing failed/withdrawn record subject to the explicit retention/delete
contract; it must not appear as a recording with captured material.

Filesystem path, display name, project name, device name, timestamp, and
`session_id` are not recording identity. Rename or move must not change
recording identity.

The final encoding and generation mechanism are deferred.

### 3. Exact content revisions are distinct from recording identity

`recording_id` identifies the continuing logical recording. A
`RecordingContentRevision` identifies one exact recoverable payload extent and
the format facts required to decode it.

A content revision must bind at least:

- `recording_id`;
- the exact committed payload boundary;
- the selected format/container interpretation revision;
- an integrity identity sufficient to reject mismatched bytes.

The hash or integrity algorithm, serialization, and public tag are deferred to
mechanism selection.

ASR or playback that claims a stable source input must consume one exact
content revision. `Finalized` produces a final content revision. A
`PartialRecoverable` recording may produce a salvage content revision only
after explicit validation and user selection; it is never silently relabeled
as complete.

Finalized source bytes are immutable. Any operation that would change source
bytes creates a new recording identity or an explicitly governed successor;
in-place reinterpretation is forbidden.

### 4. Per-recording capture authorization precedes the writer

Every recording requires a caller-declared material-use authorization bound to
its `recording_id`.

The allowed bases initially align semantically with existing local product
declarations:

```text
SelfOwned
ExplicitPermission
```

This is a caller assertion only. It does not independently prove consent,
ownership, contract validity, copyright status, publication rights, training
rights, or legal sufficiency.

Microphone/TCC permission and material-use authorization are separate:

- OS permission allows device access but grants no material-use authority;
- material-use authorization does not grant OS device access.

Required failure behavior:

- missing or denied authorization creates no live writer and records no source
  bytes; when validation fails before identity allocation, no RecordingRoot is
  created;
- withdrawal before capture starts is equivalent to denial;
- withdrawal after durable authorization binding but before first source byte
  prevents writer creation or immediately closes a writer that has admitted no
  samples, then classifies the authorization-bound root as `FailedClosed`;
- withdrawal after capture begins immediately stops admission of new samples,
  commits or reports the last safe boundary, classifies a valid committed
  prefix as `PartialRecoverable` or no safely interpretable committed source as
  `FailedClosed`, and enters an explicit retention/delete flow;
- withdrawal after bytes exist does not fabricate a claim that recording never
  occurred.

OS permission denial or device-open failure after durable authorization binding
classifies the non-capturing root as `FailedClosed`. No declared failure may
leave a root in `Creating`, `Capturing`, `Paused`, or `Finalizing` after sample
admission or writer activity has stopped.

A later review session still requires its existing
`ApplicationMaterialUseDeclaration`. Session material-use authority is not
silently inherited from the recording declaration.

### 5. Lifecycle states and legal transitions

The minimum lifecycle is:

```text
Creating
Capturing
Paused
Finalizing
Finalized
PartialRecoverable
FailedClosed
Tombstoned
```

Minimum transition rules:

```text
authorization accepted
→ Creating

Creating + writer initialized
→ Capturing

Creating + permission/device/writer failure or authorization withdrawal
→ FailedClosed

Capturing + durable pause boundary acknowledged
→ Paused

Paused + compatible explicit resume
→ Capturing

Capturing or Paused + stop requested
→ Finalizing

Finalizing + final boundary committed and independently reopenable
→ Finalized

Finalizing + finalization/fault failure + valid committed prefix
→ PartialRecoverable

Finalizing + finalization/fault failure + no safely interpretable committed source
→ FailedClosed

interruption + valid committed prefix
→ PartialRecoverable

interruption + no safely interpretable committed source
→ FailedClosed

Capturing or Paused + withdrawal/device/permission/disk/I/O failure + valid committed prefix
→ PartialRecoverable

Capturing or Paused + withdrawal/device/permission/disk/I/O failure + no safely interpretable committed source
→ FailedClosed

explicit deletion completed
→ Tombstoned
```

`Stop` is intent, not finalization. A backend callback, closed UI, elapsed
clock, or successful ASR operation cannot establish `Finalized`.

Creating a root, or selecting a relocation/publication/finalization
destination, must refuse to destroy, replace, reuse, or mutate any distinct
pre-existing destination or collision root regardless of lifecycle or recovery
class. The subject root may undergo only its validated legal lifecycle,
recovery, revision, or relocation transition; that transition cannot overwrite
another root's source bytes or authority metadata. A display-name or path
collision must be resolved by a distinct location/name or separately governed
presentation metadata on the subject root. A tombstone prevents
recording-identity reuse.

No state may transition from `FailedClosed` or `Tombstoned` to `Finalized`
without creating an explicitly governed successor recording.

### 6. Durable acknowledgement and watermark

Capture uses a monotonic committed watermark defined by the selected mechanism.
The watermark identifies the latest payload boundary that the mechanism claims
can be independently reopened after process interruption.

A durable capture acknowledgement is valid only after:

1. source payload through the boundary is committed according to the selected
   mechanism;
2. authoritative recovery metadata binds that same boundary;
3. an independent reopen path can validate and decode the committed extent.

These are not durability acknowledgements:

- audio samples delivered to a callback;
- bytes accepted by a userspace buffer;
- UI timer advancement;
- backend `flush` or `stop` naming without demonstrated semantics;
- catalog state committed before the corresponding payload boundary;
- payload growth without matching authoritative recovery metadata.

The exact commit unit and cadence are deferred to mechanism selection. The UI
may display capture activity separately from the last committed watermark but
must not label uncommitted activity as safely saved.

### 7. Recovery truth and classification

The selected RecordingRoot must provide a versioned, bounded, independently
readable recovery contract that can validate:

- recording identity;
- capture authorization reference and status;
- lifecycle state;
- format interpretation;
- committed watermark;
- content revision identity;
- writer ownership state;
- payload/recovery-metadata consistency;
- finalization or salvage classification.

Lifecycle state, active-operation ownership, and recovery classification are
distinct. A valid active root has authoritative ownership for its current
operation: creator in `Creating`, capture writer in `Capturing` or `Paused`, or
finalizer in `Finalizing`. A read-only open beside a validated active owner
reports the current lifecycle plus `ActiveOperationOwnershipVerified` and the
validated owner role; recovery classification is not applicable and no
takeover or mutation is permitted.

After process interruption, an explicit recovery open, abandoned/uncertain
active-operation ownership, or a normal open of a non-active root, exactly one
bounded recovery classification is reported:

```text
Finalized
PartialRecoverable
FailedClosed
Tombstoned
UnsupportedVersion
Corrupted
WriterOwnershipUncertain
```

Recovery must never expose an uncommitted tail as committed source or silently
mark a partial recording as `Finalized`.

During recovery classification, an authorization-bound root with no safely
interpretable committed source bytes is `FailedClosed`. It is not
`PartialRecoverable`, is not automatically `Tombstoned`, and is not silently
deleted.

Presentation metadata may be absent or corrupt without changing recording
authority. Authoritative recovery metadata may not be implemented as an
extension of the fail-soft desktop presentation sidecar.

### 8. Single-writer ownership

At most one live capture writer owns a RecordingRoot.

Required behavior:

- a second writer refuses; a reader beside a validated creator, capture writer,
  or finalizer may open only an explicitly non-mutating view that reports the
  active lifecycle and `ActiveOperationOwnershipVerified` owner role;
- writer ownership is verifiable against authoritative recording state;
- stale ownership is not cleared solely because a PID is absent;
- takeover requires integrity validation and recovery classification;
- while a validated creator, capture writer, or finalizer owns the root, only
  that owner may perform the lifecycle, recovery-metadata, revision, watermark,
  payload-commit, or finalization mutations authorized for its current state;
  every competing or non-owner mutation must refuse or defer;
- deletion and destructive cleanup must always refuse or defer during any
  validated active-owner role, regardless of caller; ASR, playback, export, and
  other non-mutating readers remain subject to the read-view contract above;
- deletion excludes new app-managed readers, then completes or cancels active
  playback, ASR, export, and decode readers before removing content;
- Finder and other external readers are outside the application's enforceable
  exclusion scope and must be disclosed rather than represented as closed.

The lock, lease, heartbeat, process, and takeover mechanisms are deferred.

### 9. Pause, device interruption, permission change, and sleep

Before pause acknowledgement, every sample admitted before the pause request
must be represented through the last valid committed watermark and independently
reopenable under the durable-acknowledgement contract. The watermark therefore
advances when admitted samples were pending; an idle pause with no pending
samples need not numerically advance it. Commit failure produces no pause
acknowledgement, stops sample admission, and classifies the root as
`PartialRecoverable` when a valid committed prefix exists or `FailedClosed`
otherwise. Callback receipt, backend `flush`/`stop` naming, and UI clock cannot
invent a boundary.

Resume may continue the same `recording_id` only when the selected mechanism
validates format and timeline compatibility and the user explicitly resumes.

The following must not be silently spliced into one apparently continuous
timeline:

- input-device replacement;
- permission revocation and restoration;
- an unclassified sleep/wake gap;
- incompatible sample format;
- backend restart with an unknown committed boundary.

When compatibility is not established, the current root becomes
`PartialRecoverable` or `FailedClosed`; continued capture requires a new
recording identity or an explicitly governed successor.

### 10. Fault behavior

On disk full or I/O failure:

- stop admitting new samples;
- preserve the last committed prefix;
- record a typed failure classification when authoritative metadata can still
  be committed safely;
- exit `Creating`, `Capturing`, `Paused`, or `Finalizing` to
  `PartialRecoverable` when a valid committed prefix exists or `FailedClosed`
  otherwise;
- never report the uncommitted tail as durable;
- never report `Finalized` solely because capture stopped.

On process crash:

- reopen through an independent reader;
- recover at least the last committed watermark when the mechanism claims
  `ProcessCrashRecovery`;
- classify any remaining tail explicitly.

A `ProcessCrashRecovery` scenario requires a declared real process
interruption, such as kill or forced termination, before independent
filesystem reopen from a new reader or process. Graceful
stop/finalize/reopen may support `LogicalStateTransition` only.

`ProcessCrashRecovery` does not establish `FilesystemDurability`.
`FilesystemDurability` does not establish `HardwarePowerLoss`.
A macOS result does not establish `CrossPlatform`.

### 11. Retention, deletion, tombstone, and provenance

Recording bytes are retained by default until explicit user deletion.

Deletion must:

- refuse or defer while a validated creator, capture writer, or finalizer owns
  the root;
- exclude new app-managed playback, ASR, export, decode, and inspection readers,
  then complete or cancel active readers before mutation;
- inventory every app-managed source-bearing copy, including encoded source,
  decoded PCM, sealed or unsealed chunks, ASR audio scratch, temporary exports,
  waveform/audio caches, and recovery copies;
- remove every inventoried app-managed source-bearing copy before reporting
  deletion complete;
- identify separately whether non-audio TranscriptObservation text,
  presentation metadata, and minimal provenance are retained;
- never rely on incomplete reachability to perform destructive cleanup;
- preserve no hidden copy of deleted audio in negative-evidence artifacts.

Deletion completion means verified logical removal from the declared
app-managed deletion scope. A tombstone may be committed only after removal
succeeds for that complete declared scope. It may retain only the minimum
non-content provenance required to prevent identity reuse and explain
unavailable historical media. It must not retain audio payload or transcript
surfaces under the label of negative evidence.

The product must disclose that deletion does not guarantee physical secure
erasure and cannot remove Finder copies, user backups, filesystem snapshots, or
other external copies outside the declared app-managed scope.

Deleting recording audio does not silently delete or rewrite:

- a separately committed review-session transcript source;
- ReviewLedger decisions;
- reviewed projections;
- Project Memory governance history.

Those authorities remain governed by their own decisions. The product must
surface that reviewed text or a non-audio TranscriptObservation may remain
while source audio is unavailable.

The exact tombstone fields, deletion transaction, retention duration, and
secure-erasure guarantee are deferred. No secure-erasure claim is accepted.

### 12. ASR is an isolated derived observer

An ASR attempt consumes one exact `RecordingContentRevision` and produces an
immutable, non-authoritative `TranscriptObservation`.

Every observation immutably binds the producing `recording_id` and exact
`RecordingContentRevision`, including committed extent and integrity identity.

ASR retry, cancellation, failure, model change, cleanup, or observation
deletion must not mutate, rename, truncate, finalize, or delete recording
source bytes.

ASR may run only against:

- a validated `Finalized` content revision; or
- an explicitly user-selected and validated `PartialRecoverable` salvage
  revision whose partial status remains visible.

This decision does not define an `AnalysisJob`, scheduler, model runtime,
provider registry, attachment lifecycle, or reconciliation semantics. Proposed
MD-013 remains non-authoritative.

### 13. Explicit later adoption creates session authority

Recording and ASR completion do not create a review session automatically.

A future adoption command must atomically create the review-session authority
from all required inputs:

```text
recording_id + exact RecordingContentRevision reference
+ selected immutable TranscriptObservation
+ product-generated project binding for the approved recording-first path
+ ApplicationMaterialUseDeclaration
+ DeclaredSessionAuthority
+ derived TranscriptRevisionId and embedded canonical parsed transcript
```

Atomicity here means the new session either commits all required session
authority and bindings or no session is created. It does not require a
two-store transaction that mutates RecordingArtifact.

Before committing session authority, adoption must independently reopen the
referenced content revision and validate its integrity identity and exact
committed extent. It must verify equality among the supplied `recording_id`,
the reopened revision's recording/revision/extent/integrity binding, and the
selected observation's immutable source binding. Any mismatch or validation
failure creates no session and does not mutate RecordingArtifact.

The recording remains independently addressable and unchanged if session
creation fails.

Capture remains project-independent. Adoption through the approved
recording-first sellable path requires one product-generated project identity;
project binding is set once at session creation and cannot be rebound. Unbound
sessions remain available only to separately governed non-recording-first
flows and cannot satisfy this adoption path. The persisted session must
contain enough adopted transcript authority to preserve reviewed-output
reconstruction if recording audio is later explicitly deleted.

The exact `TranscriptObservation` contract and session format revision remain
future Gate 5/6 decisions. This section fixes only the authority transition and
failure boundary.

### 14. Candidate topology eligibility

The following materially distinct classes may proceed to bounded pre-screening
or evidence design:

1. directory RecordingRoot with append media and atomic finalize publication;
2. sealed segment chain with authoritative index/manifest;
3. filesystem media with a dedicated recording catalog;
4. single transactional blob or container;
5. OS capture-file growth with app-owned identity and recovery metadata.

At least two materially distinct eligible classes must receive comparable
recording-specific evaluation unless a class is excluded through recorded
pre-screening against this decision.

Each candidate must demonstrate its own declared authoritative and recovery
model, including payload/recovery-metadata consistency for its physical
topology and its actual commit boundary. Passing shared `CaptureBackend`
InterfaceBehavior alone is not candidate-class conformance or comparison
readiness.

No class is selected or preferred by this decision.

The following are ineligible as RecordingArtifact authority:

- product `session.db`;
- Project Memory database;
- `desktop-presentation.json`;
- ASR output or scratch storage;
- a path or display name used as identity;
- an in-memory capture buffer without authoritative recovery metadata.

MD-019's SQLite selection for product sessions neither selects nor rejects a
recording topology.

### 15. Evidence and selection boundary

Before mechanism selection, a later recording-specific evidence package must
trace:

```text
claim
→ contract
→ candidate implementation
→ physical write/finalize operation
→ fault point
→ independent reopen/recovery
→ scenario assertion
→ oracle result
```

The evidence-strength ladder is explicit:

```text
InterfaceBehavior
→ LogicalStateTransition
→ ProcessCrashRecovery
→ optional FilesystemDurability
```

A contract-only fake backend may support at most `InterfaceBehavior`.
Golden-path start/stop/finalize plus independent reopen may support at most
`LogicalStateTransition`. `ProcessCrashRecovery` additionally requires the
declared real process-interruption fault described above.

The first required recovery ceiling is `ProcessCrashRecovery`.

`FilesystemDurability` requires dedicated ENOSPC/I/O evidence.
`HardwarePowerLoss` and `CrossPlatform` are outside the first selection claim
unless separately authorized and evidenced.

Gate 4 session persistence evidence cannot receive recording claim credit.

A recording scenario must declare before execution:

- `claim`;
- `pre_state`;
- `operation`;
- `fault_point`;
- `persisted_observation`;
- `reopen_or_recovery_step`;
- `expected_result`;
- `required_error_classification`;
- `forbidden_shortcuts`;
- `evidence_strength`.

Evidence packages preserve `Unsupported`, `NotRun`, `Inconclusive`,
expected-failure, and negative results. A zero denominator is
`Undefined`/`NotRun`, never PASS.

A later owner Material Decision must select the recording mechanism and
authoritative physical topology before durable product integration.

### 16. Privacy and local-first boundary

Recording payload, transcript observations, device details, paths, and project
associations may be sensitive.

Required behavior:

- default-redact source content, transcript surfaces, absolute paths, device
  labels, host/user identity, project associations, and authorization metadata
  from operational logs, diagnostics, crash reports, and evidence;
- treat content and identity digests as pseudonymous, not anonymous;
- include a normally redacted field only under a scoped, explicit diagnostic or
  evidence authorization with declared purpose, recipients, and retention;
- default-deny outbound transfer of recording payload, TranscriptObservation
  content, source-bearing derivatives, and sensitive recording metadata;
- permit a future outbound boundary only after a separate accepted remote-
  processing decision and explicit per-operation user disclosure and
  authorization; recording authorization is not inherited as remote-transfer
  authorization;
- request microphone permission only when capture is invoked;
- make actual local storage location visible without treating the path as
  identity;
- report at-rest encryption as absent unless a separate mechanism decision and
  evidence establish it;
- retain consent/authorization evidence as pseudonymous metadata or digests in
  repository evidence, never as unredacted private correspondence by default.

This decision does not establish legal compliance, encryption, secure erase,
telemetry, cloud sync, or remote model use.

## Invariants

1. No recording source byte is written before per-recording authorization.
2. Recording identity exists before the first source byte and is not a path or
   session identity.
3. RecordingArtifact authority is separate from ASR, review, and Project
   Memory authority.
4. Stop, callback receipt, and UI clock do not establish finalization or
   durability.
5. A committed watermark never includes an unvalidated tail.
6. Recovery never silently promotes partial source to Finalized.
7. At most one live writer owns a RecordingRoot.
8. Device, permission, sleep, or format discontinuities are not silently
   spliced.
9. ASR cannot mutate recording source.
10. Explicit audio deletion does not silently rewrite separate transcript or
    decision authority.
11. Negative evidence cannot retain deleted audio.
12. Session adoption is explicit and atomic within the new session authority.
13. Session SQLite and Gate 4 evidence do not select or prove recording
    storage.
14. Mechanism selection occurs only after recording-specific evidence and an
    owner Material Decision.
15. Display-name or path collision cannot overwrite an existing recording
    root.

## Banned designs

1. Using `session_id` as `recording_id`.
2. Requiring a review session before recording can begin.
3. Treating filesystem path or display name as source identity.
4. Treating microphone permission as material-use authorization.
5. Creating the live writer before authorization succeeds.
6. Treating `desktop-presentation.json` or UI state as recovery truth.
7. Marking a recording Finalized when only backend stop or close succeeded.
8. A catalog-only Finalized state that does not validate payload bytes.
9. Reconstructing missing source bytes from ASR output.
10. Allowing ASR retry or cleanup to rename, truncate, or delete recording
    source.
11. Clearing writer ownership solely because a PID is absent.
12. Silently switching devices or joining an unknown sleep gap.
13. Automatically deleting orphan or partial roots before recovery
    classification.
14. Retaining deleted audio inside evidence or diagnostics.
15. Embedding recording bytes into product `session.db` for convenience.
16. Selecting a recording backend from Gate 4 session evidence.
17. Claiming FilesystemDurability from process-kill tests.
18. Claiming HardwarePowerLoss or CrossPlatform without dedicated evidence.
19. Automatically creating transcript or correction authority from ASR output.
20. Implementing proposed MD-013 job semantics through this decision.
21. Overwriting an existing RecordingRoot because a display name or path
    collided.
22. Counting graceful stop/finalize/reopen as ProcessCrashRecovery.
23. Treating shared fake-backend or adapter behavior as candidate-class
    conformance.

## Explicitly deferred

- capture API and library;
- audio codec, sample format, and container;
- physical RecordingRoot layout;
- manifest/catalog serialization;
- digest or integrity algorithm;
- commit cadence and segment size;
- lock, lease, heartbeat, and takeover implementation;
- macOS TCC API and Info.plist text;
- final permission UX;
- ASR runtime, model, distribution, and scheduling;
- `TranscriptObservation` field-level contract;
- review-session format revision for adoption;
- exact delete/tombstone schema and secure-erasure semantics;
- encryption and key management;
- automatic backup;
- system-audio and meeting capture;
- speaker diarization;
- live transcript;
- markers/bookmarks;
- Windows/Linux capture;
- grouped-review implementation;
- production mechanism selection;
- evidence execution;
- canonical product-direction documentation changes.

## Consequences

If accepted:

- a separate R2 contract-only work package may define `CaptureBackend`,
  `FakeCaptureBackend`, typed lifecycle values, opaque handles, and contract
  tests;
- R2 may prove only `InterfaceBehavior`;
- macOS capture, microphone permissions, durable storage, fault evidence, and
  mechanism selection remain separately gated;
- later candidate implementations must satisfy the same recording-specific
  oracle and fault model;
- the owner must select a mechanism in a later Material Decision before durable
  product integration.

Acceptance would not authorize R2 implementation by itself. R2 still requires a
bounded owner-authorized work package.

## Implementation authorization boundary

This proposed decision contains no implementation authorization.

Implementation may begin only after:

1. owner acceptance of this Material Decision;
2. completion of required specialist and final-conflict review;
3. a separately authorized bounded work package for the next phase.
