Status: exploratory
Owns: Unvalidated market, user, and future-product hypotheses.
Does not own: Current v0.1 scope, architecture commitments, implementation tasks, roadmap promises, or validated claims.
Last reviewed against code/evidence: v0.1 is established by MD-008 only as the bounded deterministic core mechanism defined by MD-007 and MD-008. Product effectiveness, external usability, workflow value, adoption, time savings, product-market fit, and every hypothesis below remain unproven.

# Product Hypotheses

This document records assumptions that may guide discovery. None of these hypotheses are validated.

Concrete discovery leads are recorded in `docs/discussion/2026-07-09.md`. A lead is a person willing to talk or try the tool; it is not validation.

## Primary Commercial Wedge Validation Hypothesis — Interview Transcript Review

```yaml
status: PRIMARY_VALIDATION_HYPOTHESIS
accepted_product_pivot: false
canonical_product_direction_changed: false
implementation_authority: false
```

Hypothesis:

> Local-first interview transcript verification for multi-interview projects
> may be a stronger first commercial wedge than subtitle-first correction.

The target is defined by workflow characteristics rather than profession:

- one project produces multiple long-form interviews;
- material is Chinese or mixed Chinese-English;
- names, numbers, negation, and technical terms have meaningful error cost;
- recordings may be sensitive or unpublished;
- transcripts may support later quotation or formal deliverables;
- prior human-confirmed corrections may reduce repeated review work.

The core job is:

> Turn AI-generated interview transcripts into audio-grounded, reviewable
> records with explicit human confirmation history.

The initial external claim must not imply that unreviewed text is verified.
A bounded articulation is:

> Turn AI transcripts into interview transcripts whose reviewed issues can be
> checked against source audio and retain their human confirmation history.

### Complete-Product Validation Posture

The primary product validation should use a complete but narrow end-to-end
product, not an operator-assembled research harness. The intended first-product
shape is:

```text
create a project
→ record or import a one-to-one interview
→ obtain a local non-authoritative transcript observation
→ establish usable, correctable speaker attribution
→ review audio-grounded windows
→ make explicit human decisions
→ explicitly promote eligible exact reusable knowledge
→ apply governed reuse to later interviews in the same project
→ export reviewed output, unresolved state, and a review receipt
```

Product completeness means owning the user-visible journey, including failure,
recovery, progress, privacy, resume, and delivery behavior. It does not require
generic meeting-assistant breadth. Calendar integration, meeting bots,
real-time transcription, CRM integration, cloud sync, team collaboration,
generic multi-party meetings, automatic summaries, and action-item extraction
are not required to test this hypothesis.

An assisted workflow using external transcripts may still produce useful
mechanism evidence. It does not by itself validate the onboarding, capture,
transcription, speaker, recovery, or self-service experience of the complete
product hypothesis.

This posture does not authorize implementation or bypass existing execution
gates. In particular, recording authority requires an accepted decision;
built-in ASR, media integration, speaker attribution, and cross-material reuse
must satisfy their applicable claim and evidence boundaries.

### Validation Shape

Validation should use one real project with at least two, preferably three,
related interviews so both audio-grounded review and cross-material governed
reuse are exercised. The baseline is the participant's existing transcription
tool, transcript editor, audio player, glossary, search/replace, and manual-note
workflow—not unaided transcription from scratch.

Relevant signals include:

- time from a suspicious span to its source audio;
- decision time per reviewed issue;
- confirmed consequential errors in names, numbers, negation, and terminology;
- visibility of reviewed versus unresolved coverage;
- delivery preparation time;
- repeated correction workload across later interviews;
- continued use with real material;
- willingness to pay for a later project.

The hypothesis is not supported by audio playback convenience or glossary value
alone. The intended combined value is:

```text
audio-grounded verification
+ explicit human decision history
+ cross-material governed reuse
```

What would falsify it:

