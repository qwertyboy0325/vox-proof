# MD-023: Governed Project Knowledge as Derived Analysis Input

Status: accepted

Date: 2026-08-16

Accepted: 2026-08-16 per explicit owner authorization

Decision authority: Ezra

Classification: Prospective explicit allowed-effects on Project Memory promotions; freeze-bound non-authoritative derived canonical-terminology detector input; distinct proposal kind; Project Memory format 3; session format v4

## Context

MD-018 accepted exact-pair reusable influence and forbade phonetic reusable influence and naive projection of promoted pairs into `SessionTermEntry`. MD-021 accepted Project Memory, freeze-bound exact reuse proposals, and Gate 7 attribution for exact reuse. MD-022 accepted HumanRaised promotion into the same exact-pair store.

Exact reuse covers a later occurrence of the same observed form X. It does not authorize catching a new form X2 merely because a prior correction established canonical Y.

This decision accepts one additional prospective analysis effect: freeze-bound derivation of eligible confirmed replacement Y values into the existing ASCII-Latin phonetic detector, as non-authoritative `ProjectTerminologyProposal` suggestions.

This decision does not create terminology authority. Project Memory remains the only governed durable semantic source.

## Product principle

Terminology normally accumulates through governed human corrections during review rather than as a prerequisite prepared before review. Manual Session Terms remain optional session-local seeding.

## Decision

### 1. Historical promotions remain exact-only

Existing `PromotionAccepted` records, including format 1 and format 2 JSON that lacks `allowed_effects`, retain:

```text
ExactObservedFormProposalGeneration only
```

They must never be reinterpreted as authorizing derived terminology or phonetic influence. Missing effect metadata is historically exact-only, not an invitation to apply current code, detectors, or copy.

No retrospective widening. Historical rows are not rewritten.

### 2. Prospective same-button consent

For new promotions after this decision, the existing gesture "Use this correction in related reviews" may authorize:

```text
ExactObservedFormProposalGeneration
and, when Y is eligible,
DerivedCanonicalTerminologyProposalGeneration
```

No second "remember as terminology" button in this slice. No auto-promotion.

### 3. Explicit allowed-effect semantics

New `PromotionAccepted` payloads MUST persist an explicit `allowed_effects` set. Consent is in the record, not inferred from software version, detectors, display copy, or absent fields.

```text
Ineligible Y → {ExactObservedFormProposalGeneration}
Eligible Y → that set plus DerivedCanonicalTerminologyProposalGeneration
```

`allowed_effects` is durable consent fixed at promotion time. It is authority. Eligibility and derivation algorithms interpret under that authorized set; they must not add, remove, or rewrite stored consent.

A historical `{Exact}` record must not gain `DerivedCanonicalTerminologyProposalGeneration` because a later detector or eligibility rule is stronger. An `{Exact, Derived}` record must not be rewritten because a later eligibility rule would have chosen a different set.

Later software or detector versions MUST NOT infer additional effects.

Do not use a serde default that grants `DerivedCanonicalTerminologyProposalGeneration`.

Eligibility is evaluated from confirmed replacement Y only, never from observed X.

Examples:

- `Postgres → PostgreSQL` may grant Exact + DerivedTerminology
- `他 → 她` remains Exact only
- `50 → 15` remains Exact only and must not become terminology

Forbidden in this slice: CJK generalization, pronoun generalization, sentence-semantic learning, aliases inferred from X, `observed_error_forms` generated from X, fuzzy matching, embeddings, or LLM logic.

### 4. Project Memory format 3

First promotion that writes explicit `allowed_effects` bumps Project Memory `format_version` to 3 additively, from format 1 or format 2, inside the same authoritative transaction.

Historical event rows are not rewritten. Old format-1/2 rows retain historical meaning. Old software fail-closed on unknown newer format.

Format 1/2 snapshot hashing remains byte-identical to pre-MD-023 behavior.

Format 3 snapshot hashing includes deterministic sorted `allowed_effects`.

Keep the `project-memory-snapshot:sha256-v1:` tag prefix; `format_version` is already a hash input.

### 5. Project Memory is the sole governed semantic source

Do not persist generated `SessionTermEntry` as project or session truth. Derived canonical vocabulary is ephemeral and freeze-bound. Seeded Session Terms remain a separate operator-provided session-local analysis input. No Project Memory ↔ Session Terms sync.

### 6. Derived terminology is non-authoritative

