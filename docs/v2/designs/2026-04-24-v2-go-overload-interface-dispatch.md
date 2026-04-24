---
feature: go-overload-interface-dispatch
status: plan_ready
created_at: 2026-04-24T10:04:30+09:00
---

# Go Overload Interface Dispatch

## Goal

Evaluate whether generated Go facade APIs for overloaded C++ functions and methods should move from explicit signature-suffixed exports such as `SetFlagBool` / `SetFlagInt32` to a SWIG-like dispatcher that exposes one Go name and branches by `args`.

## Context / Inputs
- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
  - `docs/v2/completed/go-facade-overloaded-constructors.md`
  - `docs/v2/completed/go-false-overload-suffix-detection-for-underscore-bool-field-setters.md`
- Existing system facts:
  - Raw C ABI symbols are already overload-safe through deterministic signature suffixes in `src/codegen/ir_norm.rs`.
  - Go facade free functions and methods currently append Go-facing overload tokens only when the raw symbol has a real overload suffix.
  - Constructor overloads already use explicit generated names such as `NewWidgetWithNItemMax` and `NewWidgetFromCopy`.
  - Go has no native function or method overloading; a single exported name requires either `...interface{}` / `...any`, generic constraints with fixed arity helpers, or a generated wrapper type.
- User brief:
  - Current overloaded methods are split by suffixes such as `Int32Bool`.
  - Review whether to wrap them behind an interface-based dispatcher that branches by `args`, similar to SWIG.

## Plan Handoff
### Scope for Planning
- Compare the current explicit suffix API against a SWIG-like `args ...interface{}` dispatcher for overloaded Go facade functions and methods.
- Decide the recommended public API direction:
  - full replacement of suffixed exports,
  - additive dispatcher while preserving typed suffixed exports,
  - or no dispatcher.
- If implementation is recommended, keep the first slice limited to generated Go facade output for overloaded free functions and class methods.
- Preserve raw C ABI symbol suffixing; this feature only concerns the Go-facing facade API.

### Success Criteria
- The plan identifies overload cases that are safe, ambiguous, or unsuitable for `interface{}` dispatch.
- The recommendation accounts for Go compile-time type safety, numeric alias handling, nil pointer ambiguity, model pointer/reference semantics, callback arguments, return-shape differences, and backward compatibility.
- If code changes are included, generated Go output keeps deterministic direct typed functions available as the stable backing API.
- Regression coverage includes at least one overloaded free function and one overloaded method.

### Non-Goals
- Do not redesign raw C ABI symbol naming.
- Do not remove existing suffixed Go exports in the first implementation slice.
- Do not implement support for overloads that are ambiguous after Go type projection unless the generated code returns a clear runtime error.
- Do not change constructor overload naming in this slice.

### Open Questions
- Should the dispatcher be opt-in by config, always emitted for overload groups, or only emitted when every overload is unambiguous after Go type projection?
- Should ambiguous overload groups fail generation, skip dispatcher generation, or generate a dispatcher that returns an ambiguity error?
- Should dispatcher functions panic on bad argument combinations or return an `error` alongside the native return value?

### Suggested Validation
- `cargo test --test overload_collisions`
- `cargo test go_facade`
- Add generated-output assertions for:
  - overloaded free function dispatcher routing by arity and Go type,
  - overloaded method dispatcher routing by arity and Go type,
  - ambiguous projected signatures that keep only explicit suffixed exports.
- If a dispatcher returns errors, add compile-oriented checks for the generated Go signatures.

### Parallelization Hints
- Candidate write boundaries:
  - `src/codegen/go_facade.rs` for overload grouping, dispatcher naming, and rendering.
  - `tests/overload_collisions.rs` for focused overload facade assertions.
  - optionally a small research note under `docs/v2/research/` if the first task is decision-only.
- Shared files to avoid touching in parallel:
  - `src/codegen/go_facade.rs`
  - `docs/v2/plans/go-overload-interface-dispatch.md`
- Likely sequential dependencies:
  - classify supported and ambiguous overload groups first,
  - decide dispatcher error/signature shape second,
  - only then render and test generated output.
