# MD-018: Minimal Governed Reusable Influence Semantics

Status: accepted

Date: 2026-07-30

Accepted: 2026-07-30 per explicit owner authorization

Accepted commit: `6b9170c4692ecc6f6545c6025abf75480ee9148a`

Decision authority: Ezra

Classification: minimal exact-pair, project-scoped, explicitly promoted, revocable reusable influence semantics for future exact proposal generation

## Acceptance boundary

This Material Decision is **accepted** by the decision authority.

It records the minimal governed reusable-influence semantics for Gate 3.

Acceptance does **not**:

- authorize Gate 3 implementation;
- authorize Rust, test, Cargo, or application changes;
- select a persistence mechanism;
- establish v0.3;
- rename the Gate 3 roadmap label;
- establish a public Experience Pack or Language Pack format;
- accept MD-012 ontology, ranking, suppression, embeddings, ASR, or HumanRaised semantics.

Implementation still requires a separately authorized bounded work package.

“Reusable Influence Authority” is the bounded authority defined by this
decision. It does not mean knowledge truth, correction authority over future
material, automatic acceptance, or automatic canonical text modification.

## Context

MD-017 authorizes occurrence-specific Manual Replacement only. An effective
Manual Replacement may provide evidence for future reuse but does not
automatically gain cross-material effect.

Cross-material reuse expands both scope and allowed effect. That expansion
requires a separate explicit governance transition; it is not implied by
`ReviewLedgerEvent::DecisionRecorded`.

Existing `SessionTermEntry` infrastructure cannot be reused naively because:

- it lacks reusable-record, scope, and promotion provenance;
- the canonical pipeline applies phonetic detection to canonical terms;
- its identity hashes flattened strings rather than governed record identity.

Gate 3 must define the smallest governed reusable-record boundary before
persistence, ASR, broad Experience Packs, or related-material evidence.

VP-ARCH-002 is non-authoritative research. It may explain scope/effect
transition reasoning. It does not authorize this decision and does not claim
architectural novelty for this decision.

## Terminology

```text
ExactReusableCorrection
ReuseCandidate
PromotionAccepted
PromotionCandidateRejected
ReusableInfluenceRevoked
ReusableInfluenceSuperseded
SourceDecisionLocator
ProjectScope
ReusableInfluenceSnapshot
```

These are conceptual semantic terms. This decision does not freeze Rust enum
names or concrete type layouts.

## Decision

### 1. Exact correction-pair payload only

The first reusable payload is exactly:

```yaml
ExactReusableCorrection:
  observed_text: exact selected source bytes
  confirmed_replacement: exact accepted ManualReplacement bytes
```

Payload validation must remain compatible with MD-017 exact replacement bytes.
No trimming, normalization, or semantic abstraction may be silently added.

The pair must not be interpreted as:

- `alias_of`;
- `observed_error_of` as a general semantic truth;
- `preferred_rendering_of`;
- entity identity;
- language truth;
- domain classification;
- creator terminology;
- generalized pattern;
- phonetic equivalence.

The reusable record only authorizes a bounded exact observed-form proposal
effect.

### 2. ReuseCandidate is a non-authoritative derivation

For every currently effective MD-017 Manual Replacement, the application may
deterministically derive one conceptual:

```yaml
ReuseCandidate:
  exact_payload:
  source_locator:
  proposed_scope:
  proposed_allowed_effects:
```

This derivation:

- may be automatic;
- is deterministic;
- carries no reusable authority;
- does not create an active record;
- does not alter future analysis;
- does not create a review decision;
- does not create negative memory;
- disappears from the effective candidate view if the source Manual Replacement
  is no longer the effective source decision, unless it was already promoted.

No extra human gesture is required merely to create the candidate.

### 3. Explicit promotion is mandatory

Reusable influence is created only through an explicit human promotion event.

Conceptual transition:

```text
effective ManualReplacement
→ deterministic ReuseCandidate
→ explicit human PromotionAccepted
→ active governed reusable record
```

Automatic promotion is forbidden.

Frequency, repetition, model confidence, or detector confidence are not
authority.

Promotion must be separate from `ReviewLedgerEvent::DecisionRecorded` because
occurrence correction authority and reusable-governance authority are different
histories and effects.

Direct reusable-rule creation without a source Manual Replacement is deferred.

Model-proposed promotion may be considered later only as a non-authoritative
proposal.

### 4. Source-decision locator

The current review ledger does not provide a durable global event identity.
This decision does not invent or imply that it does.

