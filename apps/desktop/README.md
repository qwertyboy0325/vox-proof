# VoxProof Native Desktop Foundation

This crate is the bounded native review surface selected by MD-016. It runs
in one Rust process and calls the core `ApplicationReviewSession` through
`DurableApplicationSession`. The GUI owns only transient presentation state;
human decisions remain durable in the local SQLite session store.

## Run (development)

```sh
cargo run -p voxproof-desktop
```

## Run (macOS pilot build)

```sh
./scripts/macos-bundle.sh
open dist/VoxProof.app
```

The bundle script builds a release binary, assembles `dist/VoxProof.app`, and
applies ad-hoc signing when `codesign` is available. Notarization and
distribution signing are out of scope for this bounded UX closure.

The resolved desktop dependencies are eframe `0.35.0`, egui `0.35.0`,
font-kit `0.14.3`, and rfd `0.15.4`. No CJK font file is bundled; the app
tries a bounded list of installed system fonts.

## New review flow

1. Choose an SRT transcript.
2. Add terms to watch in the app (optional import from a terms file).
3. Confirm permission and role with separate declarations.
4. Start review.

Session terms may also be imported from the legacy line-oriented format:

```text
canonical term | alias:alternate form | error:observed form
```

## Review controls

- `Up` / `Down`: previous or next review item.
- `1`–`9`: select a suggested correction.
- `A`: accept the selected suggestion.
- `R`: reject.
- `D`: defer.
- `M`: mark as needing manual correction.
- The **Correct text** field records an exact replacement for the selected item.
- `Cmd+L` on macOS or `Ctrl+L` elsewhere: focus search.
- `Escape`: release search focus.

Project-memory features remain available under **Advanced — project memory** but
are not part of the primary external-user workflow.

## Preview and export

**Preview reviewed subtitles** shows the current reviewed projection. Export
writes a reviewed SRT plus companion decision and session-summary files into a
chosen folder. Export requires every review item to have a decision.

## Resume and presentation metadata

Recent reviews are listed by human-readable source name when available. A
non-authoritative `desktop-presentation.json` sidecar beside each `session.db`
stores display metadata such as the source file name. The sidecar is optional,
fail-soft, and never participates in authority reconstruction.

Writable resume refuses when another process already holds writer ownership.
Explicit read-only open is offered only after a writable open reports that
condition; other writable failures do not change access mode automatically.

## Limitations and non-goals

This bounded surface does not claim production readiness, installer polish,
auto-update, cross-platform packaging, accessibility conformance, built-in ASR,
media playback, or version establishment.
