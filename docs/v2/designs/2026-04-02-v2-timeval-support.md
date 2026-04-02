---
feature: timeval-support
status: plan_ready
created_at: 2026-04-02T14:00:00+09:00
---

# Timeval Support

## Goal

`timeval*` / `struct timeval*` / `struct timeval&` 같은 외부 C struct 시그니처가 Go facade에서 문법 오류 없이 생성되도록 한다.

## Context / Inputs
- Source docs:
  - 사용자 에러:
    - `../gopkg/sil/is_sip_trunk_wrapper.go:787:42: expected '{', found timeval`
    - `../gopkg/sil/is_sip_trunk_wrapper.go:791:18: expected 'IDENT', found 'struct'`
- Existing system facts:
  - 현재 `src/ir.rs`는 primitive, string, known-model 외의 `*` / `&` 타입을 `raw_safe_model_handle_name()` 경유로 `model_pointer` / `model_reference`로 분류할 수 있다.
  - `struct timeval*`가 이 경로로 들어가면 Go 렌더링에서 `*struct timeval` 같은 잘못된 타입 문자열이 나올 수 있다.
  - `normalize_type_with_canonical()`은 alias 표기(`timeval*`)가 실패해도 canonical 표기(`struct timeval*`)로 재시도할 수 있다.
- User brief:
  - `timeval` 때문에 생성된 Go wrapper가 깨진다. 이를 지원할 수 있는 경로가 필요하다.

## Plan Handoff
### Scope for Planning
- `src/ir.rs`에서 canonical/display 타입이 `struct <name>*` 또는 `struct <name>&`인 경우를 외부 C struct 포인터/레퍼런스로 구분한다.
- 새 IR kind는 Go facade에서 `*C.struct_<name>`로 렌더링하고, 호출 인자는 `unsafe.Pointer` 기반 캐스트로 전달한다.
- `struct timeval`가 별도 모델 핸들처럼 취급되지 않도록 기존 model fallback보다 먼저 처리한다.
- 최소 회귀 테스트를 추가해 alias 표기(`timeval*`)와 canonical 표기(`struct timeval*`)가 모두 안전하게 통과하는지 확인한다.

### Success Criteria
- `timeval*` 또는 `struct timeval*`가 포함된 시그니처를 가진 선언이 더 이상 `model_pointer`로 분류되지 않는다.
- 생성된 Go facade 타입에 `*struct timeval` 같은 문법 오류가 남지 않는다.
- Go facade는 `*C.struct_timeval` 기반 파라미터를 생성한다.
- 관련 Rust 테스트와 전체 `cargo test`가 통과한다.

### Non-Goals
- 임의의 by-value C struct를 Go 값 타입으로 승격하지 않는다.
- C struct 필드를 Go struct로 매핑하지 않는다.
- `struct timeval**` 같은 다중 포인터를 이번 범위에 넣지 않는다.

### Open Questions
- 없음

### Suggested Validation
- 외부 C struct pointer/reference 정규화 단위 테스트 추가
- facade 렌더링 테스트에서 `*C.struct_timeval` 및 호출 캐스트 확인
- `cargo test`

### Parallelization Hints
- Candidate write boundaries:
  - `src/ir.rs`
  - `src/facade.rs`
  - `tests/...`
- Shared files to avoid touching in parallel:
  - IR kind 정의가 바뀌므로 `src/ir.rs`와 `src/facade.rs`는 같은 wave에서 분리하지 않는 편이 안전함
- Likely sequential dependencies:
  - IR 정규화 추가 후 facade 렌더링 반영, 마지막에 테스트 보강
