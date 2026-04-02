---
feature: repo-docs-license-readme-refresh
status: plan_ready
created_at: 2026-04-02T12:25:00+09:00
---

# Repo Docs, License, And README Refresh

## Goal

Recover local-only documentation that was left uncommitted, add an explicit `MIT` license for open-source distribution, and rewrite `README.md` as a detailed bilingual project guide without SIL-internal examples such as `iSiLib` or `IsAAMaster`.

## Context / Inputs

- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
  - `AGENTS.md`
- Existing system facts:
  - the repository is currently on `v2` with `active_plan: none`
  - previously observed local-only docs included design/completed notes for abstract-class constructor skipping, Go struct name capitalization, timeval wrapper stability, and a missing plan for raw output flattening
  - the current README still references older documentation layout and SIL-specific examples that the user wants removed
  - the repository does not yet expose a top-level open-source license file
- User brief:
  - recover the uncommitted docs instead of dropping them
  - add `MIT` licensing so the project can remain open source
  - rewrite `README.md` in Korean and English with as much practical detail as possible
  - remove explicit references to `iSiLib`, `IsAAMaster`, and similar SIL-specific internal examples from README

## Plan Handoff

### Scope for Planning

- recreate the missing `docs/v2/designs/`, `docs/v2/plans/`, and `docs/v2/completed/` records that were observed locally but are no longer present in `master`
- add a root `LICENSE` file with standard `MIT` text and align public-facing docs with that license choice
- replace `README.md` with a detailed bilingual guide that explains purpose, architecture, usage, config concepts, examples, development workflow, and repository layout without relying on SIL-private naming
- keep the change limited to repository documentation and licensing; do not broaden generator behavior or code semantics

### Success Criteria

- the missing v2 documentation records are restored in a coherent, reviewable form
- the repository contains a standard top-level `MIT` license file
- `README.md` is detailed, bilingual, and free of `iSiLib` / `IsAAMaster` references
- the resulting diff is limited to documentation/license material and is ready to commit and push

### Non-Goals

- changing generator logic, parser behavior, or test fixtures
- rewriting all architecture/versioned docs into bilingual form in one pass
- documenting private SIL integration details that should stay out of the public README
- retroactively rewriting unrelated completed records

### Open Questions

- whether the bilingual README should keep Korean-first ordering for every section or use separate Korean/English sections
- whether `README.md` should mention a future `CONTRIBUTING.md` even though that file does not yet exist

### Suggested Validation

- review restored docs for naming/date consistency with existing `v2` conventions
- search the new `README.md` to ensure removed SIL-specific identifiers no longer appear
- confirm `LICENSE` is present at the repository root
- inspect `git diff --stat` to verify the scope remains documentation/license only

### Parallelization Hints

- Candidate write boundaries:
  - `docs/v2/designs/`, `docs/v2/plans/`, `docs/v2/completed/`
  - `README.md`
  - `LICENSE`
- Shared files to avoid touching in parallel:
  - `README.md`
- Likely sequential dependencies:
  - restore docs context before final README wording so the public description matches the recovered repository state
