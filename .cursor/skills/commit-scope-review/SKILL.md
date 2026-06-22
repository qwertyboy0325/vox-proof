---
name: commit-scope-review
description: Read-only workflow for reviewing a mixed working tree or proposed commit before staging or committing. Use when checking commit scope, separating accidental changes, or deciding what belongs in a commit.
---

# Commit Scope Review

Use this workflow to review commit scope before staging or committing. It is read-only by default.

## Workflow

1. State the intended commit purpose, or explicitly state that it is unknown.
2. Inspect the narrowest relevant Git evidence:
   - `git status --short`
   - `git diff --name-status`
   - `git diff --cached --name-status`
   - `git diff --check`
   - unstaged diff and staged diff only as needed
   - untracked files via `git ls-files --others --exclude-standard` when relevant
3. Classify every changed file or hunk as one of:
   - intended commit;
   - separate logical change;
   - local or private material;
   - generated or accidental artifact;
   - likely revert candidate;
   - requires user decision.
4. Produce a proposed commit grouping.
5. When a file contains multiple unrelated edits, identify whether hunk-level staging is needed and recommend `git add -p`.
6. Check whether suspicious untracked material is ignored correctly before suggesting a commit.
7. Report the smallest safe next action.

## Safety Boundaries

Do not run or recommend automatic execution of these commands unless the user explicitly asks for that operation after seeing the review result:

- `git add`
- `git commit`
- `git push`
- `git reset`
- `git restore`
- `git stash`
- `git rebase`
- `git commit --amend`

You may describe safe commands for the user to run after approval.

## Already-Committed Changes

Distinguish between:

- uncommitted mixed work;
- committed but not pushed work;
- pushed work.

For pushed commits, default to preserving published history and propose a corrective follow-up commit or revert plan. Do not suggest history rewriting unless the user explicitly requests it.

## Required Report

Return:

- intended commit purpose;
- evidence inspected;
- file/hunk classification;
- proposed commit groups;
- files that should remain local or ignored;
- validation result;
- commands requiring explicit user approval;
- unresolved decisions.

Do not invent Git state, commit history, or push status.
