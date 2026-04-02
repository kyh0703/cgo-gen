# c-go

[English](./README.md)

`c-go`는 보수적인 C/C++ 헤더 subset을 파싱해서 아래 산출물을 만드는 Rust CLI입니다.

- C ABI wrapper header/source
- 선택적 normalized IR dump
- 같은 출력 디렉터리에 놓이는 Go `cgo` facade 파일

임의의 현대 C++ 전체를 처리하는 도구가 아니라, 통제 가능한 헤더 표면을 안정적으로 감싸는 도구에 가깝습니다.

## 상태

`c-go`는 의도적으로 보수적인 범위를 유지합니다. 외부 사용자 기준의 공개 계약은 이 README에 적힌 현재 CLI와 설정 동작입니다. 저장소 안의 [`docs/`](./docs/)에는 과거 기획/설계 문서도 있지만, 코드보다 더 강한 진실의 원천은 아닙니다.

## 생성물

지원되는 엔트리 헤더마다 `c-go`는 한 출력 디렉터리에 다음 파일들을 생성할 수 있습니다.

- `<name>_wrapper.h`
- `<name>_wrapper.cpp`
- `<name>_wrapper.go`
- `--dump-ir` 사용 시 `<name>_wrapper.ir.yaml`

`.go`, `.h`, `.cpp`, `.ir.yaml`를 같은 디렉터리에 두는 이유는 downstream `cgo` 패키지가 한 위치에서 함께 빌드할 수 있게 하기 위해서입니다.

## 요구사항

- Rust toolchain
- 런타임에 발견 가능한 `libclang`
- 실사용 헤더를 다룰 때는 Clang 호환 compile 환경
- 생성된 Go 패키지를 실제로 빌드할 때만 Go toolchain

이 크레이트는 `clang-sys`의 `clang_18_0` feature로 빌드되므로, LLVM/Clang 18 계열 `libclang` 환경을 맞추는 것이 가장 안전합니다.

## 설치

저장소에서 바로 실행:

```bash
cargo run --bin c-go -- check --config cppgo-wrap.yaml
```

로컬 CLI로 설치:

```bash
cargo install --path .
c-go check --config cppgo-wrap.yaml
```

## 빠른 시작

루트에 들어 있는 설정 파일이 가장 작은 end-to-end 예제입니다.

```yaml
version: 1

input:
  headers:
    - examples/simple-cpp/include/foo.hpp
  compile_commands: examples/simple-cpp/build/compile_commands.json

output:
  dir: gen

naming:
  prefix: cgowrap
  style: snake_case
```

주요 명령:

```bash
cargo run --bin c-go -- check --config cppgo-wrap.yaml
cargo run --bin c-go -- ir --config cppgo-wrap.yaml --format yaml
cargo run --bin c-go -- generate --config cppgo-wrap.yaml --dump-ir
```

예제 프로젝트:

- [`examples/simple-go`](./examples/simple-go)
- [`examples/simple-go-struct`](./examples/simple-go-struct)

## CLI

현재 제공하는 서브커맨드는 세 가지입니다.

- `generate --config <path> [--dump-ir]`
- `ir --config <path> [--output <path>] [--format yaml|json]`
- `check --config <path>`

## 설정 키 설명

지원되는 사용자 설정값은 YAML config 키 기준입니다. 상대 경로는 모두 config 파일 위치를 기준으로 해석되고, 실제로 존재하는 경로는 canonicalize 되기 때문에 symlink 경로를 써도 로드 시점에 실제 경로로 정규화됩니다.

| Key | 현재 동작 |
| --- | --- |
| `version` | 선택적 schema marker입니다. 현재는 읽기만 하고 동작 분기에는 쓰지 않습니다. |
| `input.dir` | 디렉터리 소유 모드입니다. `generate`는 이 디렉터리 바로 아래 헤더마다 wrapper 세트를 하나씩 만듭니다. |
| `input.headers` | 명시적 엔트리 헤더 목록입니다. 가장 좁고 예측 가능한 방식입니다. |
| `input.header_dirs` | 디렉터리를 재귀적으로 돌며 헤더를 찾아 `input.headers`로 확장합니다. header-only 샘플에 적합합니다. |
| `input.dirs` | 디렉터리를 재귀적으로 돌며 헤더와 translation unit을 함께 확장합니다. |
| `input.translation_units` | 명시적 parse entry입니다. 값이 있으면 파싱은 `input.headers`보다 이 목록을 우선 사용합니다. |
| `input.compile_commands` | `compile_commands.json`에서 compiler flag와 source TU 후보를 읽어옵니다. |
| `input.include_dirs` | `input.clang_args` 앞에 `-I...` include flag를 prepend 합니다. |
| `input.clang_args` | 추가 libclang 인자입니다. 상대 `-I...`, `-I <path>`, `-isystem` 경로는 config 파일 기준으로 해석됩니다. |
| `input.allow_diagnostics` | `true`면 libclang diagnostic이 발생한 translation unit을 실패 대신 skip 합니다. |
| `output.dir` | 출력 디렉터리입니다. 상대 경로는 config 파일 기준입니다. |
| `output.header` / `output.source` / `output.ir` | 출력 파일명 override입니다. 기본값을 유지하면 single-header 모드에서 `<header_stem>_wrapper.*`로 자동 추론됩니다. |
| `naming.prefix` | 생성되는 C ABI symbol prefix입니다. `<prefix>_string_free`에도 사용됩니다. |
| `naming.style` | `preserve`면 원본 케이스를 최대한 유지합니다. 그 외 값은 현재 symbol part를 소문자화하는 쪽으로 동작하며, 저장소 예제는 이 동작을 `snake_case`로 사용합니다. |

