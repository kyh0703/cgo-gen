---
feature: add-diverse-testcases
status: plan_ready
created_at: 2026-04-08T13:56:51+09:00
---

# Add Diverse Testcases

## Goal

Expand regression coverage around the current generic model fixture flow so recent test-fixture refactors and future generator changes fail fast across more naming, field-shape, and config cases.

## Context / Inputs

- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - the latest checked-in test refactor replaced the business-specific `isaamaster` fixture with the generic `model_record` fixture and renamed related config coverage to `gen_model_config`
  - current `model_record` coverage proves basic parsing/generation and one compile-smoke path, but only asserts a subset of generated accessors and wrapper names
  - current config coverage proves the dir-only input shape and one generated Go wrapper path, but does not exercise more than the minimal scoped-header success case
- User brief:
  - `다양한 케이스로 testcase도 좀 추가해줄래?`

## Plan Handoff

### Scope for Planning

- inspect the current checked-in regression tests around `model_record`, generated Go wrappers, and dir-only model config flow
- add diverse assertions for multiple numeric/string fields instead of only one or two representative accessors
- lock in the naming contract for underscore-heavy members so raw C names and normalized Go method names are both covered
- expand config-oriented regression checks using the existing generic fixture/config inputs rather than introducing large new fixtures
- keep the change test-focused unless new assertions expose a real bug that requires the smallest safe production fix

### Success Criteria

- tests cover more than the current minimal `Id`/`Name` happy path for the `model_record` fixture
- generated wrapper assertions cover underscore-heavy members such as `Slot1_Val` and adjacent slot variants
- config-oriented tests cover the checked-in generic model config flow with stronger expectations than simple file existence
- targeted cargo tests for the touched test files pass, and full `cargo test` passes if the scope remains small

### Non-Goals

- adding new generator features or widening supported language surface
- replacing the current generic fixture with another new fixture set
- rewriting unrelated tests or refreshing snapshots that are not directly needed for the added coverage

### Open Questions

- whether the new coverage remains purely test-only or exposes a small generator mismatch that must be fixed to make the tests meaningful
- which file should own each added case most cleanly between `tests/model_record_fixture.rs`, `tests/gen_model_config.rs`, and existing compile/generation regression files

### Suggested Validation

- `cargo test model_record_fixture`
- `cargo test gen_model_config`
- `cargo test compile_smoke`
- `cargo test`

### Parallelization Hints

- Candidate write boundaries:
  - `tests/model_record_fixture.rs` for richer fixture-based wrapper and runtime assertions
  - `tests/gen_model_config.rs` for dir-only config regression coverage
  - optionally one existing generation/compile regression file if an additional cross-check belongs there
- Shared files to avoid touching in parallel:
  - `tests/model_record_fixture.rs`
  - `tests/gen_model_config.rs`
  - any shared fixture/config file if it must be edited
- Likely sequential dependencies:
  - confirm the highest-signal missing cases first, then place each case into the smallest existing test file before running targeted cargo tests
