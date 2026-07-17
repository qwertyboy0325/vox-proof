Status: current
Owns: Quality expectations, fixture principles, ground-truth distinctions, metrics, and regression expectations.
Does not own: Product scope, architecture, implementation tasks, benchmark results, or validated performance claims.
Last reviewed against code/evidence: Track 1 local code loop and raw-versus-final comparison/change inventory for strict skeleton-compatible inputs exist. Two exploratory real-material bounded ASCII-Latin phonetic mechanism probes have exercised zero-candidate and reference-supported emitted-candidate paths. v0.1 is not established; effectiveness, threshold, facilitated-user, and product validation remain pending.

# Quality and Evaluation

No benchmark or validated performance result exists yet.

## Provisional Calibration Comparison

Implemented local command:

```text
vox-proof compare <raw-input.srt> <final-input.srt> <comparison-report.json>
```

Boundary:

- schema revision: `voxproof-calibration-comparison-v0`
- compatibility policy: `identical-cue-count-index-and-timing-v0`
- provisional calibration artifact only; not canonical Evidence, not ground truth, and not a Material Decision
- both inputs must parse with zero validation issues and identical cue count, cue index, `start_ms`, and `end_ms` at every `segment_position`
- incompatible structure fails closed with a deterministic refusal and no report file
- per-cue records use only `unchanged` and `text_changed` with exact parsed Unicode text preserved
- no precision, recall, correctness, accuracy/formatting/editorial classification, or product-performance claims
- mismatched segmentation or timing requires a future compatibility policy; v0 does not align cues

## Exploratory Real-Material Phonetic Mechanism Probes

Classification: exploratory real-material mechanism observations only. They are not benchmarks, product validation, threshold validation, precision/recall evidence, human correctness adjudication, or general ASR claims.

The strongest permitted aggregate conclusion is:

> The committed bounded ASCII-Latin phonetic producer has exercised both a zero-candidate real-material path and a reference-supported emitted-candidate real-speech path. These observations support mechanism viability only; effectiveness, thresholds, precision/recall, and v0.1 establishment remain unresolved.

### Probe A: AMI ES2004d zero-candidate path

- Input was an immutable real AMI meeting-ASR transcript with 258 cues and four frozen session-term entries.
- The authoritative canonical pipeline completed with 0 exact-alias, 0 observed-error-form, and 0 phonetic ReviewCases.
- Reviewed output remained byte-identical to the raw input.
- Permitted conclusion: the committed pipeline can complete without forcing candidates on this material.
- The absence of candidates does not establish that every cue was correct or establish a zero false-positive rate, precision, recall, correctness, or product effectiveness.

### Probe B: FLEURS en_us target-positive path

- Source was official `google/fleurs`, configuration `en_us`, split `train`, pinned at revision `ab93cf03f9d0cd083c853fad065a6377067408aa`.
- Before audio or ASR access, the frozen targets were `Google Translate`, `Microsoft`, `MySpace`, `Yahoo`, `ASUS`, and `Apple`.
- A deterministic metadata rule selected all 15 rows whose references contained a frozen target, totaling 152.28 seconds. Selection was intentionally target-conditioned and is not an unbiased corpus sample.
- Local Whisper `small.en` and its inference configuration were fixed before inference. No human audio listening or audio-based human adjudication occurred.
- The skeleton-compatible raw/reference pair contained 15 cues: 3 `unchanged` and 12 `text_changed`.
- The authoritative review pipeline emitted exactly one phonetic ReviewCase: observed surface `ASIS`, canonical target `ASUS`, edit distance 1, `ratio_permille` 750, and matched key `ASS`.
- The frozen FLEURS reference contains `ASUS` at the deterministically localized compatible position. This is a **reference-supported real-speech positive-path mechanism observation**, not a human correctness label or audio adjudication.
- A synthetic reject control was used only to render and complete the probe. It is not a human judgment; reviewed SRT remained byte-identical to raw.
- Two runs produced identical candidate content.
- The 12 `text_changed` cues are not a detector-opportunity denominator. No `1/12` recall, accuracy ratio, or similar effectiveness value may be computed from this probe.

Local evidence identifiers:

- package: `voxproof-fleurs-en-positive-control-24363d9-20260717-final.zip`
- package SHA-256: `d23bf1b0fc84b7276b98668de660639d500221e44ca30c22fdfd2c57250ee46b`
- seal: `package-seal-fleurs-en-positive-control-24363d9-20260717-final.json`
- seal SHA-256: `05a84e53e859120727118117d9b8b979779d2f95d45b96b4146186b2c16f4164`

These package and seal artifacts are local untracked research artifacts; this document does not imply that they are available from the public repository.

Prohibited interpretations include a true-positive rate, precision or recall, human correctness adjudication, product validation, threshold validation, proof of general effectiveness, or a claim that synthetic reject controls are human decisions.

### Canonical-only session-term provenance

Canonical-only entries are now represented natively with empty alias and observed-error-form collections. Prefixed `alias:` and `error:` fields remain required only for non-canonical source forms. The sealed FLEURS probe ran before this support existed and therefore used self-referential `alias:<canonical>` entries solely for then-required syntactic compliance; no alternate observed forms were invented. Those aliases remain historical sealed-study provenance, not the current required input form, and this implementation change does not rewrite or reinterpret the sealed study or authorize a persistence schema.

## Fixtures

Future fixtures should use authorized short mixed Chinese-English transcript samples. Fixtures should be small enough to review manually and explicit about rights to use the transcript and any related audio.

## Ground Truth

Ground truth must distinguish:

- Actual transcript errors.
- Acceptable variants.
- Terminology corrections.
- Edits that alter meaning.

## Evaluation Category Boundaries

Future evaluation must distinguish:

- accuracy restoration;
- representation and formatting normalization;
- disfluency cleanup;
- editorial transformation;
- human-facing output quality;
- machine-facing output utility.

Results from one category must not be reported as evidence for another. In particular, a readable output is not proof of source accuracy, and machine-facing utility is not proof that the same representation is preferable for people. The category definitions are owned by `product/correction-system-boundaries.md`.

This slice does not define metrics or numeric gates for these categories.

## Future Evaluation Areas

Future evaluation should consider:

- Terminology recall.
- False-positive review burden.
- Unsafe correction count.
- Manual correction time.
- Deterministic output for the same inputs and configuration.
- Regression behavior when active Domain Collections, language resources, resolved policies, detector configurations, or future models change.

## Claims and Data Rights

The project must not claim that it saves a specific percentage of correction time until measured.

Any future training or fine-tuning data must have explicit rights and opt-in. Local processing does not imply training permission.
