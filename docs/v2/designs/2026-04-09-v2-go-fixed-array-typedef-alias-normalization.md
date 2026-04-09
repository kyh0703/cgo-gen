---
feature: go-fixed-array-typedef-alias-normalization
status: plan_ready
created_at: 2026-04-09T16:10:00+09:00
---

# Go Fixed Array Typedef Alias Normalization

## Goal

Fix generated Go wrappers so fixed arrays declared through primitive typedef aliases preserve the correct unsigned element type, eliminating the `smmanager/public_wrapper.go` build errors around `tRsnCode[64]` and `tSubscribeId[...]`.

## Context / Inputs
- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - `D:/Project/IPRON/DI/PUBLIC/DiMonitorDef.h` declares `STATTNTM_INFO.NrdRsnCodeSet` and `STATTNTM_INFO.AcwRsnCodeSet` as `tRsnCode[64]`.
  - `D:/Project/IPRON/DI/PUBLIC/Public.h` declares `SUBSCRIBE_CODE.SubScrIds` as `tSubscribeId[...]`.
  - `tRsnCode` and `tSubscribeId` both resolve to `uint32`.
  - Generated C wrappers already preserve the unsigned ABI:
    - `smmanager/public_wrapper.h` uses `unsigned int*` for `cgowrap__STATTNTM_INFO_SetNrdRsnCodeSet`, `cgowrap__STATTNTM_INFO_SetAcwRsnCodeSet`, and `cgowrap__SUBSCRIBE_CODE_SetSubScrIds`.
  - Generated Go facade is inconsistent:
    - `smmanager/public_wrapper.go` currently emits `[]int32` and `*C.int32_t` for the same fields.
  - `src/codegen/go_facade.rs` determines `FixedArray` element types from `ty.cpp_type` and falls back to `int32` / `C.int32_t` when the array element is an unresolved alias name.
- User brief:
  - The actual built `smmanager` output fails with:
    - `cannot use cArg0 (variable of type *_Ctype_int32_t) as *_Ctype_uint`
    - at `public_wrapper.go:37744`, `37769`, `38090`

## Plan Handoff
### Scope for Planning
- Keep the fix scoped to Go facade `FixedArray` type inference.
- Resolve fixed-array primitive aliases by checking canonical C pointee type when the C++ display type still contains typedef names.
- Add regression tests covering primitive typedef arrays that should render as `[]uint32` and `*C.uint32_t`.
- Re-run generation or equivalent local verification for `smmanager` and confirm the three failing sites no longer cast through `int32`.

### Success Criteria
- Generated Go setter/getter signatures for `tRsnCode[64]` and `tSubscribeId[...]` become `[]uint32`.
- Generated cgo casts for those fixed arrays become `C.uint32_t`.
- Existing primitive fixed-array generation for already-canonical element types keeps working.
- Regression tests fail before the fix and pass after it.

### Non-Goals
- Do not redesign IR normalization for all typedef aliases.
- Do not change raw C ABI generation that is already emitting `unsigned int*`.
- Do not manually patch `smmanager/public_wrapper.go`; the fix must come from generator output.

### Open Questions
- none

### Suggested Validation
- `cargo test typedef_alias_resolution`
- `cargo test generator`
- `cargo test`
- Re-generate `smmanager` locally and inspect the three previously failing call sites for `uint32`/`C.uint32_t`

### Parallelization Hints
- Candidate write boundaries:
  - `src/codegen/go_facade.rs`
  - `tests/`
  - generated `smmanager/` output
- Shared files to avoid touching in parallel:
  - `src/codegen/go_facade.rs`
  - any shared generator test file updated for the new regression
- Likely sequential dependencies:
  - implement the `FixedArray` type inference fix first
  - then add/update regression tests
  - then re-generate `smmanager` and validate the output