- users prefer their existing workflow after completing realistic delivery;
- the complete local workflow adds more friction than verification value;
- consequential errors are too rare to justify focused review;
- later interviews do not contain enough eligible repetition for governed reuse;
- speaker and source-grounding requirements make the bounded product
  impractical;
- users continue only under operator assistance or show no willingness to pay.

This section records a primary validation hypothesis only. It is not an
accepted pivot, Material Decision, roadmap replacement, or validated market.

## Mixed Chinese-English Technical Content

- Hypothesis: Technical-content creators with mixed Chinese-English terminology may have a meaningful transcript QA problem.
- Why it may be true: Mixed terminology, product names, acronyms, and localized pronunciations may produce errors that are hard to spot manually.
- What would falsify it: Target users do not experience frequent transcript terminology errors, or existing tools already solve the problem well enough.
- Validation method: Interview target users, review authorized transcript samples, and measure error patterns against manual correction workflows.
- Current lead: a high-frequency YouTube content editor with expressed willingness to try the tool (Lead A in `docs/discussion/2026-07-09.md`).
- Status: Unvalidated.

## Cross-ASR Post-Processing

- Hypothesis: Cross-ASR post-processing may be more useful than replacing ASR itself.
- Why it may be true: Users may already have preferred ASR tools and need targeted QA after transcription.
- What would falsify it: Users prefer changing ASR systems over adding a post-processing review step.
- Validation method: Compare user willingness to run post-processing against willingness to replace their transcription workflow.
- Current lead: a film subtitle corrector already using AI-assisted correction whose residual pain is subtitle timing drift (Lead B in `docs/discussion/2026-07-09.md`).
- Status: Unvalidated.

## Domain Collection Reuse

- Hypothesis: Reusable Domain Collections may reduce repeated correction effort.
- Why it may be true: Specialized terms, aliases, names, and recurring ASR confusions may repeat across related transcripts.
- What would falsify it: Corrections are too one-off, context-dependent, or inconsistent to benefit from reusable language memory.
- Validation method: Analyze authorized transcript sets for repeated correction patterns and measure review burden with and without a relevant Domain Collection.
- Current lead: a teacher organizing recurring lecture content across sessions (Lead C in `docs/discussion/2026-07-09.md`).
- Status: Unvalidated.

## Scoped Language Memory

- Hypothesis: Speaker, project, team, and domain scoped language memory may have long-term value.
- Why it may be true: Pronunciations, names, abbreviations, and product terminology may vary by speaker, project, team, or domain.
- What would falsify it: Scoped memory adds complexity without improving candidate quality or review efficiency.
- Validation method: Compare correction patterns across scopes using authorized samples and observe whether scoped records reduce false positives or missed terminology issues.
- Status: Unvalidated.

## Local-First Processing

- Hypothesis: Local-first processing may matter for privacy-sensitive or technically sophisticated users.
- Why it may be true: Transcript and audio data may contain private, unpublished, or proprietary information.
- What would falsify it: Target users do not value local processing enough to affect adoption or workflow choice.
- Validation method: Interview users about privacy constraints, data-handling requirements, and willingness to use local tooling.
- Status: Unvalidated.

## Stable Distribution Value

- Hypothesis: An official stable distribution may eventually be worth paying for even if code and weights remain open source.
- Why it may be true: Some users may value trusted builds, update reliability, packaging, documentation, and support.
- What would falsify it: Users are unwilling to pay for distribution quality or prefer self-managed builds.
- Validation method: Run pricing and packaging discovery after there is a usable product surface.
- Status: Unvalidated.

## Stress-Test Scenarios

- Hypothesis: Medical, research, multilingual, and cross-border collaboration scenarios are stress-test environments only, not current market commitments.
- Why it may be true: These contexts may expose demanding terminology, privacy, traceability, and review requirements.
- What would falsify it: The project intentionally commits to one of these markets with validated requirements, compliance scope, and product ownership.
- Validation method: Treat these scenarios as evaluation stress tests unless separate discovery establishes a committed market direction.
- Status: Unvalidated.
