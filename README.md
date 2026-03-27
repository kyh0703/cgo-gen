# c-go

`c-go`는 보수적인 C/C++ 헤더 subset을 입력으로 받아 C ABI wrapper와 관련 산출물을 생성하는 Rust CLI입니다.

문서 시작점은 `AGENTS.md`입니다. 핵심 문서는 다음 순서로 보면 됩니다.

1. `AGENTS.md`
2. `docs/STATE.md`
3. `docs/ROADMAP.md`
4. `docs/ARCHITECTURE.md`

현재 작업 문서 구조는 `docs/v2/` 아래로 정리되어 있습니다.

- 설계 문서: `docs/v2/designs/`
- 리서치/상태/레퍼런스: `docs/v2/research/`
- 완료된 작업 기록: `docs/v2/completed/`
- 활성 plan: 없음

## 주요 명령

```bash
cargo run --bin c-go -- check --config path/to/wrapper.yaml
cargo run --bin c-go -- ir --config path/to/wrapper.yaml --format yaml
cargo run --bin c-go -- generate --config path/to/wrapper.yaml --dump-ir
```

## 현재 상태

- 문서 구조는 `v2` 기준으로 정리되었습니다.
- 기존 레거시 문서 경로는 제거되었습니다.
- 이전 작업 기록은 모두 아카이브로 간주합니다.
- 새 작업은 빈 상태에서 새 plan으로 다시 시작합니다.

## 참고

- 예시 설정: `configs/sil-wrapper.example.yaml`
- 예시 프로젝트: `examples/simple-cpp`, `examples/simple-go`, `examples/simple-go-struct`