```text
Project Memory
  → freeze ProjectMemorySnapshot N
  → fold active records at the frozen governance boundary
  → select records whose explicit allowed_effects include
      DerivedCanonicalTerminologyProposalGeneration
  → take confirmed replacement Y
  → existing ASCII-Latin phonetic canonical-target structural rules
  → dedupe identical Y deterministically
  → run the existing phonetic detector as derived project analysis
  → ProjectTerminologyProposal
```

Output changes only after a human decision. No auto-accept, auto-promote, source rewrite, or live Project Memory mutation of an established analysis.

### 7. Frozen snapshot semantics

Material B binds `ProjectMemorySnapshotIdentity` N at analysis freeze. Derive terminology from N only.

If Project Memory becomes N+1: existing proposal identities and decisions stay; no live case mutation; new knowledge applies to future material or explicit new analysis only.

### 8. New identity domains

Do not reuse these tags with changed inputs or meaning:

```text
session-terms:sha256-v1
project-memory-snapshot:sha256-v1   (except format_version already hashed)
reusable-influence-snapshot:sha256-v2
frozen-project-reuse-analysis:sha256-v1
reuse-proposal-target:sha256-v1
```

Required new domains:

```text
project-derived-terminology-analysis:sha256-v1
project-terminology-proposal-target:sha256-v1
```

Bind: source revision, freeze N, governance boundary, derivation-rule version, operator `SessionTermsIdentity`, detector/config/algorithm identities, occurrence, proposed Y, contributing Project Memory provenance.

Different freeze snapshots must not alias merely because Y strings match.

### 9. Distinct proposal kind

`ProjectTerminologyProposal`

User-facing: "Suggested using terminology from previous corrections"

Not a seeded `CanonicalTermCase`, not `ProjectReuseProposal`, not HumanRaised.

Unacted proposals stay derived. First human Accept / Reject / Edit persists a thin target + decision atomically. The thin target must contain occurrence + Y + enough coordinates to materialize without current Project Memory.

`reuse_proposal_targets` must not store `ProjectTerminologyProposal`.

### 10. Collision semantics

Same occurrence + same Y: one human decision target.

Precedence, not UI order:

```text
seeded canonical
  > ProjectTerminologyProposal
  > exact reuse
```

Lower sources may be supporting provenance only.

Same occurrence + different Y: keep the conflict visible; the human decides.

### 11. Attribution semantics

Keep classes distinct:

- seeded canonical detection
- exact reuse
- progressive terminology (X2 ≠ taught X1)

Progressive-terminology hits are not Gate 7 `repeated_correction_avoided`.

If derived terminology independently proposes the same Y, exact-reuse Gate 7 for that occurrence is ineligible.

### 12. Session format v4

v3 has no authorized silent extensibility path for a new proposal family or thin-target table.

v4 is new-session-only. No automatic migration. Do not add the new target table to existing v3 files on open.

v1/v2/v3 remain readable and retain original semantics. They cannot create `ProjectTerminologyProposal` decisions. Existing exact reuse behavior remains compatible.

### 13. Missing-project behavior

After a committed human decision on a `ProjectTerminologyProposal`:

- reviewed output remains reconstructable from thin target + ledger
- output does not depend on current Project Memory

If the project store is later missing or unverified:

- provenance may be unavailable or unverified
- no new derived-terminology proposal may be authorized
- committed reviewed output remains readable

### 14. Optional seeded terminology

Unchanged session-local analysis input. Not project knowledge. Not auto-promoted into Project Memory.

## Banned

- Copying Project Memory into Session Terms
- A second durable glossary authority
- Retrospective widening of historical `PromotionAccepted`
- Inferring derived effects from missing fields
- Storing `ProjectTerminologyProposal` in `reuse_proposal_targets`
- Silent v3 schema or meaning mutation
- Auto-accept / auto-promote / output rewrite
- Fuzzy / embeddings / LLM matching
- Claiming Gate 7 from progressive-terminology hits

## Deferred

- CJK / pronoun / sentence terminology
- Revoke/supersede UX changes
- Project-level glossary import
- Gate 7 / v0.3 establishment
- Second terminology-consent button
- Native GUI dogfood of the progressive-terminology path

## Consequences

Implementation may persist explicit `allowed_effects`, bump Project Memory to format 3 on first such write, write new product sessions as format v4, and derive freeze-bound `ProjectTerminologyProposal` values from eligible Y.

This decision does not establish Gate 7, v0.3, or product effectiveness.
