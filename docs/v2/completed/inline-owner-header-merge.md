# Inline Owner Header Merge

## Goal
- `-inl.h` 를 standalone wrapper 단위로 생성하지 않고 owner header wrapper 로 흡수해 실제 SIL inline 확장 패턴을 보존한다.

## References
- docs/STATE.md
- docs/ROADMAP.md
- docs/ARCHITECTURE.md
- docs/v2/designs/2026-04-03-v2-inline-owner-header-merge.md
- docs/v2/research/status/sil-conversion-status.md

## Workspace
- Branch: feat/v2-inline-owner-header-merge
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: 현재 생성기에서 `-inl.h` 입력이 별도 wrapper 출력 단위가 되는 분기와 owner 정보를 추적해 최소 수정 지점을 확정한다.
- Depends on:
  - none
- Write Scope:
  - src/...
- Read Context:
  - docs/v2/designs/2026-04-03-v2-inline-owner-header-merge.md
  - src/...
  - tests/...
- Checks:
  - cargo test
- Parallel-safe: no

### Task T2
- Goal: owner-qualified inline 정의를 parent header wrapper 로 귀속시키고 `*_inl_wrapper.*` 독립 산출을 막는 최소 구현을 추가한다.
- Depends on:
  - T1
- Write Scope:
  - src/...
- Read Context:
  - docs/v2/designs/2026-04-03-v2-inline-owner-header-merge.md
  - src/...
- Checks:
  - cargo test
- Parallel-safe: no

### Task T3
- Goal: `-inl.h` 독립 산출 부재와 owner wrapper 유지 여부를 검증하는 회귀 테스트 또는 fixture 검증을 추가한다.
- Depends on:
  - T2
- Write Scope:
  - tests/...
  - fixtures/...
- Read Context:
  - docs/v2/designs/2026-04-03-v2-inline-owner-header-merge.md
  - tests/...
  - fixtures/...
- Checks:
  - cargo test
- Parallel-safe: no

## Notes
- 구현은 linked worktree 안에서만 진행한다.
- 사용자 제공 `gopkg/` 산출물은 현재 root workspace 의 참고 자료로만 보고, 관련 없는 기존 변경은 건드리지 않는다.
