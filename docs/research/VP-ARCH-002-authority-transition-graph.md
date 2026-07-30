Status: EVIDENCE_COLLECTED
Authority: NON_CANONICAL
Research ID: VP-ARCH-002
Owner: Ezra
Opened: 2026-07-29
Tracking issue: https://github.com/qwertyboy0325/vox-proof/issues/14
Owns: The evidence, falsification results, working terminology, limitations, and re-evaluation conditions for the authority-transition graph hypothesis.
Does not own: Canonical architecture, Material Decisions, Gate 3 semantics, Correction Memory implementation, persistence semantics, ASR semantics, version establishment, or public novelty claims.

# VP-ARCH-002: Authority-Transition Graph as a VoxProof Architecture Reasoning Model

## Research Boundary

This memo records completed non-authoritative architecture research. It does not
accept an architecture, alter product semantics, authorize implementation, or
rename a roadmap gate.

The working terminology in this memo is research vocabulary only:

- authority-transition graph;
- authority-expanding transition;
- non-authoritative observation or proposal;
- authority-bearing commitment;
- deterministic projection;
- authority-preserving continuity;
- reusable influence.

## Verdict

```yaml
hypothesis_verdict: partially_supported

preferred_model:
  strict_linear_stack: false
  directed_graph: true
  alternative_model: >
    A layered, event-sourced review application organized by a typed,
    provenance-constrained directed graph of observation, proposal,
    commitment, projection, validation and continuity transitions.
```

The graph is useful as an internal reasoning model. Current evidence does not
support adopting it as VoxProof's canonical or foundational architecture.

## Supported Conclusions

Repository evidence supports the following conclusions:

- observations and machine evidence do not directly authorize transcript
  edits;
- human intent becomes correction authority only through
  `ReviewLedgerEvent::DecisionRecorded`;
- effective case status is a deterministic fold over append-only decision
  history;
- reviewed output is a deterministic fail-closed projection;
- GUI drafts and presentation state remain non-authoritative;
- persistence must preserve rather than reinterpret accepted authority;
- cross-session reusable influence would require an explicit future promotion
  boundary.

## Unsupported Conclusions

Repository evidence does not support these conclusions:

- VoxProof is already canonically established as an authority-transition
  architecture;
- every authority event has durable actor attribution;
- authority concepts form a strict total ordering;
- export provides persistence, hydration, or session continuity;
- governed reusable influence is accepted or implemented;
- proposed MD-011, MD-012, or MD-013 is authoritative;
- the reasoning model is architecturally novel.

## Strongest Counterexample

`ReviewLedgerEvent::DecisionRecorded` currently records `case_id`,
`observed_revision`, and `decision`, but not durable event identity, actor
identity, authorization identity, timestamp, or durable sequence identity.

Therefore, only this weaker claim is supported:

> Within the current in-memory application session, decisions are associated
> with one caller-declared operator context.

The context is not authenticated, durable, or independently verifiable actor
attribution.

## Falsification Results

### Strict linear ordering

A strict linear model is falsified:

- ASR output is an observation rather than a higher authority level;
- review-object creation does not create correction authority;
- effective status derives authority but does not create a new commitment;
- projection does not create authority;
- persistence preserves authority rather than creating it;
- export is a representation, not a continuity mechanism.

A directed graph remains coherent only when its edges distinguish observation,
proposal creation, commitment, effective-state derivation, projection,
representation, continuity, promotion, and validation.

### Existing-boundary contradiction search

No accepted or implemented path was found in which a detector, GUI draft, or
export projection directly creates a canonical transcript edit. The principal
limitation is incomplete durable attribution rather than a hidden edit path.

### Predictive discrimination

The model gives bounded, non-circular discrimination for future reuse:

- automatic promotion from a Manual Replacement would expand its scope and
  effect without a new commitment;
- a derived candidate followed by explicit acceptance preserves the proposal
  versus commitment boundary;
- direct reusable-rule creation could be valid only as its own explicit,
  attributable, scope-bearing transition;
- a session-local cache remains derived state unless it becomes a declared
  behavior-affecting input;
