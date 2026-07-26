Status: exploratory
Owns: The non-authoritative research hypothesis, open questions, evidence requirements, and decision boundary for VP-ARCH-001.
Does not own: Canonical architecture, accepted domain semantics, implementation scope, persistence contracts, or Material Decisions.
Issue: https://github.com/qwertyboy0325/vox-proof/issues/1

# VP-ARCH-001: Loose Inference, Strict Commitment

## Memory-Recovery Brief

### What this is

A research question about separating probabilistic synthesis from canonical commitment.

### What this is not

- Not accepted architecture.
- Not a Material Decision.
- Not implementation authorization.
- Not a data-model proposal.
- Not permission to rename `CandidateSpan`, `ReviewCase`, or other existing concepts.
- Not a reason to invalidate accepted Persistence Packages.
- Not evidence that an external memory system's behavior is a VoxProof requirement.

### Core hypothesis

VoxProof may currently bind inference rigor and commitment rigor too tightly.

The direction worth investigating is:

```text
source observations
→ high-recall / relatively unconstrained synthesis
→ non-authoritative candidate beliefs
→ strict authority boundary
→ accepted decisions
→ deterministic canonical projection
```

In shorthand:

```text
loose inference
strict commitment
```

### Current next action

```text
VP-ARCH-LOOSE-INFERENCE-STRICT-COMMITMENT-INVESTIGATION-01
```

That next action is a read-only architecture investigation. It is not authorized by this memo or by the linked issue alone.

### Do not proceed directly to

- belief-layer implementation;
- canonical-semantics changes;
- persistence-schema changes;
- `CandidateSpan` or `ReviewCase` renaming;
- policy auto-accept implementation;
- temporal-state materialization;
- a new Material Decision.

## Research Question

> Should VoxProof evolve from a bounded AI extraction pipeline into a high-recall probabilistic synthesis plus governed deterministic commitment pipeline?

## Motivation

The current pipeline direction can be summarized as:

```text
source
→ bounded extraction
→ bounded candidate
→ human review
→ canonical output
```

This protects provenance and authority, but may also constrain upstream reasoning more than canonical safety requires. A model that must avoid uncertain, cross-source, temporal, or competing hypotheses during candidate generation may lose recall and conflict-detection capability before any authority boundary is reached.

A more capable structure may be:

```text
source observations
→ high-recall synthesis
→ competing non-authoritative hypotheses
→ explicit authority transition
→ deterministic projection
```

The proposal is not to let model output become truth. It is to allow a wider epistemic search space while preserving a strict commitment boundary.

## Why This Is Relevant to VoxProof

Persistent-memory and continuous-state systems synthesize current operational state from distributed, historical, and sometimes contradictory observations. Typical questions include:

- which information remains valid;
- which information has expired;
- which source supersedes another;
- which preferences are durable;
- which observations conflict;
- which current state should guide later behavior.

VoxProof faces a structurally related problem:

```text
distributed observations
→ synthesis
→ conflict handling
→ current state
```

The crucial difference is authority:

```text
general memory systems:
probabilistic synthesis may directly affect later behavior

VoxProof:
probabilistic synthesis must not directly acquire canonical authority
```

The likely design center is therefore not strict reasoning everywhere. It is a strict transition from belief to accepted state.

## Existing Repo Alignment

The repository already contains partial separation between proposals, decisions, and projection:

- `CandidateAlternative` is a non-binding suggestion and cannot silently rewrite source text (`src/candidate.rs`).
- `ReviewCase` is distinct from `CandidateSpan` and intentionally carries no decision or history (`src/review.rs`).
- review decisions are recorded as append-only ledger events (`src/review.rs`).
- reviewed output is deterministically derived from the immutable transcript plus recorded accepted decisions (`src/reviewed_output.rs`).
- rejected, deferred, and manual-correction states do not rewrite the source projection (`src/reviewed_output.rs`).

This means VP-ARCH-001 does not begin from zero. The existing strict-commitment substrate may already be useful. The unresolved question is whether the upstream candidate model is too detector-local, single-span, single-run, or prematurely bounded for future needs.

