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
aid and does not establish Gate 6. On 2026-09-07, the owner authorized the
product-direction clarification that review interaction should reduce
repetitive work without weakening occurrence-attributable authority; grouped
review is the leading next candidate, while project automation remains
evidence-gated and unaccepted. On 2026-10-02, the owner authorized a
direction adjustment that orders product value as detection value first,
names accountable graduated automation as the target differentiator without
changing its evidence gates, and records high-accountability verbatim domains,
agent-facing proposal intake, and verifiable review receipts as directions.
In the same session the owner added education (lectures, with student,
teacher, and school roles) as a candidate wedge, together with
speaker-scoped inspectable learning and a verbatim-plus-meaning layering
direction.

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

## Review Efficiency Direction

VoxProof's product value is not that a user must perform one separate UI action
for every reusable proposal. The intended value is **less repetitive review
without surrendering control over any output-changing occurrence**.

The leading next product candidate is a bounded grouped-review surface for a
frozen, homogeneous proposal set:

```text
fully inspectable occurrences with the same bounded proposal
→ user keeps or removes exceptions
→ one grouped human gesture (interaction only)
→ expansion into independently attributable occurrence decisions
  under the applicable accepted decision contract
→ reviewed output determined by those decisions, not by the gesture
```

The interaction may be grouped, but authority must not collapse into one
opaque document-level acceptance. Every occurrence that changes reviewed
output must remain attributable and, where already accepted for that proposal
family, reconstructable from the persisted thin target plus ReviewLedger under
the applicable decision contract. Exact group eligibility, acknowledgement
and transaction behavior remain implementation and governance questions; this
direction does not authorize a batch command or new persistence semantics.

Per-occurrence review remains the fallback for ambiguous, conflicting, stale,
or context-dependent proposals. A user may also leave a reusable proposal
unacted; that path retains source text and creates no authority to change the
output. The product should distinguish such retained-source proposals from
accepted, rejected, deferred, or otherwise resolved work rather than implying
that every visible proposal must be accepted or separately clicked.

Project-scoped automation policy is deferred until grouped and per-occurrence
review are compared on real related materials with quality, provenance and
rubber-stamp oracles. Any future policy that can authorize output changes
requires a separate Material Decision and occurrence-attributable decision
provenance. Silent rewriting remains outside the product direction.

This is owner-authorized product direction, not validation evidence. External
HITL, automation-bias and CAT background may motivate reducing rubber-stamp
per-item review; it is not VoxProof product evidence. Sparse local owner
calibration is non-repository and non-canonical: the first-minute PACKER-01
pass used no Project Memory, did not measure grouped review or cross-material
exact-pair repeat rates, and observed ASR surface-form variance compatible with
exact-pair recall limits. It does not justify automation. Review-time savings,
safe grouping, reusable-asset value and product-market fit remain unvalidated
product-effectiveness hypotheses.

## Competitive Positioning and Value Order

On 2026-10-02, the owner authorized the following direction adjustment. It is
product direction, not validation evidence, a Material Decision, or
implementation authorization.

### Observed landscape (background, not product evidence)

AI transcription for interviews and research is crowded and increasingly
bundled with automatic analysis whose message is that users need not read
transcripts line by line. Separately, human-in-the-loop practice for AI agents
is converging on risk-tiered approval: low-risk, well-supported actions proceed
under an auditable policy, uncertain ones escalate to a person, and autonomy
is widened only after demonstrated reliability. Both observations are external
background and do not establish VoxProof product value.

### Value order

Users are expected to perceive value in this order:

```text
1. detection value
   — VoxProof finds consequential errors the user would otherwise miss
2. accountable review efficiency
   — grouped and per-occurrence review with attributable decisions
3. governed reuse
   — human-confirmed corrections improve later related material
4. accountable graduated automation
   — policy-authorized changes that remain attributable and revocable
```

Authority semantics are the foundation of every step, but they are not the
first value a user sees. Detection value therefore takes priority over further
authority-mechanism depth when the two compete for near-term effort.

