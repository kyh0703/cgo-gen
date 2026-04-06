# Char Array String Normalization

## Goal
- `char[N]` 배열 필드가 모델 핸들로 잘못 승격되지 않도록 막고, generated C/C++/Go wrapper에서 문자열 타입으로 안정적으로 노출되게 만든다.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-06-v2-char-array-string-normalization.md`
- `docs/v2/designs/2026-04-03-v2-structure-field-accessors.md`

## Workspace
- Branch: feat/v2-char-array-string-normalization
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: `char[N]`가 IR 정규화와 model handle 후보 판정에서 문자열 타입으로 분류되도록 최소 수정하고, 배열 기반 불법 식별자 생성 경로를 차단한다.
- Depends on:
  - none
- Write Scope:
  - `src/ir.rs`
  - 필요 시 `src/facade.rs`
  - 필요 시 `src/generator.rs`
- Read Context:
  - `docs/v2/designs/2026-04-06-v2-char-array-string-normalization.md`
  - `docs/v2/designs/2026-04-03-v2-structure-field-accessors.md`
  - 기존 string/c_string 렌더링 경로
- Checks:
  - `cargo test facade_generate`
  - `cargo test generator`
- Parallel-safe: no

### Task T2
- Goal: `char[N]` 회귀를 고정하는 테스트를 추가하고, 생성 결과와 컴파일 경로에서 `char[33]Handle` 같은 출력이 재발하지 않음을 검증한다.
- Depends on:
  - T1
- Write Scope:
  - `tests/compile_smoke.rs`
  - 필요 시 `tests/facade_generate.rs`
  - 필요 시 관련 테스트 파일
- Read Context:
  - `src/ir.rs`
  - `src/facade.rs`
  - `src/generator.rs`
- Checks:
  - `cargo test compile_smoke`
  - `cargo test`
- Parallel-safe: no

## Notes
- 이번 작업은 `char[N]` 문자열 정규화만 다룬다.
- `uint32[8]` 같은 일반 배열과 익명 struct 배열은 비목표로 남기고, 이번 수정에서 묵시적으로 지원하지 않는다.
