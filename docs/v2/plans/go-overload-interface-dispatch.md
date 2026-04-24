# Go Overload Interface Dispatch

## Goal
- SWIG-like `args ...interface{}` overload dispatcher를 Go facade에 도입할지 검토하고, 현재 suffix 기반 API와 병행/대체 여부를 결정 가능한 문서로 남긴다.

## References
- docs/STATE.md
- docs/ROADMAP.md
- docs/ARCHITECTURE.md
- docs/v2/designs/2026-04-24-v2-go-overload-interface-dispatch.md
- docs/v2/completed/go-facade-overloaded-constructors.md
- docs/v2/completed/go-false-overload-suffix-detection-for-underscore-bool-field-setters.md
- src/codegen/ir_norm.rs
- src/codegen/go_facade.rs
- tests/overload_collisions.rs

## Workspace
- Branch: feat/v2-go-overload-interface-dispatch
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: 현재 overload suffix 생성과 Go facade export naming 경로를 확인하고, dispatcher로 바꿀 때 깨지는 타입/호출 케이스를 분류한다.
- Depends on:
  - none
- Write Scope:
  - docs/v2/research/go-overload-interface-dispatch.md
- Read Context:
  - docs/v2/designs/2026-04-24-v2-go-overload-interface-dispatch.md
  - src/codegen/ir_norm.rs
  - src/codegen/go_facade.rs
  - tests/overload_collisions.rs
- Checks:
  - rg -n "go_overload_suffix|has_disambiguated_raw_overload_suffix|go_method_export_name|go_facade_export_name" src/codegen/go_facade.rs
- Parallel-safe: no

### Task T2
- Goal: Go-facing API recommendation을 확정하고, 채택한다면 첫 구현 slice의 안전한 범위와 검증 조건을 적는다.
- Depends on:
  - T1
- Write Scope:
  - docs/v2/research/go-overload-interface-dispatch.md
- Read Context:
  - docs/v2/research/go-overload-interface-dispatch.md
  - docs/ARCHITECTURE.md
- Checks:
  - rg -n "Recommendation|Ambiguous|First implementation slice" docs/v2/research/go-overload-interface-dispatch.md
- Parallel-safe: no

## Notes
- 이 plan은 검토/의사결정 slice다. 코드 생성 변경은 recommendation이 확정된 뒤 별도 feature로 나누는 것을 기본값으로 둔다.
- 첫 검토 결론은 기존 suffixed typed exports를 제거하지 않는 방향을 우선 검증한다.
