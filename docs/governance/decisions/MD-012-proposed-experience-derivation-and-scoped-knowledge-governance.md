# MD-012: Experience Derivation and Scoped Knowledge Governance

Status: proposed

Date: 2026-07-19

Decision authority: Ezra

Classification: correction-to-knowledge derivation, scoped reusable knowledge governance, knowledge snapshots, and analysis reproducibility inputs (not accepted)

## Context

MD-002 and proposed MD-011 establish `ReviewLedger` events and correction decisions as correction authority.

MD-003 establishes that reviewed output is derived only from source transcript plus applicable human correction decisions.

MD-004 establishes effective v0.1 analysis identity from transcript revision, session terms, detector set, detector configuration, and algorithm version.

`product/versioning.md` permits bounded Session and Project/Domain Collection knowledge to support a credible v0.2 external review workflow, while reusable knowledge remains a first-class measurable product capability in v0.3.

`product/correction-system-boundaries.md` describes Domain Collections, assisted policy authoring, and future reusable knowledge direction without accepting durable schemas, promotion contracts, or persistence.

v0.2 planning requires separating immutable correction facts from derived experience proposals and accepted reusable knowledge, binding analysis to immutable knowledge snapshots, preserving visible conflicts, and distinguishing historical reproduction from current-knowledge reanalysis.

This proposed Material Decision records the v0.2 semantic boundary. It does not authorize implementation, does not select persistence technology, and does not claim experimental validation of a knowledge subsystem.

## Terminology

This decision uses the following conceptual layers and terms:

```text
CorrectionFact
ExtractionRun
ExperienceProposal
KnowledgeItem
KnowledgeSnapshot
ReviewCase
ReviewLedger
ManualReplacement
AcceptAlternative
```

`ProjectDomainCollection` is the v0.2 scope identifier. In prose it means **Project/Domain Collection**, aligned with `product/correction-system-boundaries.md`.

## Decision

### Four conceptual layers

#### 1. Correction fact

A **correction fact** is the immutable historical fact that a reviewer made a correction decision.

It includes or references, conceptually:

* the correction event identity;
* `ReviewCase` identity;
* observed source revision;
* source anchor;
* accepted replacement payload where applicable;
* reviewer action provenance;
* decision history.

The original correction fact is never rewritten because a later extractor, proposal, promotion, revocation, ranking policy, or ranking result changes.

A long replacement remains preserved in full.

Not every correction must become reusable knowledge.

Correction facts are sourced from authoritative `ReviewLedger` events and related review context. They are not detector output and not accepted knowledge.

#### 2. Extraction run

An **extraction run** examines one or more immutable correction facts and may produce zero or more experience proposals.

Each run has an immutable identity and records, conceptually:

* extractor identity;
* extractor version;
* policy or configuration identity;
* source correction identities;
* execution time or sequence provenance;
* generated proposal identities;
* status such as completed, failed, or cancelled where applicable.

A new extractor version or rerun creates a new extraction run.

It never overwrites a previous run.

Extraction is downstream of correction. It does not create, modify, or replace a correction decision.

#### 3. Experience proposal

An **experience proposal** is a derived, reviewable candidate for reusable knowledge.

It is not yet accepted knowledge.

A proposal may represent a bounded learnable fragment derived from a longer correction.

One correction may yield zero, one, or many proposals.

A proposal must retain provenance back to:

```text
proposal
→ extraction run
→ correction fact
→ ReviewCase
→ source anchor
→ source revision
```

Proposal review outcomes may conceptually include `Pending`, `Accepted`, `Rejected`, `Deferred`, and `Withdrawn`. Final enum shapes remain implementation-defined.

Re-review or re-extraction creates new history. It does not rewrite an old proposal's provenance.

#### 4. Accepted knowledge item

A **knowledge item** is reusable guidance accepted through an explicit governance action.

It is distinct from:

* a correction fact;
* an extraction run;
* an experience proposal;
* a detector candidate;
* an analysis suggestion;
* reviewed output.

An accepted knowledge item retains full provenance to the proposal and original correction evidence.

Knowledge items are versioned rather than edited in place.

A later change to knowledge creates a new version, supersession, narrowing, or revocation record.

Historical analysis remains bound to the knowledge version and snapshot it originally used.

### Knowledge scope

The canonical v0.2 scope ladder is:

```text
Session
ProjectDomainCollection
```

#### Session scope

Default scope.

Knowledge is available only within one review session.

It must not leak into another session merely because content appears similar.

#### Project/Domain Collection scope

