# Operator Skip Generation

## Goal
- C++ operator 선언을 지원하지 않고 생성 대상에서 제외하도록 구현하고 검증한다.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-02-v2-operator-skip-generation.md`

## Workspace
- Branch: feat/v2-operator-skip-generation
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: operator 선언을 식별하고 skip metadata에 기록하면서 generation 대상에서 제외한다.
- Depends on:
  - none
- Write Scope:
  - `src/ir.rs`
- Read Context:
  - `docs/v2/designs/2026-04-02-v2-operator-skip-generation.md`
  - 기존 skipped declaration 처리
- Checks:
  - `cargo test function_pointer_skip`
  - 관련 단위 테스트 직접 실행
- Parallel-safe: no

### Task T2
- Goal: operator fixture 회귀 테스트를 추가하고 전체 테스트를 통과시킨다.
- Depends on:
  - T1
- Write Scope:
  - `tests/`
- Read Context:
  - `src/ir.rs`
  - 기존 generator/facade 테스트 패턴
- Checks:
  - `cargo test`
- Parallel-safe: no

## Notes
- 구현은 “skip”만 다루고 대체 이름 생성은 하지 않는다.
