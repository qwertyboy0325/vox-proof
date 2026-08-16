# Lead A Assisted Pilot 01 — Evidence Packet

Gate 8 evidence packet for the first v0.2 Lead A assisted external pilot.

Protocol: `docs/product/v0.2-lead-a-assisted-pilot-protocol.md`

## Status

```text
pilot_status: not_run
establishment_adjudication: pending
```

Do not pre-fill PASS. Update status only after a real facilitated session and evidence adjudication.

## Packet Layout

Copy each `*.template.*` file to its non-template name before the session. Example:

```text
participant-profile.template.json  → participant-profile.json
input-manifest.template.json       → input-manifest.json
session-observation.template.json  → session-observation.json
facilitator-log.template.md        → facilitator-log.md
participant-response.template.md   → participant-response.md
export-manifest.template.json      → export-manifest.json
establishment-evaluation.template.md → establishment-evaluation.md
```

Committed artifacts should contain **metadata and digests only**. Keep transcript text, media, reviewed exports, and other sensitive material local.

## Minimum Committed Content

- pseudonymous participant ID (for example `lead-a-01`)
- SHA-256 digests for authorized inputs and exported outputs
- cue count and duration where available
- authorization basis and source type
- session timing, counts, intervention log, participant responses, and criteria adjudication

## Not Required In Repository

- participant legal name
- raw or reviewed subtitle text
- audio or video files
- screen recording (optional locally with consent)

## Negative Evidence

If the pilot fails, retain this packet with FAIL adjudication. Do not delete or replace it to hide failure. Successor pilots should use a new directory (for example `lead-a-pilot-02`).