- a learned model may produce observations or proposals but does not acquire
  edit authority;
- negative suppression would require its own explicit, visible, scoped, and
  revocable effect rather than being inferred from ordinary rejection.

## Concept Separation

Not every concept carries authority.

| Class | Meaning in this research |
| --- | --- |
| Source artifact | Supplied material from which validated source state may be constructed |
| Observation | An immutable, source-linked input that does not establish truth or correction authority |
| Machine evidence | Inspectable machine-produced reasons that may influence review |
| Proposal | A non-binding candidate for review or later promotion |
| Human testimony | An attributable human assertion that is not automatically a correction decision |
| Review object | A canonical unit that may receive a decision but does not itself authorize an edit |
| Authority-bearing event | An explicit accepted commitment recorded in canonical history |
| Effective authority state | A deterministic view derived from authority-bearing history |
| Deterministic projection | Rebuildable output derived from accepted inputs |
| Reusable influence record | A proposed future record allowed to influence later proposals within an accepted scope |
| Continuity envelope | A future durable representation that preserves authority without reinterpreting it |
| Transport envelope | A representation for moving or rendering data without independent semantic authority |
| Presentation state | Transient UI state with no canonical authority |
| Validation evidence | Evidence about behavior or claims, not correction authority |

Key classifications:

```yaml
ASR_output:
  class: observation
  authority: none

CandidateSpan:
  class: proposal_and_machine_evidence
  authority: none

ReviewCase:
  class: canonical_review_object
  correction_authority: none

DecisionRecorded:
  class: authority_bearing_event
  authority: case_scoped_human_decision_history

effective_ReviewCaseStatus:
  class: deterministic_effective_authority_state

reviewed_SRT:
  class: deterministic_projection
  independent_authority: none

ApplicationReviewExportBundle:
  class: projection_and_transport_envelope
  persistence_or_hydration_authority: none

GUI_draft:
  class: presentation_state
  authority: none

accepted_reusable_record:
  class: proposed_future_reusable_influence
  implementation_status: unaccepted

persistence_record:
  class: future_continuity_envelope
  authority_creation: none
```

## Architecture-Principle Candidate

Research hypothesis only:

> Components may not expand the scope or effect of their outputs without an
> explicit authorized transition. Authority-expanding transitions should be
> bounded, provenance-preserving, attributable at the level currently
> supported, and compatible with deterministic replay or verification.

This is not yet a canonical architecture rule:

- current durable attribution is incomplete;
- not every transition creates authority;
- canonical review-object creation is not necessarily correction authority;
- projection and persistence do not create new authority.

## Over-Governance Boundary

The framing is useful only for changes involving:

- canonical human or owner commitment;
- authority to affect reviewed output;
- cross-session reusable influence;
- scope expansion;
- promotion, revocation, or suppression;
- persistence reinterpretation;
- externally visible claim boundaries.

It does not require Material Decisions for:

- layout;
- navigation;
- keyboard shortcuts;
- transient GUI state;
- caches that do not alter semantics;
- implementation-only refactors;
- detector performance tuning that preserves evidence meaning and authority;
- ordinary reversible adapter choices.

## Gate 3 Implication

```yaml
Gate_3_working_term: Reusable Influence Authority

meaning: >
  Bounded permission for a governed reusable record to influence future
  proposal generation or ordering inside an accepted scope.

does_not_mean:
  - global truth
  - automatic text change
  - automatic DecisionRecorded creation
  - automatic acceptance
  - implicit scope expansion
```

The term remains provisional until an owner Material Decision accepts Gate 3
semantics. This memo does not create those semantics.

## Re-evaluation Conditions

The model remains non-canonical until:

1. Gate 3 demonstrates a useful promotion and reuse boundary;
2. persistence demonstrates continuity without semantic reinterpretation;
3. actor-attribution limitations are resolved or explicitly accepted;
4. over-governance risks are re-evaluated;
5. no novelty claim is made without external comparative research.

## Next Action

Apply the model as a non-authoritative reasoning aid to Gate 3 and later
persistence. Reassess canonicalization only after those domains provide
implementation evidence.
