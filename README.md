# c-go

`c-go` is a Rust CLI that parses a conservative subset of C/C++ headers and generates C ABI wrapper artifacts that can be consumed from Go through cgo.

이 프로젝트는 보수적인 범위의 C/C++ 헤더를 입력으로 받아, Go에서 cgo로 사용할 수 있는 C ABI wrapper 산출물을 생성하는 Rust CLI입니다.

## Korean

### 프로젝트 개요

`c-go`의 목적은 "C++ API를 그대로 직접 바인딩"하는 것이 아니라, 비교적 안전하게 다룰 수 있는 헤더 표면만 골라 안정적인 C ABI 레이어와 Go 래퍼 출력을 만드는 것입니다.

핵심 흐름은 다음과 같습니다.

1. YAML 설정을 읽습니다.
2. libclang 기반으로 헤더 또는 번역 단위를 파싱합니다.
3. 선언을 내부 IR로 정규화합니다.
4. C ABI wrapper 헤더/소스와 IR 덤프를 생성합니다.
5. 같은 출력 디렉터리에 Go facade/model 출력을 함께 생성합니다.

이 도구는 광범위한 현대 C++ 전체를 자동으로 Go에 매핑하는 범용 바인딩 생성기가 아닙니다. 템플릿, STL 컨테이너, 예외 같은 고위험 요소는 기본 정책상 제한하거나 건너뜁니다.

### 주요 특징

- 보수적인 C/C++ subset 기반 wrapper 생성
- Rust CLI + libclang 파이프라인
- `check`, `ir`, `generate` 명령 제공
- co-located output layout 지원
  - `.go`, `.h`, `.cpp`, `.ir.yaml` 파일이 같은 `output.dir` 아래 생성됩니다.
- 명시적 파일 분류 지원
  - `files.model`, `files.facade` 기반으로 상위 Go 출력 성격을 나눌 수 있습니다.
- 예제 프로젝트 포함
  - `examples/simple-cpp`
  - `examples/simple-go`
  - `examples/simple-go-struct`

### 언제 적합한가

- 기존 C/C++ 헤더를 바로 Go에서 쓰기 전에 안정적인 C ABI 층이 필요한 경우
- 대규모 SDK 전체가 아니라 검토 가능한 subset부터 점진적으로 wrapper를 늘리고 싶은 경우
- 생성된 산출물을 하나의 Go 패키지 디렉터리 안에 모아 cgo로 소비하고 싶은 경우

### 언제 적합하지 않은가

- 최신 C++ 기능 전반을 자동으로 Go에 그대로 투영하고 싶은 경우
- 템플릿 메타프로그래밍, 복잡한 STL 표면, 예외 중심 API를 폭넓게 처리해야 하는 경우
- 생성기 바깥의 비즈니스 로직까지 함께 자동 생성하고 싶은 경우

### 요구 사항

- Rust toolchain
- libclang / LLVM 개발 환경
  - 현재 저장소는 `clang-sys`의 `clang_18_0` feature를 사용합니다.
- 대상 C/C++ 헤더를 해석할 수 있는 include path
- 필요하면 `compile_commands.json`
- Go 예제를 빌드하려면 Go toolchain과 cgo가 필요합니다.
- 실제 native wrapper를 빌드하려면 플랫폼에 맞는 C/C++ 컴파일러가 필요합니다.

### 빠른 시작

1. 저장소를 clone합니다.
2. Rust와 clang/libclang 환경을 준비합니다.
3. 예제 설정 파일을 복사해 대상 헤더 경로에 맞게 수정합니다.
4. 먼저 `check`로 파싱 가능 여부를 확인합니다.
5. `ir`로 정규화 결과를 확인합니다.
6. `generate`로 실제 산출물을 생성합니다.

```bash
cargo run --bin c-go -- check --config path/to/config.yaml
cargo run --bin c-go -- ir --config path/to/config.yaml --format yaml
cargo run --bin c-go -- generate --config path/to/config.yaml --dump-ir
```

### CLI 명령

#### `check`

설정 파일을 읽고 입력 헤더/번역 단위가 현재 환경에서 해석 가능한지 확인합니다.

```bash
cargo run --bin c-go -- check --config path/to/config.yaml
```

#### `ir`

정규화된 내부 표현을 YAML 또는 JSON으로 출력합니다. 어떤 선언이 들어오고 어떤 타입으로 정리되는지 확인할 때 유용합니다.

```bash
cargo run --bin c-go -- ir --config path/to/config.yaml --format yaml
cargo run --bin c-go -- ir --config path/to/config.yaml --format json --output out/ir.json
```

#### `generate`

wrapper 헤더/소스, Go 출력, 선택적 IR dump를 생성합니다.

