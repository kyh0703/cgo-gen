# Model Pointer Return Support

## Goal
- C++ 메서드가 `SomeClass*`를 반환할 때 (`model_pointer` kind) Go facade 코드를 생성한다.
- 비소유(non-owning) 래핑 전략: `&GoStruct{ptr: raw}` 패턴을 사용한다.

## References
- docs/STATE.md
- docs/v2/designs/2026-04-02-v2-model-pointer-return.md
- docs/v2/completed/2026-04-02-primitive-pointer-support.md
- src/facade.rs

## Workspace
- Branch: feat/v2-model-pointer-return
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph

### Task T1
- Goal: `go_return_supported()` 에 `model_pointer` kind 허용 추가 및 `go_model_pointer_return_info()` 헬퍼 함수 작성
- Depends on: none
- Write Scope:
  - src/facade.rs (`go_return_supported` 함수, 새 헬퍼 함수)
- Read Context:
  - docs/v2/designs/2026-04-02-v2-model-pointer-return.md
  - src/facade.rs (`go_param_type`의 `model_pointer` 처리 참고)
- Checks:
  - cargo test
- Parallel-safe: no

### Task T2
- Goal: 4개 render 함수(`render_method`, `render_free_function`, `render_callback_method`, `render_callback_free_function`)에 `model_pointer` 반환 분기 추가
- Depends on: T1
- Write Scope:
  - src/facade.rs (4개 render 함수의 return kind match 블록)
- Read Context:
  - src/facade.rs (기존 `pointer` 반환 처리 패턴 참고)
- Checks:
  - cargo test
- Parallel-safe: no

### Task T3
- Goal: `model_pointer` 반환 단위 테스트 추가 — Go 코드 생성 검증
- Depends on: T2
- Write Scope:
  - src/facade.rs (test module)
- Read Context:
  - src/facade.rs (기존 테스트 패턴)
- Checks:
  - cargo test
- Parallel-safe: no

## Notes
- 모든 변경이 `src/facade.rs` 단일 파일이므로 순차 실행
- `go_return_supported()`는 config 파라미터 없이 `|| ty.kind == "model_pointer"` 추가 (leaf name fallback이 항상 존재하므로)
- render 함수에서 Go 타입 결정: `config.known_model_projection()` → fallback `leaf_cpp_name()`
- 생성 패턴: `raw := C.xxx(); if raw == nil { return nil }; return &GoStruct{ptr: raw}`
