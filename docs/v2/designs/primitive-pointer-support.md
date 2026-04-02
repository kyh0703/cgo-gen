---
feature: primitive-pointer-support
status: plan_ready
created_at: 2026-04-02T00:00:00+09:00
---

# Primitive Pointer Parameter Support

## Goal

`int32*`, `uint32*` 등 primitive pointer 타입 파라미터를 가진 C++ 메서드를 Go facade에서 올바르게 생성한다.

## Context / Inputs

- Source docs: 사용자 버그 리포트 (PSC 포팅 시 발생)
- Existing system facts:
  - `src/ir.rs:is_supported_primitive()` 에 `"int32"`, `"uint32"` 등 (`_t` 없는 형태)이 누락됨
  - 결과: `int32*` → `raw_safe_model_handle_name("int32*")` → `Some("int32Handle")` → kind `"model_pointer"` 로 잘못 분류
  - `facade.rs` 코드 생성: `cArg0 = nPos.ptr` (Go `*int32` 에 `.ptr` 접근 → 컴파일 에러)
  - 추가로 `"pointer"` kind가 `go_param_supported` 에서 `false` 반환 → 함수 자체가 skip됨
- User brief: PSC 헤더에 `int32*`, `uint32*`, `pLogLvl *uint32`, `nRouteId *uint32` 등 primitive pointer 파라미터를 가진 메서드들이 다수 존재

## Plan Handoff

### Scope for Planning

1. `src/ir.rs` — `is_supported_primitive()` 에 `_t` 없는 primitive 이름 추가
   - 추가 대상: `"int8"`, `"int16"`, `"int32"`, `"int64"`, `"uint8"`, `"uint16"`, `"uint32"`, `"uint64"`
   - 이로써 `int32*` → kind `"pointer"` 로 올바르게 분류됨

2. `src/facade.rs` — `go_param_type()` 에 `"pointer"` case 추가
   - `*int32`, `*uint32` 등 Go pointer type 반환

3. `src/facade.rs` — `render_call_prep()` (두 곳) 에 `"pointer"` case 추가
   - 생성 코드: `cArgN := (*C.int32_t)(unsafe.Pointer(name))`

### Success Criteria

- `int32*` 파라미터를 가진 C++ 메서드가 `go_param_supported` 를 통과함
- 생성된 Go 코드: `cArg0 := (*C.int32_t)(unsafe.Pointer(nPos))`
- 기존 tests (isaamaster fixture) 통과

### Non-Goals

- struct 포인터(`*IsWebHook` 등 model_pointer)는 수정 대상 아님 — 기존 `.ptr` 패턴 유지
- const primitive pointer, double-pointer 지원은 이번 범위 아님

### Open Questions

- 없음

### Suggested Validation

- `cargo test` 전체 통과
- `is_supported_primitive` 단위 테스트 추가 (int32, uint32 포함)
- facade.rs 에 `"pointer"` kind 파라미터 렌더링 테스트 추가

### Parallelization Hints

- Candidate write boundaries: `src/ir.rs` vs `src/facade.rs` — 독립 파일
- Shared files to avoid touching in parallel: 없음 (두 파일 독립)
- Likely sequential dependencies: ir.rs 수정 → facade.rs 수정 → test 추가 순서 권장 (논리적 순서지만 실제로는 병렬 가능)
