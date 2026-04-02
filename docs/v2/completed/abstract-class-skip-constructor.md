# Abstract Class Constructor Skip

## Goal
- Omit constructor wrapper generation for abstract C++ classes so generated native code no longer attempts invalid instantiation.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-02-v2-abstract-class-skip-constructor.md`

## Workspace
- Branch: `feat/v2-abstract-class-skip-constructor`
- Base: `master`
- Isolation: required
- Created by: `exec-plan` via `git-worktree`

## Task Graph
### Task T1
- Goal: detect abstract classes during parsing by marking classes that contain pure virtual methods.
- Depends on:
  - none
- Write Scope:
  - `src/parser.rs`
- Read Context:
  - parsed class metadata
  - libclang class/method traversal
- Checks:
  - abstract class fixture parses without losing method information
- Parallel-safe: no

### Task T2
- Goal: skip `_new()` wrapper generation for abstract classes while preserving destructor and method generation where valid.
- Depends on:
  - T1
- Write Scope:
  - `src/ir.rs`
- Read Context:
  - `src/parser.rs`
  - existing constructor emission logic
- Checks:
  - skipped metadata records the abstract-class omission
- Parallel-safe: no

### Task T3
- Goal: add a regression test covering abstract-class omission and normal concrete-class constructor generation.
- Depends on:
  - T2
- Write Scope:
  - `tests/abstract_class_skip.rs`
- Read Context:
  - IR normalization behavior for classes
- Checks:
  - `cargo test abstract_class_skip`
- Parallel-safe: no

## Notes
- The change intentionally stops only constructor wrapper emission. It does not suppress destructor or non-constructor wrapper generation for abstract classes.
