# Rename Project Branding To cgo-gen

## Goal
- 현재 저장소의 package name, crate import, 공개 문서, 활성 문서의 프로젝트 표기를 `cgo-gen`으로 맞춘다.

## References
- docs/STATE.md
- docs/ROADMAP.md
- docs/ARCHITECTURE.md
- docs/v2/designs/2026-04-02-v2-rename-project-to-cgo-gen.md
- Cargo.toml
- README.md
- README.ko.md

## Workspace
- Branch: feat/v2-rename-project-to-cgo-gen
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: `Cargo.toml` package name을 `cgo-gen`으로 바꾸고 이에 맞는 Rust import/lockfile 정합성을 맞춘다.
- Depends on:
  - none
- Write Scope:
  - Cargo.toml
  - Cargo.lock
  - src/main.rs
  - tests/*.rs
- Read Context:
  - docs/v2/designs/2026-04-02-v2-rename-project-to-cgo-gen.md
  - Cargo.toml
- Checks:
  - cargo test --test config
  - cargo test --test facade_generate
- Parallel-safe: no

### Task T2
- Goal: 현재 공개 README와 examples 명령/설명을 `cgo-gen` 표기로 정리한다.
- Depends on:
  - T1
- Write Scope:
  - README.md
  - README.ko.md
  - examples/simple-go/README.md
  - examples/simple-go/Makefile
  - examples/simple-go-struct/Makefile
- Read Context:
  - Cargo.toml
  - docs/v2/designs/2026-04-02-v2-rename-project-to-cgo-gen.md
- Checks:
  - manual: README와 examples 명령이 새 이름과 일치함
- Parallel-safe: yes

### Task T3
- Goal: active architecture/product/status 문서의 현재 프로젝트 명칭을 `cgo-gen`으로 정리한다.
- Depends on:
  - T1
- Write Scope:
  - docs/ARCHITECTURE.md
  - docs/v2/designs/PRODUCT.md
  - 필요 시 현재 active design 문서 중 user-facing naming만 포함한 문서
- Read Context:
  - docs/v2/designs/2026-04-02-v2-rename-project-to-cgo-gen.md
  - README.md
- Checks:
  - manual: active docs에 `c-go` user-facing naming이 남지 않음
- Parallel-safe: yes

## Notes
- `docs/v2/completed/` 와 generated fixture snapshot은 historical record로 유지한다
- package rename 후 crate import는 `cgo_gen`으로 따라간다
- binary name이 실제로 바뀌면 README/examples 명령도 같은 이름으로 맞춘다
