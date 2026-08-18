Status: current
Owns: Product-level strategic direction: the current wedge, long-term thesis,
product-level conceptual vocabulary, governed reusable-learning framing, and
the status of strategic horizons.
Does not own: Material Decisions, accepted architecture, data contracts,
implementation authorization, current execution order, GitHub tracker status,
or research conclusions.
Last reviewed against accepted direction: transcript verification remains the
current wedge; v0.2 is in progress; Gate 3 reusable influence is accepted;
Gate 4 product-session SQLite integration is owner-accepted at
`3e8ec0acd7a1e881bb70f3aa07b2dc910c2a4c5f`; MD-021, MD-022, and MD-023 accept
Project Memory, HumanRaised correction, and governed project-knowledge
derivation without establishing Gate 7 or v0.3; built-in ASR and
deterministic cross-material reuse demonstration remain unimplemented;
desktop media-assisted review playback exists as a non-authoritative review
aid and does not establish Gate 6.

# VoxProof Strategic Direction

## Role and Authority Boundary

This document records the current product direction without promoting
strategic ideas into architecture or implementation commitments.

```text
strategic direction
≠ Material Decision
≠ accepted architecture
≠ implementation authorization
```

It gives future collaborators a stable answer to two different questions:

- Why is the current transcript workflow intentionally narrow?
- What broader problem might the same authority-preserving approach address?

The detailed current execution order remains owned by
[`v0.2-execution-order.md`](v0.2-execution-order.md). Accepted durable
semantics remain owned by the applicable Material Decisions. Active research
remains non-authoritative under [`../research/README.md`](../research/README.md).

## Current Product Wedge

VoxProof's concrete product wedge is **local-first, human-governed transcript
verification**.

The practical workflow is:

```text
existing transcript / SRT
+ optional original-audio context
+ optional reusable domain or language knowledge
→ suspicious ReviewCases
→ bounded correction candidates and evidence
→ explicit human review
→ CorrectionDecision
→ append-only governed authority history
→ deterministic reviewed output
```

The optional inputs in this conceptual workflow do not assert that every input
or capability is implemented today. In particular, built-in ASR is a planned
non-authoritative observation producer and onboarding capability, not
transcript authority.

VoxProof is not:

- an ASR engine;
- an automatic transcript rewriter;
- an AI subtitle-fixer wrapper;
- a meeting-summary product;
- a cloud-first workflow; or
- a model showcase.

Transcript verification is the current domain wedge, not the architectural
ceiling of the product thesis.

## Long-Term Product Thesis

VoxProof's long-term thesis is:

> **VoxProof is an evidence, authority, and commitment layer between
> probabilistic AI and trusted formal state.**

A user-facing articulation is:

> **VoxProof helps people use AI without surrendering control over what becomes
> official.**

The central problem is not only whether an AI prediction is accurate. It is:

```text
When may a probabilistic result become official state,
on whose authority,
with what evidence,
and can that commitment later be audited and replayed?
```

This is a product thesis. It does not claim that a generic platform, graph
runtime, control plane, or execution-authority system already exists in this
repository.

## Product-Level Conceptual Vocabulary

The preferred long-term conceptual vocabulary is:

```text
Inference
→ Evidence
→ Authority
→ Commitment
→ Projection
```

This is a vocabulary and responsibility boundary, not a mandated runtime
pipeline, graph schema, data model, or implementation plan.

### Inference

Inference is probabilistic or otherwise non-authoritative computation. It may
include ASR, deterministic detectors, LLMs, retrieval, ranking, model routing,
or future local adaptation. Inference may propose; it does not establish
canonical truth.

### Evidence

Evidence supports, opposes, explains, or contextualizes a candidate. It may
include provenance and uncertainty, but it remains non-authoritative.

### Authority

Authority is the rule or actor permitted to decide whether a proposed semantic
change becomes formal state. Current VoxProof authority is intentionally
human-governed. Model confidence and UI state are not authority.

### Commitment

Commitment is the governed act that records an accepted decision. It must be
explicit enough to support the applicable replay and audit boundary. This
concept does not select a persistence mechanism; product-session SQLite
integration is owned by MD-019, MD-020, and Gate 4, not by this document.

### Projection

Projection is the deterministic current output derived from source material and
accepted commitments. It is rebuildable within its declared contract and does
not mutate or replace the original source.

The distinctions remain deliberate:

```text
source
≠ observation
≠ candidate
≠ evidence
≠ authority decision
≠ canonical projection

AI confidence
≠ authorization

UI state
≠ authority

model output
≠ canonical state

reviewed output
= deterministic projection of source + accepted decisions
```

In compact form:

```text
AI may infer.
AI may propose.
AI may rank.
AI may learn.

AI may not silently become authority.
```

## Governed Reusable Learning

Human correction is more than a convenience edit when it is preserved with its
source, scope, evidence, and explicit authority boundary. It can become
reusable verification knowledge without granting AI the authority to rewrite
future material.

The strategic learning loop is:

```text
source + context
→ ReviewCase
→ human accept / reject / replace
→ CorrectionDecision
→ governed reusable correction asset
→ regression or evaluation asset
→ improved future candidate
→ potentially less repeated human work
```

`Studio → Experience → Trainer → Trust` is useful shorthand for this strategic
framing. It is not an accepted subsystem decomposition.

