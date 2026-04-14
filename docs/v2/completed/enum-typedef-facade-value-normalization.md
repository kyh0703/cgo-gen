# Enum Typedef Facade Value Normalization

## Goal
- `typedef enum` alias parameters used by value in class methods should stay value enums across raw and Go facade generation instead of becoming opaque `Handle*` model wrappers.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-10-v2-enum-typedef-facade-value-normalization.md`
- `sample_manager/public_wrapper.go`
- `sample_manager/manager_wrapper.h`
- `sample_manager/manager_wrapper.cpp`
- `sample_manager/manager_wrapper.go`

## Workspace
- Branch: feat/v2-enum-typedef-facade-value-normalization
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: typedef enum aliases that resolve to by-value enums are classified consistently for facade generation, so class method parameters stop flowing through model-handle rendering.
- Depends on:
  - none
- Write Scope:
  - `src/codegen/`
  - `src/analysis/`
- Read Context:
  - `docs/v2/designs/2026-04-10-v2-enum-typedef-facade-value-normalization.md`
  - `src/codegen/`
  - `src/analysis/`
- Checks:
  - powershell -NoProfile -Command "Set-Location 'D:/Project/cgo-gen/.worktrees/enum-typedef-facade-value-normalization'; cargo test enum_typedef"
  - powershell -NoProfile -Command "Set-Location 'D:/Project/cgo-gen/.worktrees/enum-typedef-facade-value-normalization'; cargo test generator"
- Parallel-safe: no

### Task T2
- Goal: add a focused regression that proves facade output for typedef enum by-value parameters emits value enums in C and Go wrappers, then verify the `sample_manager` sample no longer contains `PROCESS_KINDHandle*`.
- Depends on:
  - T1
- Write Scope:
  - `tests/`
  - generated `sample_manager/` output for verification only
- Read Context:
  - `src/codegen/`
  - `src/analysis/`
  - `sample_manager/`
- Checks:
  - powershell -NoProfile -Command "Set-Location 'D:/Project/cgo-gen/.worktrees/enum-typedef-facade-value-normalization'; cargo test"
  - manual: if `sample_manager` is regenerated later, confirm `PROCESS_KINDHandle` / `type ProcessKind struct` no longer appear
- Parallel-safe: no

## Notes
- Keep the fix limited to enum typedef alias routing and the regression needed to prevent the facade/public mismatch from returning.
