# FixedModelArray Field Accessor C ABI Fix

## Goal
- Fix raw C ABI generation so structure field accessors for model arrays `T[N]` render valid C++ source instead of invalid array-type constructions like `new T[N](...)` and `reinterpret_cast<T[N]*>(...)`.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-03-v2-structure-field-accessors.md`
- `src/codegen/c_abi.rs`
- `src/codegen/ir_norm.rs`
- `tests/generator.rs`
- `tests/compile_smoke.rs`

## Workspace
- Branch: feat/v2-fixed-model-array-field-accessors
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: Confirm the `FixedModelArray` rendering path and patch raw C ABI generation so it uses the array element cpp type, not the full array cpp type, when constructing and casting model values.
- Depends on:
  - none
- Write Scope:
  - `src/codegen/c_abi.rs`
- Read Context:
  - `src/codegen/c_abi.rs`
  - `src/codegen/ir_norm.rs`
- Checks:
  - `cargo test generator`
- Parallel-safe: no

### Task T2
- Goal: Add a generator regression test for a structure field like `Item items[3];` and assert that generated source uses `new Item(...)` and `reinterpret_cast<Item*>(...)`, never array-type casts or array new syntax.
- Depends on:
  - T1
- Write Scope:
  - `tests/generator.rs`
  - `tests/fixtures/`
- Read Context:
  - existing generator fixture tests
  - `src/codegen/c_abi.rs`
- Checks:
  - `cargo test generator`
- Parallel-safe: no

### Task T3
- Goal: Add a compile smoke regression using an internal temporary header with a fixed model array field and verify the generated wrapper compiles and links through getter/setter usage.
- Depends on:
  - T2
- Write Scope:
  - `tests/compile_smoke.rs`
- Read Context:
  - existing compile smoke tests
  - generated field accessor conventions
- Checks:
  - `cargo test compile_smoke`
  - `cargo test`
- Parallel-safe: no

## Notes
- Keep the public C ABI signatures unchanged. Only the generated `.cpp` implementation should change.
- Do not patch `smmanager/public_wrapper.cpp` directly. The fix must live in the generator and be validated through repository-local regression tests.
- Do not broaden ownership or memory-management semantics for model array accessors in this plan.
