Status: current
Owns: Cross-version product boundaries for correction categories, evidence, context, policy, transformations, projections, user authorization, and Domain Collection authority.
Does not own: Current version scope, final data schemas, runtime pipeline design, persistence, implementation tasks, quality metrics, pack terminology resolution, or accepted Material Decisions.
Last reviewed against code: The current local loop implements bounded accuracy-oriented term restoration through human-reviewed replacements. Context evaluation, policy models, typed transformation intent, multiple projections, Domain Collections, automation, typed ASR metadata, and phonetic evidence are not implemented.

# Correction System Boundaries

## Product Position

VoxProof is not a fixed phonetic autocorrector, a universal transcript cleaner, an automatic editorial rewriter, or a system that decides what every user should preserve or remove.

VoxProof aims to combine an immutable ASR source, optional typed ASR evidence, transcript context, a selected recording scenario, active Domain Collections, user-controlled preferences, explicit automation authorization, and human review. The purpose is to produce explainable, auditable, and reversible correction or transformation suggestions.

No detector, dependency, model, context evaluator, or policy is transcript truth. Source remains immutable; suggestions do not become edits without human acceptance or explicit authorization within a defined scope.

## Correction Taxonomy

The correction system distinguishes four categories:

- **Accuracy correction** attempts to restore what the speaker said or the correct identity of a referenced entity.
- **Representation / formatting normalization** changes presentation without intending to change entity identity or meaning, such as casing, spacing, punctuation, or number formatting.
- **Disfluency cleanup** handles fillers, repetition, false starts, backchannels, and related speech phenomena according to user policy.
- **Editorial transformation** rewrites content for concision, formality, structure, or style and may change tone, uncertainty, or syntax.

The current v0.1 implementation covers only a bounded part of accuracy-oriented term restoration:

```text
session-scoped term input
-> exact alias / observed-error-form evidence
-> human review
-> accepted replacement
-> reviewed SRT + audit artifacts
```

It does not implement the other categories as general capabilities, and it is not a complete correction, cleanup, formatting-policy, editorial, Domain Collection, or semi-automatic policy system.

## Responsibility Boundaries

The following are conceptual ownership boundaries, not a commitment to a fixed linear runtime pipeline:

- **Immutable Source** owns the original parsed source and stable source identity. Analysis and output derivation do not overwrite it.
- **Analysis View** provides derived representations for analysis. Analysis normalization is not automatically visible output and must remain traceable to source anchors.
- **Evidence Producer** explains why a source span may deserve review. It produces inspectable evidence, not truth, policy, or edits.
- **Contextual Evaluation** assesses relevance or compatibility using available transcript, scenario, domain, and ASR context. Context does not authorize edits.
- **Policy Evaluation** applies user-controlled handling preferences and authorization boundaries. Policy does not invent source facts.
- **Human or Explicitly Authorized Decision** is the authority that permits a suggested transformation to proceed within its stated scope.
- **Typed Transformation** records the semantic intent of an accepted operation rather than only its resulting text.
- **Projection Materialization** derives a requested output from immutable source plus applicable accepted or explicitly authorized transformations.

These responsibilities may be composed in different ways. For example, policy may determine whether a particular contextual analysis or suggestion class should run. The boundary is semantic ownership, not mandatory stage ordering.

The governing invariants are:

- evidence is not truth;
- context does not authorize edits;
- policy does not invent source facts;
- suggestions are not edits;
- only accepted or explicitly authorized transformations materialize;
- source remains immutable.

## User-Controlled Policies

Policy responsibilities remain conceptually separate:

- **Matching Policy** controls comparison behavior used during analysis.
- **Suggestion Policy** controls which supported finding classes should be surfaced and how they should be presented for review.
- **Cleanup Policy** controls handling preferences for disfluency and cleanup findings.
- **Presentation Policy** controls representation choices for a requested output projection.
- **Automation Policy** controls which actions are authorized, under what conditions, and within what scope.

Dependencies and detectors may provide normalization functions, phonetic representations, pattern matching, or scores. They must not determine product-visible case sensitivity, canonical casing, filler removal, repetition removal, punctuation style, source preservation, or auto-apply behavior.

This document does not define final Rust types, serialized schemas, persistence, policy precedence, or conflict resolution.

## Projections

One immutable source may yield multiple derived projections, including:

```text
Reviewed Transcript
Clean Verbatim
Machine-Normalized View
Human Presentation View
Editorial / Polished View
```

These names describe possible product responsibilities, not currently implemented output types.