Within the current product boundary, governed learning can include known
confusions, human-confirmed corrections, frozen examples, regression fixtures,
reusable evidence, deterministic evaluation, and project-scoped reusable
influence. It does not mean automatic model fine-tuning, silent self-training,
automatic canonical rewriting, AI promotion of its own proposals, voice-profile
authority, or autonomous policy mutation.

The accepted Gate 3 slice is deliberately narrower: an effective Manual
Replacement may produce a non-authoritative candidate, and only explicit human
promotion may create a project-scoped exact reusable influence. That influence
can produce a later non-binding exact-match proposal; a fresh human decision is
still required for later material. The accepted semantics and limits are owned
by [MD-018](../governance/decisions/MD-018-proposed-minimal-governed-reusable-influence.md).
Later accepted MD-021 Project Memory, MD-022 HumanRaised promotion, and MD-023
freeze-bound derived terminology remain non-authoritative inference inputs and
do not establish Gate 7, v0.3, or general reuse value.

Future model adaptation may be explored, but an adapted model remains an
inference producer rather than an authority source.

## Strategic Horizons

The horizons preserve important product knowledge without changing the active
roadmap or granting implementation authority.

### Horizon 1 — Active Product Direction

```yaml
status: active
implementation_relationship: current roadmap
```

This horizon includes the local-first transcript-verification wedge, governed
human review, Manual Replacement, durable session authority as a current
product need, governed reusable influence, local media-to-review workflow,
non-authoritative built-in ASR, related-material reuse demonstration, assisted
external pilot, and fundraising demonstration. Their detailed order, status,
and implementation boundaries remain owned by the current execution-order
document.

### Horizon 2 — Adjacent Strategic Opportunities

```yaml
status: exploratory
implementation_authority: false
```

Credible adjacent directions include richer evidence fusion, uncertainty and
abstention, model routing, local adaptation, domain-specific reusable
verification knowledge, Experience or Evidence assets, stronger
regression/evaluation loops, and multimodal evidence. They are not current
architecture mandates.

### Horizon 3 — Research and Platform Ceiling

```yaml
status: exploratory_research
binding: false
implementation_authority: false
```

This horizon includes Evidence Graph and Authority-Transition Graph ideas,
GNN-based evidence inference, offline non-authoritative "dreaming", an
Evidence Runtime or SDK, Control Plane concepts, Training/Experience/Evidence
Packs, marketplace concepts, governed agent mutation, and general
execution-authority infrastructure.

These ideas express a possible ceiling for the thesis. They are not active
roadmap work and must not be treated as accepted architecture merely because
they are recorded here.

### Specific Research Boundaries

An Evidence Graph may eventually help retain conflicting observations,
hypotheses, and supporting inputs without prematurely collapsing them into one
answer. A high score for one answer is still not an authority transition. No
graph schema or graph semantics is accepted here.

GNNs, other graph models, LLMs, local adaptation, and model-routing systems may
eventually improve evidence propagation, ranking, conflict detection, or
candidate generation. Their outputs remain inference infrastructure, not
authority. No GNN or training infrastructure is authorized by this document.

Offline local computation may someday replay governed decisions, generate
counterexamples, discover recurring patterns, evaluate candidate rules or
models, and propose reusable verification knowledge. Its outputs remain
proposal, evidence, or evaluation; they do not self-promote into authority.

The durable strategic abstraction for future reusable assets is **reusable
governed verification knowledge**. Possible contents include terminology,
known confusions, confirmed corrections, negative examples, evidence providers,
ranking hints, regression fixtures, and evaluator metadata. This does not
create pack formats, marketplace commitments, or separate accepted product
objects.

An adjacent high-ceiling research direction is execution authority
infrastructure for autonomous systems, also described as a governed mutation
substrate. Its general research principle is:

> No irreversible mutation without attributable authority and durable evidence.

That principle is retained as horizon research only. VoxProof must not be
repositioned as a generic agent platform, and this document does not change the
current transcript-focused roadmap.

## Near-Term Discipline and Demonstration Meaning

The active roadmap remains focused on proving the transcript wedge before any
platform expansion. This strategy does not insert an Evidence Graph, GNN,
marketplace, Control Plane, generic agent framework, or broad cloud platform
into that roadmap.

The fundraising delivery target is intended to show:

```text
local media
→ non-authoritative ASR observation
→ governed review
→ Manual Replacement
→ human-confirmed correction
→ governed Correction Memory
→ related material
→ useful reused proposal
→ reviewed export
```

The deeper thesis demonstrated is that human correction can become reusable
intelligence without surrendering canonical authority to AI. A coherent
demonstration alone does not establish product-market fit, production
readiness, general time savings, general learning effectiveness, v0.2, v0.3,
or fundraising success.

## Related Canonical Sources

- [`correction-system-boundaries.md`](correction-system-boundaries.md) owns
  correction, evidence, policy, transformation, projection, and automation
  boundaries.
- [`v0.2-execution-order.md`](v0.2-execution-order.md) owns the current
  execution order and gate status.
- [Material Decisions](../governance/material-decisions.md) own accepted
  durable decisions; this document does not create one.
- [`../research/README.md`](../research/README.md) owns active research
  discovery and non-authoritative research lifecycle.
- [`../README.md`](../README.md) owns documentation navigation and the
  repository knowledge-authority hierarchy.
