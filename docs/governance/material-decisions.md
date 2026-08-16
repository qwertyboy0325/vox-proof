# Material Decisions

Status: active governance

Owns:
The process for recording accepted durable decisions that affect VoxProof product semantics, data contracts, privacy/security posture, identity model, persistence/serialization, review-decision semantics, or major architecture boundaries.

Does not own:
Exploratory ideas, discussion notes, implementation status, ordinary refactors, test-only changes, or daily development progress.

## Rule

A Material Decision is required before implementing a durable change to any of these boundaries:

- source identity or fingerprinting
- source anchoring semantics
- evidence semantics
- review-case identity or lifecycle
- correction-decision semantics
- reviewed-output derivation
- persistence or serialization formats
- project bundle layout
- privacy, data rights, consent, retention, or local-first guarantees
- speaker identity, voice clusters, or biometric-adjacent data
- audio/source coordinate systems
- major architecture boundaries

## Authority

Ezra is the decision authority.

AI tools, agents, reviewers, discussion documents, and possibility-queue entries may propose decisions, identify risks, and draft tradeoffs.

They do not approve Material Decisions.

## Non-authoritative inputs

The following documents may inform decisions but do not authorize implementation by themselves:

- discussion notes
- review notes
- possibility queue entries
- pending contracts
- speculative architecture sketches
- implementation suggestions from AI tools

In particular:

```text
Nothing in the Possibility Queue authorizes implementation.
Nothing in discussion notes authorizes implementation.
```

## Decision record location

Accepted Material Decisions are recorded under:

```text
docs/governance/decisions/
```

Each decision should use a stable identifier:

```text
MD-001
MD-002
MD-003
MD-008
```

Accepted v0.1 establishment decision: `decisions/MD-008-v0.1-core-mechanism-establishment.md`.

Proposed licensing decision (not accepted): `decisions/MD-009-proposed-dual-licensing.md`. Readiness audit: `licensing-readiness.md`.

Proposed v0.2 correction-history decision (not accepted): `decisions/MD-011-proposed-human-raised-manual-replacement-correction-history.md` — human-raised review cases, `ManualReplacement`, append-only withdrawal/supersession/reapplication semantics, and the v0.2 active-decision fold rule.

Proposed v0.2 knowledge-governance decision (not accepted): `decisions/MD-012-proposed-experience-derivation-and-scoped-knowledge-governance.md` — correction facts, extraction runs, experience proposals, scoped accepted knowledge, promotion/revocation/conflict semantics, immutable knowledge snapshots, and analysis reproducibility inputs.

Proposed v0.2 analysis-execution decision (not accepted): `decisions/MD-013-proposed-analysis-jobs-source-revisions-and-reconciliation.md` — bounded analysis jobs, immutable attached analysis snapshots, stale-result rejection, source re-import, reanalysis history, case reconciliation, and human-raised case preservation.

Accepted v0.2 session-durability decision: `decisions/MD-014-proposed-session-durability-recovery-and-retention-requirements.md` — durable session authority, canonical versus derived state, crash consistency, single-writer ownership, lifecycle and recovery, retention and GC, compaction constraints, and privacy requirements. Acceptance records requirements only; it does not authorize persistence implementation or mechanism selection; a later mechanism-selection Material Decision remains mandatory.

Accepted v0.2 persistence-evidence decision: `decisions/MD-015-proposed-persistence-mechanism-evidence-protocol.md` — bounded comparative spike protocol, shared semantic fixture and oracle, fault-injection experiments, platform coverage, evidence artifacts, and pass/fail gates before mechanism selection. Acceptance authorizes the bounded spike only; a later mechanism-selection Material Decision remains mandatory.

Accepted v0.2 native-desktop decision: `decisions/MD-016-native-desktop-framework-and-in-process-integration.md` — eframe/egui `0.35.0`, one native Rust process, direct in-process integration, `ApplicationReviewSession` as canonical application state, and non-authoritative GUI state. IME composition and accessibility inspection remain open risks. Acceptance records the durable foundation choices; implementation still requires a bounded owner-authorized work package.

Accepted bounded v0.2 Manual Replacement decision: `decisions/MD-017-governed-manual-replacement-on-existing-review-case.md` — exact validated single-line Manual Replacement payloads on existing detector-raised `ReviewCase` values, append-only `DecisionRecorded` authority, last-decision-wins effective status, shared canonical materialization and overlap refusal, replay closure, frozen application export v2 renderers, and explicit exclusion of deletion, HumanRaised cases, persistence, and Correction Memory authority.

