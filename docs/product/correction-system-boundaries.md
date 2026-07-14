Status: current
Owns: Cross-version product boundaries for correction categories, evidence, context, policy, transformations, projections, user authorization, Domain Collection authority, and provisional pack terminology relationships.
Does not own: Current version scope, final data schemas, runtime pipeline design, persistence, implementation tasks, quality metrics, final pack schemas or storage, or accepted Material Decisions.
Last reviewed against code: The current local loop implements exact, case-sensitive, bounded accuracy-oriented term restoration through human-reviewed replacements and provisional session-term input. Duplicate source forms are rejected. `SessionContext`, policy resolution, Domain Collections, LLM or automation runtimes, typed transformation intent, multiple projections, typed ASR metadata, multi-alternative cross-collection resolution, and phonetic evidence are not implemented.

# Correction System Boundaries

## Product Position

VoxProof is not a fixed phonetic autocorrector, a universal transcript cleaner, an automatic editorial rewriter, or a system that decides what every user should preserve or remove.

VoxProof aims to combine an immutable ASR source, optional typed ASR evidence, transcript context, a selected recording scenario, Active Domain Collection Selections, user-controlled preferences, explicit automation authorization, and human review. The purpose is to produce explainable, auditable, and reversible correction or transformation suggestions.

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

- **Matching Policy** controls analysis comparison semantics, such as case handling, Unicode normalization, spacing or punctuation tolerance, script handling, number normalization, phonetic-comparison eligibility, source/target compatibility, and span constraints. It does not control visible output casing, replacement authority, deletion, auto-apply, or editorial rewriting.
- **Suggestion Policy** controls which supported finding classes run or are surfaced, which evidence enters review, ambiguity presentation, repeated-finding grouping, candidate-volume presentation, and evidence visibility. It may affect analyzer invocation before evidence production and surfacing after evidence production, but it does not make evidence true or authorize edits.
- **Cleanup Policy** represents user preferences for handling filled pauses, immediate repetitions, false starts, backchannels, discourse markers, and verbatim or clean-verbatim tendencies. It does not establish that speech is objectively noise, decide whether every finding is surfaced, define deletion/materialization repair, or authorize auto-apply.
- **Presentation Policy** controls representation for a requested projection, such as canonical casing, spacing, punctuation, number or unit formatting, Traditional or Simplified Chinese, source-form preservation, and projection-specific rendering. The same accepted accuracy correction may render differently in different projections.
- **Automation Policy** controls review requirements, batch review, assisted auto-apply eligibility, scoped auto-apply, transformation-class handling, ambiguity escalation, and revocation. It does not determine truth, transformation content, canonical terms, or permanent policy promotion.

Dependencies and detectors may provide normalization functions, phonetic representations, pattern matching, or scores. They must not determine product-visible case sensitivity, canonical casing, filler removal, repetition removal, punctuation style, source preservation, or auto-apply behavior.

These names describe peer-input responsibilities, not accepted Rust types. Matching, suggestion, cleanup, presentation, and automation policies do not belong inside a catch-all `SessionContext`.

### Recommendation, Active Policy, and Authorization

The following layers remain distinct:

```text
recommended default
-> user accepts or modifies
-> resolved active policy
-> separate explicit authorization, when automatic materialization is allowed
```

Recommended defaults may come from the product, output-mode presets, recording scenarios, Domain Collections, Language Packs, or Knowledge Packs. A recommendation is not active behavior. Collection activation does not activate a recommendation, grant replacement or deletion authority, or authorize automation.

An active policy is the resolved behavior selected for a run or projection. It is still not automation authorization. Explicit authorization separately grants a transformation class permission to materialize automatically within a declared scope. Missing authorization means review-required.

### Resolved-Policy Construction

The provisional direction is explicit resolved-policy construction with conflict reporting rather than one silent global precedence chain. Contributions should conceptually retain their source, scope, applicability, recommendation-versus-explicit-choice status, and revision or provenance.

A future resolver should produce effective values, per-value provenance, unresolved conflicts, and a conservative fallback. Explicit user overrides take priority over recommendations; conflicting explicit rules do not silently use last-write-wins; collection and scenario inputs remain recommendations; per-occurrence decisions govern only that occurrence; and recommendations never grant automation.

No generic policy schema, complete precedence matrix, persistence contract, or policy engine is accepted here.

### Analyzer Disposition

Future analyzer configuration may distinguish:

- **Disabled**: the analyzer does not run.
- **Shadow / Hidden**: the analyzer runs and findings may be retained for authorized calibration, but they do not enter the ordinary review UI.
- **Enabled**: the analyzer runs and eligible findings may be surfaced.