High-recall non-authoritative proposal production, including model-based
proposal producers, is the leading direction for detection value. Its
architecture question remains owned by
[VP-ARCH-001](../research/VP-ARCH-001-loose-inference-strict-commitment.md);
this direction does not resolve that research, accept a belief layer, or
change ReviewCase, candidate, or persistence semantics.

### Target differentiator

The intended long-term differentiator is **accountable graduated automation**:

> Where other tools are either fully automatic or fully manual, VoxProof aims
> to let users automate only what has earned it, with every automatic change
> attributable to an explicit policy authorization and revocable from
> immutable source.

This changes the strategic standing of automation from a deferred risk to a
target capability. It does not change its gates. The evidence sequence,
transformation-class direction, and authorization layering remain owned by
[`correction-system-boundaries.md`](correction-system-boundaries.md), and any
policy that can authorize output changes still requires a separate Material
Decision. Silent rewriting remains outside the product direction.

### Wedge direction

The first commercial wedge should favour workflows where a wrong word carries
accountability cost and where local processing is a requirement rather than a
preference. The interview-review hypothesis and adjacent high-accountability
verbatim domains are recorded in [`hypotheses.md`](hypotheses.md); none is an
accepted market commitment.

Education is a second candidate wedge with an owner-accessible validation
path: the owner is a student with access to students, teachers, and a school.
A semester of lectures repeats one speaker and one domain, which favours
scoped reuse more than one-off material. Interview and education are both
candidates; neither is selected until validation evidence compares them.

### Speaker-scoped, inspectable learning

Users who repeatedly hear the same speaker need recognition that stays
consistent for that speaker's accent, verbal habits, and coined terms. The
intended value is not that a model learns, but that:

> the same mistake does not recur, and the user can see, adjust, and revoke
> what was learned, within an explicit scope such as a speaker or a course.

Learned knowledge stays scoped and attributable to the human decisions it came
from. Cheaper reuse layers — vocabulary biasing and post-recognition
correction proposals — come before acoustic model adaptation, which remains a
later, separately governed option. A learned profile or adapted model is an
inference producer, never authority.

### Verbatim plus meaning

The speaker's original words are preserved as the authoritative layer. Verbal
habits and coined terms are speaker characteristics, not errors. Meaning is
added beside the verbatim record, never in place of it:

```text
verbatim record (authoritative, human-reviewed)
→ meaning gloss (same-language explanation of dialect, habits, coined terms)
→ translation (cross-language)
```

AI-inferred meaning must remain visibly distinct from human-confirmed meaning.
How glosses and translations are represented, and whether they change
projection or persistence semantics, is unresolved and requires owner
decisions.

### Education roles and scoped authority

In education, authority follows scope:

```text
student correction        → affects only that student
→ proposed to the teacher → teacher confirmation
→ course knowledge        → applies to the course
school                    → privacy, deployment, and recording policy
```

A student correction never becomes course knowledge without explicit teacher
confirmation. Recording requires school and teacher authorization under
MD-024; a student edition should consume authorized course recordings rather
than encourage unilateral capture. Local processing keeps the marginal cost of
a student user near zero, which makes a free or low-cost student edition
plausible. Suggested sequencing is teacher-side course knowledge first, the
student edition second, and school packaging third.

A personal knowledge map built from a student's unfamiliarity signals is a
separate downstream product that consumes reviewed records; it is not a
summary feature of VoxProof. Live classroom mode remains deferred.

The hypotheses, boundaries, validation shape, and falsifiers for this
direction are owned by [`hypotheses.md`](hypotheses.md).

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

The final step is an unvalidated product hypothesis. Current implementation
and evidence establish governed proposal and decision mechanisms, not a
general reduction in review time or effort.

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

Two adjacent directions were added on 2026-10-02:

- **Agent-facing proposal intake** — external AI agents (for example through
  MCP) submit correction proposals into VoxProof, and VoxProof remains the
  governed commitment boundary. Agent output is inference, never authority.
- **Verifiable review receipts** — exports state which spans were reviewed,
  which remain unresolved, and on what recorded basis, in a form a downstream
  reader can check. No receipt format, signing scheme, or provenance standard
  is selected.

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
