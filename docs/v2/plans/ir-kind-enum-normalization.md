# IR Kind Enum Normalization

## Goal
- `IrType.kind`, `IrFunction.kind`를 내부 enum으로 바꾸고 외부 IR dump 문자열은 유지한다.
- 첫 단계에서는 생성된 C/Go 출력 변화 없이 내부 개념 경계만 정리한다.

## References
- docs/STATE.md
- docs/ARCHITECTURE.md
- src/ir.rs
- src/facade.rs
- src/generator.rs

## Workspace
- Branch: feat/v2-ir-kind-enum-normalization
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph

### Task T1
- Goal: `IrTypeKind`, `IrFunctionKind` enum 도입과 serde 문자열 호환 유지
- Depends on: none
- Write Scope:
  - src/ir.rs
- Checks:
  - cargo test ir
- Parallel-safe: no

### Task T2
- Goal: `src/ir.rs` 내부 분기를 enum match로 전환하고 기존 normalize/naming/support 동작 유지
- Depends on: T1
- Write Scope:
  - src/ir.rs
- Checks:
  - cargo test ir
  - cargo test compile_smoke --test compile_smoke
- Parallel-safe: no

### Task T3
- Goal: `src/facade.rs`, `src/generator.rs`의 `kind` 사용부를 enum 기반으로 전환
- Depends on: T2
- Write Scope:
  - src/facade.rs
  - src/generator.rs
- Checks:
  - cargo test facade_generate --test facade_generate
  - cargo test generator --test generator
- Parallel-safe: no

### Task T4
- Goal: 외부 IR 문자열 호환과 전체 회귀를 테스트로 고정
- Depends on: T3
- Write Scope:
  - src/ir.rs
  - tests/compile_smoke.rs
  - tests/generator.rs
- Checks:
  - cargo test
- Parallel-safe: no

## Notes
- 이번 단계에서는 `Config` 구조를 바꾸지 않는다.
- `known model` 정책, `char[N]` 문자열 정책 같은 의미 변경은 이번 범위 밖이다.
- IR YAML/JSON의 `kind` 문자열 값은 기존과 동일해야 한다.
