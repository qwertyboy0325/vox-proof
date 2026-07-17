Status: current
Owns: Pre-1.0 version semantics — what a VoxProof version number means, the pre-1.0 version ladder, what it takes for a version to be established, and the claims each version allows and forbids.
Does not own: The scope of any individual version (owned by that version's scope document, e.g. `v0.1.md`), execution order, hypotheses, data contracts, or material decisions.
Last reviewed against code: Track 1 code closed loop exists; v0.1 is in progress and not established (see Current Version Status).

# VoxProof Pre-1.0 Version Semantics

Recorded 2026-07-10.

## What a pre-1.0 version is

Pre-1.0 VoxProof versions are validation milestones, not feature releases, not general software releases, and not product-readiness stages by themselves. Each version answers one falsifiable question. A version number is a claim about what has been proven, not about how much code exists.

Version numbers before 1.0 promise nothing about API, schema, CLI, or file-format stability.

## What "established" means

A version is established only when all of the following are defined and satisfied:

1. Target pain or validation question: the one question the version answers.
2. Minimum loop: the smallest end-to-end flow that can answer it.
3. Allowed inputs and outputs.
4. Non-goals for the version.
5. Measurable criteria, satisfied on evidence of the required class (see MD-007 and the evidence rules in `v0.1-execution-order.md`: validation evidence comes from real material and real human correction).
6. Allowed claims, stated in advance.

Until then the version is in progress. An in-progress version grants no claims.

Code completeness never establishes a version; evidence does. The standing example: the Track 1 code loop is complete and runs locally, exploratory real-material mechanism probes have exercised phonetic and calibration-correspondence paths, and qualifying owner-operated FLEURS human-review evidence under MD-007 D8 was recorded at repository HEAD `7efe8ba` with all ten MD-007 D9 gates passing; v0.1 remains unestablished because the mixed zh-EN fixture, release mechanics, and explicit establishment decision are still pending.

Engineering completion and evidence collection may proceed in parallel. A version does not require a separately completed error-distribution study before its bounded implementation work begins, but the implementation cannot establish the version until the required real-material evidence and measurable criteria are satisfied. "Build while measuring" changes execution order; it does not relax claim rules.

## Claim rules

- A version may claim only what its validation question answered.
- A version may never make a claim that belongs to a later version.
- Falsified criteria are a result, not an embarrassment: they redirect the version's scope before more is built on it.

## The pre-1.0 ladder

Versions are ordered by validation dependency, not by calendar. Skipping or reordering a rung requires an explicit note here. Re-scoping a version happens in this document plus the affected scope document; durable boundary changes still require Material Decisions.

### v0.1 — Mechanism validity

Question: can VoxProof complete the pain-specific evidence-backed review loop on real material?

Established when: the pain-point MVP loop (defined in `v0.1-execution-order.md`) runs end to end on real, self-owned or authorized material; qualifying human-correction evidence under MD-007 D8 is recorded; all ten frozen mechanism gates in MD-007 D9 pass; the mixed zh-EN fixture required by MD-007 D10 exists; release mechanics in MD-007 D11 are satisfied; outputs conform to MD-003; and an explicit establishment decision is recorded.

Allowed claim: "the reproducible correction mechanism with explicit human authority works on real material under the bounded v0.1 establishment scope defined in MD-007 D7."

Forbidden beyond MD-007 D7: product effectiveness, general ASR quality, precision/recall, time savings, external-user usability, or detector effectiveness beyond explicitly bounded mechanism evidence.

Forbidden claims: adoption, workflow value, time savings, Domain Collection, Language Pack, or Knowledge Pack value, product-market fit, or that anyone wants the product.

### v0.2 — External testability

Question: can a target-adjacent human understand and complete a bounded test session?

Established when: one consolidated facilitated session with a real target-cohort person (Lead A) completes, with the session metrics defined in `v0.1-execution-order.md` recorded. The conditional thin GUI and third-party readiness work belong to this version's establishment, not to v0.1.

Allowed claim: "a real target user completed the review flow and its outputs were legible to them."

Forbidden claims: value or time-saving claims, adoption beyond the observed session.

### v0.3 — Reuse / correction-memory signal

Question: do prior decisions or observed error forms improve future candidate quality?

Established when: measured across at least two related real materials. Any machine-readable correction-profile re-import or reusable correction/domain asset is decided here at the earliest, not in v0.1 or v0.2. Its eventual Domain Collection, Language Pack, Knowledge Pack, or other format remains unresolved.

Allowed claim: "recorded corrections measurably improve later review on related material."

Forbidden claims: general reusable-asset or pack product value.

### v0.4 — Workflow value signal

Question: does the process reduce review burden or improve confidence versus the user's current workflow?

Established when: a measured comparison against a real user's normal workflow on their real work exists.

Allowed claim: the measured value statement, scoped to the observed workflow.

Forbidden claims: market-level or cohort-level value.

### v0.5 — Self-serve prototype

Question: can someone outside the project run the flow with minimal operator help?

Established when: unassisted or lightly assisted completion is observed with a real external user.

Allowed claim: "the flow is operable without the project team."

Forbidden claims: production readiness, support commitments.

### v1.0 — Stable product promise

Not a validation milestone; a product commitment. A stable local-first, human-in-the-loop subtitle QA product with defined supported workflows, persistence boundaries, and user-facing guarantees. Requires all prior milestones and its own explicit decision process.

## What belongs to v0.1 versus later

- v0.1: diff study, deterministic detector ladder rungs as justified, CLI/operator review flow, Track 2 mechanism validation.
- v0.2: conditional thin GUI, third-party test readiness, the Lead A session.
- v0.3+: correction-profile re-import, reusable correction/domain assets, and reuse measurement; final pack format remains deferred.
- Not version-bound: a local non-LLM evidence model (ladder rung 3) is gated by residual-class evidence plus an accepted Material Decision, whenever that evidence appears.

Anything not assigned to a version by its scope document does not silently belong to v0.1.

## Current Version Status

- v0.1: in progress, not established. The code loop exists; qualifying owner-operated human-correction evidence under MD-007 D8 is recorded and all ten MD-007 D9 mechanism gates passed at repository HEAD `7efe8ba`; the mixed zh-EN pre-tag fixture, full isolated fmt/clippy/tests after that fixture, final gate matrix and release notes, explicit establishment Material Decision, final release audit, and annotated `v0.1.0` tag remain pending.
- v0.2 and later: not started.
