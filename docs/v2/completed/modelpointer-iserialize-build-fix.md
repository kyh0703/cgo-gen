# ModelPointer iSerialize Build Fix

## Goal
- `ModelPointer` 추가 이후 `iSerialize` wrapper 생성 결과가 깨진 원인을 재현하고, 최소 수정으로 다시 빌드 가능한 상태로 복구한다.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-03-v2-modelpointer-iserialize-build-fix.md`
- `docs/v2/designs/2026-04-02-v2-model-pointer-return.md`

## Workspace
- Branch: feat/v2-modelpointer-iserialize-build-fix
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: `iSerialize` wrapper 실패를 재현하고, `ModelPointer` 이후 변경 중 enum/primitive alias 처리와 연결된 원인 경로를 코드 수준에서 식별한다.
- Depends on:
  - none
- Write Scope:
  - `tests/...`
  - 필요 시 재현 fixture 경로 1곳
- Read Context:
  - `docs/v2/designs/2026-04-03-v2-modelpointer-iserialize-build-fix.md`
  - `src/...`
  - `tests/...`
- Checks:
  - `cargo test`
  - 재현 가능한 경우 해당 fixture/golden 검증
- Parallel-safe: no

### Task T2
- Goal: 식별된 생성기 경로만 수정해 `eSeriType` 중복 정의와 `uint64` 오버로드 선택 실패를 막고, 회귀 검증을 통과시킨다.
- Depends on:
  - T1
- Write Scope:
  - `src/...`
  - `tests/...`
- Read Context:
  - `src/...`
  - `tests/...`
  - T1 재현 결과
- Checks:
  - `cargo test`
  - 가능하면 `iSerialize` 생성 결과 확인
- Parallel-safe: no

## Notes
- 루트 workspace에서는 문서만 갱신하고 구현은 `.worktrees/modelpointer-iserialize-build-fix`에서만 진행한다.
- 수정 범위는 빌드 실패를 직접 유발한 생성 규칙으로 한정한다.