## 예약됐거나 과거 문서에만 남은 키

내부 문서나 오래된 설정에서 보여도 현재 공개 CLI 동작 스위치로 믿으면 안 되는 키들입니다.

| Key | 현재 상태 |
| --- | --- |
| `project_root` | Rust config struct에는 있지만 generator는 사용하지 않습니다. |
| `policies.string_mode` | 파싱은 되지만 현재 동작 분기에 쓰이지 않습니다. |
| `policies.enum_mode` | 파싱은 되지만 현재 동작 분기에 쓰이지 않습니다. |
| `policies.unsupported.templates` | 파싱은 되지만 현재 동작 분기에 쓰이지 않습니다. |
| `policies.unsupported.stl_containers` | 파싱은 되지만 현재 동작 분기에 쓰이지 않습니다. |
| `policies.unsupported.exceptions` | 파싱은 되지만 현재 동작 분기에 쓰이지 않습니다. |
| `files.model` / `files.facade` | 과거 내부 문서와 테스트에는 보이지만, 현재 공개 `Config` loader가 읽는 키는 아닙니다. 활성 설정값으로 의존하면 안 됩니다. |

## 외부 프로젝트를 심볼릭 링크로 붙이는 방법

외부 SDK나 private C++ 프로젝트를 이 저장소 밖에 두고, 저장소 안에는 symlink만 두는 방식이 잘 맞습니다.

```bash
mkdir -p third_party
ln -s /absolute/path/to/external-sdk third_party/external-sdk
```

그 다음 config에서는 저장소 안의 symlink 경로를 사용합니다.

```yaml
version: 1

input:
  dir: third_party/external-sdk/include
  compile_commands: third_party/external-sdk/build/compile_commands.json
  clang_args:
    - -Ithird_party/external-sdk/include

output:
  dir: gen/external-sdk

naming:
  prefix: ext
  style: preserve
```

실제로는 이렇게 처리됩니다.

- 상대 경로 기준점은 shell 현재 디렉터리가 아니라 YAML 파일 위치입니다.
- config 로딩 시 symlink target을 canonicalize 하므로 TU 매칭과 파싱은 실제 경로 기준으로 동작합니다.
- `compile_commands.json` 안에 `input.dir` 아래 source file이 있으면 header entry보다 그 source TU를 우선 사용합니다.
- `input.dir` 밖의 imported header는 타입 해석에는 도움을 주지만, 이 프로젝트가 소유한 public entry header로 취급되지는 않습니다.

외부 프로젝트가 이미 괜찮은 `compile_commands.json`을 만들고 있다면, 많은 `clang_args`를 수동으로 복제하는 것보다 그 파일을 그대로 쓰는 편이 낫습니다.

## 현재 지원 범위

- free function
- non-template class
- constructor / destructor
- 단순 public method
- generated wrapper 이름에서의 deterministic overload disambiguation
- `int32`, `uint64`, `size_t` 같은 primitive / fixed-width alias
- `const char*`, `char*`, `std::string`, `std::string_view`
- Go 쪽 primitive pointer / reference write-back
- 지원되는 API에 연결된 named callback typedef
- `struct timeval*`, `struct timeval&`
- native wrapper와 같은 디렉터리에 생성되는 handle-backed Go wrapper

## 비지원 또는 의도적 제한

- `operator+`, `operator==` 같은 operator declaration
- `void (*cb)(int)` 형태의 raw inline function pointer parameter
- template와 STL-heavy API
- anonymous class
- exception translation
- 고급 inheritance modeling
- raw-safe하게 표현할 수 없는 by-value object parameter / return

일부 비지원 선언은 전체 실행을 abort 하지 않고 skip 됩니다. 이런 경우 이유는 normalized IR의 `support.skipped_declarations`에 기록됩니다.

## 실무 팁

- `input.allow_diagnostics: true`는 복구용 스위치이지 품질 향상 스위치가 아닙니다. 실패한 translation unit 자체를 통째로 skip 합니다.
- multi-header directory 모드에서는 `output.header`, `output.source`, `output.ir`를 기본값으로 두는 편이 안전합니다. 그래야 헤더별 출력 파일명을 자동 추론할 수 있습니다.
- 플랫폼이 `libclang`를 못 찾는다면 먼저 시스템 loader나 LLVM 설치를 고쳐야 합니다. `c-go` 자체가 별도의 프로젝트 전용 runtime env var 레이어를 제공하지는 않습니다.

## 라이선스

[MIT](./LICENSE)
