Status: exploratory
Owns: The non-authoritative research question, candidate list, comparison method, and decision boundary for VP-ASR-001.
Does not own: Gate 5 ASR runtime, model, backend, or distribution selection; Material Decisions; product implementation; or the education pilot procedure (`../product/education-lecture-pilot-protocol.md`).
Issue: pending owner authorization

# VP-ASR-001: Local ASR Candidate Comparison

## What this is

An experiment to compare local ASR candidates on real, authorized lecture
audio, first to choose the fixed baseline ASR for `education-pilot-01`, and
second to collect evidence that may later inform Gate 5.

## What this is not

- Not a Gate 5 runtime, model, backend, or distribution selection.
- Not a Material Decision.
- Not implementation authorization for an ASR adapter.
- Not a general ASR benchmark; results describe one course and one speaker.

ASR output remains a non-authoritative observation regardless of which
candidate is chosen.

## Research question

> Which locally runnable ASR candidate gives the best usable baseline for
> Taiwanese Mandarin lectures with Chinese-English code-switching, and does it
> support the vocabulary biasing needed for the pilot's L1 layer?

## Candidates (verify availability before running)

Candidates are starting points, not endorsements. Confirm the current
version, license, and local runtime on the owner's Mac before inclusion.

| Candidate | Why included | Verify |
|---|---|---|
| Whisper large-v3 / large-v3-turbo via whisper.cpp or MLX | Widely used local baseline; prompt biasing via initial prompt | Traditional vs Simplified output; prompt length limits |
| Breeze ASR (MediaTek, Whisper-derived) | Tuned for Taiwanese Mandarin and code-switching | Current release, license, runtime |
| FunASR Paraformer / SenseVoice | Strong Mandarin accuracy; hotword support in some models | Hotword support in the local build; Traditional output |
| One additional local candidate if a credible new option exists | Avoid locking in a stale list | Same checks |

Cloud services are excluded: pilot consent promises local-only processing.

## Criteria

```yaml
primary:
  - CER on reference windows (after a recorded Simplified→Traditional
    conversion where needed; conversion is a presentation transform)
  - vocabulary biasing support (prompt or hotwords) usable for L1
must_have:
  - runs locally on the owner's Mac
  - timestamped output convertible to SRT
  - license compatible with research use
secondary:
  - CER on lexical items (terms, names, numbers, code-switched words)
  - real-time factor on the owner's hardware
  - segmentation quality for review
```

## Method

1. Use the lecture-1 reference windows from `education-pilot-01` (already
   covered by the teacher's authorization).
2. Run every candidate with default settings, then with the confirmed term list
   as prompt or hotwords where supported.
3. Record engine, model, version, settings, and hardware for each run.
4. Compute CER per window and on lexical items; note qualitative failure
   patterns.
5. Pick the pilot baseline by the primary criteria; if two candidates are
   within 2 CER points, prefer the one with usable vocabulary biasing.

Raw audio and transcripts stay under `local/`. Only aggregate numbers go to
`evidence/` after owner review.

## Outputs

- Chosen pilot baseline recorded in `education-pilot-01` `setup.yaml`.
- A comparison summary usable as later Gate 5 input.

## Blocks without separate owner authorization

- selecting a Gate 5 product ASR runtime, model, or distribution;
- implementing an ASR adapter or bundling a model;
- treating ASR output as transcript authority;
- creating a Material Decision from these results.

## Lifecycle

```text
PROPOSED_RESEARCH
→ INVESTIGATING
→ EVIDENCE_COLLECTED
→ OWNER_DECISION_PENDING
→ ACCEPTED_AS_MATERIAL_DECISION
  | REJECTED
  | DEFERRED
  | SUPERSEDED
```

Choosing the pilot baseline does not resolve this item. Resolution follows the
register's resolution rule.

## Current project effect

```yaml
canonical_semantics_changed: false
material_decision_created: false
gate5_runtime_selected: false
repository_implementation_authorized: false
```
