---
feature: modelpointer-iserialize-build-fix
status: plan_ready
created_at: 2026-04-03T14:45:00+09:00
---

# ModelPointer iSerialize Build Fix

## Goal

`ModelPointer` 지원 추가 이후 생성된 `iSerialize` 래퍼가 다시 빌드되도록, enum 중복 정의와 `uint64_t` 매핑 오류를 최소 수정으로 복구한다.

## Context / Inputs

- Source docs:
  - `docs/STATE.md`
  - `docs/ARCHITECTURE.md`
  - `docs/v2/designs/2026-04-02-v2-model-pointer-return.md`
- Existing system facts:
  - 사용자 제공 빌드 로그상 `i_serialize_wrapper.h`가 `eSeriType`를 다시 정의해 원본 `../inc/SIL/iSerialize.h`와 충돌한다.
  - 같은 로그에서 `uint64_t`가 C++의 `uint64`와 정확히 맞지 않아 `GetVal`, `Add`, `Get` 오버로드 선택이 실패한다.
  - 저장소에는 생성기 로직이 있고, 실제 수정은 generated wrapper 결과가 아니라 생성 규칙 쪽에 들어갈 가능성이 높다.
- User brief:
  - "ModelPointer 제공하고 나서" `iSerialize` wrapper 빌드 에러가 발생했고, 원인 확인과 수정이 필요하다.

## Plan Handoff

### Scope for Planning

- `ModelPointer` 관련 최근 변경과 현재 타입 매핑 규칙을 비교해, 왜 `iSerialize` wrapper가 enum과 `uint64_t`를 잘못 생성하는지 재현 가능한 원인으로 좁힌다.
- 원인이 생성기 로직이면 해당 경로만 수정해 `eSeriType` 중복 정의를 막고 `uint64` 계열 호출이 올바른 C++ 타입으로 생성되게 한다.
- 같은 실패를 막는 회귀 검증을 추가하거나, 최소한 재현 입력으로 생성 결과를 검증할 수 있게 한다.

### Success Criteria

- `iSerialize` 관련 generated wrapper가 `eSeriType`를 중복 정의하지 않는다.
- `GetVal`, `Add`, `Get`에서 `uint64` 오버로드가 모호하지 않게 생성된다.
- 관련 테스트 또는 재현용 검증이 추가되고 통과한다.

### Non-Goals

- `ModelPointer` 기능 전체 재설계
- `iSerialize` 외 다른 SIL 헤더 전반 리팩터링
- 요청되지 않은 생성기 구조 변경

### Open Questions

- 실제 실패가 checked-in fixture에서 재현되는지, 아니면 사용자 SIL 헤더 셋에서만 재현되는지는 worktree에서 확인이 필요하다.

### Suggested Validation

- 관련 Rust 테스트 실행
- 가능하면 `iSerialize` 입력으로 wrapper 생성 후 C++ 컴파일 재현 확인
- 최소한 generated output diff에서 `eSeriType`와 `uint64` 처리 결과 확인

### Parallelization Hints

- Candidate write boundaries:
  - `src/` 생성기 로직
  - `tests/` 회귀 검증
- Shared files to avoid touching in parallel:
  - 공용 타입 매핑 로직 파일
- Likely sequential dependencies:
  - 재현/원인 확인 후 생성기 수정, 그 다음 회귀 검증 추가
