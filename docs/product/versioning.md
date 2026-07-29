Status: current
Owns: Pre-1.0 version semantics — what a VoxProof version number means, the pre-1.0 version ladder, what it takes for a version to be established, and the claims each version allows and forbids.
Does not own: The scope of any individual version (owned by that version's scope document, e.g. `v0.1.md`), execution order, hypotheses, data contracts, or material decisions.
Last reviewed against code: v0.1 is established by MD-008; v0.2 is in progress; MD-016 accepts the native desktop foundation and the bounded Gate 1 implementation is owner-accepted at `0104ea2519cfd18b3ec639e5ec1566b734a6655e`; MD-017 accepts bounded Manual Replacement semantics and the current branch implements that Gate 2 slice for remote milestone review; external-user validation remains pending (see Current Version Status).

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

Code completeness never establishes a version; evidence does. The standing example: the Track 1 code loop is complete and runs locally, qualifying owner-operated FLEURS human-review evidence under MD-007 D8 was recorded at repository HEAD `7efe8ba` with all ten MD-007 D9 gates passing, the authorized mixed Traditional-Chinese / ASCII-Latin fixture required by MD-007 D10 exists at implementation baseline `05b7a2f`, final post-commit isolated validation passed at `05b7a2f`, historical tag-target validation passed at `cde7fd9`, gate matrix and release-notes draft are recorded in `product/v0.1.0-release-preparation.md`, and v0.1 is established by MD-008 as a bounded core mechanism only; local annotated tag pending recreation.

Engineering completion and evidence collection may proceed in parallel. A version does not require a separately completed error-distribution study before its bounded implementation work begins, but the implementation cannot establish the version until the required real-material evidence and measurable criteria are satisfied. "Build while measuring" changes execution order; it does not relax claim rules.

## Claim rules

- A version may claim only what its validation question answered.
- A version may never make a claim that belongs to a later version.
- Falsified criteria are a result, not an embarrassment: they redirect the version's scope before more is built on it.

## Capability versus establishment

A capability may appear in a version without being the evidence required to establish that version.

For v0.2, supporting capabilities may exist to make a credible external pilot possible — for example desktop delivery, local media review, durable sessions, human-raised cases, manual replacements, bounded Session knowledge, or bounded Project or Domain Collection knowledge. Their presence does not make reusable knowledge the primary v0.2 establishment claim, and none of them need to be accepted or implemented before the Lead A establishment criterion is satisfied.

For v0.3, reusable correction and domain knowledge assets themselves become a principal establishment target.

MD-016 selects eframe/egui `0.35.0`, a single native Rust process, and direct `ApplicationReviewSession` integration as the native desktop foundation. The bounded in-memory Gate 1 implementation is owner-accepted at `0104ea2519cfd18b3ec639e5ec1566b734a6655e`; that implementation acceptance does not establish v0.2. Built-in ASR may support the v0.2 external-testability workflow and is required by the current fundraising-demonstration delivery target, but its runtime, model, backend, and distribution mechanism remain unresolved. External SRT import remains a supported path.

The current durable execution order and the distinction between assisted-pilot and fundraising-demo readiness are owned by `v0.2-execution-order.md`. Delivery targets do not alter the validation questions or establishment evidence defined here.

## The pre-1.0 ladder

Versions are ordered by validation dependency, not by calendar. Skipping or reordering a rung requires an explicit note here. Re-scoping a version happens in this document plus the affected scope document; durable boundary changes still require Material Decisions.

### v0.1 — Mechanism validity

Question: can VoxProof complete the pain-specific evidence-backed review loop on real material?

Established when: the pain-point MVP loop (defined in `v0.1-execution-order.md`) runs end to end on real, self-owned or authorized material; qualifying human-correction evidence under MD-007 D8 is recorded; all ten frozen mechanism gates in MD-007 D9 pass; the mixed zh-EN fixture required by MD-007 D10 exists; release mechanics in MD-007 D11 are satisfied; outputs conform to MD-003; and an explicit establishment decision is recorded.

Allowed claim: "the reproducible correction mechanism with explicit human authority works on real material under the bounded v0.1 establishment scope defined in MD-007 D7."

Forbidden beyond MD-007 D7: product effectiveness, general ASR quality, precision/recall, time savings, external-user usability, or detector effectiveness beyond explicitly bounded mechanism evidence.

Forbidden claims: adoption, workflow value, time savings, Domain Collection, Language Pack, or Knowledge Pack value, product-market fit, or that anyone wants the product.

### v0.2 — External testability

Identity: v0.2 is the first externally testable, local-first review application.

Question: can a target-adjacent human understand and complete a credible Lead A review workflow?

Established when: one consolidated facilitated session with a real target-cohort person (Lead A) completes the complete review workflow, with the session metrics defined in `v0.1-execution-order.md` recorded. Establishment remains centered on proving that this workflow is externally usable and trustworthy. Third-party test readiness and any supporting pilot capabilities belong to this version's work, not to v0.1.

Supporting capabilities may appear in v0.2 when needed for a credible pilot, including desktop application delivery, local audio/video review, durable review sessions, human-raised review cases, manual replacements, bounded Session knowledge, and bounded Project or Domain Collection knowledge. None of these are the primary establishment criterion by themselves, and this document does not assert that any of them are already accepted or implemented.

Allowed claim: "a real target user completed the review flow and its outputs were legible to them."

Forbidden claims: value or time-saving claims; adoption beyond the observed session; mature cross-project knowledge portability; broad user-wide or global reusable knowledge; that reusable knowledge assets are the primary v0.2 product claim; automatic knowledge promotion or automatic correction.

### v0.3 — Reuse / correction-memory signal

Identity: v0.3 is the stage where reusable correction and domain knowledge assets become a first-class, measurable product capability.

Question: do prior decisions, governed knowledge, or observed error forms measurably improve future review on related material?

Established when: measured across at least two related real materials, with evidence that reusable correction or domain knowledge materially affects later review quality or burden. Machine-readable correction-profile re-import, reusable correction or domain asset import or export, and broader knowledge portability are principal establishment targets here, not in v0.2. Final Domain Collection, Language Pack, Knowledge Pack, or other formats remain unresolved until separately decided.

Scope may include measurable reuse across sessions; more mature promotion and conflict governance; knowledge portability; broader user-level scope; reusable-asset evaluation; import or export of governed knowledge; and stronger cross-project or cross-domain boundaries. These directions do not by themselves assert accepted implementation detail.

Allowed claim: "recorded corrections and governed reusable knowledge measurably improve later review on related material."

Forbidden claims: general reusable-asset or pack product value beyond the measured signal; global knowledge scope; automatic correction authority.

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
- v0.2: local-first external testability; Lead A session; third-party test readiness; supporting pilot capabilities such as desktop delivery, local media review, durable sessions, human-raised cases, manual replacements, and bounded Session or Project/Domain Collection knowledge when needed for a credible workflow.
- v0.3+: measurable reuse of correction and domain knowledge assets across related material; correction-profile re-import; more mature promotion, conflict governance, portability, user-level scope, and governed import or export; final pack format remains deferred.
- Not version-bound: a local non-LLM evidence model (ladder rung 3) is gated by residual-class evidence plus an accepted Material Decision, whenever that evidence appears.

Anything not assigned to a version by its scope document does not silently belong to v0.1.

## Current Version Status

- v0.1: established; local annotated tag pending recreation on the documentation-synchronized replacement tag target. The bounded core mechanism is established by MD-008 at implementation baseline `05b7a2f`; historical tag-target validation passed at MD-008 establishment commit `cde7fd9` (2026-07-18T04:09:43Z). A prior unpublished local annotated tag was deleted before push to synchronize canonical release-state documentation. Product and external-user validation remain pending beyond v0.1.
- v0.2: in progress. MD-016 has selected the native desktop foundation, and the bounded in-memory Gate 1 implementation is owner-accepted at `0104ea2519cfd18b3ec639e5ec1566b734a6655e`. MD-017 accepts bounded Manual Replacement semantics on existing detector-raised ReviewCases; the current branch implements that Gate 2 slice for remote milestone review. External-user validation remains pending. Neither capability establishes v0.2. Built-in ASR is a planned supporting capability and a requirement of the current fundraising-demo delivery target, not transcript authority or version-establishment evidence by itself.
- v0.3 and later: not established. Governed correction reuse remains a v0.3 validation target and is unproven. A fundraising demonstration may include v0.3-oriented reuse capabilities without establishing v0.3.
