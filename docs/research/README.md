Status: current
Owns: Navigation, status, lifecycle, and discovery rules for active VoxProof research items.
Does not own: Canonical architecture, accepted product semantics, implementation authorization, or Material Decisions.

# VoxProof Research Register

This register exists to prevent unresolved design work from becoming ghost architecture.

Every active research item must have:

- one stable research ID;
- one canonical research memo;
- one open tracking issue;
- one explicit non-authoritative status;
- one next action or reopening condition;
- one terminal resolution record.

Research items are not canonical architecture and must not be implemented merely because they appear here.

## Mandatory Re-entry Rule

Before proposing or implementing work involving architecture, inference, candidate generation, review semantics, decisions, canonical projection, temporal state, conflict handling, supersession, revocation, expiry, or persistence, inspect this register.

If a proposed change implicitly resolves, assumes, or bypasses an open research question, stop and obtain an explicit owner decision. Do not silently choose an answer through implementation.

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

Only `ACCEPTED_AS_MATERIAL_DECISION` may alter canonical architecture semantics.

## Active Research

### VP-ARCH-002 — Authority-Transition Graph as a VoxProof Architecture Reasoning Model

```yaml
id: VP-ARCH-002
title: Authority-Transition Graph as a VoxProof Architecture Reasoning Model
status: EVIDENCE_COLLECTED
authority: NON_CANONICAL
owner: Ezra
opened: 2026-07-29
canonical_memo: ./VP-ARCH-002-authority-transition-graph.md
tracking_issue: https://github.com/qwertyboy0325/vox-proof/issues/14
research_question: >
  Does a typed authority-transition graph provide useful design discrimination
  across correction, reusable influence and continuity without becoming an
  over-governing or misleading foundational architecture claim?
next_action: >
  Apply the model to Gate 3 and later persistence, then reassess.
```

Blocks without separate owner authorization:

- canonicalizing the model;
- renaming the entire architecture;
- treating every transition as authority-bearing;
- adding durable actor identity;
- changing Gate 3 semantics;
- changing persistence semantics;
- creating a Material Decision from the hypothesis.

Does not block:

- Gate 3 read-only semantic research;
- existing accepted Gate 1 or Gate 2 behavior;
- bounded documentation corrections;
- implementation already authorized under accepted Material Decisions.

### VP-ARCH-001 — Loose Inference / Strict Commitment

```yaml
id: VP-ARCH-001
title: Loose Inference / Strict Commitment
status: PROPOSED_RESEARCH
authority: NON_CANONICAL
owner: Ezra
opened: 2026-07-26
canonical_memo: ./VP-ARCH-001-loose-inference-strict-commitment.md
tracking_issue: https://github.com/qwertyboy0325/vox-proof/issues/1
research_question: >
  Should VoxProof evolve from a bounded AI extraction pipeline into a
  high-recall probabilistic synthesis plus governed deterministic
  commitment pipeline?
next_action: VP-ARCH-LOOSE-INFERENCE-STRICT-COMMITMENT-INVESTIGATION-01
```

Blocks without separate owner authorization:

- introducing a generic belief layer;
- renaming `CandidateSpan`, `ReviewCase`, or existing domain concepts;
- changing canonical rebuild inputs;
- adding temporal belief persistence;
- implementing policy auto-accept semantics;
- changing persistence semantics based on this hypothesis;
- creating a Material Decision for this direction.

Does not block:

- reviewing already-implemented commit `3234b4926c8ced90716f533f28a936f7f0863406`;
- work already accepted within the existing runner, detector-snapshot, authority, or persistence claim boundaries;
- read-only investigation of the research question.

## Resolution Rule

Do not delete resolved research memos.

A research item may leave the active list only after its memo and issue record one of:

- `PROMOTED_TO_MATERIAL_DECISION` with the Material Decision link;
- `REJECTED_WITH_RATIONALE`;
- `DEFERRED_UNTIL_<CONDITION>`;
- `SUPERSEDED_BY_<RESEARCH_ID_OR_DECISION_ID>`.

Historical or resolved items should move to a clearly labeled resolved section while retaining their original evidence and rationale.
