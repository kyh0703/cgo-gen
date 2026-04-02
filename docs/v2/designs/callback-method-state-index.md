---
feature: callback-method-state-index
status: plan_ready
created_at: 2026-04-02T00:00:00+09:00
---

# Callback Method State Index Mismatch

## Goal

메서드 콜백 등록 시 전역 상태 변수 이름(`_cb1`)과 메서드 본문 참조(`_cb0`) 불일치를 수정한다.

## Context / Inputs

- User brief: `SetHACallback` 메서드에서 cb0→cb1 참조 불일치 발견
- Root cause:
  - `callback_usages_for_function()`: `function.params.iter().enumerate()` → self=0, pFunc=1 → `param_index=1` → 전역 상태 `_cb1` 선언
  - `render_callback_method()`: `function.params.iter().skip(1)` → method_params에서 pFunc는 index 0 → `callback_state_name_from_function(function, 0)` → 메서드 본문에서 `_cb0` 참조
  - **결과**: 전역 상태는 `_cb1`인데 메서드는 `_cb0`을 lock/set → 잘못된 슬롯에 콜백 등록, silent runtime bug

## Plan Handoff

### Scope for Planning

`src/facade.rs` 한 파일 수정:

1. `render_callback_call_prep()` 시그니처에 `param_offset: usize` 추가
2. 내부에서 state name 계산: `index + param_offset`
3. 호출 위치 수정:
   - `render_callback_method()` (line ~632): `param_offset = 1` (self 제거했으므로)
   - `render_callback_free_function()` (line ~705): `param_offset = 0` (변경 없음)

### Success Criteria

- 생성된 메서드 본문이 `_cb1` (또는 원래 인덱스)으로 전역 상태 참조
- `callback_usages_for_function`의 `param_index`와 메서드 본문 state name 일치
- 기존 테스트(isaamaster fixture) 통과
- free function 콜백은 동작 그대로 유지

### Non-Goals

- 콜백 인덱싱 전략 전반 리팩터링
- 다중 콜백 파라미터 동시 처리 변경

### Open Questions
- 없음

### Suggested Validation

- `cargo test` 통과
- 생성된 wrapper 코드에서 `_cb` 번호가 `param_index`와 일치하는지 확인

### Parallelization Hints

- Candidate write boundaries: `src/facade.rs` 단일 파일
- Shared files: 없음
- Sequential: 단일 task로 충분
