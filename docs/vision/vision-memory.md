# VoxProof Vision Memory

Status: exploratory / non-binding

Date: 2026-07-09

Owns:
Cross-session vision context for product positioning, long-term strategy, technical boundaries, market hypotheses, commercialization assumptions, competition analysis, Knowledge Pack flywheel thinking, and collaboration principles for future agents.

Does not own:
Current implementation scope, accepted architecture, data contracts, Material Decisions, validated market claims, roadmap commitments, or implementation authorization.

Rule:
Nothing in this document authorizes implementation.

This document is an inheritance note for vision discussions. It is not a finished specification. Concrete discovery leads are recorded in `docs/discussion/2026-07-09.md`; long-term possibilities are governed by `docs/vision/possibility-queue.md`; unvalidated assumptions are governed by `docs/product/hypotheses.md`. If this document overlaps those documents, those documents are authoritative.

---

## 1. Core Positioning

VoxProof should not be understood as an ASR tool, and it should not be reduced to an "AI fixes your transcript" wrapper.

The more accurate positioning is:

```text
VoxProof is a local-first, evidence-backed, human-in-the-loop
transcript review and transcript trust layer.
```

VoxProof consumes transcripts, subtitles, or text drafts produced by existing ASR and subtitle tools, such as SRT, VTT, TXT, or similar formats. It turns them into a reviewable, correctable, traceable, and reusable human decision workflow.

Core principles:

- ASR and AI output are drafts, not truth.
- Analyzers and models produce candidates and evidence, not silent edits.
- Human correction decisions are canonical.
- Reviewed output should be derived from source transcript plus accepted decisions.
- Source anchors, time ranges, segment boundaries, and review state should be preserved.
- Each human correction is not only a cost; it can become a reusable correction asset, regression asset, or future review signal.

One-sentence framing:

```text
VoxProof does not make another transcript tool.
It turns transcripts into reviewable, correctable, traceable,
and reusable knowledge assets.
```

## 2. Long-Term Direction: From Post-ASR QA To ASR Ecosystem Control Layer

VoxProof should not start by building an ASR engine.

A healthier long-term sequence is:

1. Post-ASR Transcript QA
2. Reviewed Transcript Workflow
3. Correction Decision Asset
4. Regression / Evaluation Asset
5. Audio-aware Review System
6. ASR Companion / ASR Evaluator
7. ASR Profile / Routing Layer
8. Transcript Trust Ecosystem
9. Optional ASR Execution
10. MCP / agent-accessible transcript trust layer
11. Edge / recorder package integration

VoxProof should start as a post-ASR trust layer. If it matures, it may wrap ASR workflows and eventually become a control plane for an ASR ecosystem.

Potential future inputs:

- Whisper / whisper.cpp
- MacWhisper
- YouTube subtitles
- Zoom / Teams / Google Meet transcripts
- Cloud ASR
- Manual transcripts
- Phone or recorder-generated transcripts

Potential future outputs:

- Reviewed SRT
- Reviewed transcript
- Decision log
- Unresolved review cases
- Glossary / correction assets
- Regression cases
- Review report
- Confirmed-only notes
- MCP-readable resources

## 3. Whether Stronger LLMs Threaten VoxProof

Stronger LLMs can threaten low-level surface features, but they do not necessarily threaten the core product.

Likely to be absorbed by stronger LLMs:

- Ordinary typo correction
- General summaries
- Basic transcript cleanup
- Simple glossary replacement
- Generic question answering
- Low-risk personal note cleanup

Harder for generic LLMs to replace:

- Source identity
- Source anchors
- Time ranges and subtitle boundaries
- ReviewCase state
- Human accepted / rejected / edited decision history
- Reviewed output derivation
- Regression assets
- Correction memory
- Confirmed / inferred / unresolved state distinctions
- Local-first data boundaries
- Audit-friendly MCP tools

Key judgment:

```text
If VoxProof is an AI transcript fixer, it can be replaced.
If VoxProof is a transcript trust, review decision,
and evidence asset layer, stronger LLMs make it more necessary.
```

The long-term role should be to become a trustworthy transcript review engine that ChatGPT, Claude, Gemini, local agents, and future tools can call into. It should not become merely a weaker transcript UI around a general chat model.

## 4. Strategic Value Of Not Depending On External LLMs

