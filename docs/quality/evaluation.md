Status: current
Owns: Quality expectations, fixture principles, ground-truth distinctions, metrics, and regression expectations.
Does not own: Product scope, architecture, implementation tasks, benchmark results, or validated performance claims.
Last reviewed against code/evidence: Track 1 local code loop and raw-versus-final comparison/change inventory for strict skeleton-compatible inputs exist. Strict skeleton-compatible calibration correspondence evaluation (`vox-proof evaluate`, committed at `e21be2e`) exists. Qualifying owner-operated FLEURS real-speech human review completed at repository HEAD `7efe8ba`; all ten MD-007 mechanism gates passed on that evidence; sealed package, final seal, and detached final closure attestation passed 155/155 checks. The authorized mixed Traditional-Chinese / ASCII-Latin fixture required by MD-007 D10 is implemented at implementation baseline `05b7a2f`. Final post-commit isolated validation passed at implementation baseline `05b7a2f` (2026-07-18T03:35:31Z). Historical tag-target validation passed at MD-008 establishment commit `cde7fd9` (2026-07-18T04:09:43Z). Gate matrix and release-notes draft are recorded in `product/v0.1.0-release-preparation.md`. v0.1 is established by MD-008 as a bounded core mechanism only; local annotated tag pending recreation. Product and external-user validation remain deferred beyond v0.1.

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
- provisional calibration artifact only; not canonical Evidence, certified ground truth, correctness, precision/recall, detector effectiveness, or product validation
- both inputs must parse with zero validation issues and identical cue count, cue index, `start_ms`, and `end_ms` at every `segment_position`
- incompatible structure fails closed with a deterministic refusal and no report file
- per-cue records use only `unchanged` and `text_changed` with exact parsed Unicode text preserved
- no precision, recall, correctness, accuracy/formatting/editorial classification, or product-performance claims
- mismatched segmentation or timing requires a future compatibility policy; v0 does not align cues

## Provisional Calibration Correspondence

Implemented local command:

```text
vox-proof evaluate <raw-input.srt> <final-input.srt> <session-terms.txt> <evaluation-report.json>
```

Boundary:

- schema revision: `voxproof-calibration-correspondence-v0`
- compatibility policy: `identical-cue-count-index-and-timing-v0` (unchanged from compare)
- provisional calibration artifact only; not canonical Evidence, certified ground truth, correctness, precision/recall, detector effectiveness, or product validation
- stdin is never read; decision logs are not inputs in v0
- canonical ReviewCases are generated afresh from raw plus parsed effective session terms under the committed canonical analysis profile
- both SRT inputs must parse with zero validation issues and identical cue count, cue index, `start_ms`, and `end_ms` at every `segment_position`
- incompatible structure, analysis refusal, or local diff work-budget refusal fails closed with a deterministic message and no report file
- local edits use Unicode-scalar Hirschberg LCS v1 with `max_lcs_cells = 4_000_000` per changed cue; this is a resource-safety boundary only
- report emits deterministic local edit inventories, ReviewCase bindings, neutral correspondence facts, and precisely defined summary counts only
- no precision, recall, correctness, effectiveness, or ratio claims in v0
- existing `vox-proof compare` command, schema, and bytes remain unchanged

## Exploratory Real-Material Phonetic Mechanism Probes

Classification: exploratory real-material mechanism observations only. They are not benchmarks, product validation, threshold validation, precision/recall evidence, human correctness adjudication, or general ASR claims.

The strongest permitted aggregate conclusion is:

> The committed bounded ASCII-Latin phonetic producer has exercised both a zero-candidate real-material path and a reference-supported emitted-candidate real-speech path. The committed strict skeleton-compatible `evaluate` mechanism has exercised a deterministic calibration correspondence path on frozen public FLEURS material. These observations support mechanism viability only. They do not establish product or detector-effectiveness thresholds, precision/recall, or v0.1 establishment. Qualifying owner-operated human-review evidence under MD-007 D8 was recorded separately at repository HEAD `7efe8ba`.

### Probe A: AMI ES2004d zero-candidate path

