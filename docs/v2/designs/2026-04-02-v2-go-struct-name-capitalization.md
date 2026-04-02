---
feature: go-struct-name-capitalization
status: plan_ready
created_at: 2026-04-02T15:45:00+09:00
---

# Go Struct Name Capitalization

## Goal

Ensure generated Go struct and model projection names remain exported even when the original C++ class name starts with a lowercase letter.

## Context / Inputs

- Source docs:
  - user-reported facade/model generation issue for lowercase-leading C++ class names
- Existing system facts:
  - `collect_facade_classes()` and `build_model_projection()` currently use `leaf_cpp_name(...)` directly for Go type naming
  - direct reuse of the original C++ casing produces unexported Go identifiers when the first character is lowercase
  - unexported generated structs make the public wrapper surface inconsistent and difficult to consume
- User brief:
  - generated Go struct names should be capitalized and exported

## Plan Handoff

### Scope for Planning

- route Go-facing class/model type names through `go_export_name(...)` instead of preserving raw C++ leading case
- apply the same export normalization consistently in both facade class analysis and known-model projection building
- add regression coverage for the name helper itself and for rendered Go output from a lowercase-leading C++ class

### Success Criteria

- lowercase-leading C++ class names generate exported Go struct names
- constructor-style helper names derived from those classes are also exported correctly
- existing already-exported C++ class names remain stable
- regression tests cover helper behavior and rendered Go output shape

### Non-Goals

- renaming underlying C wrapper symbols
- changing package names or overload token rules
- introducing style transforms beyond the first-character export requirement

### Open Questions

- whether later naming work should preserve special acronym casing more aggressively; not required for this fix

### Suggested Validation

- helper-level unit test for `go_export_name(...)`
- rendered facade test proving `type MyApi struct` and `func NewMyApi()` style output for lowercase C++ class names

### Parallelization Hints

- Candidate write boundaries:
  - `src/facade.rs`
- Shared files to avoid touching in parallel:
  - `src/facade.rs`
- Likely sequential dependencies:
  - helper update before rendered output test assertions
