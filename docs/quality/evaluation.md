Status: current
Owns: Quality expectations, fixture principles, ground-truth distinctions, metrics, and regression expectations.
Does not own: Product scope, architecture, implementation tasks, benchmark results, or validated performance claims.
Last reviewed against code: Track 1 local code loop and raw-versus-final comparison/change inventory for strict skeleton-compatible inputs exist. v0.1 is not established; real-material validation remains pending.

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