- Input was an immutable real AMI meeting-ASR transcript with 258 cues and four frozen session-term entries.
- The authoritative canonical pipeline completed with 0 exact-alias, 0 observed-error-form, and 0 phonetic ReviewCases.
- Reviewed output remained byte-identical to the raw input.
- Permitted conclusion: the committed pipeline can complete without forcing candidates on this material.
- The absence of candidates does not establish that every cue was correct or establish a zero false-positive rate, precision, recall, correctness, or product effectiveness.

### Probe B: FLEURS en_us phonetic producer path (repository HEAD `24363d9`)

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

### Probe C: FLEURS en_us calibration correspondence evaluate path (repository HEAD `e21be2e`)

Classification: exploratory mechanism-level calibration correspondence probe only. It validates only that the committed strict skeleton-compatible `evaluate` mechanism produces a deterministic, internally consistent calibration correspondence artifact on frozen public real-speech material.

This probe is distinct from Probe B. Probe B exercised the authoritative review phonetic producer at repository HEAD `24363d9`. Probe C exercised `vox-proof evaluate` at repository HEAD `e21be2e` on the same frozen 15-cue raw/reference pair and the same six frozen targets, but with a study-local canonical-only projection of the historical self-alias session-term representation. Target set and ordering were preserved; only the self-referential alias suffix was removed.

Observed mechanism facts:

- 15 cues; 41 local edits; 1 canonical ReviewCase; 1 edit/candidate correspondence.
- Two evaluate runs exited 0 with byte-identical reports.
- Independent verification passed 95/95 checks.
- Sole phonetic candidate: observed surface `ASIS`, canonical target `ASUS`, edit distance 1, ratio 3/4, `ratio_permille` 750, matched key `ASS`.
- Candidate raw anchor bytes 86..90 within its cue; replacement edit `I` -> `U` at bytes 88..89; relation `CandidateContainsEdit`, not `Exact`.
- The frozen reference cue contains `ASUS` at the deterministically localized compatible position. This is text localization only; no audio listening or human correctness adjudication occurred.

Not validated by this probe: correctness or ground truth, precision/recall, detector effectiveness or threshold quality, general ASR quality, human review quality, product effectiveness, or v0.1 establishment.

Local evidence identifiers:

- implementation commit: `e21be2e8fb337f30c1fcff6be26eba499fbfab23`
- package: `voxproof-fleurs-calibration-evaluate-e21be2e-20260717-final.zip`
- package SHA-256: `8d345e4009ffa7684d16d3b3976ac4f812c15509f05a089ccc2abfdc63c54d26`
- package bytes: 34304
- entry count: 22
- entry-manifest SHA-256: `58c96c8b25aaf9ba57eef4c33ea6442f9d328287abb059b1967041048189313a`
- seal: `package-seal-fleurs-calibration-evaluate-e21be2e-20260717-final.json`
- seal SHA-256: `e445a407d6fda5d38a47db3ca6b103419b0dc523cfa97dff8e13a9da3866b6c0`
- evaluation report SHA-256: `5c2d2d5b45447415e0e8b10e9b16e764f0dc68e69f889fa887591a14c2c55e15`
- handoff SHA-256: `2c0e9520918152ce15480ab26668c7876b5185f68db12829523faceb68309cfd`
- closure verification: 84/84 PASS
- closure output SHA-256: `b63a9974754a52f940ec382e8c43bc9ce4a28832781c9f583d6bf749016b5355`

These package and seal artifacts are local untracked research artifacts; this document does not imply that they are available from the public repository.

Prohibited interpretations include a true-positive rate, precision or recall, human correctness adjudication, product validation, threshold validation, proof of general effectiveness, or a claim that synthetic reject controls are human decisions.

### Canonical-only session-term provenance

Canonical-only entries are now represented natively with empty alias and observed-error-form collections. Prefixed `alias:` and `error:` fields remain required only for non-canonical source forms. The sealed FLEURS phonetic producer probe (Probe B, repository HEAD `24363d9`) ran before this support existed and therefore used self-referential `alias:<canonical>` entries solely for then-required syntactic compliance; no alternate observed forms were invented. Those aliases remain historical sealed-study provenance, not the current required input form. The calibration correspondence evaluate probe (Probe C, repository HEAD `e21be2e`) projected the same six frozen targets study-locally to native canonical-only entries without changing target set or ordering. This implementation change does not rewrite or reinterpret the sealed phonetic producer study or authorize a persistence schema.

