# Go Package Metadata

## Goal
- `generate --go-module <module-path>` opt-in 모드에서 외부 import 가능한 Go package metadata를 생성한다.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-03-v2-go-package-metadata.md`
- `src/config.rs`
- `src/cli.rs`
- `src/generator.rs`
- `src/facade.rs`

## Workspace
- Branch: feat/v2-go-package-metadata
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: `generate --go-module <module-path>` CLI 옵션과 metadata generation 진입 경로를 추가하고, raw `input.clang_args` authored tokens를 읽을 수 있는 config surface를 만든다.
- Depends on:
  - none
- Write Scope:
  - `src/cli.rs`
  - `src/config.rs`
  - `src/main.rs`
  - `tests/config.rs`
- Read Context:
  - `docs/v2/designs/2026-04-03-v2-go-package-metadata.md`
  - `src/config.rs`
  - `src/cli.rs`
- Checks:
  - `cargo test config`
- Parallel-safe: no

### Task T2
- Goal: exported metadata renderer를 추가해 `build_flags.go`와 `go.mod`를 opt-in으로 생성하고, allowlist/exclude 정책을 구현한다.
- Depends on:
  - T1
- Write Scope:
  - `src/generator.rs`
  - `src/config.rs`
  - `src/facade.rs`
  - `tests/generator.rs`
  - `tests/compile_smoke.rs`
- Read Context:
  - `docs/v2/designs/2026-04-03-v2-go-package-metadata.md`
  - `src/generator.rs`
  - `src/facade.rs`
- Checks:
  - `cargo test generator`
  - `cargo test compile_smoke`
- Parallel-safe: no

### Task T3
- Goal: example/README와 end-to-end 검증을 갱신해 새 opt-in metadata generation contract를 문서화하고 회귀를 막는다.
- Depends on:
  - T2
- Write Scope:
  - `README.md`
  - `README.ko.md`
  - `tests/example_simple_go_struct.rs`
- Read Context:
  - `docs/v2/designs/2026-04-03-v2-go-package-metadata.md`
  - `examples/simple-go-struct`
  - `README.md`
- Checks:
  - `cargo test example_simple_go_struct`
  - `cargo test`
- Parallel-safe: no

## Notes
- exported metadata의 source of truth는 raw `input.clang_args`만 사용한다.
- `input.include_dirs`는 exported metadata 생성에서 무시한다.
- `build_flags.go`는 `CFLAGS: -I${SRCDIR}`를 항상 포함하고 `CXXFLAGS`에 allowlist subset만 쓴다.
- `LDFLAGS`는 이 plan 범위에서 자동 생성하지 않는다.
- `go.mod`는 `--go-module` 값이 있을 때만 생성하고 `go 1.25`를 고정한다.
