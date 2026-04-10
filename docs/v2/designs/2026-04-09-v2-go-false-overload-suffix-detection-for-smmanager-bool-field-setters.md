---
feature: go-false-overload-suffix-detection-for-smmanager-bool-field-setters
status: plan_ready
created_at: 2026-04-09T17:46:44+09:00
---

# Go False Overload Suffix Detection For Smmanager Bool Field Setters

## Goal

Fix generated Go method names so non-overloaded bool field setters do not pick up a spurious `Bool` suffix in `smmanager/public_wrapper.go`.

## Context / Inputs
- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - struct field setter IR names are synthesized as `Set<FieldName>` in `src/codegen/ir_norm.rs`.
  - Go facade export naming currently treats any raw C symbol containing `__` as an overloaded API.
  - underscore-backed owner names such as `_SYS_IF_MONITOR_IODSM` naturally produce raw symbols like `cgowrap__SYS_IF_MONITOR_IODSM_SetBModifyFlag`, even when there is no overload.
  - that false positive causes `go_overload_suffix()` to append the bool token, producing names like `SetBModifyFlagBool`.
  - generated `smmanager/public_wrapper.go` currently contains repeated false positives such as `SetBModifyFlagBool`.
- User brief:
  - `SetBModifyFlagBool` should not have the trailing `Bool`.

## Plan Handoff
### Scope for Planning
- Keep the fix scoped to Go facade export-name detection.
- Distinguish true overload-disambiguated raw symbols from symbols that merely contain `__` because of preserved owner names.
- Add regression coverage for a non-overloaded underscore-backed bool setter and confirm it renders as `SetBModifyFlag`.
- Re-check the representative `smmanager` output for the known `SetBModifyFlagBool` sites.

### Success Criteria
- Non-overloaded bool field setters render as `SetBModifyFlag`, not `SetBModifyFlagBool`.
- Real overload-disambiguated methods still keep deterministic suffixes where required.
- Regression tests fail before the fix and pass after it.

### Non-Goals
- Do not broaden this slice to `GetItem` or `operator[]` handling.
- Do not redesign raw C symbol naming.
- Do not manually patch generated `smmanager/public_wrapper.go`; the fix must come from the generator.

### Open Questions
- none

### Suggested Validation
- `cargo test go_facade`
- `cargo test generator`
- `cargo test`
- `rg -n "SetBModifyFlagBool|SetBModifyFlag\\(" smmanager/public_wrapper.go`

### Parallelization Hints
- Candidate write boundaries:
  - `src/codegen/go_facade.rs`
  - `tests/generator.rs`
- Shared files to avoid touching in parallel:
  - `src/codegen/go_facade.rs`
  - `tests/generator.rs`
- Likely sequential dependencies:
  - tighten overload detection first
  - then add regression coverage
  - then verify the `smmanager` output
