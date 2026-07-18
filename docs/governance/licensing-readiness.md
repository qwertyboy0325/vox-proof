Status: current audit / does not license the repository

Owns: Repository licensing readiness findings, evidence, and blocking questions pending Ezra review.

Does not own: Accepted licensing terms, legal agreements, contribution contracts, SPDX metadata, badges, or implementation of MD-009.

Last reviewed against repository state: 2026-07-18 at HEAD `e49e4945255d61a97f79b95851d3b34075f97c04`

# Licensing Readiness Audit

## Purpose

This document records a bounded licensing feasibility audit for a proposed model of:

```text
AGPL-3.0-only community license
+
separately negotiated proprietary/commercial licenses
```

It supports review of proposed `MD-009` and does not apply any license to the repository.

## Current repository licensing state

| Item | Finding |
| --- | --- |
| `LICENSE` / `COPYING` | not found |
| `NOTICE` | not found |
| `CONTRIBUTING.md` | not found |
| CLA files | not found |
| `.github/` workflows or policy templates | not found |
| Cargo `[package].license` | absent in `Cargo.toml` |
| README license section | absent |
| Accepted Material Decision for licensing | none (`MD-009` remains proposed) |

No explicit repository license was found.

The public repository is source-visible, but this audit does not classify it as open source.

## Authorship and copyright signals

### Git authorship

```text
git shortlog -sne --all
    64  Ezra Wu <43637570+qwertyboy0325@users.noreply.github.com>
    12  Ezra Wu <Ezra40907@gmail.com>
```

Total commits inspected: 76.

Unique human author names in commit metadata: `Ezra Wu` only.

No merged pull requests from external human authors were observed in local history (`git log --merges` empty).

### Co-author trailers

Commit-body scan found `Co-authored-by: Cursor <cursoragent@cursor.com>` on 59 commits.

Interpretation for this audit:

- repository metadata requiring Ezra declaration;
- not automatically classified as third-party human copyright ownership;
- may affect provenance review even if Ezra remains sole human author.

### Source/file notices

Targeted search across tracked `.rs`, `.md`, and `.toml` files found:

- no per-file copyright headers in Rust sources;
- no SPDX identifiers in tracked project files;
- no vendored third-party source notices in tracked tree;
- `Cargo.lock` marked as Cargo-generated;
- programmatic generation noted in `examples/phonetic_characterization.rs`.

## Potential ownership risks

| Risk | Classification | Note |
| --- | --- | --- |
| Non-Ezra human code in Git history | verified absent in inspected metadata | still not a legal ownership proof |
| Employer/client ownership of authored code | requires Ezra declaration | cannot be inferred from Git |
| Copied code from tutorials, prior jobs, other repos | requires Ezra declaration | no explicit copied-source markers found |
| AI-assisted code authorship | requires Ezra declaration | extensive Cursor co-author metadata |
| Future external contributions without relicensing rights | blocking if dual licensing retained | no contribution policy exists yet |
| Trademark use of `VoxProof` | requires Ezra declaration | separate from copyright licensing |

## Third-party code and dependency boundaries

### Direct Rust dependencies (`Cargo.toml`)

- `pinyin = "0.11.0"`
- `rphonetic = "=3.0.6"`
- `serde`
- `serde_json`
- `sha2`

`Cargo.lock` resolves additional transitive crates but does not, by itself, provide a complete SPDX inventory in this repository state.

Compatibility review status: **partial**. Upstream crate licenses were not exhaustively verified in this audit because no license metadata is recorded in-repo and no dependency scanner was installed.

Known distribution implication: AGPL on VoxProof-authored code does not relicense dependencies; binary or source distribution may still require upstream license notices.

### Vendored code

No vendored third-party source directory was found in the tracked tree.

## Fixtures, datasets, and research artifacts

### Committed fixtures

Tracked synthetic fixtures exist under:

```text
tests/fixtures/mixed-zh-tw-ascii-latin/
tests/fixtures/contextual-resolution-synthetic/
```

These appear to be project-authored test inputs. MD-007 D10 mixed Traditional-Chinese / ASCII-Latin fixture is synthetic mechanism evidence, not third-party dataset redistribution.

### Referenced but not committed as full datasets

