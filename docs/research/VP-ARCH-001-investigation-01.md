Status: exploratory
Owns: The record of the first read-only VP-ARCH-001 investigation (`VP-ARCH-LOOSE-INFERENCE-STRICT-COMMITMENT-INVESTIGATION-01`), its findings, and its slice recommendation.
Does not own: The research question and boundaries (`VP-ARCH-001-loose-inference-strict-commitment.md`); accepted semantics; Material Decisions; implementation authorization.
Issue: https://github.com/qwertyboy0325/vox-proof/issues/1

# VP-ARCH-001 Investigation 01 — Detection Value Lens

Date: 2026-10-02. Authorized as read-only by the owner under the 2026-10-02
direction adjustment, which prioritizes detection value. Code references are
to commit `101b486` plus the owner's uncommitted recording work, which this
investigation did not read or rely on.

This record is non-authoritative. It changes no semantics and authorizes no
implementation.

## Method

- Read the research memo, accepted boundaries, and the candidate, review,
  projection, reuse, experimental, and persistence-identity code.
- Ran one synthetic dry run (TTS audio, whisper.cpp tiny, `vox-proof evaluate`)
  in `local/education-pilot-01/dryrun/`: lecture-1 errors were written as
  `error:` session terms and applied to a second synthetic lecture.

The dry run is scaffold-level evidence: synthetic speech, one tiny model, one
text. It illustrates mechanisms; it does not measure product recall.

## 1. Existing semantic boundary map

```text
detector (deterministic, closed set)
  → CandidateSpan  (src/candidate.rs:218; constructor pub(crate) :228)
      Evidence: closed enum of 4 variants (src/candidate.rs:189)
  → ReviewCase::DetectorRaised wraps one CandidateSpan (src/review.rs:76-107)
  → ReviewLedger append-only events (src/review.rs:248)
      CorrectionDecision: Reject | Defer | AcceptAlternative{index}
                          | NeedsManualCorrection | ManualReplacement
  → derive_reviewed_srt_with_proposals (src/reviewed_output.rs:42)
      = transcript + re-derived ReviewCases + ledger
        + persisted ReuseProposalTargets + ProjectTerminologyProposalTargets

frozen proposal families (decidable, outside ReviewCase lists)
  ReuseProposalTarget (src/reuse_proposal_target.rs:75) freezes occurrence,
  proposed_replacement, snapshot and detector/config/algorithm identities.
  ProjectTerminologyProposalTarget follows the same pattern.

experiment-only sidecar
  experimental_retrieval / experimental_ranking (incl. Han pinyin distance)
  → JSON sidecar; a user selection only records "add an alias and rerun"
    (src/main.rs:395-411); never a ReviewCase, Evidence, or ledger event.
```

