# AGENTS

이 저장소의 문서 목차와 작업 진입점은 이 파일을 기준으로 봅니다.

## 문서 원칙

- 루트의 지속 문서는 `docs/STATE.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`만 유지합니다.
- 버전 문서는 현재 버전인 `docs/v2/` 아래에서만 관리합니다.
- 새 문서를 추가할 때 레거시 경로(`docs/design-docs`, `docs/exec-plans`, `docs/references`, `docs/roadmaps`, `docs/status`)를 다시 만들지 않습니다.

## 읽는 순서

1. `docs/STATE.md`
2. `docs/ROADMAP.md`
3. `docs/ARCHITECTURE.md`
4. `docs/v2/plans/`
5. `docs/v2/designs/`
6. `docs/v2/research/`
7. `docs/v2/completed/`

## 현재 기준 경로

- 현재 버전: `v2`
- 활성 plan: 없음
- 설계 문서: `docs/v2/designs/`
- 리서치/상태/레퍼런스: `docs/v2/research/`
- 완료된 plan 기록: `docs/v2/completed/`
- 새 작업 시작 상태: reset-ready

## 문서 변경 규칙

- 구조를 바꾸면 `AGENTS.md`, `docs/STATE.md`, `docs/ROADMAP.md`를 함께 갱신합니다.
- 활성 작업 문서는 `docs/v2/plans/`에 둡니다.
- 종료된 작업 문서는 `docs/v2/completed/`로 둡니다.
- 제품/기획 성격 문서는 `docs/v2/designs/`에 둡니다.
- 참고 자료, 상태 기록, 매핑 노트는 `docs/v2/research/`에 둡니다.
- 처음부터 다시 시작할 때는 기존 활성 문서를 모두 완료 처리한 뒤 새 plan을 만듭니다.
