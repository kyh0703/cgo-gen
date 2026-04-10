---
feature: enum-typedef-facade-value-normalization
status: plan_ready
created_at: 2026-04-10T11:20:00+09:00
---

# Enum Typedef Facade Value Normalization

## Goal

Keep `typedef enum` aliases as by-value enums throughout facade generation so class methods like `CSmManager::GetProcStartupState(IPRON_DI_PROC_TYPE eProc)` render Go and C wrappers with enum values instead of `Handle*` model wrappers.

## Context / Inputs
- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - `D:/Project/IPRON/DI/PUBLIC/Public.h` declares `typedef enum _IPRON_DI_PROC_TYPE { ... } IPRON_DI_PROC_TYPE;`.
  - `D:/Project/IPRON/DI/PUBLIC/SmManager.h` uses `IPRON_DI_PROC_TYPE` by value in `GetProcStartupState`, `SetProcStartupState`, and `ClearProcStartupSts`.
  - Generated public output already models the enum as a value type in `smmanager/public_wrapper.go`:
    - `type _IPRON_DI_PROC_TYPE int64`
    - enum constants `DI_PROC_IDCD` through `DI_PROC_REST`
  - Generated facade output is inconsistent for the same type:
    - `smmanager/sm_manager_wrapper.h` declares `typedef struct IPRON_DI_PROC_TYPEHandle IPRON_DI_PROC_TYPEHandle;`
    - `smmanager/sm_manager_wrapper.cpp` casts `IPRON_DI_PROC_TYPEHandle*` back to `_IPRON_DI_PROC_TYPE*`
    - `smmanager/sm_manager_wrapper.go` emits `type IPRONDIPROCTYPE struct { ptr *C.IPRON_DI_PROC_TYPEHandle }`
  - The current result forces Go callers to fabricate a pointer-backed wrapper for an enum value parameter, which is unusable for the intended API shape.
- User brief:
  - actual Linux-built `./smmanager` output shows `typedef enum` values are generated correctly in the public wrapper but incorrectly wrapped as pointer handles in facade methods
  - build input headers live under `../IPRON/DI/PUBLIC`

## Plan Handoff
### Scope for Planning
- Trace how typedef-backed enums are classified between IR normalization, model analysis, and facade rendering.
- Make facade generation reuse enum value typing for typedef aliases instead of treating them as model handles.
- Add a regression test that exercises a class method taking a typedef enum by value and asserts no `Handle*` wrapper is emitted in C or Go facade output.
- Re-run the generator against `smmanager` or an equivalent focused fixture to confirm `GetProcStartupState` and related methods accept enum values directly.

### Success Criteria
- A typedef enum used by value in class methods is emitted as a value enum in generated facade output.
- `sm_manager_wrapper.h/.cpp` no longer declare or use `IPRON_DI_PROC_TYPEHandle*` for the affected methods.
- `sm_manager_wrapper.go` no longer emits `IPRONDIPROCTYPE struct { ptr *C...Handle }` for the enum parameter path.
- Regression tests fail before the fix and pass after it.

### Non-Goals
- Do not redesign general object-handle ownership rules.
- Do not manually patch generated `smmanager` artifacts as the source of truth.
- Do not broaden the change to unrelated enum container or callback cases unless the failing path requires it.

### Open Questions
- none

### Suggested Validation
- `cargo test enum_typedef`
- `cargo test generator`
- `cargo test`
- re-generate `smmanager` and inspect `GetProcStartupState`, `SetProcStartupState`, and `ClearProcStartupSts`

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
