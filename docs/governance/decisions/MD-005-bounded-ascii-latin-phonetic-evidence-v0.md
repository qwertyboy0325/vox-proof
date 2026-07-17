# MD-005: Bounded ASCII-Latin Phonetic Evidence v0

Status: accepted

Date: 2026-07-17

Decision authority: Ezra

## Context

MD-004 records the effective analysis-identity prerequisite and the minimum
future canonical phonetic-evidence boundary. MD-004 explicitly does not
authorize phonetic matching implementation.

This decision authorizes a bounded v0.1.0 ASCII-Latin phonetic evidence
producer that enters the canonical `ReviewCase` flow under the identities and
human-authority rules already established by MD-004.

## Decision

### Authorization scope

MD-005 authorizes only:

* ASCII-Latin phonetic evidence entering the canonical `ReviewCase` flow;
* cue-local operation on parsed segment text;
* canonical session-term **canonical terms and aliases** as phonetic targets;
* exclusion of observed `error:` forms as phonetic targets;
* exact-target suppression for structurally identical canonical/alias token
  vectors;
* `DoubleMetaphone::new(None)` from pinned `rphonetic` 3.0.6;
* Levenshtein ratio permille >= 500 plus non-empty DoubleMetaphone key overlap;
* deterministic same-owner winner and duplicate-collapse rules;
* cross-owner ambiguity suppression when multiple canonical owners qualify for
  the same source anchor;
* inspectable typed `PhoneticSimilarityEvidence` with bound detector-config and
  algorithm identities;
* human authority and reviewed-output materialization only through applicable
  human `AcceptAlternative` decisions per MD-003.

### Eligibility limits (v0.1 mechanism policy)

These limits are a **provisional versioned v0.1 mechanism policy**. They are
**not** a validated effectiveness threshold.

Source windows and target surfaces must be structurally valid end-to-end:

* 1..=3 maximal `[A-Za-z]+` tokens;
* each token length 2..=32 bytes;
* tokens separated only by one or more ASCII whitespace bytes
  (`u8::is_ascii_whitespace()`); NBSP and every other non-ASCII separator
  terminate eligibility;
* no leading or trailing whitespace;
* no punctuation, digits, symbols, non-ASCII, CJK, emoji, or ignored residual
  bytes;
* normalized lowercase letter total 3..=64;
* total surface <= 96 bytes.

Targets must not be constructed by extracting valid substrings from an otherwise
invalid surface.

### Same-owner winner priority

When multiple qualified matches share one canonical owner, the winner is chosen
deterministically by:

1. larger `ratio_permille`;
2. smaller edit distance;
3. `CanonicalTerm` before `Alias`;
4. lexicographically smaller ASCII-lowercase target surface;
5. lexicographically smaller original target surface.

### Same-owner duplicate collapse

Within one canonical owner, duplicate targets sharing the same
`normalized_letters` collapse deterministically by:

1. `CanonicalTerm` before `Alias`;
2. lowercase target surface ascending;
3. original target surface ascending.

Cross-owner identical normalized targets remain separate so ambiguity
suppression can observe them.

### Identities

The authorized producer uses:

* detector: `ascii-latin-phonetic-similarity/0.1.0`;
* detector config: `canonical-session-term-cue-local/0.2.0`;
* algorithm: `canonical-exact-plus-ascii-double-metaphone-levenshtein/rphonetic-3.0.6-v1`;
* active canonical detector set including glossary, observed-error-form, and
  phonetic detectors in fixed order.

Detectors fail closed when the supplied `AnalysisRun` does not match the
transcript revision, session terms, detector set, detector config, and
algorithm they require.

### Human authority and materialization

Phonetic evidence is non-binding. It must never directly modify canonical
transcript state, create an accepted decision, auto-apply an alternative, or
bypass `ReviewLedger`.

## Explicitly not authorized

This decision does **not** authorize or claim:

* CJK, pinyin, or non-ASCII-Latin phonetic comparison;
* cross-cue matching;
* generic acronym normalization such as `K8s` -> `Kubernetes`;
* ranking, prioritization, or auto-edit authority for phonetic findings;
* validated mechanism effectiveness or product-level recall/precision thresholds;
* use of the experimental retrieval or ranking sidecar in the canonical detector
  path;
* durable storage of suppressed ambiguities.

Future versions may revise this policy based on user problems and measured
evidence.

## Relationship to MD-004

MD-004 owns the effective analysis-identity boundary and phonetic-evidence shape.
MD-004's historical statement that it does not authorize phonetic matching
implementation remains unchanged.

MD-005 is the producer authorization that follows once those boundaries were
implemented.

## Consequences

The canonical session-term pipeline may emit bounded phonetic `CandidateSpan`
values alongside exact glossary and observed-error findings.

Effectiveness, threshold tuning, and broader language support remain pending
real-material measurement and future governance decisions.
