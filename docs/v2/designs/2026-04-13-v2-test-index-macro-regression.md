---
feature: test-index-macro-regression
status: plan_ready
created_at: 2026-04-13T11:26:31+09:00
---

# Test Index Macro Regression

## Goal

Add a focused regression test that reproduces the newly supported `#define TEST_INDEX 10` path and locks in the expected parsed macro and generated Go constant behavior.

## Context / Inputs

- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - `tests/generator.rs` already covers standalone integer-like macros and verifies that object-like defines become parsed macros and Go constants while function-like macros stay filtered out
  - there is no current assertion that explicitly captures the `TEST_INDEX` name/value pair the user asked to preserve
- User brief:
  - `우리 #define TEST_INDEX 10 을 개발했는데 이거 재현하는 테스트 코드좀 짜줄래?`

## Plan Handoff

### Scope for Planning

- extend the existing generator regression test coverage instead of introducing a new fixture file or new production behavior
- add a concrete `#define TEST_INDEX 10` sample to the macro fixture input
- assert that parsing keeps the exact macro name/value pair and that normalized IR and generated Go output preserve it as an exported constant

### Success Criteria

- a targeted regression test contains `#define TEST_INDEX 10`
- parsed macro assertions prove `TEST_INDEX` is captured as value `10`
- generated Go output assertions prove `TEST_INDEX = 10` is emitted
- targeted Rust tests for the touched file pass

### Non-Goals

- changing macro parsing rules beyond what is required for this regression
- adding new fixture directories or broad documentation restructuring
- changing unrelated generator assertions

### Open Questions

- none

### Suggested Validation

- `cargo test generator`
- optionally `cargo test`

### Parallelization Hints

- Candidate write boundaries:
  - `tests/generator.rs`
- Shared files to avoid touching in parallel:
  - `tests/generator.rs`
- Likely sequential dependencies:
  - update the existing macro regression first, then run the targeted cargo test
