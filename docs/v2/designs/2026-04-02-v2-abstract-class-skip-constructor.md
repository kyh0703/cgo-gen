---
feature: abstract-class-skip-constructor
status: plan_ready
created_at: 2026-04-02T17:10:00+09:00
---

# Abstract Class Constructor Skip

## Goal

Prevent constructor wrapper generation for abstract C++ classes so the generated native wrapper layer no longer emits invalid `_new()` functions for types that cannot be instantiated.

## Context / Inputs

- Source docs:
  - user-reported compile failure from generated wrapper output
- Existing system facts:
  - the current parser records public methods and constructors but does not mark whether a class is abstract
  - IR normalization currently emits a constructor wrapper whenever a class is treated as constructible
  - abstract classes with pure virtual methods cannot be instantiated with `new`
  - the failure mode is a native compile error similar to `invalid new-expression of abstract class type`
- User brief:
  - skip constructor generation for abstract classes instead of generating invalid code

## Plan Handoff

### Scope for Planning

- extend parsed class metadata so abstractness is detected during libclang traversal
- detect pure virtual methods and mark the owning class as abstract
- skip constructor wrapper generation for abstract classes during IR normalization
- keep destructor and callable concrete methods available when they are otherwise valid
- record the skipped constructor path in `support.skipped_declarations`
- add regression coverage proving abstract classes omit `_new()` while concrete classes still generate it

### Success Criteria

- abstract classes no longer emit `_new()` wrapper functions
- destructor and non-constructor wrapper generation for abstract classes remains intact
- concrete classes still emit constructor wrappers as before
- skipped metadata records the abstract-class constructor omission explicitly

### Non-Goals

- changing how abstract classes are represented in Go beyond constructor omission
- adding support for constructing abstract classes through factories or subclasses
- broader class-model redesign unrelated to abstractness detection

### Open Questions

- whether future docs should distinguish interface-like abstract classes from partially abstract classes; not required for this fix

### Suggested Validation

- targeted regression test with one abstract class and one concrete class
- inspect generated IR/wrapper symbols to confirm `_new()` is absent only for the abstract class

### Parallelization Hints

- Candidate write boundaries:
  - `src/parser.rs`
  - `src/ir.rs`
  - `tests/abstract_class_skip.rs`
- Shared files to avoid touching in parallel:
  - `src/ir.rs`
- Likely sequential dependencies:
  - abstractness detection in parsing before constructor omission in normalization
