# Timeval Wrapper Stability

## Goal
- Stabilize wrapper generation for `timeval`-based signatures by normalizing them as external C structs and rendering compile-safe header, Go, and C++ wrapper output.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-02-v2-timeval-wrapper-stability.md`
- `docs/v2/designs/2026-04-02-v2-timeval-support.md`

## Workspace
- Branch: `feat/v2-timeval-wrapper-stability`
- Base: `master`
- Isolation: required
- Created by: `exec-plan` via `git-worktree`

## Task Graph
### Task T1
- Goal: classify `timeval` / `struct timeval` pointer and reference signatures as explicit external-struct IR kinds, including canonical alias fallback from `timeval*`.
- Depends on:
  - none
- Write Scope:
  - `src/ir.rs`
- Read Context:
  - current type normalization logic
  - existing timeval-support behavior
- Checks:
  - IR normalization tests for pointer/reference/alias forms
- Parallel-safe: no

### Task T2
- Goal: render header and Go facade output for external timeval structs using `<sys/time.h>` and `*C.struct_timeval` where needed.
- Depends on:
  - T1
- Write Scope:
  - `src/generator.rs`
  - `src/facade.rs`
- Read Context:
  - `src/ir.rs`
  - current generated header and cgo preamble behavior
- Checks:
  - rendered header contains `<sys/time.h>`
  - rendered Go output uses `*C.struct_timeval`
- Parallel-safe: no

### Task T3
- Goal: verify end-to-end timeval stability with regression tests for normalization, rendered header/go output, and generated C++ reference-call handling.
- Depends on:
  - T2
- Write Scope:
  - `tests/timeval_support.rs`
- Read Context:
  - `src/ir.rs`
  - `src/generator.rs`
  - `src/facade.rs`
- Checks:
  - `cargo test timeval_support`
- Parallel-safe: no

## Notes
- Keep the scope focused on compile-stable wrapper generation for timeval signatures. Do not broaden this plan into generic external-struct modeling.