VoxProof should not require an external LLM in order to be useful.

That does not mean VoxProof should never use models. The distinction is:

- Core workflow does not depend on external LLMs.
- LLMs are optional.
- Cloud is optional.
- Local models are optional.
- ASR is optional.
- Model output never owns canonical truth.

Without LLMs, VoxProof should still support:

- Parsing
- Validation
- Source anchoring
- Glossary matching
- Phonetic / fuzzy matching
- ReviewCase generation
- Human decision recording
- Reviewed output derivation
- Regression cases

With local models, VoxProof may add:

- Embeddings
- Semantic retrieval
- Reranking
- Explanation hints
- Confirmed-only summaries

With external LLMs or agents, VoxProof can expose MCP-accessible capabilities, but the external model must not own canonical state.

The local-first / LLM-optional posture creates advantages around:

- Privacy
- Offline capability
- No token billing dependency
- Lower recurring cost
- Model vendor independence
- Education, research, and enterprise acceptability
- Edge and hardware integration potential

## 5. Recorder And Edge Device Vision

VoxProof could eventually become part of a recorder, education device, edge device workflow, or review package standard.

Hardware is not current scope.

A plausible hardware division of responsibilities:

Edge / recorder device:

- High-quality recording
- Timestamping
- Metadata capture
- Rough segmentation
- Marker capture
- Keyword / glossary spotting
- Optional small ASR draft
- Export VoxProof review package

Desktop VoxProof:

- Full transcript review
- Evidence display
- Human decisions
- Correction memory
- Reviewed output
- Regression assets
- Reports
- MCP integration

A phone or recorder should not become a miniature desktop application. Its better role is capture, marker entry, lightweight review, and package export.

Possible future VoxProof Review Package contents:

- `audio.m4a` / `audio.wav`
- `draft_transcript.srt`
- `anchors.json`
- `candidate_flags.json`
- `recording_metadata.json`
- `user_markers.json`
- `project_profile.json`

## 6. Mobile Should Not Copy The Desktop Pipeline

Running the full desktop pipeline on mobile would likely produce poor quality and poor user experience.

Constraints:

- RAM limits
- Battery limits
- Thermal limits
- Background execution limits
- High cost of long-audio processing
- Low-density review UI
- Awkward file workflow

More plausible mobile modes:

1. Recorder Companion
   Record classes, interviews, or filming sessions; capture metadata; add markers; export packages.

2. Quick Review Companion
   After a desktop app or model has produced ReviewCases, the phone handles accept / reject / uncertain / quick note / audio snippet playback.

3. Report Reader
   Read confirmed notes, unresolved candidates, review reports, and study material.

The most valuable mobile role is not running full AI on-device. It is capturing clean source, preserving timelines, saving human markers at capture time, and giving later review enough context.

## 7. Weight Of The Full Vision

The full vision can become heavy, but users should not be forced through the heaviest pipeline by default.

The product should support layered profiles.

### Profile A: Core / CLI / Open-source Base

Capabilities:

- Parser
- Validation
- Source anchors
- Glossary / basic phonetic detection
- ReviewCase
- Decision log
- Reviewed output
- Basic report

Approximate weight:

- RAM below 1 GB
- Disk below 200 MB excluding user data
- No GPU
- Fully offline

### Profile B: Lite GUI

Capabilities:

- GUI review flow
- Glossary manager
- Deterministic ReviewCase generation
- Reviewed transcript export

Approximate weight:

- RAM 1-2 GB
- No GPU
- Ordinary laptop viable

### Profile C: Semantic Review Pack

Capabilities:

- Embeddings
- Vector search
- Reranker
- Similar correction lookup
- Semantic candidate ranking

Approximate weight:

- RAM 4-8 GB
- Disk 2-10 GB
- 16 GB RAM laptop reasonable

### Profile D: Local LLM Assistant Pack

Capabilities:

- Explanations
- Bounded alternatives
- Confirmed-only summaries
- Review notes
- Semantic extraction suggestions

Approximate weight:

- RAM 8-16 GB
- Disk 5-20 GB
- Optional GPU/NPU

### Profile E: Audio Pro Pack

Capabilities:

- Audio anchors
- Forced alignment
- Audio snippet playback
- Optional ASR
- Alignment evidence

Approximate weight:

- RAM 8-24 GB
- Disk 10-50 GB+
- GPU/NPU strongly helpful

