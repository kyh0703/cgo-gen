---
feature: char-array-string-normalization
status: plan_ready
created_at: 2026-04-06T12:00:00+09:00
---

# Char Array String Normalization

## Goal

`char[N]` 타입이 raw/model handle 타입으로 잘못 승격되지 않도록 막고, generated wrapper와 Go API에서 문자열 타입으로 일관되게 노출되게 만든다.

## Context / Inputs

- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
  - `docs/v2/designs/2026-04-03-v2-structure-field-accessors.md`
- Existing system facts:
  - 현재 `char[33]`, `char[11]`, `char[128]` 같은 배열 필드가 model handle 후보로 잘못 분류되어 `char[33]Handle` 같은 불법 식별자가 생성된다.
  - 이 오분류는 `gopkg/sil/i_sil_db_data_wrapper.go`의 `*char[33]`와 대응 C/C++ wrapper의 `char[33]Handle` 생성으로 이어져 실제 빌드를 깨뜨린다.
  - 기존 fixture에서는 내부 `char[]` 필드를 public wrapper에서 `const char*`로 노출하는 패턴이 이미 검증되어 있다.
- User brief:
  - `char` 배열은 문자열로 반환/전달되게 처리하고, 그 기준으로 계획을 세워 `exec-plan`까지 진행한다.

## Plan Handoff

### Scope for Planning

- `char[N]`와 다른 배열 타입이 현재 IR 정규화에서 어떤 kind로 분류되는지 재현 가능한 테스트로 고정한다.
- `char[N]`만 문자열 계열 타입으로 정규화하는 최소 수정 경로를 선택한다.
- model handle 후보 판정에서 `char[N]`가 빠지도록 방어 로직을 추가한다.
- generator가 `char[N]` getter/setter를 문자열 경로로 렌더링하는지 검증한다.
- 컴파일 또는 생성 결과 테스트로 `char[33]Handle` 같은 출력이 다시 나오지 않음을 확인한다.

### Success Criteria

- `char[N]`가 더 이상 `model_value`, `model_pointer`, `model_reference`로 승격되지 않는다.
- generated header/source/go에 `char[33]Handle` 같은 식별자가 생성되지 않는다.
- `char[N]` getter/setter가 문자열 타입 경로로 생성된다.
- 회귀 테스트가 추가되어 동일 문제가 다시 발생하면 테스트에서 즉시 드러난다.

### Non-Goals

- `uint32[8]` 등 일반 배열 전체를 이번 작업에서 지원한다.
- 익명 struct 배열/중첩 구조체 필드 전체를 이번 작업에서 설계 완료한다.
- 구조체 필드 accessor 전체 정책을 다시 설계하거나 대규모 리팩터링한다.

### Open Questions

- `char[N]`를 IR에서 직접 `c_string`으로 정규화할지, field accessor 전용 특례로 처리할지 구현 전에 경로를 비교해야 한다.
- setter에서 `const char*` 복사 로직을 기존 raw renderer가 그대로 수용하는지 확인이 필요하다.

### Suggested Validation

- `cargo test --test compile_smoke`
- `cargo test facade_generate`
- 필요 시 `cargo test ir -- --nocapture`
- generated output 문자열 확인: `char[33]Handle`, `char[11]Handle`, `char[128]Handle`가 없어야 한다.

### Parallelization Hints

- Candidate write boundaries:
  - `src/ir.rs`의 타입 정규화/handle 후보 판정
  - raw/go generator 경로의 문자열 렌더링 검증
  - 관련 Rust 테스트 보강
- Shared files to avoid touching in parallel:
  - `src/ir.rs`
  - 구조체 field accessor 생성 테스트 파일
- Likely sequential dependencies:
  - 먼저 IR 분류 테스트와 정책을 고정한 뒤 generator 출력 테스트를 보강하는 순서가 안전하다.
