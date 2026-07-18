# MD-009: Proposed AGPL-3.0-only and Commercial Dual Licensing

Status: proposed

Date: 2026-07-18

Decision authority: Ezra

Classification: licensing / distribution boundary (not accepted)

## Context

VoxProof is source-visible on GitHub but currently has no explicit repository license, no `LICENSE` file, no Cargo package license metadata, no contribution policy, and no commercial-license agreement. Public visibility alone does not establish open-source licensing terms.

This proposed Material Decision records a bounded licensing direction for review. It does not apply a license to the repository.

Evidence inspected for this draft is recorded in `docs/governance/licensing-readiness.md`.

## Decision under consideration

Upon later acceptance and completion of the approval gates, VoxProof-authored source code would be offered under **AGPL-3.0-only**.

The copyright holder may separately offer proprietary commercial licenses for uses requiring terms incompatible with AGPL-3.0-only.

This proposed decision does not itself apply a license to the repository.

The proposed open-source choice is specifically **AGPL-3.0-only**, not `AGPL-3.0-or-later`.

## Business objective

The model under consideration is intended to support:

- a genuinely open-source community edition;
- transparent local-first core development;
- design-partner and paid-pilot work;
- proprietary embedding, OEM, redistribution, or closed-source integration under separate commercial terms;
- paid support, integration, workflow adaptation, and official distribution;
- future fundraising supported by public technical evidence and governed commercial rights.

This draft does not promise revenue. Sponsorship is not described as the sole business model.

## Why AGPL-3.0-only

AGPL-3.0-only is a strong copyleft open-source license. It permits commercial use, private use, modification, and distribution subject to its terms. Covered modifications and combined covered works may trigger source-disclosure and same-license obligations. Modified versions made available for remote network interaction have an additional source-availability obligation.

For VoxProof, the proposed fit is:

- local-first, evidence-backed software whose value depends on inspectable core behavior;
- a future desktop or network-facing review surface where network-use copyleft may matter;
- preserving the ability to offer separate commercial terms for closed-source embedding or incompatible redistribution;
- avoiding a permissive license that would weaken future commercial licensing leverage without adding a current product requirement.

AGPL does not prohibit commercial use. It is not a non-commercial license.

## What commercial licensing would cover

If later offered, separate proprietary commercial licenses would address categories such as:

- proprietary embedding of VoxProof components in closed products;
- closed-source redistribution;
- OEM distribution;
- integration into products whose licensing is incompatible with AGPL-3.0-only;
- negotiated warranty, indemnity, SLA, or support terms;
- alternative commercial distribution terms for official binaries or enterprise packaging.

A commercial license is not automatically required merely because an organization is commercial or uses unmodified VoxProof privately or internally to the organization.

## What this decision does not mean

- AGPL permits commercial use.
- AGPL is not a non-commercial license.
- Source availability does not guarantee paid conversion.
- Official binaries and source license are separate distribution questions.
- Trademarks and branding are separate from copyright licensing.
- External datasets, fixtures, media, model assets, and third-party dependencies retain their own terms.
- No commercial license agreement is created by this MD.
- No Contributor License Agreement is accepted by this MD.
- No contributor rights are assumed.
- Pure private or internal use is not automatically converted into a paid commercial-license requirement.
- Final legal documents require qualified legal review before paid licensing contracts are executed.

This MD does not paraphrase the AGPL into a custom replacement license and does not modify the official AGPL text.

## Copyright ownership prerequisites

This audit records repository signals; it does not make a final legal ownership determination.

| Area | Classification | Evidence / note |
| --- | --- | --- |
| Git commit authors | likely but unverified sole human author | `git shortlog -sne --all` shows only `Ezra Wu` with two email addresses across 76 commits |
| External human contributors in Git history | verified absent in inspected history | no non-Ezra author names; no merge commits from external contributors observed locally |
| `Co-authored-by: Cursor <cursoragent@cursor.com>` trailers | requires Ezra declaration | present on 59 of 76 commits; repository metadata requiring interpretation, not automatically third-party human copyright ownership |
| Employer/client-owned work | requires Ezra declaration | not inferable from Git metadata alone |
| Copied tutorial, prior-employment, or third-party example code | requires Ezra declaration | no explicit copied-source notices found in inspected tracked files |
| AI-assisted generation provenance | requires Ezra declaration | substantial Cursor co-author metadata; legal treatment of AI-assisted output not resolved here |
| Generated code in repo | verified present, scope limited | `Cargo.lock` is Cargo-generated; `examples/phonetic_characterization.rs` contains programmatically generated test vectors |
| Vendored third-party source in tracked tree | verified absent | no vendored source tree found |
| Sole copyright holder conclusion | requires Ezra declaration | Git history supports sole human commit authorship but is insufficient for legal ownership proof |

