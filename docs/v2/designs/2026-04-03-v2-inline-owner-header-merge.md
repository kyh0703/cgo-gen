---
feature: inline-owner-header-merge
status: plan_ready
created_at: 2026-04-03T16:10:00+09:00
---

# Inline Owner Header Merge

## Goal

`*-inl.h` 입력을 독립 wrapper 산출물로 생성하지 않고, 소유 facade/model 헤더의 메서드 구현으로 귀속시켜 실제 SIL의 inline 확장 패턴을 깨지 않도록 만든다.

## Context / Inputs
- Source docs:
  - `docs/STATE.md`
  - `docs/ARCHITECTURE.md`
  - `docs/v2/research/status/sil-conversion-status.md`
  - `docs/v2/designs/2026-03-26-v2-real-sil-bulk-onboarding.md`
- Existing system facts:
  - 실제 `D:/Project/IPRON/IE/SIL/iSiLib.h` 는 하단에서 `iSiLib-inl.h` 를 include 한다.
  - 실제 `D:/Project/IPRON/IE/SIL/iSiLib-inl.h` 는 `inline bool iSiLib::...` 형태의 클래스 외부 정의를 가진 구현 확장 헤더다.
  - 현재 생성 결과에는 `gopkg/sil/i_si_lib-inl_wrapper.cpp` 가 따로 생기며 `#include "iSiLib-inl.h"` 만 포함해 standalone compile 경로를 만든다.
  - 이 경로에서 `iSiLib`, `int32`, `IsNodeTenantAlarm` 같은 선언 전제가 깨져 실제 리눅스 빌드 오류가 발생한다.
- User brief:
  - 실제 리눅스 환경에서 생성한 `gopkg` 기준으로 `iSiLib-inl.h` 관련 빌드 오류를 조사했고, `-inl.h` 도 생성은 필요하지만 독립 wrapper 가 아니라 부모 헤더에 합쳐지는 방향이 맞다고 확인했다.

## Plan Handoff
### Scope for Planning
- 현재 생성기가 `-inl.h` 입력을 별도 출력 단위로 취급하는 경로를 찾는다.
- `inline bool Class::Method(...)` 같은 owner-qualified 정의를 parent header/class 선언에 귀속시키는 최소 규칙을 추가한다.
- `iSiLib-inl.h` 가 별도 `*_wrapper.*` 산출물을 만들지 않고 `iSiLib.h` wrapper 결과에 흡수되도록 출력 단위를 정규화한다.
- 가능하면 회귀 테스트 또는 fixture 검증으로 `-inl.h` 분리 산출물이 사라지고 owner wrapper 에 메서드가 유지되는지 확인한다.

### Success Criteria
- `iSiLib-inl.h` 입력이 있어도 `i_si_lib-inl_wrapper.*` 같은 독립 산출물이 생성되지 않는다.
- `iSiLib` 관련 inline 메서드는 `i_si_lib_wrapper.*` 쪽에서 계속 생성된다.
- 실제 리눅스 오류 패턴인 `iSiLib has not been declared`, `IsNodeTenantAlarm was not declared` 를 standalone inl wrapper 경로에서 더 이상 유발하지 않는다.

### Non-Goals
- 모든 inline/include 체계 일반화
- upstream SIL 헤더 자체 수정
- 요청되지 않은 facade/model 분류 규칙 확장

### Open Questions
- owner 귀속 판정에 파일명 규칙만으로 충분한지, 아니면 AST owner 정보가 이미 있는지 worktree 에서 확인이 필요하다.
- 기존 fixture 중 `-inl.h` 별도 산출물을 기대하는 경우가 있는지 확인이 필요하다.

### Suggested Validation
- 관련 Rust 테스트 실행
- 가능한 경우 `iSiLib.h` + `iSiLib-inl.h` 입력 fixture 또는 실제 SIL 입력으로 generate 검증
- 생성 결과에서 `*_inl_wrapper` 산출물 부재와 owner wrapper 메서드 존재 확인

### Parallelization Hints
- Candidate write boundaries:
  - `src/` 의 input classification / owner resolution / emit grouping 로직
  - `tests/` 의 회귀 검증
- Shared files to avoid touching in parallel:
  - 공용 generation pipeline 파일
  - 관련 fixture expectations
- Likely sequential dependencies:
  - owner resolution 경로 확인 후 emit grouping 수정, 그 다음 테스트 보강