A bounded collection explicitly defined by the user or project context.

It supports reuse across related sessions inside the same project or domain collection.

It does not imply:

* universal correctness;
* user-wide applicability;
* global publication;
* automatic cross-project transfer;
* mature cross-domain portability.

#### Deferred and excluded scopes

* User-wide scope is reserved for later work and is not implemented in v0.2.
* Global scope is outside v0.2.
* Implicit scope escalation is forbidden.

### Promotion semantics

Promotion from a narrower scope or from a proposal into accepted reusable knowledge must be:

* explicit;
* reviewable;
* append-only;
* provenance-preserving;
* reversible through later governance events;
* independent from the original correction fact.

No correction, proposal, or ranking score automatically promotes knowledge.

The system may recommend promotion, but recommendation is non-authoritative.

Promotion creates a new governance event and knowledge version or identity as applicable.

### Revocation, narrowing, and supersession

**Revocation**

* affects future use;
* does not erase historical provenance;
* does not rewrite old analyses;
* does not alter the original correction fact;
* does not remove the knowledge item from historical snapshots.

**Narrowing**

* creates an explicit later governance state or version;
* does not silently reinterpret historical uses under the old scope.

**Supersession**

* links an older knowledge item or version to a newer one;
* does not delete or mutate the older identity.

Historical snapshots remain reproducible with the exact knowledge versions they contained.

### Conflict semantics

Different active knowledge items may recommend different replacements or interpretations for overlapping context.

Such conflicts must remain explicit.

Required rules:

* same replacement with compatible meaning may aggregate multiple provenance sources;
* different replacements form an explicit conflict group;
* narrower scope may influence ranking;
* narrower scope must not hide a broader conflicting item;
* conflict must be visible to analysis and review;
* absent a separately accepted policy, conflict does not authorize automatic correction;
* insufficient or contradictory provenance must fail closed rather than fabricate one authoritative answer.

### Knowledge as evidence, not correction authority

Knowledge may:

* generate suggestions;
* rank suggestions;
* explain provenance;
* identify likely terminology;
* expose conflict.

Knowledge must not:

* directly rewrite source text;
* automatically create a correction decision;
* bypass `ReviewLedger`;
* override a reviewer's explicit decision;
* become materialized output without a valid correction decision.

The human review decision remains correction authority.

### Suggestion identity and ranking

Suggestion identity must be independent from its current rank position.

```text
suggestion identity
≠
ranking index
```

The same suggestion may move in ranking without becoming a new suggestion solely because of position.

A ranking policy must be:

* versioned;
* identifiable in provenance;
* deterministic for the same bounded inputs where claimed;
* explainable through named features or reasons;
* separate from the immutable knowledge item identity.

Historical v0.1 alternative-index decisions remain interpreted against their original fixed candidate ordering. They are not retroactively migrated to the new identity scheme.

### Immutable knowledge snapshots

Every analysis that uses reusable knowledge must bind to an immutable **knowledge snapshot**.

A snapshot conceptually contains or references:

* snapshot identity;
* governance revision;
* included knowledge item or version identities;
* scope boundary;
* unresolved conflict state;
* snapshot creation provenance;
* ranking policy identity where relevant.

Snapshot creation must observe one coherent governance revision.

A live mutable knowledge store is never read as an unversioned hidden dependency during analysis.

Once attached to an analysis identity, the snapshot is immutable.

Revocation or later promotion affects future snapshots only.

Old analyses continue to refer to their original snapshots.

### Reproducibility distinction

Two explicit user intents must remain distinct.

#### Reproduce with original inputs

Use:

* original source revision;
* original analyzer or detector identities;
* original configuration;
* original knowledge snapshot;
* original ranking policy where available.

This attempts to reproduce the historical result under its original declared inputs.

It does not promise bit-for-bit executable reproducibility when runtime, dependency, or platform preservation is unavailable.

#### Reanalyze with current knowledge

Use:

* a selected source revision;
* a newly created current knowledge snapshot;
* current accepted analyzer configuration and ranking policy.

This creates a new analysis identity and new result.

It does not overwrite historical analysis.

These intents must not be conflated in UI or domain language.

### Semantic relation graph export

A future derived artifact type is reserved conceptually as `SemanticRelationGraphExport`.

The relation graph:

* is derived;
* is versioned;
* retains provenance;
* may include factual provenance edges and inferred semantic edges;
* is not canonical correction authority;
* does not mutate correction facts, proposals, knowledge, or `ReviewLedger`;
* may be regenerated without rewriting historical source facts;
* requires inferred edges to retain extractor, version, review status, and evidence.

