# Experimental Contextual Resolution Sidecar

Status: exploratory / non-binding

Owns:
How to run the current experiment-only contextual retrieval and ranking sidecar.

Does not own:
Canonical evidence, review identity, policy, SessionContext, Domain Collection, projection, automation, provider, or persistence contracts.

Last reviewed against code: experimental contextual-resolution slice

---

`review-experiment` runs bounded experimental retrieval beside the existing exact
review loop. Its non-exact reports and ranking results are written only to a
versioned JSON sidecar. They are not `CandidateSpan`s, `ReviewCase`s,
`ReviewLedger` decisions, or materialized edits.

Experimental candidate IDs are run/request-local correlation IDs. They are not
canonical candidate or review identity and cannot be reused across runs.

## Synthetic invocation

```text
cargo run -- review-experiment \
  tests/fixtures/contextual-resolution-synthetic/input.srt \
  tests/fixtures/contextual-resolution-synthetic/session-terms.txt \
  tests/fixtures/contextual-resolution-synthetic/session-description.txt \
  rules-only \
  /tmp/experimental-report.json \
  /tmp/reviewed.srt \
  /tmp/decisions.txt \
  /tmp/session-summary.txt
```

Use `fake` in place of `rules-only` for the deterministic contextual-ranking
fake. Both modes require human review. A selection records an
experiment-only `manual_correction_requested` marker in the sidecar and gives
alias-and-rerun guidance; it cannot alter the reviewed SRT.

## Experimental retrieval eligibility profiles

The default pinyin profile is
`suppress_short_han_to_short_uppercase_acronym_v1`. It suppresses the measured
short-Han-to-short-uppercase-acronym collision class before experimental
ranking. Set:

```text
VOX_PROOF_EXPERIMENT_PINYIN_PROFILE=unfiltered-baseline-v1
```

to reproduce the earlier unfiltered candidate behavior. The unfiltered profile
is retained only as a calibration baseline, not as a recommended runtime
setting.

The independent default Latin source-span profile is
`suppress_target_embedded_in_larger_window_v1`. It suppresses Latin candidates
whose complete canonical token sequence is already embedded inside a larger
source window. Set:

```text
VOX_PROOF_EXPERIMENT_LATIN_SPAN_PROFILE=unfiltered-baseline-v1
```

to reproduce the previous Latin span behavior. This unfiltered profile is also
retained only as a calibration baseline. The default can be selected explicitly
with
`VOX_PROOF_EXPERIMENT_LATIN_SPAN_PROFILE=suppress-target-embedded-in-larger-window-v1`.

Experimental sidecar schema v3 records both `pinyin_eligibility_profile` and
`latin_span_eligibility_profile` so runs remain distinguishable. These profiles
are experiment-specific retrieval configurations. Neither establishes formal
matching-policy, language-policy, deletion, or cleanup semantics.

`external-command` is optional and requires
`VOX_PROOF_EXPERIMENT_COMMAND` to name a local executable. The command receives
one versioned JSON request on standard input and must return one strict JSON
response on standard output. Invalid output, unknown candidate IDs, a timeout,
or a command failure falls back to rules-only behavior. No credential is read by
this experiment. The runner drains bounded standard output and standard error
while its direct child runs, then kills that direct child on timeout or output
overflow. It does not guarantee cleanup of descendant processes created by the
external command.

## Current limitations

- Candidates are restricted to the current provisional session terms.
- Latin distance and bounded pinyin are retrieval signals, not confidence or
  phrase truth.
- Symbols, acronyms, numeronyms, and transliterations require explicit session
  aliases; the non-exact producers deliberately skip them.
- The provided fixture is synthetic and does not validate product behavior.
- This experiment does not establish final SessionContext, policy, Domain
  Collection, phonetic-evidence, LLM, automation, or projection architecture.
- The sidecar, reviewed SRT, decision log, and session summary are separate
  filesystem writes; they are not atomically committed together and a
  filesystem failure can leave partial experiment outputs.