```bash
cargo run --bin c-go -- generate --config path/to/config.yaml --dump-ir
```

### 설정 개요

대표적인 설정 키는 다음과 같습니다.

- `version`
- `input.dir`
- `input.headers`
- `input.dirs`
- `input.header_dirs`
- `input.translation_units`
- `input.compile_commands`
- `input.include_dirs`
- `input.clang_args`
- `input.allow_diagnostics`
- `output.dir`
- `output.header`
- `output.source`
- `output.ir`
- `naming.prefix`
- `naming.style`
- `policies.string_mode`
- `policies.enum_mode`
- `policies.unsupported.*`

예시:

```yaml
version: 1

input:
  headers:
    - include/foo.hpp
  compile_commands: build/compile_commands.json
  clang_args:
    - -std=c++17

output:
  dir: gen
  header: wrapper.h
  source: wrapper.cpp
  ir: wrapper.ir.yaml

naming:
  prefix: cgowrap
  style: preserve

policies:
  string_mode: c_str
  enum_mode: c_enum
  unsupported:
    templates: error
    stl_containers: skip
    exceptions: error
```

경로는 기본적으로 설정 파일 위치를 기준으로 해석됩니다.

### 출력 산출물

기본적으로 다음 파일들이 `output.dir` 아래 함께 생성됩니다.

- generated wrapper header
- generated wrapper source
- generated IR dump
- generated Go facade/model file

이 저장소의 현재 방향은 raw/native 산출물과 Go 산출물을 분리된 서브디렉터리로 나누기보다, 같은 패키지 디렉터리에 co-locate해서 downstream cgo 패키지가 바로 소비할 수 있게 하는 것입니다.

### 예제

#### `examples/simple-cpp`

작은 C++ 입력 표면을 제공합니다. 생성기 입력을 테스트할 때 가장 간단한 출발점입니다.

#### `examples/simple-go`

생성된 wrapper를 Go + cgo에서 end-to-end로 사용하는 최소 예제입니다.

#### `examples/simple-go-struct`

클래스/구조체 성격의 출력과 패키지 레이아웃을 함께 확인하기 좋은 예제입니다.

### 저장소 구조

```text
src/                    CLI, parser, IR, generator 구현
tests/                  회귀 테스트와 fixture 기반 검증
configs/                예제/로컬 검증용 설정 파일
examples/               simple-cpp, simple-go, simple-go-struct
docs/STATE.md           현재 문서 상태
docs/ROADMAP.md         현재 로드맵 상태
docs/ARCHITECTURE.md    현재 아키텍처 요약
docs/v2/designs/        feature/design handoff
docs/v2/plans/          실행 plan
docs/v2/research/       참고 자료와 상태 기록
docs/v2/completed/      완료된 작업 기록
```

### 개발 워크플로우

- 코드 변경 전에는 관련 문서를 먼저 확인합니다.
- 설정 검증은 `check`
- 타입/선언 흐름 검증은 `ir`
- 실제 산출물 검증은 `generate`
- 회귀 검증은 `cargo test`

자주 쓰는 명령:

```bash
cargo fmt
cargo test
cargo test timeval_support
cargo test abstract_class_skip
```

### 문서 시작점

공개 문서 기준으로 먼저 보면 좋은 파일:

1. `docs/STATE.md`
2. `docs/ROADMAP.md`
3. `docs/ARCHITECTURE.md`
4. `docs/v2/designs/`
5. `docs/v2/plans/`
6. `docs/v2/research/`
7. `docs/v2/completed/`

### 한계와 설계 원칙

- 이 프로젝트는 "안전하게 설명 가능한 것만 생성한다"는 쪽에 가깝습니다.
- 지원하지 않는 선언은 전체를 무조건 뚫기보다, 실패시키거나 skip metadata에 남기는 방향을 우선합니다.
- wrapper 계층은 비즈니스 로직 계층이 아닙니다.
- public Go surface는 가능한 한 의도적으로 좁게 유지합니다.

### 라이선스

이 저장소는 `MIT` 라이선스로 배포됩니다. 자세한 내용은 루트의 `LICENSE` 파일을 확인하세요.

## English

### Overview

`c-go` is designed to generate a stable C ABI wrapper layer plus Go-facing output from a conservative, reviewable subset of C/C++ headers. It is not intended to mirror arbitrary modern C++ surfaces into Go automatically.

The main pipeline is:

1. load YAML configuration
2. parse headers or translation units with libclang
3. normalize declarations into IR
4. generate native wrapper header/source artifacts
5. generate Go-facing output into the same package directory

### Key Features

