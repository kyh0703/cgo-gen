# Go False Overload Suffix Detection For Smmanager Bool Field Setters

## Goal
- non-overloaded underscore-backed bool field setters가 generated Go facade에서 `SetBModifyFlagBool`이 아니라 `SetBModifyFlag`로 렌더링되게 고치고 회귀 검증을 추가한다.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-09-v2-go-false-overload-suffix-detection-for-smmanager-bool-field-setters.md`

## Workspace
- Branch: feat/v2-go-false-overload-suffix-detection-for-smmanager-bool-field-setters
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: Go facade overload 판정이 실제 overload-disambiguated raw symbol에만 suffix를 붙이도록 좁혀서, underscore-backed non-overloaded setter가 `Bool` suffix를 받지 않게 한다.
- Depends on:
  - none
- Write Scope:
  - `src/codegen/go_facade.rs`
- Read Context:
  - `docs/v2/designs/2026-04-09-v2-go-false-overload-suffix-detection-for-smmanager-bool-field-setters.md`
  - `src/codegen/ir_norm.rs`
  - `smmanager/public_wrapper.go`
- Checks:
  - `cargo test go_facade`
- Parallel-safe: no

### Task T2
- Goal: non-overloaded underscore-backed bool setter와 실제 overload 유지 케이스를 고정하는 회귀 테스트를 추가하고 전체 생성 경로를 재검증한다.
- Depends on:
  - T1
- Write Scope:
  - `tests/generator.rs`
- Read Context:
  - `src/codegen/go_facade.rs`
  - existing generator fixture tests
- Checks:
  - `cargo test generator`
  - `cargo test`
- Parallel-safe: no

### Task T3
- Goal: representative generated output를 다시 확인해서 `smmanager/public_wrapper.go`의 known `SetBModifyFlagBool` sites가 `SetBModifyFlag`로 바뀌었는지 검증한다.
- Depends on:
  - T2
- Write Scope:
  - `smmanager/`
- Read Context:
  - `smmanager/public_wrapper.go`
  - local generation command/config
- Checks:
  - `rg -n "SetBModifyFlagBool|SetBModifyFlag\\(" smmanager/public_wrapper.go`
- Parallel-safe: no

## Notes
- 변경 범위는 Go export naming detection에 한정한다.
- raw C symbol naming과 parser access rules는 이번 slice에서 건드리지 않는다.
- exact regeneration command가 저장소에 고정돼 있지 않으면 가능한 로컬 재생성 경로를 사용하고, 재현 제약이 있으면 결과에 명시한다.
