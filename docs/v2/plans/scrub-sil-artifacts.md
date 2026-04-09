# Scrub SIL Artifacts

## Goal
- 저장소에서 체크인된 SIL/company-source 관련 설정, 테스트 문자열, 문서 흔적을 제거하고 generic한 회귀 커버리지만 남긴다.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-08-v2-scrub-sil-artifacts.md`

## Workspace
- Branch: `feat/v2-scrub-sil-artifacts`
- Base: `master`
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: 체크인된 SIL example config, 그 직접 참조 테스트, 그리고 generic하게 유지할 필요가 있는 SIL-prefixed 테스트 literal을 삭제 또는 중립 이름으로 치환한다.
- Depends on:
  - none
- Write Scope:
  - `configs/`
  - `tests/config.rs`
  - `tests/facade_generate.rs`
  - 필요 시 관련 테스트 파일
- Read Context:
  - `docs/v2/designs/2026-04-08-v2-scrub-sil-artifacts.md`
  - `src/config.rs`
  - `src/cli.rs`
- Checks:
  - `cargo test --test config`
  - `cargo test facade_generate`
- Parallel-safe: no

### Task T2
- Goal: 저장소 문서 전반에서 real-SIL onboarding, internal SIL header/type 이름, SIL-specific output path, local validation 흔적을 scrub하고 generic 설명으로 정리한다.
- Depends on:
  - T1
- Write Scope:
  - `docs/ARCHITECTURE.md`
  - `docs/v2/designs/`
  - `docs/v2/research/`
  - `docs/v2/completed/`
  - 필요 시 `README.md`
  - 필요 시 `README.ko.md`
- Read Context:
  - `docs/v2/designs/2026-04-08-v2-scrub-sil-artifacts.md`
  - `AGENTS.md`
  - `README.md`
  - `README.ko.md`
- Checks:
  - `rg -n -i "sil|iSiLib|IsAAMaster|IsCluster|IsCSTASession|SetHACallback|HACallback" .`
  - `cargo test`
- Parallel-safe: no

## Notes
- 목표는 historical company-source trace scrub이며, generator 기능 확장은 금지한다.
- 검색 결과에서 generic English 단어 일부가 우연히 `sil`을 포함할 수 있으므로 최종 검토에서는 실제 company-source 흔적만 남지 않았는지 문맥으로 확인한다.
