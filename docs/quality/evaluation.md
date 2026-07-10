Status: current
Owns: Quality expectations, fixture principles, ground-truth distinctions, metrics, and regression expectations.
Does not own: Product scope, architecture, implementation tasks, benchmark results, or validated performance claims.
Last reviewed against code: Track 1 local code loop exists. v0.1 is not established; real-material validation remains pending.

# Quality and Evaluation

No benchmark or validated performance result exists yet.

## Fixtures

Future fixtures should use authorized short mixed Chinese-English transcript samples. Fixtures should be small enough to review manually and explicit about rights to use the transcript and any related audio.

## Ground Truth

Ground truth must distinguish:

- Actual transcript errors.
- Acceptable variants.
- Terminology corrections.
- Edits that alter meaning.

## Future Evaluation Areas

Future evaluation should consider:

- Terminology recall.
- False-positive review burden.
- Unsafe correction count.
- Manual correction time.
- Deterministic output for the same inputs and configuration.
- Regression behavior when rules, Language Packs, or future models change.

## Claims and Data Rights

The project must not claim that it saves a specific percentage of correction time until measured.

Any future training or fine-tuning data must have explicit rights and opt-in. Local processing does not imply training permission.
