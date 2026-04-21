# Stable Model Handle Normalization

## Goal
- typedef alias와 canonical struct tag가 섞인 model 타입이 생성기 전 구간에서 하나의 stable handle 이름으로 수렴되도록 정규화하고, Go facade가 기존 projection을 재사용하게 만든다.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `src/codegen/ir_norm.rs`
- `src/codegen/c_abi.rs`
- `src/analysis/model_analysis.rs`
- `src/codegen/go_facade.rs`
- `src/pipeline/context.rs`

## Workspace
- Branch: feat/v2-stable-model-handle-normalization
- Base: master
- Isolation: required
- Created by: manual closeout recovery for an already-isolated worktree

## Task Graph
### Task T1
- Goal: parsed API 전체에서 alias/canonical model 쌍을 수집하고, IR 정규화와 헤더별 pipeline context가 동일한 stable handle 이름을 재사용하게 만든다.
- Depends on:
  - none
- Write Scope:
  - `src/codegen/ir_norm.rs`
  - `src/codegen/c_abi.rs`
  - `src/pipeline/context.rs`
- Read Context:
  - `src/codegen/ir_norm.rs`
  - `src/codegen/c_abi.rs`
  - `src/pipeline/context.rs`
- Checks:
  - `cargo test --lib`
- Parallel-safe: no

### Task T2
- Goal: model projection과 facade class wrapper가 raw owner 이름으로 handle을 재계산하지 않고 IR에서 확정된 handle을 사용하며, 기존 projection이 있는 model은 Go facade에서 중복 opaque 선언을 만들지 않게 한다.
- Depends on:
  - T1
- Write Scope:
  - `src/analysis/model_analysis.rs`
  - `src/codegen/go_facade.rs`
- Read Context:
  - `src/analysis/model_analysis.rs`
  - `src/codegen/go_facade.rs`
- Checks:
  - `cargo test --lib`
- Parallel-safe: no

### Task T3
- Goal: alias-backed stable handle과 duplicate-opaque 회귀 테스트를 추가하고, 생성 산출물에 대한 최소 컴파일 스모크 검증까지 통과시킨다.
- Depends on:
  - T2
- Write Scope:
  - `src/codegen/ir_norm.rs`
  - `src/analysis/model_analysis.rs`
  - `src/codegen/go_facade.rs`
- Read Context:
  - `src/codegen/ir_norm.rs`
  - `src/analysis/model_analysis.rs`
  - `src/codegen/go_facade.rs`
- Checks:
  - `cargo test --lib`
  - `cargo test --test compile_smoke`
- Parallel-safe: no

## Notes
- 이 plan은 docs 누락 상태에서 이미 분리된 worktree 위에 구현이 진행된 케이스를 복구하기 위한 문서 엔트리다.
- 로컬 worktree에는 이 작업과 무관한 변경도 존재하므로, closeout과 PR에는 stable-handle-normalization 관련 파일만 선택적으로 반영한다.