### Profile F: Full Vision / Workstation

Capabilities:

- Embeddings
- Reranker
- Local LLM
- ASR
- Alignment
- Diarization
- Speaker clusters
- Semantic asset extraction
- MCP
- GUI

Approximate weight:

- 32 GB RAM comfortable
- 30-100 GB disk possible
- Workstation-class experience

Important principle:

```text
Core must remain useful without heavy models.
```

These figures are exploratory planning ranges, not performance commitments.

## 8. Performance And Acceptable User Experience

VoxProof does not need the entire pipeline to finish instantly, but users should see content and start review quickly.

Target experience:

- Import / parse / validation: under 1-3 seconds when practical; long files should still become usable within seconds.
- Basic ReviewCase generation: seconds to low tens of seconds.
- Glossary / phonetic deterministic detectors: seconds to low tens of seconds.
- Embedding / semantic index: can run in background; tens of seconds to a couple minutes may be acceptable.
- Audio alignment / ASR: may be minutes, but must run in background, be cancellable, and support chunking.
- Local LLM: must never block the basic review path.

Bad product flow:

```text
Import
-> Validate
-> Glossary
-> Embed
-> Rerank
-> Audio Alignment
-> LLM
-> Summary
-> finally show UI
```

Better product flow:

```text
Stage 0: show transcript immediately
Stage 1: show parse / validation issues
Stage 2: produce deterministic ReviewCases quickly
Stage 3: build semantic index in the background
Stage 4: add reranked candidates in the background
Stage 5: add audio anchors in the background
Stage 6: optionally add local LLM explanations / summaries
```

Product principle:

```text
The user should not wait for the full AI pipeline before reading the transcript,
handling basic ReviewCases, or recording decisions.
```

## 9. Concrete Leads And Inferred Cohorts

Concrete discovery leads, inferred cohorts, and validation ordering are recorded in `docs/discussion/2026-07-09.md`. This document does not duplicate them to avoid drift.

Additional vision-level observations:

- Lead A is currently the closest lead to a high-frequency commercial content workflow. This lead has expressed willingness to try VoxProof, and the pain is tied directly to work hours and income.
- Lead B reframes the need from transcript correction to subtitle QA. Correct text does not guarantee deliverable subtitles; timestamp, segment boundary, and alignment QA need long-term space.
- Lead C points toward a lecture / teaching transcript review surface and is a natural probe for the Language Pack reuse hypothesis.
- Inferred cohorts must not be presented as validated markets.

## 10. Whether VoxProof Could Be More Mainstream Than LRTimelapse

The potential audience may be larger than LRTimelapse because transcript, lecture, meeting, subtitle, teaching, and content workflows are more common than timelapse photography.

But a larger possible audience does not mean easier selling.

LRTimelapse has visible pain:

- Flicker
- Exposure jumps
- Day-night transition problems
- Lightroom keyframing workflow

VoxProof has more hidden pain:

- The transcript is wrong.
- The summary made unsupported inferences.
- AI correction polluted source text.
- Human corrections were not saved.
- The same error repeats next time.
- Source anchors are missing.
- Review has no decision trail.

This means VoxProof likely has higher market education cost.

Core product education:

```text
Human-in-the-loop is not a cost center.
Human decisions are the most valuable data asset.
```

More precisely:

```text
Humans are not merely kept in the loop.
Humans create the system's most valuable judgment data while in the loop.
```

## 11. Commercialization Hypotheses

Potential models:

- Open-source core
- Paid GUI
- Paid stable installers
- Paid validated model profiles
- Free verified student tier
- Education / lab license
- Studio / team license
- Enterprise / private deployment
- MCP / agent integration tier

Open-source core could include:

- Parser
- Source-preserving transcript model
- SourceAnchor
- CandidateSpan
- Evidence
- ReviewCase
- CorrectionDecision
- ReviewedOutput
- Deterministic detectors
- CLI
- Open formats

Paid layers could include:

- Polished GUI
- Installer
- Batch workflow
- Glossary workspace
- Validated model profiles
- Local model packs
- Education / subtitle workflow templates
- Team review
- Private deployment
- MCP server / controlled tools
- Support

Commercial value should not depend on "the model is smarter." It should come from:

- Workflow depth
- Trusted data semantics
- Review UX
- Correction assets
- Local-first packaging
- Domain packs
- Evaluation / regression assets
- User habits
- Open-core trust

