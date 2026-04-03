# Structure Field Accessors

## Goal
- `structure` 타입의 내부 필드에 대해 getter/setter를 생성해서 소비자가 값을 읽고 쓸 수 있게 한다.

## References
- docs/STATE.md
- docs/ROADMAP.md
- docs/ARCHITECTURE.md
- docs/v2/designs/2026-04-03-v2-structure-field-accessors.md
- src/parser.rs
- src/ir.rs
- src/generator.rs
- src/facade.rs

## Workspace
- Branch: feat/v2-structure-field-accessors
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: `structure` 타입이 현재 parser/IR/generator 경로에서 어떤 shape로 전달되는지 확인하고 accessor 생성에 필요한 필드 메타데이터 사용 지점을 확정한다.
- Depends on:
  - none
- Write Scope:
  - src/parser.rs
  - src/ir.rs
  - src/generator.rs
- Read Context:
  - docs/v2/designs/2026-04-03-v2-structure-field-accessors.md
  - src/parser.rs
  - src/ir.rs
  - src/generator.rs
- Checks:
  - cargo test
- Parallel-safe: no

### Task T2
- Goal: `structure` 렌더링 경로에 필드별 getter/setter 생성 규칙을 추가하고 기존 naming/style과 충돌하지 않게 출력한다.
- Depends on:
  - T1
- Write Scope:
  - src/generator.rs
  - 필요 시 src/facade.rs
  - 필요 시 src/ir.rs
- Read Context:
  - src/generator.rs
  - src/facade.rs
  - src/config.rs
- Checks:
  - cargo test
- Parallel-safe: no

### Task T3
- Goal: 대표 `structure` 입력에 대해 getter/setter 생성 결과를 검증하는 테스트를 추가하거나 갱신한다.
- Depends on:
  - T2
- Write Scope:
  - src/generator.rs
  - 관련 테스트 fixture 또는 snapshot 파일
- Read Context:
  - src/generator.rs
  - 기존 테스트 패턴
- Checks:
  - cargo test
- Parallel-safe: no

## Notes
- 현재 검색 결과 기준으로 구조체 관련 생성 책임은 `src/generator.rs`와 `src/facade.rs` 주변에 모여 있으므로, 실제 accessor 출력 위치를 먼저 확정한 뒤 변경 범위를 최소화한다.
- `structure`가 외부 C struct(`extern_struct_*`)와 다른 경로라면 두 개념을 섞지 말고 이번 범위는 사용자 요청의 구조체 필드 접근에만 한정한다.