- Analysis normalization is not automatically human-visible output.
- A machine-friendly representation is not universally preferable.
- A human-friendly representation may preserve hesitation, tone, uncertainty, or discourse information.
- Identical rendered text can result from different transformation intents and audit histories.

Projection materialization must preserve source traceability and applicable decision provenance. A projection request is not itself evidence, context, or edit authorization.

## Transformation Intent

A future accepted operation needs semantic intent beyond `source -> replacement text`. Illustrative intents include:

```text
RestoreTerm
NormalizeCasing
NormalizeFormatting
RemoveFilledPause
CollapseRepetition
ResolveFalseStart
InsertPunctuation
EditorialRewrite
```

These are conceptual examples, not accepted final enums or schemas.

Deletion must not silently become a general empty-string replacement contract. Deletion semantics, whitespace repair, punctuation effects, timestamp behavior, empty segments, and audit behavior require later dedicated governance before implementation.

## Domain Collection Boundary

**Domain Collection** is the product-level concept for contextual and terminology knowledge used during analysis. It may eventually contain canonical terms, aliases, observed forms, pronunciation hints, related terms, official casing, scenario applicability, matching preferences, output preferences, and provenance.

A Domain Collection may constrain or expand candidate space, provide context, and provide canonical representations. It is evidence or context input, not replacement authority, edit authorization, or automatic correction memory.

The current session-term file is only a provisional, local, session-scoped adapter for canonical terms, aliases, and observed forms. It is not a Domain Collection schema, persistence format, reusable policy store, or product-ready pack format.

The terminology and ownership relationships among:

```text
Domain Collection
Knowledge Pack
Language Pack
```

remain unresolved. This document does not declare that one replaces another and does not finalize Domain Collection schema or persistence.

## Provisional SessionContext Boundary

`SessionContext` provisionally means immutable, session-scoped context inputs describing the circumstances under which analysis occurs.

Possible responsibilities include:

- selected recording scenario;
- active Domain Collection selections;
- available surrounding transcript context;
- optional ASR evidence references.

It explicitly does not own:

- the transcript;
- `ReviewLedger` decisions;
- mutable learned preferences;
- UI state;
- replacement authority;
- detector orchestration;
- persistence.

Matching policies, projection requests, and automation authorization may be separate peer inputs rather than fields inside one catch-all `SessionContext`. No final type or schema is accepted here.

## Assisted Policy Authoring

The intended future learning loop is:

```text
observe decisions
-> identify a repeated preference
-> propose a policy
-> user chooses scope
-> explicitly authorize
-> apply within that scope
-> audit and revise
```

Possible scopes include an occurrence, session, scenario, Domain Collection, output mode, or user-wide preference. These are conceptual possibilities, not current persistence or authorization semantics.

Previous decisions do not automatically become permanent rules. Decisions may inform policy suggestions, but promotion requires explicit user choice. This is assisted policy authoring, not silent automatic learning.

## Semi-Automation

Semi-automation is driven by explicit user authorization, not detector score alone.

Possible future behavior includes:

- exact authorized mappings may auto-apply within an approved scope;
- phonetic-only findings may remain review-required;
- filler deletion may be batch-reviewed;
- official casing may be auto-applied under an explicit presentation policy;
- ambiguous findings with multiple alternatives may remain review-required.

These are examples, not current implemented behavior or accepted defaults.

## Current Implementation Mapping

Implemented:

- immutable transcript and revision identity;
- source anchors;
- exact alias and observed-error-form evidence;
- non-binding alternatives;
- human `ReviewLedger` decisions;
- accepted replacement materialization;
- reviewed SRT;
- decision log;
- session summary;
- provisional session-scoped term input.

Not implemented:

- `SessionContext`;
- contextual evaluation;
- policy models;
- transformation intent;
- deletion;
- cleanup;
- multiple projections;
- Domain Collections;
- automation;
- policy promotion;
- typed ASR metadata;
- phonetic detection.

This implementation mapping proves only that the bounded local code loop exists. It does not establish v0.1 mechanism validity or product validation.

## Governance Near Implementation

Recording these product and architecture boundaries does not itself require a Material Decision. Future durable semantics are evaluated individually under `docs/governance/material-decisions.md`.

Changes involving transformation intent, deletion and materialization, projection ownership, automation authorization, or phonetic evidence semantics may require narrow decisions when their concrete durable contracts are proposed. Internal algorithm choices, characterization methods, and non-contractual implementation details do not automatically require a Material Decision.