- conservative wrapper generation for a controlled subset of C/C++
- Rust CLI built on top of libclang
- three primary commands: `check`, `ir`, `generate`
- co-located output layout under one `output.dir`
- explicit file-role classification via `files.model` and `files.facade`
- example projects for end-to-end validation

### Good Fit

- when you need a stable C ABI layer before consuming a C++ API from Go
- when you want to onboard a large header surface incrementally instead of all at once
- when you want generated `.go`, `.h`, `.cpp`, and `.ir.yaml` files to live together in one package directory

### Not a Good Fit

- broad automatic binding of arbitrary modern C++
- heavy template/STL/exception-driven APIs
- code generation that also owns downstream business logic

### Requirements

- Rust toolchain
- libclang / LLVM development environment
  - the current repository uses `clang-sys` with the `clang_18_0` feature
- valid include paths for your target headers
- optional `compile_commands.json` for real-world parsing environments
- Go toolchain and cgo if you want to build the Go examples
- a platform-appropriate C/C++ compiler for native wrapper compilation

### Quick Start

```bash
cargo run --bin c-go -- check --config path/to/config.yaml
cargo run --bin c-go -- ir --config path/to/config.yaml --format yaml
cargo run --bin c-go -- generate --config path/to/config.yaml --dump-ir
```

Recommended flow:

1. start from an example config
2. update paths and include arguments for your environment
3. run `check`
4. inspect `ir`
5. run `generate`

### CLI Commands

#### `check`

Validate that the configured input surface can be parsed in the current environment.

```bash
cargo run --bin c-go -- check --config path/to/config.yaml
```

#### `ir`

Print the normalized internal representation as YAML or JSON. This is useful when you need to inspect what declarations were accepted and how types were classified.

```bash
cargo run --bin c-go -- ir --config path/to/config.yaml --format yaml
cargo run --bin c-go -- ir --config path/to/config.yaml --format json --output out/ir.json
```

#### `generate`

Generate wrapper/native/Go artifacts, optionally dumping IR alongside them.

```bash
cargo run --bin c-go -- generate --config path/to/config.yaml --dump-ir
```

### Configuration Overview

Common configuration keys:

- `version`
- `input.dir`
- `input.headers`
- `input.dirs`
- `input.header_dirs`
- `input.translation_units`
- `input.compile_commands`
- `input.include_dirs`
- `input.clang_args`
- `input.allow_diagnostics`
- `output.dir`
- `output.header`
- `output.source`
- `output.ir`
- `naming.prefix`
- `naming.style`
- `policies.string_mode`
- `policies.enum_mode`
- `policies.unsupported.*`

Example:

```yaml
version: 1

input:
  headers:
    - include/foo.hpp
  compile_commands: build/compile_commands.json
  clang_args:
    - -std=c++17

output:
  dir: gen
  header: wrapper.h
  source: wrapper.cpp
  ir: wrapper.ir.yaml

naming:
  prefix: cgowrap
  style: preserve

policies:
  string_mode: c_str
  enum_mode: c_enum
  unsupported:
    templates: error
    stl_containers: skip
    exceptions: error
```

Paths are resolved relative to the configuration file.

### Generated Artifacts

The current layout places generated outputs together under `output.dir`, typically including:

- wrapper header
- wrapper source
- IR dump
- Go-facing generated file(s)

This layout is intended to make downstream cgo package consumption simpler by keeping native and Go artifacts co-located.

### Examples

- `examples/simple-cpp`
  - minimal header input surface
- `examples/simple-go`
  - minimal end-to-end Go + cgo consumption example
- `examples/simple-go-struct`
  - class/struct-oriented package layout example

### Repository Map

```text
src/                    CLI, parser, IR, and generator implementation
tests/                  regression coverage and fixtures
configs/                example and local validation configs
examples/               sample projects
docs/STATE.md           current documentation state
docs/ROADMAP.md         roadmap snapshot
docs/ARCHITECTURE.md    architecture summary
docs/v2/designs/        design handoffs
docs/v2/plans/          execution plans
docs/v2/research/       reference notes and status
docs/v2/completed/      completed work records
```

### Development Workflow

- review docs before broad implementation changes
- use `check` for environment/config validation
- use `ir` for normalization inspection
- use `generate` for output verification
- use `cargo test` for regression coverage

Useful commands:

```bash
cargo fmt
cargo test
cargo test timeval_support
cargo test abstract_class_skip
```

### Design Principles

- prefer explicit, reviewable support over speculative generation
- record unsupported or skipped declarations clearly
- keep the wrapper layer separate from downstream business logic
- keep the public Go surface intentionally narrow and understandable

### License

This repository is distributed under the `MIT` license. See the root `LICENSE` file for details.
