---
feature: enum-typedef-facade-value-normalization
status: plan_ready
created_at: 2026-04-10T11:20:00+09:00
---

# Enum Typedef Facade Value Normalization

## Goal

Keep `typedef enum` aliases as by-value enums throughout facade generation so class methods like `SessionManager::GetStartupState(PROCESS_KIND kind)` render Go and C wrappers with enum values instead of `Handle*` model wrappers.

## Context / Inputs
- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - `/workspace/vendor/public_api/Public.h` declares `typedef enum _PROCESS_KIND { ... } PROCESS_KIND;`.
  - `/workspace/vendor/public_api/Manager.h` uses `PROCESS_KIND` by value in `GetStartupState`, `SetStartupState`, and `ClearStartupState`.
  - Generated public output already models the enum as a value type in `sample_manager/public_wrapper.go`:
    - `type _PROCESS_KIND int64`
    - enum constants `PROCESS_KIND_PRIMARY` through `PROCESS_KIND_IDLE`
  - Generated facade output is inconsistent for the same type:
    - `sample_manager/manager_wrapper.h` declares `typedef struct PROCESS_KINDHandle PROCESS_KINDHandle;`
    - `sample_manager/manager_wrapper.cpp` casts `PROCESS_KINDHandle*` back to `_PROCESS_KIND*`
    - `sample_manager/manager_wrapper.go` emits `type ProcessKind struct { ptr *C.PROCESS_KINDHandle }`
  - The current result forces Go callers to fabricate a pointer-backed wrapper for an enum value parameter, which is unusable for the intended API shape.
- User brief:
  - representative generated `./sample_manager` output shows `typedef enum` values are generated correctly in the public wrapper but incorrectly wrapped as pointer handles in facade methods
  - build input headers live under `../vendor/public_api`

## Plan Handoff
### Scope for Planning
- Trace how typedef-backed enums are classified between IR normalization, model analysis, and facade rendering.
- Make facade generation reuse enum value typing for typedef aliases instead of treating them as model handles.
- Add a regression test that exercises a class method taking a typedef enum by value and asserts no `Handle*` wrapper is emitted in C or Go facade output.
- Re-run the generator against `sample_manager` or an equivalent focused fixture to confirm `GetStartupState` and related methods accept enum values directly.

### Success Criteria
- A typedef enum used by value in class methods is emitted as a value enum in generated facade output.
- `manager_wrapper.h/.cpp` no longer declare or use `PROCESS_KINDHandle*` for the affected methods.
- `manager_wrapper.go` no longer emits `ProcessKind struct { ptr *C...Handle }` for the enum parameter path.
- Regression tests fail before the fix and pass after it.

### Non-Goals
- Do not redesign general object-handle ownership rules.
- Do not manually patch generated `sample_manager` artifacts as the source of truth.
- Do not broaden the change to unrelated enum container or callback cases unless the failing path requires it.

### Open Questions
- none

### Suggested Validation
- `cargo test enum_typedef`
- `cargo test generator`
- `cargo test`
- re-generate `sample_manager` and inspect `GetStartupState`, `SetStartupState`, and `ClearStartupState`

### Parallelization Hints
- Candidate write boundaries:
  - `src/codegen/`
  - `src/analysis/`
  - `tests/`
- Shared files to avoid touching in parallel:
  - `src/codegen/go_facade.rs`
  - `src/codegen/c_abi.rs`
  - any shared fixture or generator snapshot file updated for the regression
- Likely sequential dependencies:
  - identify where typedef enum aliases are downgraded to handle-backed models
  - fix classification and rendering
  - then add/update regression coverage
