# Test Index Macro Regression

## Goal
- add a regression test that reproduces `#define TEST_INDEX 10` and verifies it survives parsing, IR normalization, and Go constant emission

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-13-v2-test-index-macro-regression.md`
- `tests/generator.rs`

## Workspace
- Branch: feat/v2-test-index-macro-regression
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: extend the existing standalone integer macro regression so `TEST_INDEX` is present in the header snippet and asserted in parsed macros, IR constants, and generated Go output
- Depends on:
  - none
- Write Scope:
  - `tests/generator.rs`
- Read Context:
  - `docs/v2/designs/2026-04-13-v2-test-index-macro-regression.md`
  - `tests/generator.rs`
- Checks:
  - `cargo test generator`
- Parallel-safe: no

## Notes
- keep the change test-only unless the new assertion exposes an actual regression
