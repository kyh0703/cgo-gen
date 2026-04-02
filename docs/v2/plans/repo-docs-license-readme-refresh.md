# Repo Docs, License, And README Refresh

## Goal
- Recover the missing v2 documentation records, add a root `MIT` license, and rewrite `README.md` as a detailed bilingual public project guide without SIL-specific internal examples.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `AGENTS.md`
- `docs/v2/designs/2026-04-02-v2-repo-docs-license-readme-refresh.md`

## Workspace
- Branch: `feat/v2-repo-docs-license-readme-refresh`
- Base: `master`
- Isolation: required
- Created by: `exec-plan` via `git-worktree`

## Task Graph
### Task T1
- Goal: restore the missing v2 design/plan/completed documents in a way that matches current repository conventions and the already-landed feature history.
- Depends on:
  - none
- Write Scope:
  - `docs/v2/designs/2026-04-02-v2-abstract-class-skip-constructor.md`
  - `docs/v2/completed/abstract-class-skip-constructor.md`
  - `docs/v2/designs/2026-04-02-v2-go-struct-name-capitalization.md`
  - `docs/v2/completed/go-struct-name-capitalization.md`
  - `docs/v2/designs/2026-04-02-v2-timeval-wrapper-stability.md`
  - `docs/v2/plans/timeval-wrapper-stability.md`
  - `docs/v2/plans/raw-output-flattening.md`
- Read Context:
  - current `docs/v2/designs/`
  - current `docs/v2/plans/`
  - current `docs/v2/completed/`
  - related feature commits and branch history
- Checks:
  - manual: restored doc names/content fit existing v2 naming conventions
  - `git diff -- docs/v2`
- Parallel-safe: no

### Task T2
- Goal: add top-level `MIT` licensing and rewrite `README.md` into a detailed Korean/English public-facing guide without private SIL-specific examples.
- Depends on:
  - T1
- Write Scope:
  - `LICENSE`
  - `README.md`
- Read Context:
  - `docs/ARCHITECTURE.md`
  - `docs/v2/designs/2026-04-02-v2-repo-docs-license-readme-refresh.md`
  - current CLI/config usage in the repository
- Checks:
  - manual: `README.md` no longer contains `iSiLib` or `IsAAMaster`
  - manual: usage instructions remain accurate against current repo layout
- Parallel-safe: no

### Task T3
- Goal: review the final documentation-only diff and prepare a clean commit/push.
- Depends on:
  - T2
- Write Scope:
  - staging area only
- Read Context:
  - `git status`
  - `git diff`
  - `git diff --staged`
- Checks:
  - `git status --short`
  - `git log --oneline -n 1`
  - `git push`
- Parallel-safe: no

## Notes
- Keep the scope surgical. Do not modify generator behavior or introduce unrelated documentation restructuring while performing the recovery.
