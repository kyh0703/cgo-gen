# Timeval Support

## Goal
- `timeval` 관련 외부 C struct 포인터/레퍼런스가 Go facade에서 깨지지 않도록 구현하고 검증한다.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-02-v2-timeval-support.md`

## Workspace
- Branch: feat/v2-timeval-support
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: `struct timeval*` / `struct timeval&`를 외부 C struct 포인터/레퍼런스로 정규화하고 Go facade에서 `*C.struct_timeval`로 안전하게 렌더링한다.
- Depends on:
  - none
- Write Scope:
  - `src/ir.rs`
  - `src/facade.rs`
- Read Context:
  - `docs/v2/designs/2026-04-02-v2-timeval-support.md`
  - 기존 pointer/reference/model fallback 로직
- Checks:
  - `cargo test timeval`
  - 필요 시 관련 테스트만 직접 지정 실행
- Parallel-safe: no

### Task T2
- Goal: alias/canonical 케이스와 생성 코드 문자열을 고정하는 회귀 테스트를 추가하고 전체 테스트로 마감한다.
- Depends on:
  - T1
- Write Scope:
  - `tests/`
- Read Context:
  - `src/ir.rs`
  - `src/facade.rs`
- Checks:
  - `cargo test`
- Parallel-safe: no

## Notes
- 구현은 `timeval` 증상을 기준으로 하되, 매칭 규칙은 `struct <name>` 외부 C struct pointer/reference에 재사용 가능하게 둔다.