## Qualifying Owner-Operated FLEURS Human Review

Classification: qualifying MD-007 D8 human-correction mechanism evidence on frozen real-speech material. Not product validation, effectiveness, precision/recall, external-user evidence, or v0.1 establishment.

Repository HEAD: `7efe8bad77dc2d9f37f613a1660703c0bcf9653c`

Study: owner-operated FLEURS real-speech human review on the frozen 15-cue material from Probe B.

Reviewer: Ezra, project owner, qualifying v0.1 human reviewer under MD-007.

### Human review completion

- 15/15 audio cues listened to
- 15/15 cues adjudicated
- 0 unresolved
- 14 raw-confirmed
- 1 corrected

### Authoritative correction

- ReviewCase: `local:0`
- Cue: 2
- Observed: `ASIS`
- Accepted alternative: `ASUS`
- CLI decision: `a 0`
- Decision timestamp: `2026-07-17T08:23:46Z`

The authoritative reviewed output and human-final transcript had identical text in all 15 cues. Byte-level file inequality occurred only because the human-final file had one trailing LF byte.

### Deterministic evidence

On identical inputs after human review:

- raw-vs-human-final `compare` run 1 and run 2: byte-identical reports
- raw-vs-reviewed `compare` run 1 and run 2: byte-identical reports
- `evaluate` run 1 and run 2: byte-identical reports

Observed correspondence facts (descriptive only; not product-quality ratios):

- 1 replacement edit
- 1 ReviewCase
- 1 overlap
- 0 unclassified
- 15/15 cue indices and timing skeleton preserved
- all summary counts recomputed exactly from detailed records
- all ten MD-007 D9 mechanism gates passed

### Limitations and reviewer disclosure

- Owner-operated review; not blind ground truth; not external-user evidence.
- Reviewer English/accent/domain familiarity was limited.
- The study-local temporary review GUI displayed raw ASR text; raw-text anchoring was therefore possible.
- The reviewer already knew the public/community reference supported `ASUS` for the `ASIS` cue.
- The community/reference transcript was excluded from correctness authority.
- Reviewer and community/reference transcript disagreed on 11 cues; this disagreement is descriptive only.
- The temporary review GUI was study-local tooling, not product UI.
- An audio-playback defect was repaired study-locally and disclosed in package provenance.

These limitations constrain interpretation but do not invalidate the verified mechanism properties.

### Sealed evidence identity

Evidence topology: ZIP → bound by final seal → verified by detached final closure attestation.

| Artifact | Filename | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| Human-review ZIP | `voxproof-fleurs-owner-human-review-7efe8ba-20260717-final.zip` | 88513 | `e57681bf386d819eefe63542d3164448934e03f480e8c824af98cc6a1525e42f` |
| Final seal | `package-seal-voxproof-fleurs-owner-human-review-7efe8ba-20260717-final.json` | 5636 | `e36f4c6a9b807042d04d08f5a99691b2bd7f9ef5e896f443c7aa2adde11b3151` |
| Detached closure attestation | `final-closure-attestation-voxproof-fleurs-owner-human-review-7efe8ba-20260717-final.json` | 18329 | `56eaa0daf6bc3035888667fae3d8b49dde2e8d27fd5f9347152e986f07cf28d9` |

Closure: 155/155 checks PASS.

These package, seal, and attestation artifacts are local untracked research artifacts; this document does not imply that they are available from the public repository.

## Research Artifact-Loss Incident

Missing artifact: `calibration-comparison-v0.patch`

Expected historical size: 42536 bytes

Expected historical SHA-256: `67552483588a464219f992b6492e94c7a006a77a6b2981c8089f220a6e9cda4e`

Classification: lost / not exactly recoverable

No exact copy was found in the repository, parent directories, `/private/tmp`, Trash, ZIP entries, or reachable/unreachable Git objects. Metadata references remain.

Bounded disappearance window:

- present in metadata around 2026-07-17 04:32 UTC
- first filesystem-proven absence around 2026-07-17 06:44 UTC

Likely disappearance during canonical-only session-term work is an inference, not a proven cause. Owner approved classification as lost. No replacement was generated. Future recovery requires an exact SHA-256 match and a new append-only recovery record.

Current code and accepted human-review evidence do not depend on the missing patch. The incident therefore does not independently block v0.1 establishment.