The first slice may use a structural, session-local source locator conceptually
equivalent to:

```yaml
SourceDecisionLocator:
  source_revision:
  source_analysis_snapshot:
  source_review_case_id:
  review_ledger_position:
  decision_digest:
  effective_at_ledger_length:
```

Its purpose is to locate and verify the exact decision occurrence that produced
the candidate within the current append-only session history.

Required limitation:

```yaml
current_attribution:
  scope: declared_application_session
  authenticated: false
  durable_global_event_identity: false
  independently_verifiable_actor_identity: false
```

A later persistence decision may establish durable event identity, but must not
reinterpret the accepted Gate 3 provenance semantics.

### 5. Project-only scope

The first accepted scope form is one explicit project scope:

```yaml
ProjectScope:
  stable_id: opaque_owner_declared_identifier
  display_name: mutable_non_semantic_label
```

Rules:

- scope identity and display name are distinct;
- display-name changes do not change semantic identity;
- no parent, child, or fallback hierarchy exists;
- no creator, domain, user, organization, or global scope exists;
- no implicit scope inheritance exists;
- scope widening or movement requires a new explicit promotion;
- a record may not expand its own scope;
- wrong-scope records exert no influence.

This decision does not define a general scope ontology.

### 6. Allowed effects

The first reusable authority permits only:

```yaml
allowed_effects:
  exact_observed_form_proposal_generation: allowed
```

It does not permit:

```yaml
phonetic_proposal_generation: forbidden
ranking: deferred
suppression: forbidden
automatic_text_change: forbidden
automatic_DecisionRecorded: forbidden
automatic_acceptance: forbidden
implicit_scope_inheritance: forbidden
```

The exact observed form must match according to the explicitly accepted exact
matching contract.

A reusable record may produce a non-binding candidate alternative for the
confirmed replacement.

A fresh human decision remains mandatory on every later material.

### 7. Exact-only analysis applicability

A future implementation contract must prevent an exact-only reusable record from
entering unauthorized detector paths.

A naive projection into current `SessionTermEntry` is insufficient because its
canonical term participates in the phonetic detector.

Required semantic result:

```text
Reusable exact pair
→ exact observed-form detector applicability only
→ no alias influence
→ no phonetic influence
→ no hidden ranking or suppression
```

This decision does not choose the final Rust shape.

It requires that analysis configuration, snapshot identity, and provenance make
the allowed detector effect explicit and replayable.

### 8. Reusable-governance history

Define a separate append-only conceptual event history.

Minimum event set:

```text
PromotionCandidateRejected
PromotionAccepted
ReusableInfluenceRevoked
ReusableInfluenceSuperseded
```

These are conceptual semantic events. This decision does not freeze Rust enum
names.

First-slice lifecycle decision:

```text
In the first slice, PromotionAccepted immediately makes the reusable influence
record accepted and active within its explicitly promoted project scope.

There is no separate accepted-but-inactive state in this decision.

Splitting acceptance from activation requires a later Material Decision.
```

First-slice lifecycle transitions:

```text
derived candidate
→ PromotionCandidateRejected

derived candidate
→ PromotionAccepted
→ accepted_and_active

accepted_and_active
→ ReusableInfluenceRevoked

accepted_and_active
→ ReusableInfluenceSuperseded
```

Rules:

- events are append-only;
- effective state is derived by deterministic fold;
- `PromotionAccepted` immediately yields `accepted_and_active`; there is no
  separate accepted-but-inactive state in this decision;
- splitting acceptance from activation requires a later Material Decision;
- destructive deletion is forbidden;
- revocation stops future influence;
- revocation does not erase historical provenance or past analysis snapshots;
- supersession creates or points to a distinct successor record;
- supersession must be explicit;
- no generic mutable lifecycle field is the canonical history.

Candidate rejection means only:

> This specific derived candidate is not promoted.

It does not create:

- negative memory;
- detector suppression;
- a claim that the observed text is correct;
- a global rejection of future similar candidates.

Candidate rejection history may prevent the exact same source-decision candidate
from repeatedly appearing for promotion.

### 9. Originating decision later changes

If the source Manual Replacement later ceases to be effective:

#### Unpromoted candidate

It disappears from the current derived candidate view.

#### Already promoted record

It remains active until explicitly revoked or superseded.

The system must expose that:

```text
source decision is no longer effective
```

as visible provenance or governance status.

It must not automatically revoke the reusable record.

Reason:

- occurrence correction authority and reusable influence were separately
  accepted;
- automatically coupling them would silently reinterpret the promotion event.

