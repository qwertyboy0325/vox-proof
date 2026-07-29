# VoxProof Native Desktop Foundation

This crate is the bounded native review surface selected by MD-016. It runs
in one Rust process and calls the core `ApplicationReviewSession` directly.
The GUI owns only transient presentation state; the in-memory application
session remains the canonical owner of human decisions.

## Run

```sh
cargo run --locked -p voxproof-desktop
```

The resolved desktop dependencies are eframe `0.35.0`, egui `0.35.0`,
font-kit `0.14.3`, and rfd `0.15.4`. The test-only temporary-directory
dependency resolves to tempfile `3.27.0`. No CJK font file is bundled; the app
tries a bounded list of installed system fonts.

## Inputs and declarations

The setup screen reads two user-selected UTF-8 files:

- an existing SubRip (`.srt`) transcript, parsed by the core SRT parser;
- provisional session terms, one entry per line in the core format:
  `canonical term | alias:alternate form | error:observed form`.

It also requires a material-use basis (`SelfOwned` or
`ExplicitPermission`), an operator role, and a display label. These are
caller declarations only. VoxProof does not authenticate the operator,
inspect the operating-system account, independently verify permission, or
provide legal authorization.

## Review controls

- `Up` / `Down`: previous or next review case.
- `1`–`9`: select an available alternative.
- `A`: accept the selected alternative.
- `R`: reject.
- `D`: defer.
- `M`: mark as needing manual correction (signal only; no text is captured).
- The single-line “Governed Manual Replacement” field records an MD-017 exact
  replacement for the selected existing ReviewCase through
  `ApplicationReviewSession`. Its widget draft is non-authoritative until the
  command succeeds.
- `Cmd+L` on macOS or `Ctrl+L` on other supported egui platforms: focus
  non-authoritative search.
- `Escape`: release search focus or dismiss a non-destructive native picker.

Decision shortcuts use non-repeat key-press events and are suppressed while a
text edit wants keyboard input, while detectable IME composition is active,
or while a native blocking picker owns input.

## Preview and export

“Preview — not final export” is a read-only projection derived by
`ApplicationReviewSession::derive_current_projection`. Final projections are
available only after complete decision coverage and originate from
`ApplicationReviewExportBundle` plus the core application-export renderers.

Choosing one destination directory creates exactly:

```text
<source-stem>.voxproof-reviewed.srt
<source-stem>.voxproof-decisions.txt
<source-stem>.voxproof-session-summary.txt
```

The stem falls back to `transcript`. Before any write, the adapter reports
every existing destination and writes nothing. Each actual write uses
create-new semantics, so existing files are never overwritten. On a later
write failure it best-effort removes only files created by that attempt,
preserves the original error, and separately reports cleanup failures. This
is bounded cleanup, not a transactional or crash-safe guarantee.

A successful decision revision after export clears the in-app
export-complete indicator and requires a fresh export for the current session
state. The previously exported files remain untouched on disk and represent
the earlier exported state. This is transient UI-state coherence, not
versioning, persistence, or export history.

## Limitations and non-goals

Sessions are memory-only: reset or process exit discards them. There is no
autosave, hydration, durability, or crash recovery. AccessKit integration and
meaningful widget labels are enabled, but accessibility conformance is not
claimed. Traditional Chinese glyph rendering passed the local native smoke;
IME commit/cancel and shortcut-suppression evidence is classified separately
in that smoke report because candidate/pre-edit inspection may be incomplete.

The bounded MD-017 extension adds Manual Replacement text only for existing
detector-raised ReviewCases. It does not add human-raised cases, arbitrary
ranges, deletion, multiline editing, persistence, ASR, media playback,
Correction Memory, cross-material reuse, authentication, telemetry, network
access, packaging, an installer, updater, cross-platform support claims,
production readiness, or version establishment.
