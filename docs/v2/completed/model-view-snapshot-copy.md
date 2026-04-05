# Model View Snapshot Copy

## Goal
- `model_view` 성격의 반환과 direct model field 접근을 Go에서 owned snapshot으로 노출한다.
- 변경 반영은 explicit setter/native API 경계로만 일어나게 고정한다.

## References
- docs/STATE.md
- docs/ROADMAP.md
- docs/ARCHITECTURE.md
- docs/v2/designs/2026-04-03-v2-model-view-snapshot-copy.md
- docs/v2/designs/2026-04-02-v2-model-pointer-return.md
- docs/v2/designs/2026-04-03-v2-structure-field-accessors.md
- src/ir.rs
- src/generator.rs
- src/facade.rs

## Workspace
- Branch: feat/v2-model-view-snapshot-copy
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: model-view snapshot 정책에 맞게 IR 분류와 field accessor 대상을 확장한다.
- Depends on:
  - none
- Write Scope:
  - src/ir.rs
- Read Context:
  - docs/v2/designs/2026-04-03-v2-model-view-snapshot-copy.md
  - docs/v2/designs/2026-04-03-v2-structure-field-accessors.md
  - src/parser.rs
- Checks:
  - cargo test model
  - cargo test structure
- Parallel-safe: no

### Task T2
- Goal: snapshot-copy 반환을 raw wrapper에서 fresh owned handle로 생성한다.
- Depends on:
  - T1
- Write Scope:
  - src/generator.rs
  - 관련 raw generation tests
- Read Context:
  - src/ir.rs
  - docs/v2/designs/2026-04-03-v2-model-view-snapshot-copy.md
- Checks:
  - cargo test generator
  - cargo test model
- Parallel-safe: no

### Task T3
- Goal: Go facade/model 렌더링을 snapshot semantics에 맞게 정렬하고 field getter/setter를 노출한다.
- Depends on:
  - T2
- Write Scope:
  - src/facade.rs
  - facade/model generation tests
- Read Context:
  - src/generator.rs
  - docs/v2/designs/2026-04-03-v2-model-view-snapshot-copy.md
- Checks:
  - cargo test facade
  - cargo test model
- Parallel-safe: no

### Task T4
- Goal: snapshot copy와 explicit set write-back 회귀를 fixture/test로 고정하고 전체 검증을 마감한다.
- Depends on:
  - T3
- Write Scope:
  - tests/
  - examples/ (필요 시 fixture 보강만)
- Read Context:
  - src/ir.rs
  - src/generator.rs
  - src/facade.rs
- Checks:
  - cargo test
  - 필요 시 대표 fixture 재생성/compile smoke
- Parallel-safe: no

## Notes
- public Go API는 기존 `*Model` + `Close()` shape를 유지한다.
- borrowed `ModelView` 타입은 추가하지 않는다.
- copy constructible / copy assignable 전제가 깨지는 모델은 skip하고 이유를 남긴다.
- 현재 환경의 `libclang.dylib` 로더 문제 때문에 최종 검증 전에 테스트 런타임 환경을 먼저 맞춰야 할 수 있다.
