# Go Fixed Array Typedef Alias Normalization

## Goal
- Fix Go facade fixed-array element typing so primitive typedef arrays use the canonical unsigned C type instead of falling back to `int32`, and verify the change against the failing `sample_manager` output.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-09-v2-go-fixed-array-typedef-alias-normalization.md`

## Workspace
- Branch: feat/v2-go-fixed-array-typedef-alias-normalization
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: update Go facade fixed-array type resolution to derive primitive alias arrays from canonical C pointee types when the display `cpp_type` still contains unresolved typedef element names.
- Depends on:
  - none
- Write Scope:
  - `src/codegen/go_facade.rs`
- Read Context:
  - `docs/v2/designs/2026-04-09-v2-go-fixed-array-typedef-alias-normalization.md`
  - `src/codegen/ir_norm.rs`
  - current `sample_manager/public_wrapper.go` failure sites
- Checks:
  - `cargo test typedef_alias_resolution`
  - `cargo test generator`
- Parallel-safe: no

### Task T2
- Goal: add regression coverage for primitive typedef fixed arrays and validate the fix by regenerating or re-checking `sample_manager` output so the three known `*_Ctype_uint` mismatch sites switch to `uint32`/`C.uint32_t`.
- Depends on:
  - T1
- Write Scope:
  - `tests/typedef_alias_resolution.rs`
  - `tests/generator.rs`
  - `sample_manager/`
- Read Context:
  - `src/codegen/go_facade.rs`
  - `sample_manager/public_wrapper.h`
  - `sample_manager/public_wrapper.cpp`
  - `/workspace/vendor/public_api/MonitorDefs.h`
  - `/workspace/vendor/public_api/Public.h`
- Checks:
  - `cargo test`
  - `rg -n "C\\.uint32_t|\\[\\]uint32|SetPrimaryReasonCodes|SetSecondaryReasonCodes|SetSubscriptionIds" sample_manager/public_wrapper.go`
- Parallel-safe: no

## Notes
- Prefer the smallest Go-layer change that keeps existing raw C ABI output untouched.
- If the exact `sample_manager` generation config is not stored in the repo, derive the local regeneration command from available output/context and report the limitation if full regeneration cannot be reproduced.
