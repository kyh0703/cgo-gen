---
feature: model-view-snapshot-copy
status: plan_ready
created_at: 2026-04-03T23:10:24+09:00
---

# Model View Snapshot Copy

## Goal

`model_view` 성격의 반환/필드 접근을 Go public API에서 borrowed alias가 아니라 owned snapshot으로 고정하고, 변경 반영은 explicit setter 또는 native API 호출로만 일어나게 만든다.

## Context / Inputs

- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
  - `docs/v2/designs/2026-04-02-v2-model-pointer-return.md`
  - `docs/v2/designs/2026-04-03-v2-structure-field-accessors.md`
- Existing system facts:
  - 현재 Go facade/model public shape는 `ptr *C.Handle` + `Close()`를 가진 owning wrapper다.
  - 현재 `model_pointer` 반환은 facade에서 비소유 wrap (`&GoStruct{ptr: raw}`) 로 처리되고 있다.
  - raw generator는 `model_value` 반환을 `new T(...)` copy handle로 노출하는 경로를 이미 갖고 있다.
  - struct field accessor 자동 생성은 현재 primitive field만 대상으로 제한돼 있다.
  - 테스트 실행은 현재 로컬 `libclang.dylib` 로더 문제를 먼저 해결해야 완전 검증 가능하다.
- User brief:
  - `model_view` 반환과 `model_view` set 시나리오까지 고려하면 현재 구조가 끝나지 않는다.
  - SWIG / Rust bindgen 관행을 참고해 우리 프로젝트에 맞는 지원 방식을 원한다.
  - borrowed view 대신 snapshot copy + explicit set 경로가 현재 public API와 더 잘 맞는다는 방향으로 결정했다.

## Plan Handoff

### Scope for Planning
- `src/ir.rs`에서 `Model*`, `Model&`, `const Model&`, direct model-valued field access를 snapshot-copy 대상 kind 또는 동등 정책으로 분류한다.
- `src/generator.rs`에서 snapshot-copy 반환을 raw ABI에서 fresh owned handle로 생성한다.
  - pointer return은 null passthrough 후 `new T(*ptr)`
  - reference return은 `new T(ref)`
- `src/facade.rs`에서 snapshot-copy 반환을 기존 owning `*Model` Go wrapper로 렌더링한다.
- struct field accessor 생성 규칙을 direct model field getter/setter까지 확장한다.
  - getter는 `*ChildModel` snapshot 반환
  - setter는 `*ChildModel` 입력을 native copy-in 경계로 전달
- non-copyable model, unknown model, pointer-to-model field, double indirection, container/STL model field는 이번 범위에서 skip 유지한다.
- 관련 Rust 테스트와 representative generated output 검증을 추가한다.

### Success Criteria
- Go public API에는 borrowed `ModelView` 타입이 추가되지 않는다.
- model-view 성격 반환은 Go에서 `*Model` snapshot으로 노출된다.
- snapshot 수정은 parent/native state에 즉시 반영되지 않고 explicit setter/native API 재호출 때만 반영된다.
- raw wrapper는 borrowed handle alias 대신 owned copy handle을 반환한다.
- direct model-valued struct field accessor getter/setter가 생성된다.
- unsafe하거나 copy semantics가 불명확한 케이스는 skip되고 이유가 남는다.

### Non-Goals
- Go public API에 별도 `ModelView` / borrowed view 타입 도입
- parent pinning, alias lifetime tracking, live child mutation 지원
- STL/container/nested graph 전체를 일반화한 deep copy 정책 도입
- non-copyable model을 위한 move-only ABI 설계

### Open Questions
- 없음

### Suggested Validation
- `cargo test`
- model return / field accessor 관련 targeted tests
- representative generated C++ wrapper 문자열 검증
- 가능하면 generated compile smoke로 copied handle 코드 경로 확인

### Parallelization Hints
- Candidate write boundaries:
  - `src/ir.rs` type classification / field accessor expansion
  - `src/generator.rs` raw copy-return generation
  - `src/facade.rs` Go return rendering
  - `tests/` regression coverage
- Shared files to avoid touching in parallel:
  - `src/ir.rs`
  - `src/facade.rs`
  - active plan document
- Likely sequential dependencies:
  - IR 정책 고정
  - raw generator copy semantics 구현
  - Go facade 렌더링 정렬
  - regression tests와 compile smoke 확인
