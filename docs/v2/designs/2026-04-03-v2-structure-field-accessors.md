---
feature: structure-field-accessors
status: plan_ready
created_at: 2026-04-03T17:20:00+09:00
---

# Structure Field Accessors

## Goal

`structure` 타입으로 분류되는 생성 대상에서 내부 필드를 직접 다룰 수 있도록 필드별 getter/setter를 생성한다.

## Context / Inputs

- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - 현재 생성 결과에서는 `structure` 타입 내부 필드에 대한 직접 접근 경로가 없어 소비자가 값을 읽거나 쓰기 어렵다.
  - 이번 요구는 구조체 전체 설계 변경이 아니라, 이미 식별된 `structure` 타입에 대해 필드 접근용 API를 추가하는 범위다.
- User brief:
  - "우리 type이 structure일때 안에 변수들을 지금 직접 접근할수가 없자나 getset을 따로 만들어줘야될것같은데"

## Plan Handoff

### Scope for Planning

- `structure` 타입이 현재 어떤 IR/type kind로 식별되는지 확인한다.
- 해당 타입 렌더링 경로에서 필드 메타데이터를 이용해 getter/setter 생성 지점을 정한다.
- 생성 규칙을 최소 범위로 정의한다.
  - 읽기 가능한 필드는 getter 생성
  - 쓰기 가능한 필드는 setter 생성
  - 기존 naming/style을 유지
- 대표적인 `structure` 입력에 대한 생성 결과 테스트를 추가한다.

### Success Criteria

- `structure` 타입 대상으로 필드별 getter가 생성된다.
- 수정 가능한 필드에는 setter가 생성된다.
- 생성된 API 이름과 시그니처가 기존 출력 스타일과 충돌하지 않는다.
- 관련 테스트로 생성 결과를 검증하고 기존 테스트가 유지된다.

### Non-Goals

- `structure` 외 다른 type kind에 대한 새 접근 규칙 추가
- 필드 접근 외 편의 메서드, builder, DTO 변환기 추가
- 기존 non-structure 타입 렌더링 방식 리팩터링

### Open Questions

- `const`/read-only 필드를 현재 메타데이터에서 어떻게 구분하는지 확인이 필요하다.
- 중첩 구조체나 포인터 필드에 setter를 동일 규칙으로 허용할지는 구현 지점 확인 후 결정한다.

### Suggested Validation

- 관련 Rust 테스트 추가 또는 갱신으로 getter/setter 생성 문자열 검증
- `cargo test`
- 가능하면 대표 입력으로 생성 산출물 스모크 확인

### Parallelization Hints

- Candidate write boundaries:
  - type/IR 분류 확인
  - generator 렌더링 수정
  - 테스트 보강
- Shared files to avoid touching in parallel:
  - 구조체 렌더링 중심 파일
  - 관련 snapshot/fixture 파일
- Likely sequential dependencies:
  - type 식별 확인 후 렌더링 규칙을 정하고, 그 다음 테스트를 고정하는 순서가 안전하다.
