# MD-016: Native Desktop Framework and In-Process Integration

Status: accepted

Date: 2026-07-29

Accepted: 2026-07-29 per explicit owner authorization

Decision authority: Ezra

Classification: VoxProof v0.2 native desktop foundation

## Context

The conditional GUI boundary in `docs/product/v0.1-execution-order.md` permits a thin, local validation shell for facilitated external testing while keeping domain truth and materialization in the Rust core.

The v0.2 C4 draft left the desktop framework, process model, and application integration boundary unresolved. Since then, the governed in-memory application service has established `ApplicationReviewSession` as the application-facing authority boundary for review items, human decisions, progress, current projection, and export materialization.

The owner accepted the recommendation from `GUI_FRAMEWORK_THREE_WAY_SPIKE`. The accepted evidence establishes that egui can support the interactive dense review workflow and render CJK glyphs. IME composition and accessibility inspection remain open risks rather than proven capabilities.

## Decision

VoxProof v0.2 adopts:

```yaml
framework: eframe/egui
baseline_version: 0.35.0
process_model: single_native_Rust_process
integration: direct_in_process
canonical_application_state: ApplicationReviewSession
GUI_state: non_authoritative
```

The native desktop executable links the existing VoxProof Rust library directly in the same process. The GUI presents state and forwards explicit human intent through `ApplicationReviewSession`; it does not reproduce review, decision, projection, or export semantics.

`eframe` / `egui` version `0.35.0` is the accepted implementation baseline for the first bounded desktop foundation. A dependency upgrade is not implicitly accepted merely because a newer release exists.

## Authority boundary

- `ApplicationReviewSession` is the canonical application state for the in-memory v0.2 desktop foundation.
- egui widget state, focus, selection, filters, navigation, drafts, layout, and other view-local state are non-authoritative.
- Human decisions must flow through the application-service command boundary.
- Reviewed output and export artifacts must flow through the application-service materialization/export boundary.
- The GUI must not directly mutate `ReviewLedger`, derive reviewed SRT independently, or become a second source of session truth.
- Closing or restarting the process may lose the in-memory session until a separately accepted persistence mechanism is integrated. The GUI must not imply durability that does not exist.

## Accepted evidence and limits

Accepted evidence source: `GUI_FRAMEWORK_THREE_WAY_SPIKE`.

| Capability or risk | Accepted status |
| --- | --- |
| Interactive dense review workflow | Proven in the spike |
| CJK glyph rendering | Proven in the spike |
| IME composition | Open risk |
| Accessibility inspection | Open risk |

The spike evidence supports framework selection only. It does not establish product GUI completion, production readiness, supported-platform coverage, packaging or signing readiness, user validation, accessibility conformance, or reliable IME behavior.

## Consequences

- The desktop foundation can be implemented without FFI, IPC, a localhost service, a webview, or a second application-language runtime.
- The Rust domain and application-service layers remain independently testable and UI-agnostic.
- The first GUI slice should remain the thin local flow already bounded by `docs/product/v0.1-execution-order.md`.
- IME composition and accessibility inspection require explicit validation before any related capability claim.
- Persistence, recovery, packaging, signing, media playback, platform support, and distribution remain separate decisions or work packages.

## Rejected alternatives for this foundation

- Tauri or another webview shell.
- Native Swift/AppKit as the product shell.
- A browser UI backed by a localhost server.
- FFI, IPC, gRPC, or another process boundary between the first desktop shell and the Rust application service.
- GUI-owned review or decision state.

These alternatives are rejected for the v0.2 native desktop foundation. Reconsidering the framework or process boundary requires a new or superseding Material Decision.

## Explicitly deferred

- IME acceptance criteria and cross-platform IME evidence.
- Accessibility target, inspection method, and conformance claim.
- Supported desktop platforms.
- Packaging, signing, notarization, installation, and updates.
- Session persistence mechanism and production integration.
- Media playback.
- Manual-correction payload semantics and undo semantics.
- Evaluation tooling exposure in the desktop product.

## Implementation authorization boundary

This decision records the durable framework and integration choices. It does not by itself define a substantial implementation work package.

Implementation requires an owner-authorized work package satisfying `.cursor/rules/voxproof-work-packages.mdc`, including bounded file, semantic, and claim scope; required checks; escalation conditions; stop gate; model-budget policy; and final-conflict-review posture.

## Supersedes

The unresolved desktop framework and application integration rows in `docs/architecture/v0.2-c4-architecture.md`.
