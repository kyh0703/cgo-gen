---
feature: callback-bridge-nullptr-cast
status: plan_ready
created_at: 2026-04-03T16:45:00+09:00
---

# Callback Bridge Nullptr Cast

## Goal

콜백 bridge 생성 코드가 non-capturing lambda와 `nullptr`를 삼항 연산자로 섞을 때 C++ 타입 불일치가 나지 않도록, generated wrapper가 콜백 typedef 기준으로 안정적으로 컴파일되게 만든다.

## Context / Inputs
- Source docs:
  - `docs/STATE.md`
  - `docs/ARCHITECTURE.md`
  - `docs/v2/designs/callback-facade-support.md`
- Existing system facts:
  - 현재 callback bridge는 `auto <name> = [](...) { ... };` 형태의 lambda 지역 변수를 만든다.
  - 이후 `use_cb ? <lambda_var> : nullptr` 를 그대로 호출 인자로 넘겨 실제 C++ 컴파일에서 타입 불일치가 발생한다.
  - 실제 실패 예시는 `cgowrap_iSiLib_SetHACallback_bridge(..., bool)` 에서 `SICHACALLBACK` 인자 전달 시 발생한다.
- User brief:
  - 실제 생성된 `i_si_lib_wrapper.cpp` 에서 `SetHACallback_bridge` 삼항 연산자 타입 오류를 조사하고 수정해달라는 요청.

## Plan Handoff
### Scope for Planning
- callback bridge 렌더링이 lambda 객체를 어떻게 선언하고 호출 인자로 넣는지 확인한다.
- generated C++에서 콜백 typedef와 `nullptr`가 동일 타입으로 취급되도록 최소 수정한다.
- 기존 callback facade 테스트에 회귀 검증을 추가하거나 기대 문자열을 갱신한다.

### Success Criteria
- generated callback bridge가 `use_cb ? ... : nullptr` 경로에서도 C++ 컴파일 오류를 내지 않는다.
- `SetHACallback` 같은 named callback typedef bridge가 계속 생성된다.
- 관련 Rust 테스트가 통과한다.

### Non-Goals
- callback facade 전체 재설계
- 다중 콜백 상태 관리 확장
- 요청되지 않은 callback API 표면 변경

### Open Questions
- `auto` 대신 typedef 함수 포인터 변수 선언이 가장 작은 수정인지, 또는 명시적 cast가 더 안전한지는 구현 시 확인이 필요하다.

### Suggested Validation
- `cargo test`
- callback facade 관련 테스트 재실행
- 가능하면 generated callback bridge 문자열 또는 compile path에서 typedef 함수 포인터 선언 확인

### Parallelization Hints
- Candidate write boundaries:
  - `src/generator.rs`
  - `tests/facade_generate.rs`
- Shared files to avoid touching in parallel:
  - 없음, 단일 vertical slice 권장
- Likely sequential dependencies:
  - generator 수정 후 테스트 기대치 갱신
