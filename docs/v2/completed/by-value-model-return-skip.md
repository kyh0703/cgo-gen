# By-Value Model Return Skip

## Goal
- by-value 객체 반환이 raw wrapper에서 handle 포인터 반환으로 생성되는 경로를 차단하고 재발 테스트를 추가한다.

## References
- docs/STATE.md
- docs/ROADMAP.md
- docs/ARCHITECTURE.md
- docs/v2/designs/2026-04-03-v2-by-value-model-return-skip.md
- src/ir.rs
- src/generator.rs
- tests/

## Workspace
- Branch: feat/v2-by-value-model-return-skip
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: `src/ir.rs`에서 by-value 객체 반환이 canonical fallback으로 handle-backed 타입처럼 통과하는 경계를 막는다.
- Depends on:
  - none
- Write Scope:
  - src/ir.rs
- Read Context:
  - docs/v2/designs/2026-04-03-v2-by-value-model-return-skip.md
  - src/ir.rs
- Checks:
  - cargo test
- Parallel-safe: no

### Task T2
- Goal: by-value 객체 반환은 skip되고 포인터/참조 및 timeval canonical fallback은 유지되는 테스트를 추가한다.
- Depends on:
  - T1
- Write Scope:
  - src/ir.rs
  - tests/...
- Read Context:
  - src/ir.rs
  - 기존 normalization 테스트
- Checks:
  - cargo test
- Parallel-safe: no

## Notes
- 이번 범위는 ABI를 새로 설계하지 않고, by-value 모델 반환을 handle-backed copy return으로 지원하면서 by-value 파라미터는 계속 제외하는 쪽으로 정리했다.
