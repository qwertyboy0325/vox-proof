---
name: repository-audit
description: Reusable read-only workflow for auditing VoxProof code, documentation, or tests against a specific question. Use when checking whether claims match reality, finding stale or contradictory documentation, or assessing current state without making changes.
---

# Repository Audit

Use this read-only workflow to answer an audit question about the repository. This workflow does not modify files.

## Workflow

1. State the exact audit question.
2. Inspect code, docs, and tests only as necessary to answer it.
3. Classify each finding as:
   - verified;
   - contradicted;
   - stale wording;
   - ambiguity;
   - missing evidence;
   - out of scope.
4. Distinguish current behavior from intended future behavior.
5. Propose only bounded next actions.

## Prohibition

- Never modify files unless a separate instruction explicitly authorizes changes.
