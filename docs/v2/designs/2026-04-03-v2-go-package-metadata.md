---
feature: go-package-metadata
status: plan_ready
created_at: 2026-04-03T10:30:00+09:00
---

# Go Package Metadata

## Goal

`generate --go-module <module-path>` opt-in 모드에서 외부 import 가능한 Go package metadata를 함께 생성한다.

## Context / Inputs

- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - 생성물은 `*_wrapper.go`, `*_wrapper.h`, `*_wrapper.cpp`를 `output.dir` 아래에 함께 쓴다.
  - 현재 생성된 Go 파일은 `#cgo` 플래그와 `go.mod`를 자동 생성하지 않는다.
  - `Config::load()` 이후 `input.clang_args`는 env expansion과 path canonicalization이 적용된다.
- User brief:
  - 외부 소비용 package metadata는 `input.include_dirs`를 참조하지 않는다.
  - exported compile flags의 source of truth는 raw `input.clang_args`만 사용한다.
  - authored `clang_args` spellings는 가능한 한 그대로 보존한다.
  - exported flags는 `-I`, `-D`, `-std`만 허용한다.
  - `libclang` / LLVM 및 링크 관련 플래그는 제외한다.
  - `go.mod`는 CLI의 `--go-module` 값이 있을 때만 생성하고 `go 1.25`를 쓴다.

## Plan Handoff

### Scope for Planning

- `generate` CLI에 `--go-module <module-path>`를 추가한다.
- raw YAML 기준 `input.clang_args`를 읽어 exported metadata용 토큰을 추출한다.
- `output.dir` 아래에 `build_flags.go`와 `go.mod`를 opt-in으로 생성한다.
- exported metadata용 flag filtering 정책과 관련 테스트를 추가한다.
- README에 새 opt-in 동작과 flag export 범위를 반영한다.

### Success Criteria

- `generate` without `--go-module`는 기존 출력과 동작을 유지한다.
- `generate --go-module <module-path>`는 `output.dir` 아래에 `go.mod`와 `build_flags.go`를 생성한다.
- `build_flags.go`는 항상 `#cgo CFLAGS: -I${SRCDIR}`를 포함한다.
- `build_flags.go`의 `#cgo CXXFLAGS`는 raw `input.clang_args`에서 allowlist된 토큰만 포함한다.
- `input.include_dirs`는 exported metadata 생성에 영향을 주지 않는다.
- `go.mod`는 `module <module-path>`와 `go 1.25`를 포함한다.

### Non-Goals

- `#cgo LDFLAGS` 자동 생성
- `compile_commands.json` 기반 flag export
- `libclang` / LLVM 관련 플래그 노출
- root workspace에서 직접 구현 시작

### Open Questions

- 없음

### Suggested Validation

- `cargo test config`
- `cargo test generator`
- `cargo test compile_smoke`
- `cargo test example_simple_go_struct`

### Parallelization Hints

- Candidate write boundaries:
  - CLI option parsing
  - raw config token extraction and metadata rendering
  - tests and README
- Shared files to avoid touching in parallel:
  - `src/config.rs`
  - `src/cli.rs`
  - 생성 경로 helper
- Likely sequential dependencies:
  - CLI option과 metadata policy 고정
  - metadata renderer 구현
  - tests와 README 반영
