# Clang Arg Environment Variable Expansion

## Goal
- `input.clang_args` 에서 `$VAR`, `$(VAR)`, `${VAR}` 형태의 env token을 확장해 실제 clang argument로 사용 가능하게 만든다.

## References
- docs/STATE.md
- docs/ROADMAP.md
- docs/ARCHITECTURE.md
- docs/v2/designs/2026-04-02-v2-clang-arg-env-expansion.md
- README.md
- README.ko.md
- src/config.rs
- tests/config.rs

## Workspace
- Branch: feat/v2-clang-arg-env-expansion
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: `input.clang_args` env token expansion helper를 추가하고 기존 relative path normalization 흐름에 통합한다.
- Depends on:
  - none
- Write Scope:
  - src/config.rs
- Read Context:
  - docs/v2/designs/2026-04-02-v2-clang-arg-env-expansion.md
  - src/config.rs
- Checks:
  - cargo test --test config
- Parallel-safe: no

### Task T2
- Goal: `$VAR`, `$(VAR)`, `${VAR}`, `-I<env-token>`, missing env 케이스를 고정하는 config 회귀 테스트를 추가한다.
- Depends on:
  - T1
- Write Scope:
  - tests/config.rs
- Read Context:
  - src/config.rs
  - docs/v2/designs/2026-04-02-v2-clang-arg-env-expansion.md
- Checks:
  - cargo test --test config
- Parallel-safe: yes

### Task T3
- Goal: 공개 README에 `input.clang_args` env token 지원 범위와 비지원 shell 문법 경계를 반영한다.
- Depends on:
  - T1
- Write Scope:
  - README.md
  - README.ko.md
- Read Context:
  - docs/v2/designs/2026-04-02-v2-clang-arg-env-expansion.md
  - src/config.rs
- Checks:
  - manual: README 설명이 구현 범위와 일치함
- Parallel-safe: yes

## Notes
- env token은 exact-match syntax만 지원한다: `$VAR`, `$(VAR)`, `${VAR}`
- missing env는 조용히 통과시키지 않고 config load error로 처리한다
- 구현은 primary repo root가 아니라 linked worktree에서만 시작한다
