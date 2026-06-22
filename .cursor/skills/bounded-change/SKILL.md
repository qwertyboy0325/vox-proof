---
name: bounded-change
description: Reusable workflow for making one small, bounded VoxProof implementation change. Use when implementing a narrow code change, fixing a focused defect, or extending a single capability without expanding scope.
---

# Bounded Change

Use this workflow for a small future implementation change. Keep scope narrow, reversible, and easy to review.

## Workflow

1. Read the relevant canonical VoxProof documentation under `docs/` before touching code.
2. State the narrow intended outcome.
3. State explicit out-of-scope items.
4. Inspect only the smallest relevant code surface.
5. Make one coherent change.
6. Add or update the narrowest relevant validation, only when implementation exists.
7. Report:
   - verified facts;
   - files changed;
   - validation run;
   - result;
   - unresolved ambiguity;
   - intentionally deferred work.

## Prohibitions

- Do not perform unrelated cleanup.
- Do not introduce speculative abstractions.
- Do not expand the product beyond what the canonical documents describe.
- Do not convert an inspection task into implementation without explicit authorization.