## Proposed Research Vocabulary

The following names are conceptual only. They are not approved types.

### SourceObservation

An immutable source-linked observation, such as:

- transcript span;
- file content;
- metadata;
- event record;
- human statement;
- detector output;
- other evidence-bearing input.

A source observation should preserve what was observed, where it came from, and when it was observed. It should not be rewritten to match a later interpretation.

### SynthesizedBelief

A non-authoritative proposition synthesized from one or more observations.

A future belief model may need to be:

- source-linked;
- confidence-bearing;
- time-scoped;
- derivation-linked;
- conflict-aware;
- supersedable;
- expirable;
- explicitly non-canonical.

Conceptual sketch only:

```rust
struct SynthesizedBelief {
    subject: SubjectId,
    proposition: Proposition,
    evidence: Vec<EvidenceRef>,
    temporal_scope: TemporalScope,
    confidence: Confidence,
    derivation: DerivationRef,
    conflicts_with: Vec<BeliefId>,
    status: BeliefStatus,
}
```

This sketch is not a data-structure proposal and must not be implemented without separate acceptance.

### DecisionRecord

The explicit authority transition.

A decision record must answer questions such as:

- which proposition or belief revision was accepted, rejected, deferred, revoked, or superseded;
- which evidence supported the decision;
- who or which policy had authority;
- within which scope the decision is effective;
- during which temporal interval it applies;
- whether it supersedes an earlier decision;
- whether an owner gate was required.

A decision record must not merely point to a mutable current belief. It must freeze enough authority payload to make deterministic rebuild independent of later belief-store mutation.

### CanonicalProjection

A deterministic current or historical projection derived only from accepted authority inputs.

The required closure is conceptually:

```text
CanonicalProjection =
    Project(
        immutable source observations,
        accepted decision records,
        projection policy revision
    )
```

The following must not become hidden canonical inputs:

- model summaries;
- current-state caches;
- mutable belief status;
- unaccepted synthesis;
- latest-model interpretation;
- implicit conflict resolution.

## Central Architectural Principle

```text
confidence != authority
```

A high-confidence belief is still not a decision.

Likewise:

```text
conflict != supersession
expiry != revocation
source removal != projection deletion
invalidation != deletion
```

These distinctions must remain explicit if persistent state is introduced.

## Candidate Generation Implications

A future synthesis layer may be allowed to:

- combine non-adjacent evidence;
- reason over multiple source observations;
- propose multiple competing hypotheses;
- infer temporal state;
- detect contradictions;
- produce higher-level propositions;
- revise or supersede prior beliefs;
- retain uncertainty instead of forcing one answer.

This does not imply unrestricted interfaces or side effects. Even a relatively unconstrained reasoning process should remain bounded by:

- explicit input scope;
- source identity;
- output schema;
- evidence references;
- derivation identity;
- resource budget;
- non-authoritative status;
- no direct canonical writes.

## ReviewCase Relationship

`ReviewCase` should not be renamed or generalized yet.

A useful provisional distinction is:

```text
CandidateSpan
  = detector-local, reproducible finding

SynthesizedBelief
  = potentially multi-observation epistemic proposition

ReviewCase
  = an adjudication unit that may eventually reference one or more findings or beliefs
```

`ReviewCase` is primarily a workflow and authority concept. `SynthesizedBelief` would be an epistemic concept. They may interact without being identical.

## Temporal Semantics

Temporal meaning must not be implemented by mutating an earlier observation.

Example observation:

```text
The user will travel to Singapore in July.
```

After July, the source observation must not be rewritten into:

```text
The user travelled to Singapore in July.
```

The correct model is closer to:

```text
immutable observation
+ temporal inference
+ accepted decision where required
+ versioned current-state projection
```

Future investigation should distinguish at least:

- observation time;
- source-valid time;
- belief derivation time;
- decision effective time;
- projection time.

## Conflict and Supersession

Synthesis must not silently collapse conflict.

