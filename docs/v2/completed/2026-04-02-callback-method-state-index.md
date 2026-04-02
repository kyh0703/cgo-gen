# Callback Method State Index Mismatch Fix

## Goal
- `render_callback_call_prep()`에 `param_offset` 추가하여 메서드 콜백 state name이 전역 선언과 일치하도록 수정한다.

## References
- docs/v2/designs/callback-method-state-index.md
- src/facade.rs

## Workspace
- Branch: feat/v2-callback-method-state-index
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph

### Task T1
- Goal: `render_callback_call_prep()` 에 `param_offset: usize` 추가, 호출 위치 2곳 수정
- Depends on: none
- Write Scope:
  - src/facade.rs
- Read Context:
  - docs/v2/designs/callback-method-state-index.md
- Checks:
  - cargo test
- Parallel-safe: no

## Notes
- 단일 task, `render_callback_method` 호출 시 offset=1, free function 시 offset=0