## Contribution-policy prerequisites

Future dual licensing requires an explicit contribution mechanism before material external contributions are accepted.

### Option A — inbound = outbound only

Contributions are accepted under AGPL only.

Consequence:

- straightforward open-source contribution model;
- future proprietary relicensing of contributed code may be unavailable without separate permission.

### Option B — Contributor License Agreement

Contributor grants rights sufficient for project maintainers to offer the contribution under AGPL and separate commercial terms while retaining contributor copyright.

Consequence:

- preserves dual-licensing capability;
- adds contribution friction and administrative requirements.

### Option C — copyright assignment

Contributor transfers copyright or broad ownership rights.

Consequence:

- strongest relicensing control;
- highest contributor friction;
- likely excessive for current project stage.

### Preliminary recommendation (proposed)

Use a narrowly drafted CLA before accepting material external code contributions if commercial dual licensing is intended to remain available.

Do not draft CLA legal text in this MD.

## Third-party dependency and artifact boundary

| Category | Boundary |
| --- | --- |
| VoxProof-authored source | Intended future AGPL-3.0-only subject, after acceptance and official license application |
| Cargo dependencies | Remain under their upstream licenses; AGPL on VoxProof source does not relicense dependencies |
| Tests and committed fixtures | Synthetic fixtures under `tests/fixtures/` are VoxProof-authored test inputs unless separately noted |
| Research artifacts | 53 untracked repo-root ZIP/JSON/patch artifacts; separate provenance and redistribution rules |
| External transcripts/audio/datasets | FLEURS and other real-material inputs referenced in docs/evaluation records; not committed as full datasets in tracked tree |
| Generated reports and seals | Local research outputs; not part of core licensing scope unless deliberately distributed |
| Documentation | Same copyright boundary as source unless separately marked |
| Logos/trademarks | Not covered by copyright licensing alone |
| Future binaries/installers | Distribution question separate from source license |

Placing VoxProof-authored source under AGPL would not change upstream dependency licenses or user-owned input/output ownership.

Dependency compatibility review is incomplete without SPDX metadata extraction from each crate. Direct dependencies observed in `Cargo.toml`: `pinyin`, `rphonetic`, `serde`, `serde_json`, `sha2`. No invasive dependency scanner was run for this draft.

## Repository and distribution scope

If accepted, the intended initial AGPL-covered scope would be VoxProof-authored tracked source, documentation authored for the project, and other repository files deliberately placed under the same license notice, excluding third-party materials governed by separate terms.

Distribution channels not decided by this MD include:

- crates.io publication;
- GitHub Releases binaries;
- signed desktop installers;
- commercial redistribution packages.

## Alternatives considered

| Option | Open-source status | Commercial use | Copyleft strength | Closed-source embedding pressure | Enterprise adoption friction | Monetize proprietary terms | Community contribution impact | Local-first desktop fit |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1. No explicit license / all rights reserved | No | Not granted by license | N/A | Highest | Highest | Highest | Blocks external contribution clarity | Poor community signal |
| 2. MIT | Yes | Permitted | None | Low | Low | Weak | Easy | Weak copyleft for future desktop/network use |
| 3. Apache-2.0 | Yes | Permitted | Patent grant; weak copyleft | Low | Low | Weak | Easy | Similar to MIT for embedding pressure |
| 4. MPL-2.0 | Yes | Permitted | File-level copyleft | Moderate | Moderate | Moderate | Moderate | Weaker network-use signal than AGPL |
| 5. AGPL-3.0-only without commercial dual licensing | Yes | Permitted under AGPL | Strong | High for closed derivatives | Higher for some enterprise embed patterns | None via separate license | Easy if inbound=outbound only | Strong fit for inspectable local-first core |
| 6. AGPL-3.0-only plus commercial licensing | Yes for community edition | Permitted under AGPL; alternate terms via commercial license | Strong | High unless commercial license purchased | Moderate to high depending on use case | Possible for incompatible uses | Requires CLA or equivalent for dual-licensed contributions | Strong fit if contribution and branding gates are managed |
| 7. Source-available commercial-use restriction | No (not OSI open source) | Restricted | Custom / non-standard | High | High | Possible | Usually worse | Conflicts with open community edition goal |

AGPL does not guarantee monetization.

## Consequences

If later accepted and implemented:

- the repository would gain an unmodified official AGPL-3.0 license text;
- README and package metadata would state AGPL-3.0-only clearly;
- commercial licensing would remain a separate negotiated offering;
- contribution terms would be required before accepting material external contributions if dual licensing is retained;
- dependency and third-party notices would need explicit documentation where required;
- qualified legal review would precede the first paid proprietary-license agreement.

Until then, the repository remains unlicensed for public-reuse purposes despite source visibility.

## Risks

- claiming dual licensing before copyright ownership is verified;
- accepting external contributions without rights sufficient for commercial relicensing;
- implying every commercial user must buy a commercial license;
- implying AGPL prohibits commercial use;
- distributing official binaries without clarifying how source obligations apply;
- bundling third-party datasets, fixtures, or research artifacts under the wrong license;
- adding license badges or Cargo metadata before the official license exists;
- treating Cursor co-author trailers as resolved copyright fact without Ezra declaration.

## Approval gates

MD-009 must remain **proposed** until Ezra approves all applicable gates.

| Gate | Requirement | Current status |
| --- | --- | --- |
| G1 | Repository copyright/provenance review completed | PARTIAL — audit recorded in `licensing-readiness.md`; Ezra declarations still required |
| G2 | No blocking third-party or employer/client ownership conflict | OPEN — requires Ezra declaration |
| G3 | Exact scope of AGPL-covered files defined | PARTIAL — draft scope recorded; final file list not approved |
| G4 | Commercial-license purpose and target use cases defined | PARTIAL — categories recorded; no agreement drafted |
| G5 | Contribution policy selected | OPEN — preliminary CLA recommendation only |
| G6 | Trademark/branding treatment separated | OPEN |
| G7 | Dependency and artifact boundaries documented | PARTIAL — draft boundaries recorded; SPDX review incomplete |
| G8 | README wording and badge plan reviewed | OPEN — README currently lacks license section; badge plan deferred |
| G9 | Cargo package metadata plan reviewed | OPEN — no `license` field today |
| G10 | Qualified legal review identified before first paid commercial-license agreement | OPEN |
| G11 | Ezra explicitly accepts MD-009 | OPEN |

No gate is marked PASS.

## Implementation sequence after acceptance

Proposed future order only. Do not execute in advance of acceptance.

1. Accept MD-009.
2. Add an unmodified official AGPL-3.0 license text as `LICENSE`.
3. Add SPDX/package metadata as appropriate.
4. Add a concise licensing section to README.
5. Add a commercial-licensing contact statement.
6. Add contribution terms before accepting material external contributions.
7. Document third-party notices where required.
8. Add a truthful license badge.
9. Obtain legal review before executing paid proprietary-license agreements.

## Deferred commercial terms

This MD intentionally does not draft:

- proprietary license agreement text;
- pricing;
- indemnity or warranty clauses;
- SLA terms;
- trademark license terms;
- CLA legal text;
- distributor/OEM contract templates.

## Claim boundaries

Permitted after acceptance and implementation:

- VoxProof-authored source is available under AGPL-3.0-only.
- Separate commercial licenses may be available for defined incompatible uses.

Not permitted now:

- describing the repository as already open source under AGPL;
- describing dual licensing as operational;
- claiming every enterprise must purchase a commercial license;
- claiming AGPL forbids commercial use.

## Evidence inspected

- `README.md`
- `docs/README.md`
- `Cargo.toml`
- `Cargo.lock`
- `docs/governance/material-decisions.md`
- `docs/governance/decisions/MD-001-transcript-revision-id.md`
- `docs/governance/decisions/MD-008-v0.1-core-mechanism-establishment.md`
- `docs/product/versioning.md`
- `docs/product/v0.1.md`
- `docs/product/hypotheses.md`
- `docs/architecture/v0.2-c4-architecture.md`
- repository search for `LICENSE*`, `COPYING*`, `NOTICE*`, `*CLA*`, `CONTRIBUTING*`, `.github/`
- `git shortlog -sne --all`
- `git log --all --format='%H%x09%an%x09%ae%x09%s'`
- commit-body scan for `Co-authored-by: Cursor`
- `git grep` for copyright/license/SPDX/generated/vendored/third-party markers
- `tests/fixtures/**`
- untracked repo-root research artifact inventory (53 files)

## Recommendation

Proceed toward **AGPL-3.0-only plus separately negotiated commercial licensing**, subject to the approval gates.

The recommendation is provisional.

The repository remains unlicensed under this decision until the official license text and related metadata are deliberately added after MD-009 acceptance.

## Status

**Proposed / not accepted.**

This record does not license the repository and does not authorize adding `LICENSE`, Cargo license metadata, badges, or commercial contract terms.