Run provenance must eventually distinguish not run, ran and found none, ran with hidden findings, and ran with surfaced findings. General user disablement means `Disabled`; shadow operation is a future calibration or research capability, not current behavior. Retention, privacy, and persistence semantics remain unresolved.

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

The first provisional product direction is a configurable Reviewed Projection with recommendation presets such as `Verbatim`, `Clean Verbatim`, `Machine-Friendly`, and `Custom`. A preset supplies policy recommendations; it is not a new source of truth, an active policy by itself, or deletion authority. The conceptual derivation is:

```text
immutable source
+ accepted or explicitly authorized transformations
+ resolved projection policy
-> rendered projection
```

The current runtime materializes only the existing reviewed SRT path. Multiple projections and projection-specific rendering are not implemented.

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

**Domain Collection** means a reusable domain/context knowledge unit. It may eventually contain canonical terms, aliases, observed forms, pronunciation hints, related terms, canonical spelling knowledge, scenario applicability, matching recommendations, presentation recommendations, and provenance.

A Domain Collection may constrain or expand candidate space, provide context, and provide canonical representations. It is evidence or context input, not replacement authority, edit authorization, or automatic correction memory.

**Active Domain Collection Selection** means a session-scoped selection of a specific immutable collection revision for one analysis run. Imported, available, selected, and active-for-analysis are distinct conceptual states. These distinctions do not define final types, activation contracts, storage, or schemas.

The product model allows multiple Domain Collections to be selected for one run, for example general software engineering, PostgreSQL, a company project, and personal terms. A first runtime may temporarily support only one collection, but that limitation must not become the canonical product model.

When a collection is selected, matching and presentation recommendations remain inspectable recommendations. A user may accept all, adjust individual recommendations, or load only the collection knowledge. Only accepted or modified recommendations enter resolved active policy.

The current session-term file is only a provisional, local, session-scoped adapter for canonical terms, aliases, and observed forms. It is not a Domain Collection schema, persistence format, reusable policy store, or product-ready pack format.

The provisional refined terminology direction is:

```text
Domain Collection
-> reusable domain/context knowledge unit

Language Pack
-> reusable language-specific resources

Knowledge Pack
-> packaging / import-export / distribution bundle
```

A Language Pack may provide locale metadata, pronunciation resources, and language-specific normalization or comparison resources. A Knowledge Pack may bundle collections, language resources, regression assets, recommendations, or other reusable artifacts. It is not an active runtime authority object. Import does not activate; activation does not accept recommendations; accepting recommendations does not authorize automation; confirmed mappings do not silently become permanent rules; and regression cases are not live policy.

This terminology direction remains provisional. It does not finalize schema, persistence, import/export, versioning, activation, or ownership contracts.

### Resolved Terminology Direction

Future adapters may converge conceptually as:

```text
session-term adapter
collection resolver
language-resource resolver
-> resolved terminology input
-> evidence producers
```

The resolved terminology input is a conceptual analysis boundary, not a final schema. The session-term file is not a minimal Domain Collection projection, import format, Language Pack, Knowledge Pack, persistent preference store, learned asset, auto-apply rule format, user profile, or permanent detector configuration.

### Collection Conflict Types

Conflicts require type-specific handling:

- When one source form plausibly refers to different entities, future analysis may retain multiple alternatives and use context evaluation or LLM-assisted selection; close or insufficiently supported alternatives escalate to review.
- Different canonical or display styles for the same entity keep canonical spelling knowledge separate from presentation preference.
- Conflicting collection recommendations are surfaced during activation or policy resolution and are not silently resolved.
- Structural or parsing errors are activation/configuration errors.
- Terminology contributions that cannot be merged safely remain explicit ambiguity or are excluded from activation.

The current session-term parser and detector validation reject duplicate source forms. Multi-alternative cross-collection conflicts, contextual aggregation, and assisted resolution are not implemented.

## Provisional SessionContext Boundary

`SessionContext` provisionally means immutable, resolved, session-scoped context inputs describing the circumstances under which analysis occurs.

It may contain or reference:

- selected recording scenario;
- expected locale or language mix;
- active Domain Collection selections;
- available surrounding transcript context;
- optional typed ASR evidence references;
- channel, role, or other non-identity speaker context.

It explicitly does not own:

- the transcript;
- `ReviewLedger` decisions;
- matching, suggestion, cleanup, presentation, or automation policy;
- requested projection;
- automation authorization;
- mutable learned preferences;
- UI state;
- user identity;
- replacement authority;
- detector orchestration;
- persistence.

Small descriptive values may be copied into a resolved snapshot. Large Domain Collections, ASR evidence, and other assets should use immutable revision or content references. `SessionContext` must not hold live references to mutable collections, preferences, or UI objects, copy full context into every `CandidateSpan`, or use file paths as semantic identity.

