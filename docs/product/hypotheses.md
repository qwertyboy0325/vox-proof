Status: exploratory
Owns: Unvalidated market, user, and future-product hypotheses.
Does not own: Current v0.1 scope, architecture commitments, implementation tasks, roadmap promises, or validated claims.
Last reviewed against code: Rust bootstrap exists; no end-to-end VoxProof pipeline behavior has been verified yet

# Product Hypotheses

This document records assumptions that may guide discovery. None of these hypotheses are validated.

Concrete discovery leads are recorded in `docs/discussion/2026-07-09.md`. A lead is a person willing to talk or try the tool; it is not validation.

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

## Language Pack Reuse

- Hypothesis: A Language Pack may reduce repeated correction effort.
- Why it may be true: Specialized terms, aliases, names, and recurring ASR confusions may repeat across related transcripts.
- What would falsify it: Corrections are too one-off, context-dependent, or inconsistent to benefit from reusable language memory.
- Validation method: Analyze authorized transcript sets for repeated correction patterns and measure review burden with and without a Language Pack.
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