Inventory semantics:

- 51 expected verification paths
- 50 physical repo-root research files before the human-review package
- 1 historical baseline artifact missing

Do not write “51 files present.”

Incident record in sealed package: `post-review/artifact-loss-incident.json` (SHA-256: `54298c64e66a7592eaa49d19b1ea1f4ca3761ed598373761a6339047aeeb124e`).

## Final Post-Commit Isolated Validation

Classification: repository build/test hygiene evidence at a fixed commit. Not human-review evidence, not product validation, and not v0.1 establishment.

Recorded at repository HEAD `05b7a2f90ef114817996e9db7f6aa85e4a277f0e` on 2026-07-18T03:35:31Z with a clean tracked working tree.

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --all-targets --all-features` | PASS; 377 tests (lib 299, main 0, cli 67, example 11) |
| `git diff --check` | PASS |

Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)`.

Authoritative gate matrix and release-notes draft: `product/v0.1.0-release-preparation.md`.

## Tag-target validation (MD-008 establishment commit)

Historical validation at commit `cde7fd9cd43d9b582b3475d9ec78f7f6e33805ca` on 2026-07-18T04:09:43Z supported the first local annotated `v0.1.0` tag attempt, deleted before push solely to synchronize canonical release-state documentation. Full record: `product/v0.1.0-release-preparation.md`.

Replacement tag-target validation on the documentation-synchronized commit is required before recreating the local annotated tag.

## v0.1 Establishment Mechanism Gates

The authoritative frozen v0.1 mechanism gates are owned by MD-007 D9. All ten gates passed on qualifying human-correction evidence under MD-007 D8 at repository HEAD `7efe8ba`. They are mechanism-establishment gates, not product-effectiveness thresholds. Candidate yield, correctness observations, and error distributions remain descriptive only.

No other document may restate, weaken, or diverge from the ten gates. Other documents may summarize and must link to MD-007.

The consolidated gate matrix, D10 fixture record, final validation record, release-notes draft, and remaining release actions are owned by `product/v0.1.0-release-preparation.md`.

v0.1 is established by MD-008 as a bounded core mechanism only. Local annotated `v0.1.0` tag pending recreation on the documentation-synchronized replacement tag target after final isolated validation on that exact commit.

## Fixtures

### Authorized mixed Traditional-Chinese / ASCII-Latin fixture (MD-007 D10)

Classification: authorized synthetic/deterministic mixed Traditional-Chinese / ASCII-Latin mechanism fixture only. Not real-speech evidence, detector-effectiveness evidence, precision/recall evidence, general CJK or pinyin support, or product-validation evidence.

Fixture location:

- `tests/fixtures/mixed-zh-tw-ascii-latin/input.srt`
- `tests/fixtures/mixed-zh-tw-ascii-latin/session-terms.txt`
- `tests/fixtures/mixed-zh-tw-ascii-latin/unsupported-forms.srt`

Exercised public path: `vox-proof review` through the canonical session-term pipeline (`run_term_review` → bounded ASCII-Latin phonetic evidence → explicit human decision → reviewed SRT, decision log, session summary).

Verified mechanism properties only:

- UTF-8 byte-anchor safety for an eligible ASCII-Latin span embedded in Traditional-Chinese context (`ASIS` → `ASUS` via canonical-only phonetic evidence);
- hard-boundary and eligibility behavior for unsupported or non-eligible forms (Traditional-Chinese-only text, full-width Latin, exact canonical suppression, sub-minimum ASCII token length);
- deterministic ReviewCase and evidence behavior across reruns;
- explicit human accept/reject authority with exact materialization on accept and byte-preserving source text on reject;
- cue index and timing skeleton preservation;
- no silent normalization or auto-acceptance.

Test names: `mixed_zh_tw_ascii_latin_fixture_preserves_utf8_anchors_and_human_authority`, `mixed_zh_tw_ascii_latin_rejection_preserves_source`, `mixed_zh_tw_unsupported_forms_remain_unchanged`, `mixed_zh_tw_ascii_latin_review_is_deterministic`, plus the in-crate byte-anchor unit test of the same name under `phonetic::tests`.

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

This slice does not define additional metrics or gates beyond MD-007 for v0.1 establishment.

## v0.2 Detector/Reference Join Contract Boundary

