---
feature: primitive-alias-c-header-normalization
status: plan_ready
created_at: 2026-04-02T15:30:00+09:00
---

# Primitive Alias C Header Normalization

## Goal

생성 C header에서 `int32*`, `uint32&` 같은 primitive alias pointer/reference가 비표준 C 타입으로 남지 않도록 `c_type`을 `int32_t*`, `uint32_t*` 등으로 정규화한다.

## Context / Inputs
- Source docs:
  - 사용자 관찰: 일부 생성 header가 `int32`, `uint32`, `uint16`, `uint8`를 그대로 써서 cgo preamble이 깨진다.
- Existing system facts:
  - `src/generator.rs`는 함수/콜백 선언 렌더링에서 `IrType.c_type`을 그대로 출력한다.
  - `src/ir.rs`는 value alias(`int32`, `uint32`)는 `int32_t`, `uint32_t`로 바꾸지만, pointer/reference 분기는 현재 원문 alias를 유지한다.
  - `normalize_type_with_canonical()`은 display type 정규화가 성공하면 canonical type을 보지 않기 때문에 alias pointer/reference를 자동 교정하지 못한다.
- User brief:
  - 해결 방향은 1번: IR 정규화 단계에서 primitive pointer/reference alias를 표준형으로 만드는 방식.

## Plan Handoff
### Scope for Planning
- `src/ir.rs`의 primitive pointer/reference 정규화에서 base primitive alias를 표준 C 정수형으로 치환해 `c_type`을 구성한다.
- value type alias 처리 방식과 일관되게 `int8/16/32/64`, `uint8/16/32/64`에 한정한다.
- 생성 header와 Go facade가 모두 기존 기대를 유지하는지 회귀 테스트를 추가한다.

### Success Criteria
- `int32*`, `uint32*`, `int32&`, `uint32&`가 생성 header에서 각각 `int32_t*`, `uint32_t*`로 렌더링된다.
- `render_header()` 결과에 비표준 alias 포인터/레퍼런스 타입이 남지 않는다.
- 기존 primitive pointer/reference Go facade 동작은 유지된다.
- `cargo test`가 통과한다.

### Non-Goals
- 임의 typedef graph를 전부 해석하는 일반화 작업은 하지 않는다.
- `long`, `unsigned long` 같은 platform-dependent native type 재정의는 다루지 않는다.
- 모델 포인터/외부 struct 포인터 규칙은 이번 범위에 포함하지 않는다.

### Open Questions
- 없음

### Suggested Validation
- `src/ir.rs` 단위 테스트: alias pointer/reference의 `c_type` 확인
- generator/facade 관련 회귀 테스트: header 출력 및 Go pointer/reference 기대 문자열 확인
- `cargo test`

### Parallelization Hints
- Candidate write boundaries:
  - `src/ir.rs`
  - `tests/`
- Shared files to avoid touching in parallel:
  - `src/ir.rs` 변경이 테스트 기대값에 직접 영향을 주므로 순차 진행이 안전함
- Likely sequential dependencies:
  - IR 정규화 수정 후 테스트 기대값 보강
