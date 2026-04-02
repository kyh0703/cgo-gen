---
feature: clang-arg-env-expansion
status: plan_ready
created_at: 2026-04-02T23:00:21+09:00
---

# Clang Arg Environment Variable Expansion

## Goal

`input.clang_args` 안에서 `$VAR`, `$(VAR)`, `${VAR}` 형태를 환경변수 참조로 인식하고, OS environment에서 값을 읽어 실제 clang argument 값으로 치환한다.

## Context / Inputs

- Source docs:
  - 사용자 요청: cflags 호출 시 env token 형태를 OS env에서 치환하고 싶음
- Existing system facts:
  - `src/config.rs:resolve_relative_clang_args()` 는 현재 `-I`, `-isystem`, `-I<path>`, `-isystem<path>` 형태의 경로만 config 파일 기준 상대경로로 정규화한다.
  - 현재 구현은 raw `clang_args` token 자체에 대한 환경변수 확장을 하지 않는다.
  - `resolve_relative_clang_path_arg()` 는 문자열을 바로 `Path`로 취급하므로 `$SDK_INC` 같은 값은 실제 env lookup 없이 경로 문자열로 남는다.
  - 현재 공개 README는 별도 프로젝트 전용 runtime env var layer가 없다고 설명하고 있다.
- User brief:
  - cflags/clang args 호출 시 `$환경변수`, `$(환경변수)`, `${환경변수}` 케이스를 환경변수로 인식
  - OS env에서 값을 읽어 실제 값으로 넣고 싶음

## Plan Handoff

### Scope for Planning

1. `src/config.rs` 에 env token expansion helper 추가
   - exact-match syntax만 지원:
     - `$VAR`
     - `$(VAR)`
     - `${VAR}`
   - env name을 추출해서 `std::env` 에서 조회
   - env가 없으면 config load 단계에서 명확한 에러 반환

2. `src/config.rs` 에서 `input.clang_args` 정규화 흐름에 env expansion 통합
   - standalone token이 env token이면 먼저 치환
   - `-I$VAR`, `-I${VAR}`, `-I$(VAR)` 형태도 value 부분에서 치환
   - `-isystem` + 다음 token 형태도 다음 token이 env token이면 치환
   - env 값이 상대경로이면 기존 규칙대로 config 파일 기준 상대경로 해석 후 canonicalize
   - env 값이 절대경로이면 그대로 사용

3. `tests/config.rs` 에 env expansion 회귀 테스트 추가
   - `$VAR`, `$(VAR)`, `${VAR}` 각각 성공 케이스
   - `-I<env-token>` inline prefix 케이스
   - missing env가 actionable error를 내는 케이스

4. 공개 문서 반영 필요 여부 판단
   - 구현이 들어가면 README의 `input.clang_args` 설명에 env token 지원을 추가
   - 기존 “project-specific runtime env var layer 없음” 문구는 clang arg expansion과 충돌하지 않도록 좁게 수정

### Success Criteria

- `input.clang_args` 에서 `$VAR`, `$(VAR)`, `${VAR}` 가 env reference로 인식된다
- `-I<env-token>` 와 `-isystem <env-token>` 경로가 정상 치환된다
- env 값이 상대경로면 기존 config-relative path 규칙을 그대로 따른다
- env가 없을 때 조용히 통과하지 않고 명확한 에러를 낸다
- 관련 config tests가 추가되고 통과한다

### Non-Goals

- shell 전체 문법 지원
- `${VAR:-default}` 같은 default/fallback syntax 지원
- 문자열 내부 일부 치환 (`prefix/$VAR/suffix`) 지원
- `input.headers`, `compile_commands`, 일반 output path까지 env expansion 범위 확대
- process execution 시점의 추가 env injection 레이어 도입

### Open Questions

- 없음

### Suggested Validation

- `cargo test --test config`
- `$VAR`, `$(VAR)`, `${VAR}` 각각에 대한 unit test
- `-I$VAR` inline include arg test
- missing env error message test

### Parallelization Hints

- Candidate write boundaries:
  - `src/config.rs`
  - `tests/config.rs`
  - 필요 시 `README.md`, `README.ko.md`
- Shared files to avoid touching in parallel:
  - `src/config.rs`
  - `tests/config.rs`
- Likely sequential dependencies:
  - env expansion helper 설계 → config path normalization 연결 → tests → README 정리
