---
feature: model-pointer-return
status: plan_ready
created_at: 2026-04-02T00:00:00+09:00
---

# Model Pointer Return Support

## Goal

C++ 메서드가 `SomeClass*`를 반환할 때 (`model_pointer` kind), 현재 skip되는 대신 Go facade 코드를 올바르게 생성한다.

## Context / Inputs

- Source docs: 사용자 요청 (포인터 변수 반환 미처리)
- Existing system facts:
  - `src/facade.rs:go_return_supported()` 에 `model_pointer` case가 없어서 해당 메서드가 통째로 skip됨
  - 파라미터 쪽에서는 `model_pointer` 처리가 이미 완성됨 (`known_model_projection`, `render_model_arg`)
  - 생성자 패턴 `&GoStruct{ptr: raw}` 이 이미 존재하므로 반환값에도 동일 패턴 적용 가능
  - C wrapper는 이미 `SomeClassHandle*` 형태로 raw pointer를 반환함
- User brief: `SomeClass*` 반환 메서드가 facade에서 누락되는 문제. 비소유(non-owning) 래핑 전략으로 `&GoStruct{ptr: raw}`를 반환

## Plan Handoff

### Scope for Planning

1. `src/facade.rs` — `go_return_supported()` 에 `model_pointer` case 추가
   - 조건: `ty.kind == "model_pointer"` 이고 `config.known_model_projection` 또는 leaf name으로 Go 타입을 결정할 수 있을 때

2. `src/facade.rs` — `render_method()` 반환값 처리 분기에 `model_pointer` 추가
   - Go 반환 타입: `*GoStructName`
   - nil 체크: `if raw == nil { return nil }`
   - 반환 코드: `return &GoStructName{ptr: raw}`

3. `src/facade.rs` — `render_free_function()` 반환값 처리 분기에 동일 패턴 추가

4. `src/facade.rs` — `render_callback_method()`, `render_callback_free_function()` 반환값 처리 분기에 동일 패턴 추가 (해당되는 경우)

5. 단위 테스트 추가 — `model_pointer` 반환 메서드의 Go 코드 생성 검증

### Success Criteria

- `SomeClass*` 반환 메서드가 `go_return_supported()` 를 통과함
- 생성된 Go 코드: `return &GoStruct{ptr: raw}` 패턴
- nil 반환 시 `return nil` 처리
- 기존 tests 전체 통과
- `model_pointer` 반환 단위 테스트 추가

### Non-Goals

- 소유권 관리 (owning wrap, Release 호출) — 비소유 래핑만 지원
- `model_reference` 반환 지원 — 별도 feature로 분리
- `extern_struct_pointer` 반환 지원 — 별도 feature
- double pointer (`SomeClass**`) 반환 — 이번 범위 아님

### Open Questions

- 없음

### Suggested Validation

- `cargo test` 전체 통과
- `go_return_supported()` 에서 `model_pointer` kind 허용 테스트
- `render_method()` 에서 `model_pointer` 반환 코드 생성 테스트
- 실제 헤더로 생성 후 Go 컴파일 확인 (수동)

### Parallelization Hints

- Candidate write boundaries: 모든 변경이 `src/facade.rs` 단일 파일 — 병렬화 불가
- Shared files to avoid touching in parallel: `src/facade.rs`
- Likely sequential dependencies: go_return_supported → render 함수들 → 테스트 순서
