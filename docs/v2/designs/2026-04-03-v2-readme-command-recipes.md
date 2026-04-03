---
feature: readme-command-recipes
status: plan_ready
created_at: 2026-04-03T19:10:00+09:00
---

# README Command Recipes

## Goal

`rust-bindgen`의 command line usage 문서처럼, 이 저장소 README에서 자주 쓰는 CLI 흐름을 `bash` 블록으로 바로 복붙할 수 있게 정리한다.

## Context / Inputs

- Source docs:
  - `https://github.com/rust-lang/rust-bindgen`
  - `https://rust-lang.github.io/rust-bindgen/command-line-usage.html`
- Existing system facts:
  - 현재 `README.md`와 `README.ko.md`에는 설치와 quick start 예시는 있지만, 자주 쓰는 명령을 목적별로 모아둔 전용 섹션은 없다.
  - 사용자 요청은 "지금까지 명령어들 쓸 수 있게 `bash` 안에 명령어 모음"을 넣는 것이다.
- User brief:
  - `rust-bindgen` 문서를 참조해서 이 프로젝트에도 복붙 가능한 명령어 모음을 넣고 싶다.

## Plan Handoff

### Scope for Planning

- `README.md`와 `README.ko.md`의 현재 CLI/quick start 구조를 검토한다.
- `rust-bindgen` 문서 스타일을 참고해 설치, 확인, IR 출력, 생성, 예제 실행, 외부 Go 모듈 생성까지 이어지는 명령 세트를 정리한다.
- 각 README에 별도 명령어 섹션을 추가하고, 기존 내용과 충돌하지 않게 재배치하거나 최소한으로 연결 문구를 손본다.

### Success Criteria

- `README.md`에 `bash` fenced block 기반의 command recipes 섹션이 추가된다.
- `README.ko.md`에도 같은 수준의 command recipes 섹션이 추가된다.
- 명령 예시는 현재 공개 CLI와 일치하고, `generate --go-module`을 포함한 최신 흐름을 반영한다.
- 문서 변경만으로 끝나며, 코드나 설정 파일 동작은 바꾸지 않는다.

### Non-Goals

- CLI 동작 변경
- 새 예제 프로젝트 추가
- README 외의 장문 튜토리얼 문서 신설

### Open Questions

- 없음. README 양언어 문서에 동일 구조로 넣는 것으로 고정한다.

### Suggested Validation

- `README.md`, `README.ko.md`에서 추가한 명령이 현재 CLI와 예제 경로와 맞는지 확인
- `cargo run --bin cgo-gen -- --help`
- `cargo run --bin cgo-gen -- generate --help`

### Parallelization Hints

- Candidate write boundaries:
  - `README.md`
  - `README.ko.md`
- Shared files to avoid touching in parallel:
  - 없음. 두 README는 분리 가능
- Likely sequential dependencies:
  - 영어 README 구조를 먼저 확정한 뒤 한글 README를 같은 구조로 맞추는 편이 안전하다.
