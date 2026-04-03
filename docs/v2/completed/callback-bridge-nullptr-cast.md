# Callback Bridge Nullptr Cast

## Goal
- callback bridge lambda가 `nullptr`와 함께 삼항 연산자에 들어가도 generated C++가 typedef 함수 포인터로 안정적으로 컴파일되게 만든다.

## References
- docs/STATE.md
- docs/ROADMAP.md
- docs/ARCHITECTURE.md
- docs/v2/designs/2026-04-03-v2-callback-bridge-nullptr-cast.md
- docs/v2/designs/callback-facade-support.md

## Workspace
- Branch: feat/v2-callback-bridge-nullptr-cast
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: callback bridge 렌더링에서 lambda 지역 변수와 `nullptr` 삼항 타입 오류가 생기는 지점을 확인하고 최소 수정 형태를 확정한다.
- Depends on:
  - none
- Write Scope:
  - src/generator.rs
- Read Context:
  - docs/v2/designs/2026-04-03-v2-callback-bridge-nullptr-cast.md
  - docs/v2/designs/callback-facade-support.md
  - src/generator.rs
  - tests/facade_generate.rs
- Checks:
  - cargo test
- Parallel-safe: no

### Task T2
- Goal: callback bridge trampoline 선언을 callback typedef 기준으로 렌더링하도록 수정해 generated C++ 타입 오류를 막는다.
- Depends on:
  - T1
- Write Scope:
  - src/generator.rs
- Read Context:
  - src/generator.rs
  - tests/facade_generate.rs
- Checks:
  - cargo test
- Parallel-safe: no

### Task T3
- Goal: callback bridge 회귀 테스트를 갱신 또는 보강해 typedef 함수 포인터 형태가 유지되는지 검증한다.
- Depends on:
  - T2
- Write Scope:
  - tests/facade_generate.rs
- Read Context:
  - src/generator.rs
  - tests/facade_generate.rs
- Checks:
  - cargo test
- Parallel-safe: no

## Notes
- 구현은 linked worktree 안에서만 진행한다.
