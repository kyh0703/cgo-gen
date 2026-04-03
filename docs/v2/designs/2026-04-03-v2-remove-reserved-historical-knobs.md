---
feature: remove-reserved-historical-knobs
status: plan_ready
created_at: 2026-04-03T09:30:36+09:00
---

# Remove Reserved Or Historical Knobs

## Goal

Remove obsolete or reserved config knobs that still appear in public docs or compatibility parsing paths so the public config surface matches current supported behavior.

## Context / Inputs
- Source docs:
  - `README.md`
  - `README.ko.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - `README` and `README.ko` still document a `Reserved Or Historical Knobs` section.
  - `src/config.rs` still parses `project_root` and several `policies.*` compatibility fields.
  - Repository docs still contain older `files.model` / `files.facade` references that may no longer match the intended public config contract.
- User brief:
  - Remove the remaining reserved or historical knob references instead of keeping them documented as inactive.

## Plan Handoff
### Scope for Planning
- Confirm the intended public config keys by tracing the current config loader and user-facing docs.
- Remove user-facing documentation for reserved or historical knobs.
- Remove dead compatibility parsing and update fixtures/tests if those keys are no longer part of the supported config surface.
- Keep the change narrow to config surface and directly affected docs/tests.

### Success Criteria
- Public docs no longer advertise reserved or historical config knobs as special inactive keys.
- Current config parsing and examples no longer accept or rely on the removed knobs if they are meant to be fully deleted.
- Tests and fixtures reflect the reduced config surface and continue to pass.

### Non-Goals
- Adding new replacement config knobs.
- Broad redesign of config semantics unrelated to the listed keys.
- Cleaning up unrelated historical docs outside the paths touched by this change.

### Open Questions
- Should `files.model` and `files.facade` remain active internal/public config keys or be removed only from specific public-facing docs?
- Are any listed knobs still required by checked-in examples or tests for backward-compat coverage?

### Suggested Validation
- Targeted Rust tests covering config deserialization and existing fixture-based generation flows.
- Repository search confirming the reserved/historical knob section and removed keys no longer appear in current public docs where they should not.

### Parallelization Hints
- Candidate write boundaries:
  - Docs updates in `README.md`, `README.ko.md`, and versioned docs.
  - Config/parser/test updates under `src/` and `tests/`.
- Shared files to avoid touching in parallel:
  - `src/config.rs`
  - any shared config fixtures referenced by multiple tests
- Likely sequential dependencies:
  - decide the real supported key set first, then update code/fixtures/docs to match it
