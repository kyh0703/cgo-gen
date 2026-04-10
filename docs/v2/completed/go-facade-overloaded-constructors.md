# Go Facade Overloaded Constructors

## Goal
- Go facade가 클래스당 생성자 하나만 남기는 문제를 수정해서, C++ 오버로드 생성자를 모두 별도 Go 생성자 함수로 노출한다.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `src/codegen/go_facade.rs`
- `src/parsing/parser.rs`
- `src/codegen/ir_norm.rs`
- `tests/overload_collisions.rs`

## Workspace
- Branch: feat/v2-go-facade-overloaded-constructors
- Base: master
- Isolation: required
- Created by: manual execution from Codex plan

## Task Graph
### Task T1
- Goal: facade class 분석 단계가 생성자를 owner별 단일 값으로 덮어쓰지 않고, 지원 가능한 생성자 목록 전체를 유지하도록 바꾼다.
- Depends on:
  - none
- Write Scope:
  - `src/codegen/go_facade.rs`
- Read Context:
  - `src/codegen/go_facade.rs`
  - `src/codegen/ir_norm.rs`
- Checks:
  - `cargo test overloaded_constructors`
- Parallel-safe: no

### Task T2
- Goal: Go 생성자 함수 이름을 명시적으로 렌더링해서 무인자/복사/기타 인자 생성자를 모두 안정적으로 노출한다.
- Depends on:
  - T1
- Write Scope:
  - `src/codegen/go_facade.rs`
- Read Context:
  - `src/codegen/go_facade.rs`
  - 기존 Go facade export naming 규칙
- Checks:
  - `cargo test overloaded_constructors`
- Parallel-safe: no

### Task T3
- Goal: 회귀 테스트를 추가하고, `CSetAgt`와 같은 다중 생성자 클래스에서 Go facade가 3개 생성자를 모두 내보내는지 검증한다.
- Depends on:
  - T2
- Write Scope:
  - `src/codegen/go_facade.rs`
  - `tests/overload_collisions.rs`
- Read Context:
  - 기존 renderer/unit test 패턴
  - 기존 overload collision 테스트
- Checks:
  - `cargo test overloaded_constructors`
  - `cargo test --test overload_collisions`
- Parallel-safe: no

## Notes
- raw C ABI의 constructor overload symbol은 이미 정상 생성되므로 이번 변경 범위에 포함하지 않는다.
- 기존 잘못된 단일 Go 생성자 이름과의 하위 호환은 유지하지 않는다.
