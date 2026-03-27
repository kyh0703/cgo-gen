# SIL Conversion Status

Updated: 2026-03-27

## Reset state

- 상태: archived snapshot
- 활성 SIL 작업: none
- 남아 있던 문서 작업은 모두 종료 처리되었습니다.

## Summary

Current `c-go` work has been simplified toward **file-based generation**:

- `filter` config support was removed
- generation now relies on:
  - `input.headers`
  - `files.model`
  - `files.facade`
  - `output`
  - `naming`
  - `policies`

This keeps SIL configs smaller and matches the current intended workflow.

## Test / Config Work Completed

- Added SIL config regression coverage in `tests/config.rs`
  - loads `configs/sil-wrapper.example.yaml`
  - validates `files.model` / `files.facade`
  - validates per-header output naming
- Stabilized `tests/isaamaster_fixture.rs`
  - normalizes generated IR header paths before fixture comparison
- Added callback/function-pointer skip coverage
  - `tests/function_pointer_skip.rs`
- Added typedef alias auto-resolution coverage
  - `tests/typedef_alias_resolution.rs`
- Added explicit facade routing regression coverage
  - known model out-param positive lift cases
  - negative routing cases where unknown or misplaced model types must not lift

## Real SIL Parsing Findings

### 1. `IsAAMaster.h`

Real-world parse/generate works with the correct include paths.

Observed result:

- `1 headers`
- `1 classes`
- `72 abi functions`

Generated outputs under temporary verification workspace:

- `is_aa_master_wrapper.h`
- `is_aa_master_wrapper.cpp`
- `is_aa_master_wrapper.ir.yaml`
- `is_aa_master_wrapper.go`

### 2. `iSiLib.h`

Initial blocker was an include cycle:

- `iSiLib.h` includes `iSiLib-inl.h`
- `iSiLib-inl.h` included `iSiLib.h` again

After removing the reverse include from `iSiLib-inl.h`, parsing moved forward.

Next blockers discovered during real parsing:

1. function pointer callback declarations such as `SICHACALLBACK`
2. typedef aliases such as `iMoId_t`
3. unsupported referenced model types such as `IsSipHeaderRelay&`

## Implemented Parser / IR Behavior

### Function pointer declarations

Declarations using function pointer types are now **skipped** instead of aborting
the whole normalize step.

Skipped declarations are recorded in:

- `ir.support.skipped_declarations`

This keeps partial SIL facade progress visible while leaving a clean expansion
point for future callback support.

### Typedef alias auto-resolution

IR normalization now attempts to resolve unsupported display types through their
libclang canonical types.

Examples:

- `iMoId_t` -> canonical primitive alias path
- `ieResult_t` -> canonical primitive alias path

This means users do **not** need to redundantly define typedef aliases in YAML.

### Facade routing cleanup

Facade generation now classifies class methods before rendering:

- known supported model out-param in the final supported position
  - route to model-mapped Go API generation
- otherwise supported primitive/string method
  - keep on the general facade API path

This keeps `files.model` as the only semantic source of truth for model-aware lifting and avoids name-based collection inference.

### Raw-first unknown model handling

Unknown non-classified model reference/pointer declarations are no longer treated as an automatic declaration-level failure when the raw layer can still represent them safely.

- raw header/source generation keeps them as opaque-handle-based wrapper APIs
- Go facade/model generation still excludes them unless they map to `files.model`

This keeps C/raw coverage broader without weakening the `files.model` contract for Go-facing output.

### Raw-unsafe by-value object handling

Declarations that use unsupported by-value object types are now skipped at declaration level instead of aborting normalization for the whole header.

- skipped declarations are recorded in `ir.support.skipped_declarations`
- supported methods in the same header still generate normally
- raw and Go output both exclude the raw-unsafe by-value declaration

This keeps internal/native-only object types from leaking into the Go surface while preserving as much verified output as possible from the same header.

## Real `iSiLib` Verification Status

The current macOS local IPRON environment now verifies the real `iSiLib` flow end to end:

1. `check` succeeds with the local include roots
2. `ir` succeeds and emits full normalized IR
3. `generate --dump-ir` succeeds and writes raw/Go artifacts

What changed to make this possible:

- deterministic overload-safe raw wrapper naming now disambiguates repeated symbols such as `GetAAMaster`, `Init`, `GetNodeTenant`
- Go facade generation now filters non-renderable primitive typedef aliases conservatively instead of panicking
- renderable overloads now get deterministic Go export suffixes such as:
  - `GetAAMasterUint32(...)`
  - `GetAAMasterString(...)`

Observed real-SIL evidence:

- `support.skipped_declarations` count is 8 in the current local `iSiLib` IR dump
- the skipped set is limited to:
  - function-pointer declarations such as `SetHACallback`
  - raw-unsafe by-value object declarations such as `SetUserMaster`, `SetDnTrsf`, `RestoreSubsData`
- real SIL types that carry `NsMap*` / `DsMap*` internals, such as `IsCluster` and `IsCSTASession`, can remain raw-visible without leaking those internal collection details into the generated Go facade

## Reviewed onboarding decision (2026-03-25)

Current durable evidence is still not strong enough to approve any additional checked-in `files.model` header beyond `IsAAMaster.h`.

Reviewed classification:

- keep `IsAAMaster.h` as the verified checked-in `files.model` path
- keep `iSiLib.h` as the real facade verification surface, not a shared model header
- keep `IsCluster.h` raw-only for now
- keep `IsCSTASession.h` raw-only for now

Reason:

- `IsCluster` and `IsCSTASession` are proven raw-visible in the real-SIL flow
- the current evidence does not yet prove that either header is part of the intended shared Go model contract
- both types carry transitive `NsMap*` / `DsMap*` style internals, which is exactly the boundary-widening risk this project is trying to avoid

## Recommended Next Step

For practical progress, prefer:

1. keep `IsAAMaster` as the verified real model path
2. treat raw-only internal types as non-onboarded by default unless a narrower public-model case is proven
3. add a new `files.model` header only after a header-specific review of real `iSiLib` IR/output
4. avoid widening the Go boundary just because a SIL class transitively contains `NsMap*` or other internal storage helpers