Accepted Gate 3 reusable-influence decision: `decisions/MD-018-proposed-minimal-governed-reusable-influence.md` — minimal exact-pair, project-scoped, explicitly promoted, revocable reusable influence semantics for future exact proposal generation. Accepted at `6b9170c4692ecc6f6545c6025abf75480ee9148a` per explicit owner authorization on 2026-07-30. Acceptance records semantics only; it does not authorize Gate 3 implementation, persistence mechanism selection, public pack formats, ontology, ranking, suppression, phonetic reusable influence, automatic promotion, or v0.3 establishment. A separately authorized bounded implementation work package remains mandatory.

Accepted Gate 4 mechanism-selection decision: `decisions/MD-019-sqlite-product-session-persistence-mechanism-selection.md` — SQLite as the product-session durability / persistence production mechanism with `01C-SQLITE-3` scoped-authority semantics as the selection basis; `01B-3` Append rejected for production integration and retained as verified rejected alternative and regression/reference candidate. Accepted per explicit owner authorization on 2026-08-16. Acceptance records mechanism selection and durable semantic boundary only; it does not authorize product integration, freeze spike schema or implementation, or accept evidence adapter APIs, fixtures, oracles, or runners as production architecture.

Accepted Gate 4 product-session authority decision: `decisions/MD-020-product-sqlite-session-authority-schema-and-lifecycle-boundary.md` — product SQLite session format v1, canonical versus derived boundary (including immutable ReviewCase snapshots as authority), three scoped command scopes, transactional commit contract, reopen/reconstruction contract, and fail-closed migration posture. Accepted per explicit owner authorization on 2026-08-16. Acceptance records production schema and lifecycle boundary. Session format v1 remains unchanged; session format v2 is owned by MD-021.

Accepted cross-material Project Memory decision: `decisions/MD-021-cross-material-governed-project-memory-and-reuse-decision-boundary.md` — one SQLite Project Memory store per project, product-generated opaque `ProjectScopeId`, derived reuse proposals with thin targets persisted only on first human decision, new Project Memory snapshot identity domain, v2 new-session-only binding, promotion writes project authority only, missing project blocks writable reuse without erasing committed Material-B readability, and Gate 7 avoided-correction attribution. Accepted per explicit owner authorization on 2026-08-16. Acceptance records the durable boundary; P1/P2 implementation remains a separately authorized work package and does not establish Gate 7 or v0.3.

Accepted HumanRaised Manual Correction decision: `decisions/MD-022-human-raised-single-span-manual-correction-boundary.md` — HumanRaised ReviewCase family with HumanSelectedSpan, atomic CaseRaised + ManualReplacement, session format v3 new-session-only, origin-aware SourceDecisionLocator, and additive Project Memory format 2 for HumanRaised promotion. Accepted per explicit owner authorization on 2026-08-16. Does not accept broader MD-011 withdrawal/supersession or Session Terms reanalysis.

Accepted governed project-knowledge derivation decision: `decisions/MD-023-governed-project-knowledge-as-derived-analysis-input.md` — prospective explicit `allowed_effects` on new Project Memory promotions, freeze-bound non-authoritative derived canonical-terminology proposals, distinct `ProjectTerminologyProposal` family, additive Project Memory format 3, and session format v4 new-session-only. Historical promotions remain exact-only forever. Accepted per explicit owner authorization on 2026-08-16. Does not accept retrospective widening, Session Terms projection of Project Memory, auto-promotion, or Gate 7 attribution from progressive terminology.

## Decision record shape

Each Material Decision should include:

* title
* status
* date
* amended date, if the accepted record was clarified after acceptance
* decision authority
* context
* decision
* consequences
* explicitly deferred questions
* banned or rejected designs, if any
* implementation notes, if useful

## Decision record amendments

Accepted Material Decisions should be stable records.

If a clarification is needed before implementation or durable artifacts exist, the accepted decision may be amended in place when the change preserves the same approved decision and records the amendment date.

If implementation, persisted data, regression assets, or other durable artifacts already depend on the accepted decision, a semantic change must be recorded as a new Material Decision or as an explicit superseding decision. It must not silently rewrite the accepted record.

## Implementation rule

Implementation may proceed only after the relevant Material Decision is recorded.

If a change touches a durable boundary and no accepted Material Decision exists, stop and record the decision first.

## Scope control

Material Decisions should be narrow.

They should decide the durable semantic boundary needed for upcoming implementation, not design the entire future system.

A good Material Decision prevents silent semantic drift without becoming a roadmap.