Authoritative matching is exact and case-sensitive for aliases and observed
error forms (src/candidate.rs:549-553, "non-identity normalization is still an
open decision gate"). The only non-exact authoritative detector is ASCII-Latin
phonetic similarity; it rejects non-ASCII input (src/phonetic.rs:299). Han
pinyin matching exists only in the experiment (src/experimental_retrieval.rs:6).

## 2. Inference / authority coupling findings

- **C1 — Index-based acceptance couples authority to re-detection.**
  `AcceptAlternative { alternative_index }` resolves its text from the
  re-derived ReviewCase at projection time (src/reviewed_output.rs:110-122).
  This is sound only because detectors are deterministic under a fixed
  analysis identity. A non-deterministic (model) producer cannot use this
  path without breaking rebuild closure.
- **C2 — "Reviewable" is coupled to "canonical Evidence".** To appear as a
  ReviewCase, a producer must add an `Evidence` variant, a detector identity
  in a static set, config/algorithm allowlists, persistence restore checks
  (src/session_persistence/canonical.rs:555-596), and detector-snapshot
  mapping (src/detector_snapshot.rs:125). Asking a human to look at a
  suspicious span therefore currently costs the same governance as declaring
  new canonical evidence semantics.
- **C3 — Recall limits are partly policy, not technique.** Han fuzzy
  matching already exists but is experiment-only; normalization is an open
  decision gate.
- **C4 — The sidecar cannot be acted on.** A high-recall experimental
  finding can only become a decision after the user edits session terms and
  reruns, so its detection value cannot be observed in normal review.

The frozen proposal families already decouple inference from authority: the
proposal is persisted, the decision references the frozen target, and
projection never re-runs inference. This is the existing repo pattern closest
to "loose inference, strict commitment".

## 3. Concrete failure cases

From the synthetic dry run (lecture-1 knowledge applied to lecture 2;
`tools/reuse_m3.py`): 1 repeated error proposed and matching the human final,
6 repeated-term errors missed.

| ID | Case | Raw (ASR) | Needed | Why missed |
|---|---|---|---|---|
| F1 | ASR surface variance | 踢度下架 | 梯度下降 | lecture-1 error form was 提讀下降; exact match only |
| F2 | Han near-homophone | 收獵 | 收斂 | no Han phonetic path on authoritative side |
| F3 | Latin term misheard as Han | 白頭之 | PyTorch | no cross-script similarity anywhere |
| F4 | Distorted Latin | TMIcer | optimizer | ASCII phonetic gates (DoubleMetaphone overlap, ratio ≥ 500‰) not met |
| F5 | Case difference | Loss curve | loss curve | alias `Loss Curve` is case-sensitive |
| F6 | Valid-word substitution | 下中 / 猜述 | 下週 / 參數 | needs context; no term involved |

F1–F5 are reuse-recall failures that a fuzzy or model producer could address;
F6 needs contextual inference. Both are consistent with the documented risk
that ASR surface-form variance limits exact-pair recall.

## 4. Belief-layer necessity test

For the detection-value question, a generic `SynthesizedBelief` layer is
**not necessary**. The required properties — source-linked, multiple
competing alternatives, producer identity, non-authoritative, frozen before
decision — are already expressed by the frozen proposal-target pattern.
Temporal scope, supersession, expiry, and cross-source conflict are not
exercised by single-material review and remain open for cross-material or
continuity work. Verdict: defer the belief layer; do not resolve it.

## 5. Persistence capability matrix

| Capability | Status | Basis |
|---|---|---|
| Append-only decisions | present | ReviewLedger events; MD-020 |
| Frozen non-deterministic proposals | present for reuse/terminology families | persisted targets with identity hashes |
| New proposal family without schema change | absent | closed ledger event enum; MD-020 schema |
| Revocation / rematerialization | absent | correction-system-boundaries: unresolved, not implemented |
| Temporal scope, expiry, supersession | absent | not represented |
| Producer identity for a model (model id, version, prompt/config hash) | absent | identity allowlists are static |

Persistence detail beyond these points was not re-verified in this
investigation.

## 6. Policy auto-accept authority matrix

No policy may currently authorize a decision. Any future policy must emit an
explicit, occurrence-attributable decision (memo; correction-system-boundaries
Semi-Automation). Classes and evidence gates remain owned by
`correction-system-boundaries.md`; this investigation adds no class. A model
producer does not change this: its proposals would be review-required.

## 7. Deterministic rebuild closure

Current closure:

```text
projection = f(transcript, session terms + analysis identity → re-derived
               ReviewCases, ledger, persisted proposal targets)
```

For any model producer, closure holds only if:

- its proposals are persisted as frozen targets (occurrence + replacement
  text + producer identity) before any decision;
- decisions reference the frozen target, never an index into a re-run;
- hydration and projection never invoke the model.

## 8. Minimum vertical-slice recommendation

**Stage 0 — measure, no core change (recommended next).** Add a model/fuzzy
producer to the experiment sidecar only, run it on `education-pilot-01`
material, and measure how many of F1–F6-type misses it would surface and how
much noise it adds, against the human-final reference. This answers whether
detection value exists before any semantics are touched.

**Stage 1 — only if Stage 0 shows value.** A new frozen proposal-target family
(modeled on `ProjectTerminologyProposalTarget`) for model/fuzzy proposals:
review-required, persisted before decision, own ledger event, producer
identity hashed into target identity. This needs a Material Decision because
it adds a ledger event, persistence shape, and evidence semantics.

Rejected for now: adding a model producer as an authoritative detector
(C2 cost, breaks C1 closure); a generic belief layer (Section 4).

## Stage 0 status (2026-10-02)

The owner authorized Stage 0. Implemented, experiment-only:

- `ExperimentalProducer::HanPinyinSlidingWindow` in
  `src/experimental_retrieval.rs` (retrieval version 0.4): all-Han canonical
  terms compared against equal-length sub-windows of Han runs, toneless pinyin
  with bounded heteronyms, distance ≤ max(1, target letters / 4) capped by the
  configured pinyin distance. Off by default; `review-experiment` behaviour is
  unchanged.
- `vox-proof experiment-retrieve <input.srt> <session-terms.txt> <report.json>`:
  non-interactive, writes candidate reports only (schema
  `experimental-retrieval-only-v1`), sliding window enabled, refuses to
  overwrite.

On the synthetic dry run, the exact path surfaced 1 of 7 needed term
corrections and exact plus experimental surfaced 4 of 7 with no unrelated
candidates; F3 (cross-script), F4 (distorted Latin), and F5 (case) remain
missed. This is scaffold-level evidence only. The Stage 0 measurement is the
same comparison on authorized `education-pilot-01` lectures. A local-model
producer for F3/F6 is not yet implemented because no local model is loaded.

The Stage 0 results packet will support an architecture decision and therefore
qualifies for the strong final conflict review defined in
`.cursor/rules/voxproof-work-packages.mdc` before any Stage 1 decision.

## Owner decisions this investigation surfaces

1. ~~Authorize Stage 0~~ — authorized 2026-10-02.
2. Whether case-insensitive or normalized matching (F5) should be opened as
   its own normalization decision, independent of model work.
3. Whether Han phonetic similarity should be evaluated for the authoritative
   path, or stay experiment-only until Stage 0 evidence exists.

No answer is assumed here.
