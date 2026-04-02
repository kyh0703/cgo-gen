---
feature: timeval-wrapper-stability
status: plan_ready
created_at: 2026-04-02T14:45:00+09:00
---

# Timeval Wrapper Stability

## Goal

Stabilize generated wrapper output for `timeval`-based parameters by treating `timeval` / `struct timeval` pointer and reference signatures as external C structs instead of model handles.

## Context / Inputs

- Source docs:
  - user-reported generation failure for wrappers that mention `timeval`
- Existing system facts:
  - prior normalization can misclassify `timeval*`-style signatures through the generic model-handle path
  - generated Go output can end up with invalid forms such as `*struct timeval`
  - generated header and Go preamble output may omit `<sys/time.h>` even when `struct timeval` appears in the wrapper surface
  - canonical type fallback can see `struct timeval*` even when the display spelling is `timeval*`
- User brief:
  - make timeval-oriented wrapper generation stable and compile-safe

## Plan Handoff

### Scope for Planning

- introduce explicit IR treatment for external C struct pointer/reference signatures such as `struct timeval*` and `struct timeval&`
- allow canonical fallback to normalize `timeval*` alias spelling into the same external-struct path
- render Go facade parameters for these signatures as `*C.struct_timeval`
- add `<sys/time.h>` to generated header and Go cgo preamble when required by the emitted API surface
- ensure generated C++ wrapper calls dereference external-struct references correctly
- add regression coverage for normalization, generated header/preamble content, and reference-call wrapper output

### Success Criteria

- `timeval*` and `struct timeval*` no longer fall through the model-handle path
- generated Go code avoids invalid `*struct timeval` syntax and uses `*C.struct_timeval`
- generated header/go preamble include `<sys/time.h>` whenever timeval signatures are present
- regression tests cover alias normalization and rendered wrapper output

### Non-Goals

- generating rich Go value types for arbitrary external C structs
- supporting by-value `struct timeval` lifting into Go-native structs
- broad external-struct modeling beyond the immediate timeval stability slice

### Open Questions

- whether future external-struct support should generalize beyond the current known-safe timeval case

### Suggested Validation

- IR tests for `timeval*`, `struct timeval*`, and `struct timeval&`
- rendered header/go output checks for `<sys/time.h>` and `*C.struct_timeval`
- generated C++ source assertion for reference-call dereference behavior

### Parallelization Hints

- Candidate write boundaries:
  - `src/ir.rs`
  - `src/generator.rs`
  - `src/facade.rs`
  - `tests/timeval_support.rs`
- Shared files to avoid touching in parallel:
  - `src/ir.rs`
  - `src/facade.rs`
- Likely sequential dependencies:
  - IR classification before generator/facade rendering updates, then regression tests