## 12. Competition, Copying, And Acquisition

Competition will be broad and will likely increase.

Sources of competition:

1. ASR / meeting note tools
   Otter, Fireflies, Zoom, Google, Microsoft, and similar tools may move toward meeting knowledge, agents, and transcript workflow.

2. Content / subtitle tools
   Descript, Trint, Sonix, Rev, subtitle editors, and similar products may add AI cleanup, glossary features, and review UI.

3. Research / open source
   Projects may emerge around ASR correction memory, ontology-augmented correction, technical term recall, Mandarin lecture glossary workflows, and related ideas.

4. LLM / MCP platforms
   Model platforms may build transcript review, agent access, and knowledge-base features into their own ecosystems.

Easy to copy:

- AI transcript cleanup
- Glossary matching
- Phonetic matching
- Basic SRT import/export
- Review UI
- MCP server
- Summary
- Speaker label correction

Harder to copy:

- Source-preserving data model
- Stable ReviewCase / Evidence / CorrectionDecision semantics
- Human decision asset flywheel
- Regression workflow
- Local-first project / package format
- High-quality review UX
- Domain-specific model/profile packs
- Education / subtitle vertical workflow
- MCP-safe low-ambiguity tools
- Open-core adoption
- Accumulated user review assets

Judgment:

Early features are easy to copy. Acquisition becomes plausible only after there is workflow adoption, users, community, data format traction, revenue, MCP ecosystem value, or vertical traction.

Possible acquirers:

- ASR / meeting note companies
- Education technology companies
- Content creation tools
- Subtitle / localization tools
- Knowledge management tools
- Recorder / education hardware companies
- AI workflow / agent tooling companies

## 13. Fit With The Creator's Working Style

VoxProof appears well matched to the creator's working style, but not unconditionally.

Observed or hypothesized traits:

- Backend / platform / production reliability background
- Comfort handling untrusted input, untrusted system state, and untrusted model output
- Ability to compress ambiguous problems into bounded slices
- Prior proof of scope control in messy production-adjacent systems
- Ability to turn external ambiguity into engineering pressure, not only write scope documents
- Product thinking that is systems-oriented rather than consumer-growth-oriented
- Strength in error cost, workflow trust, long-term asset formation, and data semantics
- Discomfort with being boxed into a narrow backend label
- Potential for VoxProof to become a representative artifact for AI Application Systems Engineering or Local-first AI Product Engineering

Important correction:

It would be inaccurate to say the creator lacks product thinking. A more precise statement is:

```text
The product thinking lives in architecture, workflow, trust boundaries,
and error cost, not in marketing language or UI trends.
```

## 14. Prior Governance Experience And VoxProof Governance

Prior production-adjacent work demonstrated strong scope control.

VoxProof should not copy heavy governance from production remediation contexts.

That kind of project may involve legacy systems, production-adjacent infrastructure, runtime topology, multi-service remediation, and heavy PR governance. VoxProof is an early-stage product, Rust learning project, and domain model exploration. It should not make every small change go through heavyweight process.

Better posture:

```text
Borrow the slicing discipline.
Do not borrow the governance weight.
```

Keep:

- Bounded slices
- Explicit non-goals
- Validation habit
- Prevention of future-scope smuggling

Do not keep:

- Heavy scope authority for every PR
- Overly detailed failure classification
- Production-remediation writing style
- Full governance ceremony for every small change

Appropriate VoxProof modes:

- Ordinary code change: do it directly, protected by tests.
- Active wedge: write a short note with goal, in scope, out of scope, and validation.
- Material boundary: use a formal Material Decision.
- Future vision: keep it in the Possibility Queue or other explicitly non-binding vision documents.

Current assessment of Material Decisions:

- They should govern changes to product or architecture invariants.
- Ordinary naming, local refactors, test fixtures, and implementation details that do not change durable direction do not require a Material Decision.
- Future ideas must not become approved roadmap merely because they were written down.

## 15. Rust And VoxProof

VoxProof is one of the creator's first Rust projects, and learning Rust is one goal.

More importantly:

The creator appears to have chosen Rust first, then selected a product problem where Rust's constraints and strengths can become product value.

This means VoxProof is not merely "implemented in Rust." It is a project whose product problem was selected to match Rust's advantages.