Example:

```text
A: The user prefers bounded work packages.
B: The user later accepts bounded and autonomous modes.
C: The current task explicitly requests an autonomous campaign.
```

A useful system should retain:

- the previous policy;
- the potentially superseding policy;
- current-instance selection;
- the conflict-resolution rule;
- provenance;
- effective scope.

It should not silently output only the latest convenient sentence.

## Deletion, Revocation, Expiry, and Regeneration

A persistent design may need distinct operations for:

```text
delete projection
invalidate evidence
revoke decision
expire belief
remove source
regenerate downstream materialization
```

A generic `deleted = true` flag is not sufficient evidence that these semantics can be represented correctly.

## Policy Auto-Accept Boundary

Policy auto-accept must not bypass the decision layer.

The acceptable form is:

```text
authorized policy
→ emits an explicit DecisionRecord
→ deterministic projection
```

The unacceptable form is:

```text
high-confidence belief
→ silently becomes canonical
```

Research must identify which decision classes could be policy-authorized and which require an explicit owner gate.

## Open Questions

1. What are the actual repo-local semantic boundaries of Candidate, ReviewCase, Decision, and Projection?
2. Where does the current design conflate inference correctness with authority correctness?
3. Do bounded candidate rules materially reduce recall, cross-evidence synthesis, temporal reasoning, or conflict detection?
4. Does `SynthesizedBelief` unify real lifecycle semantics, or merely wrap existing candidates with extra metadata?
5. Can the accepted persistence model represent temporal scope, conflicts, supersession, expiry, revocation, evidence invalidation, source removal, and downstream regeneration?
6. Which classes may be accepted by an authorized policy, and which require owner authority?
7. How can deterministic rebuild prove that probabilistic synthesis is not a hidden input?
8. What is the smallest vertical slice that can test the hypothesis without committing to a generic belief architecture?

## Required Read-Only Investigation Output

The first investigation should produce:

```text
existing semantic boundary map
→ inference/authority coupling findings
→ concrete failure cases
→ belief-layer necessity test
→ persistence capability matrix
→ policy auto-accept authority matrix
→ deterministic rebuild closure
→ one minimum vertical-slice recommendation
```

The investigation must be based on code, tests, accepted documents, and actual invariants. External systems may provide comparison material, but their behavior must not be imported as requirements.

## Minimum Vertical-Slice Criteria

A recommended experiment should be rejected if it requires immediate generic-domain refactoring.

A credible minimum slice should instead:

- use artificial observations;
- produce multiple source-linked competing beliefs;
- keep every belief non-authoritative;
- require an explicit decision artifact or authorized policy decision;
- rebuild one projection deterministically from source plus accepted decision;
- prove that changing unaccepted beliefs does not alter canonical output;
- exercise one temporal or supersession case;
- avoid modifying accepted Persistence Packages unless the investigation first proves a missing capability.

## Lifecycle

```text
PROPOSED_RESEARCH
→ INVESTIGATING
→ EVIDENCE_COLLECTED
→ OWNER_DECISION_PENDING
→ ACCEPTED_AS_MATERIAL_DECISION
  | REJECTED
  | DEFERRED
  | SUPERSEDED
```

Only `ACCEPTED_AS_MATERIAL_DECISION` may change canonical architecture semantics.

`EVIDENCE_COLLECTED` is not acceptance.

## Resolution Requirements

This memo must not be deleted when the item is resolved.

Resolution must record one of:

- promoted Material Decision and its link;
- rejection rationale;
- explicit deferral condition;
- superseding research or decision ID.

The linked GitHub issue must remain open until one of those outcomes is recorded.

## Current Project Effect

```yaml
canonical_semantics_changed: false
material_decision_created: false
repository_implementation_authorized: false
data_model_refactor_authorized: false
persistence_packages_affected: false
accepted_runner_or_snapshot_work_invalidated: false
```

The accepted detector snapshot, runner, authority, and persistence mechanisms remain valid within their established claim boundaries while this research item remains unresolved.