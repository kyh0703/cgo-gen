# Callback Facade Support

## Goal

Support named callback typedefs and callback registration facade APIs in generated IR and Go output so callback-driven registration APIs such as `SetEventCallback` are no longer skipped.

## Workspace

- Branch: `feat/callback-facade-support`
- Base: `main`
- Isolation: linked worktree required before implementation

## Task Graph

### T1

- Task ID: `ir-callback-types`
- Goal: preserve named callback typedefs in parser/IR and stop skipping declarations that consume those typedefs only because they are callbacks
- Depends on: none
- Write Scope: `src/parser.rs`, `src/ir.rs`
- Read Context: `docs/ARCHITECTURE.md`, callback-related tests
- Checks: callback typedef fixture normalizes successfully and is no longer recorded as skipped
- Parallel-safe: no

### T2

- Task ID: `go-callback-facade-render`
- Goal: render Go-facing callback types and callback registration facade wrappers using `func(...)` signatures plus generated bridge support
- Depends on: `ir-callback-types`
- Write Scope: `src/facade.rs`, supporting generator modules
- Read Context: current Go facade rendering path, raw wrapper include strategy
- Checks: generated Go output contains callback type aliases and callback registration functions for fixture input
- Parallel-safe: no

### T3

- Task ID: `callback-regression-tests`
- Goal: add regression coverage for callback typedef parsing and callback facade generation
- Depends on: `go-callback-facade-render`
- Write Scope: `tests/function_pointer_skip.rs`, new callback generation tests, fixture files as needed
- Read Context: existing facade generation tests and fixture layout
- Checks: targeted cargo tests pass
- Parallel-safe: no