Rust / VoxProof alignment:

- Ownership and immutability tendencies -> source transcripts should not be silently damaged.
- Strong types -> Raw / Parsed / Validated / Reviewed states can be separated explicitly.
- `Result` and error handling -> parsing, validation, and evidence failures can be represented directly.
- Native and local-first execution -> desktop core, CLI, edge, and SDK directions are plausible.
- Safe systems language -> long-term trusted core is plausible.
- Enums, newtypes, and domain types -> ReviewCase, Evidence, CorrectionDecision, and TranscriptRevisionId boundaries can be protected.

This is not Rust for display. It is a deliberate choice of a problem where Rust's type system, safety, state transitions, and local execution can become product value.

Risks:

- Rust learning must not replace product closure.
- A beautiful domain model must not become over-abstraction.
- Each Rust concept learned should protect a real VoxProof invariant when possible.

## 16. Current Implementation Wedge

Do not build now:

- ASR
- Audio runtime
- Diarization
- Speaker identity
- Tauri GUI
- MCP mutation tools
- Hardware
- Local LLM
- Model packs
- Mobile app
- Full subtitle alignment engine

The current minimum loop should remain:

```text
existing transcript
-> source-preserving parse
-> validation
-> CandidateSpan
-> Evidence
-> ReviewCase
-> human accept / reject / edit
-> CorrectionDecision
-> reviewed output
-> decision log
-> regression case
```

After this loop works, consider:

- Semantic retrieval
- Audio anchor
- Subtitle timing QA
- GUI
- Local model profile
- MCP
- Edge package format

## 17. Instructions For Future Collaboration Models

Do not treat this document as current implementation scope.

When helping with VoxProof:

1. Separate verified fact, user-reported real case, inference, and future possibility.
2. Do not present inferred cohorts as validated markets.
3. Do not convert long-term ecosystem vision directly into roadmap.
4. Do not recommend ASR, LLM, audio, MCP, mobile, or hardware work now.
5. Before suggesting implementation, ask whether it serves the current closed loop.
6. Model output can only be candidate or evidence, not truth.
7. Human decision is canonical.
8. UI is an interaction layer, not domain authority.
9. Core should remain useful without heavy models.
10. Material Decisions should be used only for real invariant changes, not for over-engineering.

## 18. Product Flywheel, Accumulated Assets, And Knowledge Packs

Canonical automation and promotion semantics are owned by `docs/product/correction-system-boundaries.md`, not by this exploratory vision. In the flows below, a preserved decision may inform a policy or reusable-asset suggestion; it does not automatically become reusable policy or enter a pack. Promotion requires explicit user authorization and scope.

VoxProof's long-term value is not only fixing one transcript or subtitle file. Each human review can produce an accumulated asset.

Core flywheel:

```text
Raw transcript / subtitle
-> VoxProof raises ReviewCases
-> human accepts / rejects / edits
-> CorrectionDecision is stored
-> decision becomes reusable asset
-> reusable assets form Knowledge Pack / Domain Pack
-> future review becomes faster and more accurate
-> user trusts the system more
-> user reviews more material
-> more decisions accumulate
-> pack becomes more valuable
```

This is the difference between VoxProof and ordinary AI transcription or subtitle cleanup tools.

Ordinary tool flow:

```text
AI produces output
-> human fixes it
-> export
-> done
```

VoxProof target flow:

```text
AI / rule / detector proposes candidate
-> human makes canonical decision
-> decision is preserved
-> decision becomes correction memory / regression case / glossary signal
-> next review benefits directly
```

In other words:

```text
Human-in-the-loop is not a cost center.
Human decision is the product's most important data asset.
```

### What A Knowledge Pack / Domain Pack Might Contain

A VoxProof Knowledge Pack should not be only a glossary. Over time, it could contain:

- Confirmed glossary terms
- Rejected false positives
- Accepted corrections
- Common ASR confusions
- Speaker / course / project-specific wording patterns
- Subtitle timing preferences
- Style guide rules
- Domain-specific terminology
- Regression cases
- Model/profile evaluation notes
- Confirmed source-backed snippets
- Unresolved ambiguous cases
- Export / delivery preferences

Different scenarios can have different pack shapes.

#### Course / Lecture Pack

For teachers, students, classes, and lectures.

Potential contents:

- Course terminology
- Instructor-specific wording
- Mixed Chinese-English terms
- Confirmed corrections
- Important markers
- Reviewed lecture transcript
- Confirmed-only notes
- Regression cases

Value:

The more a course is reviewed, the easier later transcripts become to correct, and the more trustworthy study material becomes.

#### Creator / Channel Pack

For YouTube, podcast, and similar creator workflows, including Lead A-like workflows.

Potential contents:

- Channel-specific place names
- Shop names
- Person names
- Brand names
- Catchphrases
- Common ASR mistakes
- Subtitle formatting preferences
- Confirmed correction history
- Delivery / export preferences

Value:

The more videos from the same channel are reviewed, the faster subtitle QA becomes and the fewer repeated errors remain.

#### Subtitle / Film Pack

For film subtitle correction, translation, and subtitle delivery.

Potential contents:

- Character names
- Proper nouns
- Translation preferences
- Subtitle length limits
- Timing QA rules
- Common timing drift patterns
- Accepted / rejected subtitle edits
- Segment boundary decisions
- Regression cases

Value:

Not only text corrections accumulate; timing and delivery rules can also accumulate.

#### Research / Interview Pack

For graduate students, interview cleanup, and field research material.

Potential contents:

- Interviewee-specific wording
- Project terminology
- Confirmed transcript corrections
- Source anchors
- Ambiguous terms
- Unresolved review cases
- Confirmed excerpts
- Coding / analysis preparation metadata

Value:

Research material is not merely transcribed. It becomes traceable, reviewable, and citable.

#### Organization / Team Pack

For corporate training, technical sharing, and knowledge management.

Potential contents:

- Internal product names
- Team terminology
- Person / project names
- SOP vocabulary
- Confirmed knowledge snippets
- Reviewed transcript corpus
- Decision audit log
- Agent-readable confirmed resources

Value:

This helps prevent unsupported AI summaries from polluting knowledge bases and lets organizations pass only confirmed, source-backed material into downstream knowledge workflows.

### Product Meaning Of Knowledge Packs

Knowledge Pack is one possible long-term moat.

It turns VoxProof from a one-off tool into an accumulated workflow:

```text
First use: it helps review one transcript.
Tenth use: it starts understanding a course, channel, or domain.
Hundredth use: it has become transcript correction memory.
```

This value is not easily replaced by a single LLM call.

An LLM can be intelligent, but it does not automatically know:

- Which corrections were confirmed by a human
- Which candidates were previously rejected
- Which terms belong to this course, channel, or project
- Which subtitle timing rules are delivery preferences
- Which outputs are source-backed
- Which content is inferred or unresolved

VoxProof's value is preserving those states.

### Knowledge Packs And Commercialization

Knowledge Packs can also become part of commercial layering.

The open-source core can support the basic format and local packs.

Paid / Pro / Team layers could offer:

- Polished Knowledge Pack manager
- Pack import / export
- Pack versioning
- Pack diff / audit
- Team-shared packs
- Education course packs
- Subtitle delivery packs
- Validated model profiles paired with packs
- Pack-based regression evaluation
- MCP access to confirmed pack assets

This is healthier than selling only "AI correction."

The product should not sell:

```text
Our AI is smarter than everyone else's.
```

It should sell:

```text
Every review makes your course, channel, project,
or team knowledge pack more reliable.
```

### Knowledge Pack Boundaries

A Knowledge Pack should not become black-box automatic learning.

Important boundaries:

- Only human-confirmed decisions enter canonical packs.
- Model suggestions cannot directly write trusted assets.
- Rejected decisions should be preserved to avoid repeated false positives.
- Packs should be inspectable, exportable, retractable, and versioned.
- Packs should not silently contaminate other projects unless the user explicitly allows it.
- Speaker/person identity data should be addressed very late, with stronger privacy and confirmation boundaries.

Knowledge Pack is not hidden model training. It is an inspectable asset layer produced by human-reviewed transcript workflows.

### Summary

VoxProof's product flywheel:

```text
The more you review, the better VoxProof understands your material.
The better it understands your material, the faster review becomes.
The faster review becomes, the more trustworthy assets accumulate.
The more trustworthy assets accumulate,
the harder VoxProof is to replace with an ordinary AI transcript tool.
```

This flywheel should be treated as one of VoxProof's core long-term value hypotheses.
