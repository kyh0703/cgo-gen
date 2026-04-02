# Primitive Alias C Header Normalization

## Goal
- primitive alias pointer/reference가 생성 C header에서 표준 `stdint` 타입으로 나오도록 수정하고 검증한다.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-02-v2-primitive-alias-c-header-normalization.md`
- `docs/v2/designs/primitive-pointer-support.md`

## Workspace
- Branch: feat/v2-primitive-alias-c-header-normalization
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: primitive alias pointer/reference의 `c_type`을 `stdint` 표준형으로 정규화한다.
- Depends on:
  - none
- Write Scope:
  - `src/ir.rs`
- Read Context:
  - `docs/v2/designs/2026-04-02-v2-primitive-alias-c-header-normalization.md`
  - `docs/v2/designs/primitive-pointer-support.md`
- Checks:
  - `cargo test primitive_pointer`
  - `cargo test generator`
- Parallel-safe: no

### Task T2
- Goal: 생성 header/Go wrapper 기대를 고정하는 회귀 테스트를 추가하고 전체 테스트를 통과시킨다.
- Depends on:
  - T1
- Write Scope:
  - `tests/`
- Read Context:
  - `src/ir.rs`
  - `src/generator.rs`
  - `src/facade.rs`
- Checks:
  - `cargo test`
- Parallel-safe: no

## Notes
- 수정 위치는 IR 정규화로 제한하고, 렌더러에서 후처리 치환하는 접근은 쓰지 않는다.