### 10. Conflict semantics

#### Identical pair

When multiple active records or base inputs contain:

```text
same observed_text
same confirmed_replacement
```

the deterministic projection may aggregate them into one effective matcher
entry only if all contributing provenance remains available.

The underlying authority records remain distinct.

#### Divergent pair

When active inputs contain:

```text
same observed_text
different confirmed_replacement
```

the system must produce a typed conflict and refuse effective reusable-input
projection before detector execution.

This rule applies to:

- reusable record versus reusable record;
- base session term versus reusable record;
- multiple source systems contributing to one analysis input.

Forbidden precedence:

- newer silently wins;
- narrower silently wins;
- base term silently wins;
- reusable record silently wins;
- insertion order wins;
- hidden numeric priority;
- partial projection that drops one conflicting mapping.

No complete future conflict-resolution workflow is accepted by this decision.

### 11. Reusable-influence snapshot

Future analysis must bind to an immutable deterministic reusable-influence
snapshot identity.

The identity must distinguish governed record sets even when their flattened
string values are identical.

The snapshot must preserve or bind:

- project scope identity;
- effective active record identities;
- exact payloads;
- governance-event revisions or equivalent effective-history boundary;
- revocation and supersession posture;
- deterministic ordering;
- projection version;
- provenance required to trace generated proposals.

Current `SessionTermsIdentity` alone is insufficient.

This decision does not select a persistence schema or serialized public format.

### 12. Proposal provenance

A Material B proposal produced from reusable influence must expose enough typed
provenance to identify:

- reusable influence record;
- active project scope;
- source promotion;
- source decision locator;
- reusable snapshot identity;
- exact applicability path;
- detector identity.

Detector provenance alone is insufficient.

The implementation may later decide whether this is represented as:

- a new typed evidence variant;
- an extended candidate provenance structure;
- a resource-provenance index linked from a candidate;
- another explicitly reviewed contract.

This decision owns the semantic requirement, not the concrete Rust
representation.

### 13. Projection and pack boundary

Use this model:

```text
governed reusable-governance history
→ deterministic effective reusable snapshot
→ target-specific non-authoritative adapter
```

Rules:

- an Experience Pack or Language Pack has no independent authority;
- a pack is a versioned projection or transport/resource envelope;
- import means resource availability only;
- import does not mean promotion;
- import does not mean acceptance;
- import does not mean activation;
- activation does not imply automatic correction;
- external modification cannot mutate accepted reusable history;
- external modifications may return only as new proposals;
- a stable public pack format is deferred.

This decision must not implement or freeze note-software, RAG, or
knowledge-graph semantics.

### 14. Broader downstream compatibility

The first record must preserve enough posture and provenance that future
consumers can distinguish:

```text
human_accepted
deterministic_derived
model_suggested
```

This preserves future compatibility with:

- terminology normalization;
- entity proposals;
- note-tag suggestions;
- domain-classification suggestions;
- RAG retrieval hints;
- knowledge-graph proposals.

However, the exact correction pair must not be reinterpreted as any of those
broader semantics without a later explicit decision.

Do not add fields or ontology solely for those future consumers.

### 15. Persistence dependency

The Gate 3 domain and same-process tests may remain in memory.

```yaml
same_process_semantic_evidence:
  permitted_before_Gate_4: true

restart_surviving_Material_A_to_B_evidence:
  requires_Gate_4: true
```

A meaningful tracked related-material demonstration that survives process exit
requires accepted Gate 4 persistence.

This decision must define semantic identity sufficiently for later persistence,
but must not:

- select a storage backend;
- select a serialization technology;
- define migration;
- claim durable actor authentication;
- weaken MD-014 or MD-015.

### 16. Export posture

Application export v2 meaning is frozen and must not be reinterpreted.

Gate 3 implementation must introduce a versioned successor or separate
versioned reusable-governance projection sufficient to represent:

- reuse governance events;
- effective reusable state;
- project scope;
- reusable snapshot identity;
- reusable proposal provenance;
- source-decision posture.

Preferred bounded direction:

```yaml
application_export_successor: v3
```

This decision may require a successor but does not freeze exact file names or
rendering syntax unless existing repository contracts require that level of
decision.

The result must not be called persistence or hydration.

## Required failure semantics

Fail-closed behavior is required for at least:

