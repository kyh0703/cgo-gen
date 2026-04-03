---
feature: by-value-model-return-skip
status: plan_ready
created_at: 2026-04-03T18:10:00+09:00
---

# By-Value Model Return Skip

## Goal

by-value C++ 객체 반환이 raw wrapper에서 handle 포인터 반환처럼 생성되지 않도록 막아 컴파일 오류를 없앤다.

## Context / Inputs

- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - `gopkg/sil/is_call_wrapper.cpp`에서 `MTime`/`TD_IE_CALL` 값을 `*Handle`로 `reinterpret_cast`하는 잘못된 코드가 생성됐다.
  - 현재 raw generator는 `returns.handle.is_some()`이면 반환식을 무조건 handle 포인터 캐스트로 렌더링한다.
  - by-value 객체 반환은 현재 C ABI에서 안전한 소유권/복사 규칙 없이 handle 포인터로 노출할 수 없다.
- User brief:
  - `invalid cast from type 'MTime' to type 'MTimeHandle*'`
  - `invalid cast from type 'TD_IE_CALL' to type 'TD_IE_CALLHandle*'`

## Plan Handoff
### Scope for Planning
- `src/ir.rs`에서 by-value 객체 반환이 `model_pointer`/`model_reference` 또는 기타 handle-backed 반환으로 정규화되는 경로를 확인한다.
- 값 객체 반환은 기존 raw-unsafe 규칙대로 skip되게 고정한다.
- 포인터/참조 기반 model return 지원과 extern struct canonicalization은 유지한다.
- 재현 테스트를 추가해 같은 형태의 잘못된 handle 캐스트가 다시 생기지 않게 한다.

### Success Criteria
- by-value 객체 반환 declaration이 raw wrapper 함수로 생성되지 않거나 안전한 형태로만 생성된다.
- `MTime`, `TD_IE_CALL` 같은 값 반환에서 `reinterpret_cast<...Handle*>(...)`가 생성되지 않는다.
- 기존 `model_pointer` 지원과 `timeval` canonical fallback 테스트는 유지된다.
- 관련 Rust 테스트가 통과한다.

### Non-Goals
- by-value 객체 반환을 새 ABI 설계로 지원
- Go facade의 반환 lifting 규칙 변경
- 이미 생성된 산출물을 수동 hotfix하는 임시 조치

### Open Questions
- 없음

### Suggested Validation
- `cargo test`
- 관련 단위 테스트로 canonical fallback과 by-value skip 경계 확인
- 가능하면 실제 생성 재현으로 `is_call_wrapper.cpp`에 잘못된 함수가 사라졌는지 확인

### Parallelization Hints
- Candidate write boundaries:
  - `src/ir.rs`
  - 관련 테스트 파일
- Shared files to avoid touching in parallel:
  - `src/ir.rs`
- Likely sequential dependencies:
  - 정규화 경계 수정 후 테스트 추가 및 재생성 확인 순서