Matching policies, projection requests, and automation authorization are peer inputs rather than fields inside one catch-all `SessionContext`. No final type, field list, serialization, or schema is accepted here.

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

Assisted auto-apply is a recommended future low-friction UX direction, not currently enabled default runtime behavior. The current implementation remains human-review-required, and no automation runtime exists today.

Assisted auto-apply requires both a resolved active policy and separate explicit authorization for the relevant transformation class and scope. It is driven by user authorization, not detector or model score alone. Any future provisional application must preserve source, transformation provenance, alternatives when available, a visible change summary, inspection and rematerialization/revert paths, and escalation for ambiguity.

Provisional transformation-class direction:

- accuracy/entity resolution, casing normalization, low-risk formatting normalization, and well-supported canonical term resolution may be eligible for assisted auto-apply when the transformation class and scope are authorized;
- filler removal, repetition collapse, false-start cleanup, and deletion-like transformations default to batch review;
- editorial rewriting, substantial sentence restructuring, tone or uncertainty changes, and speaker-style transformation require explicit opt-in;
- multiple plausible entities, close context evidence, unresolved collection conflicts, insufficient structured support, deletion, semantic rewriting, and previously revoked automation escalate to human review.

These are future product directions, not implemented runtime behavior, thresholds, schemas, or authorization contracts. Strict silent auto-apply is not the general recommended direction and would require more explicit authorization.

### LLM-Assisted Contextual Selection

LLM-assisted contextual selection may recommend or rank alternatives using transcript context, recording scenario, Active Domain Collection Selections, related terms, optional ASR evidence, resolved policies, and authorized reusable rules. Its output is not a decision, transformation, authorization, or edit authority.

Inspectable provenance may include resolver or model identity and version, configuration, relevant context or evidence references, alternatives considered when available, the selected suggestion, structured supporting factors when available, and the authorization source if automated materialization occurs.

This document does not require retention of free-form model rationale, hidden reasoning, or chain-of-thought. A human-readable explanation may be generated for UI use, but it is not accepted here as a durable reasoning-storage contract. No LLM runtime exists in the current implementation.

### Revocation and Rematerialization

Future revocation should rematerialize output from immutable source without the revoked transformation while preserving the audit history. A revocation may become an automation-calibration signal and may produce a visible suggestion to lower automation, narrow scope, or return to review-first behavior. It does not silently create a permanent rule or change policy, scope, or authorization without user confirmation.

Revocation storage, historical applicability, rematerialization, and audit schemas remain unresolved and are not implemented.

## Analysis Reproducibility Direction

The current `AnalysisSnapshot` records only transcript revision and is insufficient to distinguish future runs with different active collections, resolved matching behavior, detector configuration, phonetic thresholds, optional ASR evidence, or suggestion behavior that changes candidate generation or queue membership.

The future reproducibility boundary should be able to identify source revision, effective context inputs, active terminology or domain-asset revisions, resolved matching behavior, detector identities, versions and configuration fingerprints, normalization or representation version, optional ASR evidence source or revision, and candidate-affecting suggestion behavior.

This document does not decide whether the implementation extends `AnalysisSnapshot`, adds a higher-level analysis-run provenance concept, or composes multiple records. It also does not decide IDs, hash formats, serialization, or persistence. Wall-clock timestamps, UI state, user identity, file paths, session UUIDs, purely visual presentation settings, automation authorization, and projection requests that do not affect analysis do not belong in semantic analysis identity.

## Current Implementation Mapping

Implemented:

- immutable transcript and revision identity;
- source anchors;
- exact alias and observed-error-form evidence;
- exact, case-sensitive matching with duplicate source forms rejected;
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
- policy resolution;
- transformation intent;
- deletion;
- cleanup;
- multiple projections;
- Domain Collections;
- automation;
- policy promotion;
- typed ASR metadata;
- multi-alternative cross-collection conflict handling;
- LLM runtime;
- phonetic detection.

This implementation mapping proves only that the bounded local code loop exists. It does not establish v0.1 mechanism validity or product validation.

## Governance Near Implementation

Recording these product and architecture boundaries does not itself require a Material Decision. Future durable semantics are evaluated individually under `docs/governance/material-decisions.md`.

Changes involving transformation intent, deletion and materialization, projection ownership, analysis identity, automated decision provenance, automation authorization, policy promotion or revocation, pack activation or persistence, shadow-finding retention, multi-alternative review identity, LLM runtime privacy, or phonetic evidence semantics may require separate narrow decisions when their concrete durable contracts are proposed. Internal algorithm choices, characterization methods, UI copy, and non-contractual implementation details do not automatically require a Material Decision.
