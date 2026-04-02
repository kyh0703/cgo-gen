# Primitive Pointer Parameter Support

## Goal
- `int32*`, `uint32*` 등 primitive pointer 파라미터를 Go facade에서 올바르게 생성한다.
- 잘못된 `nPos.ptr` 패턴을 `(*C.int32_t)(unsafe.Pointer(nPos))` 패턴으로 수정한다.

## References
- docs/STATE.md
- docs/v2/designs/primitive-pointer-support.md
- src/ir.rs
- src/facade.rs

## Workspace
- Branch: feat/v2-primitive-pointer-support
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph

### Task T1
- Goal: `src/ir.rs` — `is_supported_primitive()` 에 `_t` 없는 primitive 이름 추가
- Depends on: none
- Write Scope:
  - src/ir.rs
- Read Context:
  - docs/v2/designs/primitive-pointer-support.md
- Checks:
  - cargo test
- Parallel-safe: yes

### Task T2
- Goal: `src/facade.rs` — `go_param_type()` 와 `render_call_prep()` 두 곳에 `"pointer"` case 추가
- Depends on: none
- Write Scope:
  - src/facade.rs
- Read Context:
  - docs/v2/designs/primitive-pointer-support.md
  - src/ir.rs
- Checks:
  - cargo test
- Parallel-safe: yes

### Task T3
- Goal: `"pointer"` 파라미터 렌더링 단위 테스트 추가 및 전체 검증
- Depends on: T1, T2
- Write Scope:
  - src/facade.rs (test module)
  - src/ir.rs (test module)
- Read Context:
  - src/facade.rs
  - src/ir.rs
- Checks:
  - cargo test
  - manual: int32* 파라미터 포함 헤더로 코드 생성 확인
- Parallel-safe: no

## Notes
- T1, T2는 독립 파일이므로 wave 1에서 병렬 실행 가능
- T3는 T1+T2 완료 후 wave 2에서 실행
- struct pointer (`model_pointer` kind) 는 건드리지 않음 — `.ptr` 패턴 유지
