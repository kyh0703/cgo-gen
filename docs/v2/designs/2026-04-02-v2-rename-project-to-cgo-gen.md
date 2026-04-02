---
feature: rename-project-to-cgo-gen
status: plan_ready
created_at: 2026-04-02T23:40:23+09:00
---

# Rename Project Branding To cgo-gen

## Goal

현재 저장소에 남아 있는 `c-go` 프로젝트 표기를 `cgo-gen`으로 정리하고, `Cargo.toml` 패키지명 변경까지 반영해서 문서와 빌드 메타데이터를 일치시킨다.

## Context / Inputs

- Source docs:
  - 사용자 요청: `c-go`로 되어 있는 문서 내용과 `Cargo.toml`을 `cgo-gen`으로 바꾸고 싶음
- Existing system facts:
  - `Cargo.toml`의 패키지명은 아직 `c-go`다.
  - Rust 패키지명이 바뀌면 crate import 이름도 `c_go` -> `cgo_gen`으로 따라 바뀐다.
  - 현재 `src/main.rs`, 다수의 `tests/*.rs`, README, examples Makefile/README, 일부 active docs에 `c-go` 표기가 남아 있다.
  - `docs/v2/completed/` 아래의 archived 문서는 과거 작업 기록이라 historical wording과 절대경로를 많이 포함한다.
  - `tests/fixtures/...expected...yaml` 같은 generated snapshot에도 과거 경로 문자열이 일부 남아 있다.
- User brief:
  - `c-go`로 되어 있는 문서 내용과 `Cargo.toml`을 `cgo-gen`으로 바꿔 달라

## Plan Handoff

### Scope for Planning

1. 빌드/코드 메타데이터 rename
   - `Cargo.toml` 패키지명을 `cgo-gen`으로 변경
   - 필요하면 `Cargo.lock`의 package entry 갱신
   - `src/main.rs` 및 모든 Rust test/source import를 `c_go` -> `cgo_gen`으로 갱신

2. 현재 사용자-facing 문서와 예제 표기 정리
   - `README.md`, `README.ko.md`, `LICENSE` copyright line
   - `examples/simple-go/README.md`
   - `examples/simple-go/Makefile`
   - `examples/simple-go-struct/Makefile`
   - 현재 active docs에서 `c-go` project naming이 남은 문서 (`docs/ARCHITECTURE.md`, `docs/v2/designs/PRODUCT.md`, `docs/v2/research/status/sil-conversion-status.md` 등)

3. rename에 따른 실행 명령 표기 정리
   - `cargo run --bin c-go` 같은 명령이 계속 유효한지 확인
   - 패키지명 변경 후 실제 바이너리명이 `cgo-gen`으로 바뀌면 문서/Makefile을 함께 갱신

4. archived history는 보존
   - `docs/v2/completed/` 과 오래된 absolute path transcript는 원칙적으로 수정하지 않음
   - generated fixture snapshot 경로 문자열도 rename 목적만으로는 수정하지 않음

### Success Criteria

- `Cargo.toml` package name이 `cgo-gen`으로 바뀐다
- Rust code/tests가 새 crate import 이름으로 빌드된다
- 현재 공개 문서와 active docs의 주요 프로젝트 표기가 `cgo-gen`으로 정리된다
- examples와 실행 명령 표기가 실제 바이너리/패키지명과 일치한다
- 최소한 Rust compile/test 경로가 rename 후 깨지지 않는다

### Non-Goals

- `docs/v2/completed/` 아래의 archived historical wording 일괄 수정
- fixture snapshot 안의 과거 경로 문자열 전면 치환
- repository directory 이름(`/Users/.../cgo-gen`) 자체 변경
- 생성 wrapper output naming 정책 변경

### Open Questions

- 없음

### Suggested Validation

- `cargo test --test config`
- `cargo test --test facade_generate`
- 필요 시 `cargo test --no-run` 또는 `cargo test`로 crate rename 후 compile 확인
- manual: README와 examples 명령이 새 이름과 일치하는지 확인

### Parallelization Hints

- Candidate write boundaries:
  - `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `tests/**/*.rs`
  - `README.md`, `README.ko.md`, `examples/**`
  - `docs/ARCHITECTURE.md`, `docs/v2/designs/PRODUCT.md`, `docs/v2/research/status/sil-conversion-status.md`
- Shared files to avoid touching in parallel:
  - `Cargo.toml`
  - `Cargo.lock`
  - `README.md`
  - `README.ko.md`
- Likely sequential dependencies:
  - package rename + import fix → validation → docs/examples wording 정리
