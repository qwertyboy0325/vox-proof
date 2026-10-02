Status: exploratory
Owns: The non-authoritative research question, candidate list, comparison method, and decision boundary for VP-ASR-001.
Does not own: Gate 5 ASR runtime, model, backend, or distribution selection; Material Decisions; product implementation; or the education pilot procedure (`../product/education-lecture-pilot-protocol.md`).
Issue: https://github.com/qwertyboy0325/vox-proof/issues/16

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

| Candidate | Why included | Notes from desk research (2026-10-02, unverified on lecture audio) |
|---|---|---|
| Whisper large-v3 / large-v3-turbo via whisper.cpp or MLX | Stock local baseline | MIT. Biasing is an initial prompt only (about 223 tokens shared with any script hint); no true hotwords. `zh` output switches between Simplified and Traditional. |
| Breeze-ASR-25 (MediaTek, Whisper-large-v2 fine-tune) | Built for Taiwanese Mandarin and code-switching; Traditional output | Apache-2.0. Runs in whisper.cpp via third-party GGML conversions, which are unaudited; prompt-following is unverified. |
| Qwen3-ASR 0.6B / 1.7B via a community MLX port | Context biasing trained in; SRT via forced aligner | Apache-2.0. The official runtime is CUDA-only; the Mac path depends on a community port. The aligner handles at most 5 min per call. Script behaviour is unverified. |
| SeACo-Paraformer (FunASR) | True hotword mechanism | Mainland-trained, Simplified output; CPU on Mac; per-model license must be checked. Secondary candidate. |

Excluded after desk research: FireRedASR2 (60-second input limit, reported
high memory on Apple Silicon, no hotwords); Qwen-Audio-3.0-ASR (no open
weights found). Desk research is a starting point, not evidence.

A 2026-10-02 pipeline smoke test (synthetic TTS audio, whisper.cpp tiny model)
showed that a Traditional-Chinese prompt also switched the output script.
Comparisons must therefore apply the same recorded Simplified→Traditional
conversion to every run, so a script switch is not counted as a biasing gain.

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
