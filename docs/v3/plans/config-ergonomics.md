# Config Ergonomics

## Goal

- make common YAML configs shorter by expanding header/include directories and inferring model roles from explicit facade selections.

## References

- `AGENTS.md`
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v3/designs/2026-03-31-v3-config-ergonomics.md`

## Workspace

- Branch: `feat/v3-config-ergonomics`
- Base: `master`
- Isolation: required
- Created by: `git worktree`

## Baseline

- Command: `cargo test`
- Expected: clean baseline in the isolated worktree

## Task Graph

### Task T1

- Goal: add config parsing helpers for `project_root`, `header_dirs`, and `include_dirs`, and expand them into the existing normalized path structures.
- Depends on:
  - none
- Write Scope:
  - `src/config.rs`
  - `tests/config.rs`
- Checks:
  - `cargo test config`
- Parallel-safe: no

### Task T2

- Goal: infer model roles from explicit facade selections and refresh example YAML to the shorter config shape.
- Depends on:
  - T1
- Write Scope:
  - `src/config.rs`
  - `examples/simple-go-struct/config.yaml`
  - config/example tests under `tests/`
- Checks:
  - `cargo test example_simple_go_struct`
  - `cargo test config`
- Parallel-safe: no

### Task T3

- Goal: prove the shorter config form does not regress the full generator path.
- Depends on:
  - T2
- Write Scope:
  - `tests/`
  - docs only if wording needs a follow-up note
- Checks:
  - `cargo test`
- Parallel-safe: no
