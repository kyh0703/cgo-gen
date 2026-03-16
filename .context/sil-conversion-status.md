# SIL Conversion Status

Updated: 2026-03-16

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

## Current Remaining Blocker for Full `iSiLib` Generation

`iSiLib.h` still references many SIL model types beyond `IsAAMaster`, for example:

- `IsSipHeaderRelay&`
- `IsSipHeaderGroup&`
- `IsCluster&`
- and many more

These currently fail unless one of the following is implemented:

1. more SIL model headers are added to `files.model`
2. unknown model-reference facade methods are skipped in v1
3. broader SIL model/header onboarding is added

## Recommended Next Step

For practical progress, prefer:

1. keep `IsAAMaster` as the verified real model path
2. make facade generation skip methods that reference unknown non-primitive,
   non-classified SIL model types
3. then inspect the partially generated `iSiLib` surface and decide which model
   headers to onboard next
