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

## Detection Value Precedes Authority Value

- Hypothesis: Users adopt VoxProof first because it finds consequential errors
  they would otherwise miss; attributable decision history is valued only after
  that detection value is felt.
- Why it may be true: Competing tools sell speed and reduced reading. Authority
  semantics are not visible until a user needs to audit, quote, or defend a
  transcript.
- What would falsify it: Users with high accountability cost value decision
  history and review receipts even when detection recall is no better than
  their existing workflow; or higher-recall proposals mostly add noise that
  increases review time.
- Validation method: In the assisted pilot, record which surfaced issues the
  participant confirms as real and would have missed, the proportion of
  surfaced proposals rejected as noise, and which value the participant names
  unprompted.
- Status: Unvalidated. Recorded under the 2026-10-02 owner direction
  adjustment in `strategic-direction.md`.

## Accountable Graduated Automation as Differentiator

- Hypothesis: Users who distrust fully automatic correction, and are tired of
  fully manual review, will prefer automation that is narrowly authorized,
  attributable per occurrence, and revocable.
- Why it may be true: Agent human-in-the-loop practice is converging on
  risk-tiered approval with earned autonomy; transcript tools mostly offer
  either full automation or none.
- What would falsify it: Users either accept unattributed automatic correction
  as good enough, or refuse any automatic change regardless of authorization
  and audit; or the shadow-evaluation and context-break oracles required by
  `correction-system-boundaries.md` show unacceptable false acceptance.
- Validation method: First ask pilot participants how they would want repeated
  corrections handled; only later, and only through the evidence sequence in
  `correction-system-boundaries.md`, evaluate a policy in shadow mode.
- Status: Unvalidated. This hypothesis does not authorize any automation
  runtime, policy schema, or Material Decision.

## High-Accountability Verbatim Domains

- Hypothesis: Domains where a wrong transcribed word carries legal,
  professional, or reputational cost may pay more for audio-grounded,
  attributable review than general interview users.
- Candidate workflows: journalism quote verification; legal statements and
  depositions; regulated-industry call or meeting records; council and formal
  meeting minutes; ethics-board-governed research interviews.
- Why it may be true: These workflows already require someone to stand behind
  the record, often cannot send recordings to cloud services, and frequently
  involve Chinese or mixed Chinese-English material poorly served by default
  tools.
- What would falsify it: These users require certified human transcription,
  formal compliance features, or integrations that a local-first tool cannot
  reasonably provide; or they show no greater willingness to pay than general
  interview users.
- Validation method: Discovery interviews within one or two candidate
  workflows before any feature commitment; compare stated error cost,
  privacy constraints, and current verification practice against the interview
  wedge.
- Status: Unvalidated. Not a market commitment. Medical and clinical decision
  use remains a non-goal; other regulated settings remain stress-test
  scenarios until separately committed.

## Agent-Facing Governed Commit Gate

- Hypothesis: Exposing VoxProof as the commitment boundary for proposals made
  by external AI agents (for example through MCP) makes the long-term authority
  thesis legible to users and investors more effectively than transcript-only
  demonstrations.
- Why it may be true: Agent workflows increasingly need an approval and audit
  step before AI output becomes a record.
- What would falsify it: Observers still read the product as a subtitle tool,
  or agent-submitted proposals add review burden without detection value.
- Validation method: Show a bounded agent-proposal flow alongside the
  fundraising demonstration and record whether observers can restate the
  authority thesis.
- Status: Unvalidated. No agent interface, protocol, or runtime is authorized.

## Education: Speaker-Scoped Recognition, Verbatim Meaning, and Three Roles

```yaml
status: CANDIDATE_VALIDATION_HYPOTHESIS
recorded: 2026-10-02
strategic_direction: candidate_wedge (see strategic-direction.md)
wedge_selected: false
implementation_authority: false
```

### Hypotheses

1. **Speaker- and course-scoped recognition.** A user who repeatedly hears the
   same speaker in the same setting wants recognition quality that stays
   consistent for that speaker's accent, verbal habits, and coined terms.
   The value is not that a model "learns", but that the same mistake does not
   recur and the user can inspect, adjust, and revoke what was learned.