Documentation and evaluation records reference frozen public FLEURS material and owner-operated real-speech review evidence. Full research packages remain local untracked repo-root artifacts.

Untracked repo-root research artifacts counted at audit time: **53** files (ZIP/JSON/patch), unchanged by this audit task.

These artifacts are not covered by a future source-code license decision unless deliberately included in distribution scope.

## Contribution history

Current state:

- sole human commit author in inspected Git metadata;
- no accepted contribution policy;
- no CLA;
- no inbound=outbound statement;
- substantial AI co-author metadata on recent commits.

Dual-licensing readiness therefore depends more on future contribution policy than on historical multi-author cleanup, but G1/G2 still require Ezra declarations about provenance and ownership.

## Dual-licensing readiness

| Prerequisite | Readiness |
| --- | --- |
| Clear community license choice | proposed only (`AGPL-3.0-only`) |
| Copyright authority to offer commercial licenses | unverified; Ezra declaration required |
| Contribution mechanism preserving commercial rights | not established |
| Dependency notice plan | not established |
| Trademark/branding separation | not established |
| Legal review before paid agreements | not established |
| Official license file and metadata | not present |

Overall readiness: **not ready to apply a license or offer commercial terms**.

Provisional direction recorded in `MD-009`: proceed toward AGPL-3.0-only plus separately negotiated commercial licensing, subject to approval gates.

## Blocking questions for Ezra

These require personal declaration. This audit does not answer them.

1. Was all VoxProof source written independently for this project?
2. Does any source derive from employer, client, school, or contract work?
3. Has any human contributor supplied code outside commits visible in Git?
4. Has copied code from articles, repositories, generated templates, or AI sessions been retained in tracked files?
5. Were any fixtures, transcripts, audio, datasets, or terminology lists obtained under third-party restrictions?
6. Is the name `VoxProof` intended to be retained as a protected project/product brand?
7. Should official signed binaries be freely available, paid, or handled separately from source licensing?
8. Is free private/internal enterprise use acceptable under the chosen open-source model?
9. How should `Co-authored-by: Cursor` commits be treated for copyright/provenance purposes?
10. Is dual licensing still desired if external contributions remain rare and inbound=outbound AGPL would suffice?

## Recommended pre-license actions

1. Answer the blocking questions above.
2. Complete G1–G11 in `MD-009`.
3. Decide contribution policy (preliminary recommendation: narrowly drafted CLA before material external contributions if dual licensing is retained).
4. Define exact files covered by the future AGPL notice.
5. Verify dependency licenses and required notices before distribution.
6. Separate trademark/branding treatment from copyright licensing.
7. Add official unmodified AGPL-3.0 text only after MD-009 acceptance.
8. Add README licensing section and Cargo metadata only after acceptance.
9. Identify qualified legal counsel before first paid commercial-license agreement.
10. Keep research artifacts and third-party datasets out of default license scope unless intentionally distributed.

## Evidence and commands

Commands run during this audit:

```bash
git status --short
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD
git rev-list --left-right --count main...origin/main
git shortlog -sne --all
git log --all --format='%H%x09%an%x09%ae%x09%s'
git log --all --format='%H%n%B%n---END---'
git log --merges --oneline
git grep -nEi 'copyright|license|licensed|SPDX|generated|vendored|third[- ]party'
find . -maxdepth 4 \( -iname 'LICENSE*' -o -iname 'COPYING*' -o -iname 'NOTICE*' -o -iname '*CLA*' \) -print
```

Files inspected include:

```text
README.md
docs/README.md
Cargo.toml
Cargo.lock
docs/governance/material-decisions.md
docs/governance/decisions/MD-001*
docs/governance/decisions/MD-008*
docs/product/versioning.md
docs/product/v0.1.md
docs/product/hypotheses.md
docs/architecture/v0.2-c4-architecture.md
tests/fixtures/**
```

## Limitations

- This is not legal advice.
- Git author metadata is not proof of copyright ownership.
- Cursor co-author trailers were recorded but not legally classified.
- Dependency licenses were not exhaustively verified from upstream registries.
- Remote GitHub state beyond fetched local history was not independently revalidated in this audit.
- No `LICENSE`, CLA, commercial agreement, Cargo license field, or badge was added by this audit.
