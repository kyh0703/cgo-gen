# Go Struct Name Capitalization

## Goal
- Capitalize generated Go struct names for lowercase-leading C++ class names so the public Go surface stays exported and usable.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-02-v2-go-struct-name-capitalization.md`

## Workspace
- Branch: `feat/v2-go-struct-name-capitalization`
- Base: `master`
- Isolation: required
- Created by: `exec-plan` via `git-worktree`

## Task Graph
### Task T1
- Goal: route facade class Go names through the export-name helper so lowercase-leading class names become exported.
- Depends on:
  - none
- Write Scope:
  - `src/facade.rs`
- Read Context:
  - facade class collection
  - Go naming helpers
- Checks:
  - rendered facade output uses exported struct names
- Parallel-safe: no

### Task T2
- Goal: apply the same capitalization rule to model projection generation so shared model wrappers stay aligned with facade naming.
- Depends on:
  - T1
- Write Scope:
  - `src/facade.rs`
- Read Context:
  - model projection building
  - constructor/destructor projection naming
- Checks:
  - lowercase-leading class names no longer leak unexported Go model types
- Parallel-safe: no

### Task T3
- Goal: add regression tests for helper behavior and rendered Go output from a lowercase-leading C++ class.
- Depends on:
  - T2
- Write Scope:
  - `src/facade.rs` test module
- Read Context:
  - current facade rendering tests
- Checks:
  - `cargo test facade`
- Parallel-safe: no

## Notes
- The fix is intentionally limited to Go-facing export naming. It does not rename C++ symbols or alter overload disambiguation.