The complete graph subsystem, graph database choice, and export format are outside v0.2 release scope.

## Invariants

1. A correction fact is immutable.
2. A correction may produce zero, one, or many proposals.
3. A proposal is not accepted knowledge.
4. Accepted knowledge retains provenance to original correction evidence.
5. Promotion never rewrites the originating correction.
6. Revocation affects future use but not historical snapshots.
7. A historical analysis remains bound to its original knowledge snapshot.
8. Knowledge conflict cannot be hidden solely by scope precedence.
9. Knowledge cannot directly materialize a correction.
10. Suggestion identity does not depend on ranking position.
11. Ranking policy identity is recorded where ranking affects analysis output.
12. Reanalysis with current knowledge creates a new analysis identity.
13. User-wide and Global scope are not silently inferred from Session or Project/Domain Collection scope.
14. Semantic graph export remains derived and non-authoritative.

## Banned designs

The following designs are rejected:

1. Rewriting the original correction into smaller knowledge fragments in place of preserving the correction fact.
2. Treating extraction output as automatically accepted knowledge.
3. Mutating a knowledge item in place.
4. Deleting revoked knowledge from historical snapshots.
5. Allowing narrower scope to erase visible conflicts.
6. Using current rank index as suggestion identity.
7. Reading a live mutable knowledge store during analysis without a snapshot.
8. Automatically applying knowledge to reviewed output.
9. Silently promoting repeated corrections.
10. Treating a semantic graph as correction authority.
11. Adding User or Global scope implicitly in v0.2.
12. Conflating reproduce-with-original-inputs and reanalyze-with-current-knowledge.

## Explicitly deferred

The following remain deferred and are not owned by this decision:

* detector implementation;
* analysis job execution and scheduling;
* source-revision reconciliation;
* persistence backend, storage schema, and serialization;
* UI design and workflow presentation;
* automatic correction;
* automatic knowledge promotion;
* model-training pipelines;
* graph database choice;
* semantic graph export format;
* User-wide scope implementation;
* Global scope;
* cross-project portability guarantees;
* v0.3 reusable-asset establishment criteria;
* exact promotion, revocation, narrowing, and supersession event payloads;
* final proposal or knowledge item public schemas.

MD-013 will later own analysis job, reanalysis, source revision, and reconciliation semantics. This decision does not pre-empt MD-013.

## Consequences

If accepted:

* new conceptual identities for extraction runs, proposals, knowledge items or versions, snapshots, and promotion or revocation events become required;
* analysis identity must eventually include knowledge snapshot and ranking-policy identity where reusable knowledge affects analysis;
* long corrections can remain intact while producing bounded reusable fragments;
* UI will need separate review surfaces for correction, proposal, knowledge, and conflict;
* storage must eventually preserve immutable provenance and historical snapshots;
* reanalysis creates new results rather than mutating old ones;
* v0.2 supports bounded knowledge use without claiming v0.3 maturity;
* future graph export remains possible without graph-first domain architecture.

This proposed decision does not authorize persistence implementation, automatic learning pipelines, graph storage, or v0.3 establishment claims.

## Compatibility with existing decisions

### MD-002 and MD-011

* `ReviewLedger` and correction events remain correction authority;
* knowledge governance does not rewrite correction history;
* `ManualReplacement` and applicable `AcceptAlternative` decisions may become source evidence for proposals;
* proposal extraction is downstream of correction, not part of the correction decision itself.

### MD-003

* knowledge does not directly materialize reviewed output;
* materialization still requires an active valid correction decision;
* knowledge-derived suggestions remain non-authoritative until reviewed.

### MD-004

* MD-004 remains authoritative for established v0.1 effective analysis identity;
* if accepted, immutable knowledge snapshot identity and ranking-policy identity become declared v0.2 analysis inputs where applicable;
* historical MD-004 artifacts are not reinterpreted.

### Version ladder

* bounded Session and Project/Domain Collection knowledge may support v0.2;
* v0.2 establishment remains centered on the complete external review workflow;
* reusable knowledge becomes a first-class measurable product capability in v0.3;
* MD-012 does not claim v0.3 maturity.

## Relationship to prior decisions

* MD-002 and proposed MD-011 remain authoritative for correction and ledger semantics.
* MD-003 remains authoritative for established v0.1 materialization.
* MD-004 remains authoritative for established v0.1 effective analysis identity.
* If accepted, MD-012 extends the correction-to-knowledge boundary without rewriting historical semantics recorded in those decisions.
