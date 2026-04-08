# Add Diverse Testcases

## Goal
- expand regression coverage for the recently generalized model fixture and config flow with more diverse accessor, naming, and compile-facing cases while keeping production behavior unchanged unless a real mismatch is exposed

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-08-v2-add-diverse-testcases.md`
- `tests/model_record_fixture.rs`
- `tests/gen_model_config.rs`

## Workspace
- Branch: feat/v2-add-diverse-testcases
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: strengthen `model_record` fixture regression coverage for multiple numeric/string slots and underscore-sensitive generated names using the existing checked-in fixture inputs
- Depends on:
  - none
- Write Scope:
  - `tests/model_record_fixture.rs`
- Read Context:
  - `docs/v2/designs/2026-04-08-v2-add-diverse-testcases.md`
  - `tests/model_record_fixture.rs`
  - `tests/fixtures/model_record/include/DataRecord.h`
- Checks:
  - `cargo test model_record_fixture`
- Parallel-safe: no

### Task T2
- Goal: expand dir-only config regression checks for the generic model fixture and run broader verification, adjusting one compile-facing regression file only if the added coverage clearly belongs there
- Depends on:
  - T1
- Write Scope:
  - `tests/gen_model_config.rs`
  - optionally `tests/compile_smoke.rs`
- Read Context:
  - `tests/gen_model_config.rs`
  - `configs/gen-model-config.yaml`
  - `tests/compile_smoke.rs`
- Checks:
  - `cargo test gen_model_config`
  - `cargo test compile_smoke`
  - `cargo test`
- Parallel-safe: no

## Notes
- prefer extending existing tests over creating new bulky fixtures
- keep production code unchanged unless a new assertion reveals a concrete bug that needs the smallest safe fix