The typed contracts `voxproof-detector-reference-join-v1` and `voxproof-overlap-adjudication-v1` define deterministic relationship derivation and immutable overlap-adjudication input records only.

Boundary:

- descriptive disposition counts only; no TP, FP, FN, precision, recall, exactness ratios, thresholds, or `primary_metrics_allowed`
- overlap requires frozen human adjudication before `accepted_overlap`; overlap geometry alone is insufficient
- NFC equality is the sole correction-equality rule for join derivation; `original_surface` equality is not authority
- exactly one detector alternative is required for join v1 derivation
- resolved join state does not establish primary calibration validity, detector effectiveness, or product validation
- human adjudication execution, real/synthetic protocol runs, and persistence remain deferred

## v0.2 Join Metric Contribution Contract Boundary

The typed contract `voxproof-join-metric-contributions-v1` (`ArtifactRole::MetricContributions`) derives per-source numerator/denominator participation from one validated join. It is separate from the future aggregate `ArtifactRole::Metrics` artifact.

Boundary:

- one contribution record per detector proposal and per reference record; mappings are deterministic and revalidated against sources
- five primary metric dimensions share one common eligibility gate; eligible sets are all-five or empty
- contribution records may be `numerator_and_denominator`, `denominator_only`, `excluded`, or `pending_adjudication`; no aggregate numerators, denominators, ratios, percentages, TP/FP/FN, thresholds, or performance claims
- wrong correction counts as localized but not correction-exact; unmatched reference is not in the conditional correction-exactness denominator; duplicate proposal shares the detector denominator with proposal precision
- diagnostic, synthetic, ambiguous, excluded, and pending posture are explicit; `qualifies_as_real_material_evidence` is separate from calibration eligibility
- execution and persistence remain deferred

## v0.2 Join Metric Aggregation Contract Boundary

The typed contract `voxproof-join-metric-aggregates-v1` (`ArtifactRole::Metrics`) deterministically aggregates one complete `JoinMetricContributionSet` into five exact numerator/denominator records. It is separate from future reporting, thresholding, and effectiveness claims.

Boundary:

- aggregation consumes validated complete contribution sets only; pending or invalidated contributions refuse aggregation
- five aggregate records in canonical order; counts use checked integer arithmetic with no floating-point metric values
- zero denominator is `undefined_zero_denominator`, never zero score, perfect score, pass, or fail
- cross-metric invariants tie detector denominators and reference localization/correction/end-to-end populations
- report class, primary eligibility, blocking reasons, and real-material qualification are copied from contributions; `qualifies_as_primary_metric_evidence` is derived and does not require non-zero denominators
- diagnostic and synthetic aggregates remain non-primary
- no decimal rendering, percentages, TP/FP/FN, thresholds, pass/fail, protocol execution, or detector-effectiveness claims
- Metrics artifacts remain private unless separately authorized

## v0.2 Synthetic Evaluation Harness Boundary

The in-memory synthetic evaluation harness exercises the accepted v0.2 contract chain on deterministic synthetic fixtures only. It validates synthetic posture fail-closed, executes legal lifecycle transitions, derives join/contribution/aggregate artifacts through the public contract APIs, serializes eight typed payloads in memory, verifies role-specific typed deserialization and exact reserialization against descriptor-bound bytes, assembles a self-consistent final artifact bundle, and revalidates historically at `Finalized` from the decoded artifact set.

It does not:

- execute the real detector or read real transcripts/audio;
- collect human adjudication or emit filesystem artifact files;
- establish primary metric evidence, thresholds, pass/fail, or detector-effectiveness claims;
- replace the semantic authority of the individual contracts.

Exact-only synthetic runs may complete derivation at `DetectorExecution`. Overlap runs remain pending until `AssistedReview` consumes frozen `SyntheticFixtureAdjudicator` records. Unresolved overlap blocks aggregation. Repeated runs must be byte-deterministic.

Compact Serde JSON is bounded deterministic serialization for harness verification; it is not RFC 8785 canonical JSON. Raw payload digest integrity and typed semantic validity are separately enforced.

Exact-only runs still pass through the required blind-reference `AssistedReview` lifecycle transition before `Finalized`. That transition context is present in the result envelope but does not consume adjudication or derive artifacts for exact-only fixtures.

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
