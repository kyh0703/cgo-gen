---
feature: operator-skip-generation
status: plan_ready
created_at: 2026-04-02T16:10:00+09:00
---

# Operator Skip Generation

## Goal

C++ `operator...` 선언은 지원하지 않고, wrapper/Go 생성 대상에서 아예 제외한다.

## Context / Inputs
- Source docs:
  - 사용자 요청: operator는 지원 안 하는 방향으로 두고 생성하지 않게 변경해야 한다.
- Existing system facts:
  - 현재 parser/IR/generator 경로에는 operator 전용 필터가 없다.
  - libclang이 method/function 이름을 `operator...` 형태로 넘기면 현재는 일반 선언처럼 normalize/generate될 수 있다.
  - 이 저장소는 unsupported 선언을 전체 실패 대신 `support.skipped_declarations`에 기록하는 패턴을 이미 사용한다.
- User brief:
  - operator는 생성 대상에서 제외.

## Plan Handoff
### Scope for Planning
- parser 또는 IR normalize 진입점에서 `operator` 선언을 식별한다.
- 식별된 operator free function / method / constructor-like special case는 wrapper 생성 대상에서 제외하고 skip reason을 남긴다.
- 기존 일반 함수/메서드 경로에는 영향이 없도록 회귀 테스트를 추가한다.

### Success Criteria
- `operator+`, `operator=`, `operator[]` 등 `operator` 이름 선언이 raw wrapper 및 Go facade 출력에 나타나지 않는다.
- 스킵된 선언은 기존 skip metadata 패턴으로 기록된다.
- 일반 메서드/함수 생성은 그대로 유지된다.
- `cargo test`가 통과한다.

### Non-Goals
- operator를 부분 지원하지 않는다.
- operator를 다른 메서드 이름으로 변환해 노출하지 않는다.
- clang AST의 모든 special member semantics를 세분화하지 않는다.

### Open Questions
- 없음

### Suggested Validation
- operator method/function fixture 테스트 추가
- skip metadata 확인
- `cargo test`

### Parallelization Hints
- Candidate write boundaries:
  - `src/ir.rs`
  - `tests/`
- Shared files to avoid touching in parallel:
  - operator 판정과 skip metadata는 `src/ir.rs`에 집중되므로 순차 진행이 안전함
- Likely sequential dependencies:
  - skip 로직 추가 후 fixture/test 보강