2. **Verbatim plus meaning.** Users want the speaker's original words preserved
   and, beside them, an explanation of what the words mean: a same-language
   gloss for dialect, verbal habits, and coined terms, and a cross-language
   translation. Verbal habits and coined terms are speaker characteristics,
   not errors.
3. **Three roles, scoped authority.** Students, teachers, and schools can share
   one engine if authority follows scope: a student's correction affects only
   the student unless proposed to and confirmed by the teacher; teacher-confirmed
   knowledge applies to the course; the school sets privacy, deployment, and
   recording policy.
4. **Free student edition.** Local processing keeps the marginal cost of a
   student user near zero, making a free or low-cost student edition viable.
5. **Personal unfamiliarity signal.** Repeated corrections a student marks as
   unfamiliar can seed a rough personal knowledge map grounded in the reviewed
   verbatim record and course materials.

Why it may be true: a semester of lectures repeats one speaker and one domain,
which favours scoped reuse more than one-off material; lecture captioning for
accessibility is an institutional need with accuracy expectations; and
default ASR handles accents, code-switching, and coined terms poorly.

### Boundaries this hypothesis must preserve

- The verbatim record is the authoritative layer. Glosses and translations are
  derived and attached to verbatim spans; they never replace them.
- AI-inferred meaning must remain visibly distinct from teacher-confirmed
  meaning. A wrong gloss presented as confirmed is a worse failure than a
  misrecognized word.
- Student corrections never become course knowledge without explicit teacher
  confirmation.
- Recording requires authorization from the school and teacher; the student
  edition should consume authorized course recordings rather than encourage
  unilateral capture. Recording authority remains owned by MD-024.
- The knowledge map is a separate downstream product built on reviewed
  records, not a summary feature of VoxProof itself.
- Acoustic model adaptation (fine-tuning on a speaker's confirmed audio) is a
  later and separately governed option; vocabulary biasing and post-recognition
  correction rules should be measured first.
- Live classroom mode remains deferred; this hypothesis concerns post-lecture
  review.

These boundaries record intent for discovery only. How glosses, translations,
and role scopes are represented, and whether they change projection or
persistence semantics, is unresolved and requires owner decisions.

### Validation shape

Use one course: three to four recorded lectures by the same teacher, with
recording authorized by the teacher and the school. The operational protocol
is [`education-lecture-pilot-protocol.md`](education-lecture-pilot-protocol.md).

1. Transcribe all lectures with the baseline ASR. The teacher reviews lecture 1
   and confirms terms, verbal habits, and their meanings.
2. Apply lecture-1 knowledge to lectures 2–4, first as vocabulary biasing, then
   as post-recognition correction proposals.
3. Measure:
   - error rate on lectures 2–4 against human-reviewed references, baseline
     versus each reuse layer;
   - the share of errors that are lexical (addressable by vocabulary or rules)
     versus acoustic (requiring model adaptation);
   - repeated corrections avoided;
   - accuracy of AI-inferred glosses and translations as judged by the teacher;
   - for a student reviewer, how many corrections the student could confirm,
     could not confirm, or marked as unfamiliar.

The owner is a student with access to teachers and a school, which makes
authorized material easy to obtain. The student-side measurement must include
at least one student who is not the teacher, because a teacher already knows
the terms and would hide the "who can confirm" problem. The owner may serve as
that student for an enrolled course, but owner use is dogfooding evidence, not
external validation: the owner knows the product and its intent. External
student participants are still required for any usability or adoption claim.

What would falsify it:

- most errors are acoustic, and vocabulary or rule reuse does not reduce
  repeated errors across lectures;
- students cannot confirm enough corrections even with course materials and
  teacher-confirmed knowledge;
- teachers will not spend the time to confirm terms and meanings;
- AI-inferred glosses are wrong often enough that showing them harms learning;
- schools or teachers will not authorize recording.

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