- invalid or missing source decision locator;
- source payload not matching the effective Manual Replacement;
- wrong project scope;
- divergent exact-pair conflicts;
- base-input versus reusable-input divergence;
- unauthorized detector applicability;
- missing reusable snapshot identity;
- missing required proposal provenance;
- revoked record appearing in a new effective snapshot;
- automatic decision or text modification attempt;
- replay mismatch;
- attempt to reinterpret export v2.

The system must not silently skip conflicting active records while claiming a
complete snapshot.

## Explicitly deferred

```text
semantic ontology
alias/preference/entity relations
context predicates
creator scope
domain scope
user or organization scope
global scope
scope hierarchy and fallback
automatic promotion
promotion by repetition
direct reusable-rule authoring
model-accepted promotion
ranking
suppression
negative memory
phonetic reusable influence
fuzzy reusable matching
embeddings
vector search
LLM inference
HumanRaised cases
ASR
public Experience Pack format
public Language Pack format
note-software adapters
RAG adapters
knowledge-graph adapters
persistence mechanism
authenticated actor identity
durable global event identity
multi-user synchronization
transport security
automatic canonical text changes
automatic future decisions
```

## Banned designs

This decision explicitly rejects:

1. Automatically promoting every Manual Replacement.
2. Treating repeated occurrence as authority.
3. Treating an exact pair as a generalized semantic relation.
4. Flattening reusable records into session terms while losing provenance.
5. Allowing exact-only records to participate in phonetic detection.
6. Letting reusable influence modify future text directly.
7. Creating future `DecisionRecorded` events automatically.
8. Using silent precedence for divergent mappings.
9. Treating pack import as acceptance or activation.
10. Treating a pack as canonical reusable authority.
11. Mutating or deleting prior governance history.
12. Automatically revoking a promoted record because its source decision later
    changes.
13. Treating candidate rejection as negative memory.
14. Reinterpreting application export v2.
15. Claiming persistence, authenticated attribution, or v0.3 establishment.

## Implementation consequences

A separately authorized Gate 3 implementation will likely require:

- exact reusable-pair domain type;
- deterministic candidate derivation;
- separate append-only reuse-governance events;
- project scope identity;
- structural source-decision locator;
- reusable-influence snapshot identity;
- typed applicability limiting influence to exact matching;
- resource/reuse proposal provenance;
- deterministic identical-pair aggregation;
- divergent-pair conflict refusal;
- application commands for accept, reject, revoke, and supersede;
- bounded GUI promotion and governance surfaces;
- export successor beyond frozen v2;
- same-process replay and deterministic tests.

This decision does **not** authorize those implementation changes. A separately
authorized bounded work package is required.

## Claims boundary

This Material Decision supports only claims equivalent to:

> VoxProof can explicitly promote a human-confirmed exact correction into a
> project-scoped, revocable reusable influence record that may generate
> provenance-visible exact-match proposals on later material while preserving a
> fresh human decision boundary.

It must not support claims that VoxProof:

- learns global truth;
- understands entities or domains;
- automatically corrects future material;
- performs semantic generalization;
- provides production persistence;
- supports authenticated multi-user governance;
- has established v0.3;
- has a stable public Experience Pack ecosystem.

## Relationship to prior decisions

```text
MD-002:
  owns ReviewCase and review-ledger decision semantics

MD-003:
  owns deterministic reviewed-output materialization

MD-017:
  owns exact Manual Replacement on existing detector-raised ReviewCases

MD-014:
  owns accepted durability/recovery/retention requirements

MD-015:
  owns persistence mechanism evidence protocol

MD-012:
  remains proposed and non-authoritative;
  MD-018 does not accept its broader ontology, scope hierarchy, or knowledge model

VP-ARCH-002:
  remains non-authoritative research;
  it may explain the scope/effect transition reasoning but does not own semantics
```

MD-018 does not claim ownership over occurrence correction or materialization.

MD-002 remains authoritative for `ReviewCase`, append-only `DecisionRecorded`,
and last-decision-wins effective status.

MD-003 remains authoritative for source immutability, canonical materialization,
anchor applicability, and overlap refusal.

MD-017 remains authoritative for exact Manual Replacement payloads and remains
the sole occurrence-correction authority for that slice.

MD-014 and MD-015 remain authoritative for durability requirements and the
persistence evidence protocol. This decision does not weaken them.

## Consequences

- Gate 3 has an accepted semantic boundary for minimal reusable influence;
- implementation still requires a separately authorized bounded work package;
- MD-012 remains unaccepted;
- persistence, public packs, ASR, ranking, suppression, and ontology remain
  deferred;
- application export v2 remains frozen; a versioned successor projection is
  required for reusable-governance representation.
