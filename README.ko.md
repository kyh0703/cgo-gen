# cgo-gen

[English](./README.md)

`cgo-gen`은 보수적인 C/C++ 헤더 subset을 파싱해서 아래 산출물을 만드는 Rust CLI입니다.

- C ABI wrapper header/source
- 선택적 normalized IR dump
- 같은 출력 디렉터리에 놓이는 Go `cgo` facade 파일

임의의 현대 C++ 전체를 처리하는 도구가 아니라, 통제 가능한 헤더 표면을 안정적으로 감싸는 도구에 가깝습니다.

## 빠른 시작

현재 저장소에서 실제로 유지되는 가장 짧은 흐름은 예제 하나를 그대로 돌려보는 것입니다.

```bash
cargo run --bin cgo-gen -- check --config examples/simple-go/config.yaml
cargo run --bin cgo-gen -- generate --config examples/simple-go/config.yaml --dump-ir
make -C examples/simple-go run
```

이 흐름은 저장소의 현재 지원 경로를 그대로 보여줍니다.

1. YAML config 로드
2. `libclang`으로 헤더 파싱
3. 선언을 normalized IR로 정규화
4. `output.dir` 아래에 wrapper 파일 생성
5. 생성된 Go 패키지를 빌드하거나 소비

## 요구사항

- Rust toolchain
- 런타임에 발견 가능한 `libclang`
- 실사용 헤더를 다룰 때는 Clang 호환 compile 환경
- 생성된 Go 패키지를 실제로 빌드할 때만 Go toolchain

이 크레이트는 `clang-sys`의 `clang_18_0` feature로 빌드되므로, LLVM/Clang 18 계열 `libclang` 환경을 맞추는 것이 가장 안전합니다.

## 설치

저장소에서 바로 실행:

```bash
cargo run --bin cgo-gen -- --help
```

로컬 CLI로 설치:

```bash
cargo install --path .
cgo-gen --help
```

## 핵심 명령

현재 제공하는 서브커맨드는 세 가지입니다.

- `generate --config <path> [--dump-ir] [--go-module <module-path>]`
- `ir --config <path> [--output <path>] [--format yaml|json]`
- `check --config <path>`

일반적인 흐름은 아래 두 줄이면 충분합니다.

```bash
cgo-gen check --config path/to/config.yaml
cgo-gen generate --config path/to/config.yaml --dump-ir
```

wrapper를 쓰지 않고 normalized IR만 확인하고 싶다면:

```bash
cgo-gen ir --config path/to/config.yaml --format yaml
```

## 최소 설정

가장 실용적인 최소 config는 대개 엔트리 헤더 하나와 `compile_commands.json` 조합입니다.

```yaml
version: 1

input:
  headers:
    - path/to/foo.hpp
  compile_commands: path/to/compile_commands.json

output:
  dir: gen

naming:
  prefix: cgowrap
  style: snake_case
```

핵심 동작:

- 상대 경로는 config 파일 위치를 기준으로 해석됩니다.
- 지원하지 않는 키는 로드 시점에 오류로 처리됩니다.
- 생성되는 `.go`, `.h`, `.cpp`, 선택적 `.ir.yaml` 파일은 모두 `output.dir` 아래에 함께 놓입니다.
- `--go-module <module-path>`를 주면 `generate`가 `go.mod`와 `build_flags.go`도 함께 생성합니다.

## 생성 결과

지원되는 엔트리 헤더마다 `generate`는 보통 아래 파일들을 만듭니다.

- `<name>_wrapper.h`
- `<name>_wrapper.cpp`
- `<name>_wrapper.go`
- `--dump-ir` 사용 시 `<name>_wrapper.ir.yaml`

`--go-module`을 사용하면 추가로 아래 파일도 생성합니다.

- `go.mod`
- `build_flags.go`

이 파일들을 한 디렉터리에 모아두는 이유는 downstream `cgo` 패키지가 한 위치에서 함께 빌드할 수 있게 하기 위해서입니다.

## Go Module 출력

`output.dir` 자체를 독립적인 Go module처럼 쓰고 싶다면 `generate --go-module <module-path>`를 사용합니다.

```bash
cgo-gen generate --config path/to/config.yaml --go-module example.com/acme/foo
```

이 옵션을 주면 추가로:

- `module <module-path>`와 `go 1.25`가 들어간 `go.mod`
- `build_flags.go`

가 생성됩니다.

현재 동작은 다음과 같습니다.

- `build_flags.go`는 항상 `#cgo CFLAGS: -I${SRCDIR}`를 포함합니다.
- `#cgo CXXFLAGS`는 raw `input.clang_args`에서만 추출합니다.
- export되는 `CXXFLAGS`는 `-I`, `-isystem`, `-D`, `-std=...`만 허용합니다.
- `input.ldflags`가 있으면 `build_flags.go`에 `#cgo LDFLAGS`도 생성합니다.
- `compile_commands.json`은 파싱에는 쓰이지만 Go package metadata로 직접 export되지는 않습니다.

생성 디렉터리 자체를 Go 패키지로 import하고 빌드하려는 경우 이 모드를 사용하면 됩니다.

## 자주 쓰는 설정 키

처음에는 모든 옵션을 알 필요는 없습니다. 실제로 자주 쓰는 것만 보면 됩니다.

- `input.headers`: 명시적 public entry header 목록
- `input.dir`: 디렉터리 바로 아래 헤더마다 wrapper 세트를 하나씩 생성
- `input.header_dirs`: 디렉터리를 재귀적으로 돌며 헤더를 `input.headers`로 확장
- `input.dirs`: 헤더와 translation unit을 함께 재귀 확장
- `input.translation_units`: 명시적 parse entry. 있으면 `input.headers`보다 우선
- `input.compile_commands`: `compile_commands.json`에서 컴파일 플래그와 source TU 후보 읽기
- `input.clang_args`: `-I`, `-isystem`, `-D`, `-std=...` 같은 추가 libclang 인자
- `input.ldflags`: 생성되는 `build_flags.go`에 전달할 링커 플래그
- `output.dir`: 출력 디렉터리
- `output.header`, `output.source`, `output.ir`: single-header 모드용 출력 파일명 override
- `naming.prefix`: 생성되는 C symbol prefix
- `naming.style`: `preserve` 또는 현재 예제들이 쓰는 lowercase/snake-style fallback

주의할 점:

- multi-header generation에서는 `output.header`, `output.source`, `output.ir`를 기본값으로 두는 편이 안전합니다.
- `input.clang_args`와 `input.ldflags`의 상대 경로는 config 파일 위치 기준으로 해석됩니다.
- env 확장은 `$VAR`, `$(VAR)`, `${VAR}`만 지원합니다.

## 설정 키 설명

지원되는 사용자 설정값은 YAML config 키 기준입니다. 상대 경로는 모두 config 파일 위치를 기준으로 해석되고, 실제로 존재하는 경로는 canonicalize 되기 때문에 symlink 경로를 써도 로드 시점에 실제 경로로 정규화됩니다. 지원하지 않는 키는 config 로드 시 오류로 처리됩니다.

| Key | 현재 동작 |
| --- | --- |
| `version` | 선택적 schema marker입니다. 현재는 읽기만 하고 동작 분기에는 쓰지 않습니다. |
| `input.dir` | 디렉터리 소유 모드입니다. `generate`는 이 디렉터리 바로 아래 헤더마다 wrapper 세트를 하나씩 만듭니다. |
| `input.headers` | 명시적 엔트리 헤더 목록입니다. 가장 좁고 예측 가능한 방식입니다. |
| `input.header_dirs` | 디렉터리를 재귀적으로 돌며 헤더를 찾아 `input.headers`로 확장합니다. header-only 샘플에 적합합니다. |
| `input.dirs` | 디렉터리를 재귀적으로 돌며 헤더와 translation unit을 함께 확장합니다. |
| `input.translation_units` | 명시적 parse entry입니다. 값이 있으면 파싱은 `input.headers`보다 이 목록을 우선 사용합니다. |
| `input.compile_commands` | `compile_commands.json`에서 compiler flag와 source TU 후보를 읽어옵니다. |
| `input.clang_args` | 추가 libclang 인자입니다. 상대 `-I...`, `-I <path>`, `-isystem` 경로는 config 파일 기준으로 해석됩니다. `$VAR`, `$(VAR)`, `${VAR}` 형태의 exact env token도 현재 OS environment에서 확장합니다. include root가 필요하면 여기에 `-I...` 토큰으로 직접 적습니다. |
| `input.ldflags` | 생성되는 `build_flags.go`의 `#cgo LDFLAGS`에 그대로 전달할 링커 플래그입니다. 상대 `-L<path>`, `-L <path>` 경로는 config 파일 기준으로 해석되며, 값 안에 포함된 `${VAR}`, `$(VAR)` env token도 확장됩니다. |
| `output.dir` | 출력 디렉터리입니다. 상대 경로는 config 파일 기준입니다. |
| `output.header` / `output.source` / `output.ir` | 출력 파일명 override입니다. 기본값을 유지하면 single-header 모드에서 `<header_stem>_wrapper.*`로 자동 추론됩니다. |
| `naming.prefix` | 생성되는 C ABI symbol prefix입니다. `<prefix>_string_free`에도 사용됩니다. |
| `naming.style` | `preserve`면 원본 케이스를 최대한 유지합니다. 그 외 값은 현재 symbol part를 소문자화하는 쪽으로 동작하며, 저장소 예제는 이 동작을 `snake_case`로 사용합니다. |

## 예제

유지되는 예제:

- [`examples/simple-go`](./examples/simple-go): 가장 작은 end-to-end free-function 흐름
- [`examples/simple-go-struct`](./examples/simple-go-struct): handle-backed model / facade 흐름

자주 쓰는 명령:

```bash
make -C examples/simple-go gen
make -C examples/simple-go build
make -C examples/simple-go run

make -C examples/simple-go-struct gen
make -C examples/simple-go-struct build
make -C examples/simple-go-struct run
```

## 저장소 구조

사용자 관점의 진입점은 아래 정도로 보면 됩니다.

- `src/cli.rs`: CLI 계약과 서브커맨드 정의
- `src/config.rs`: YAML config 로드와 경로 해석
- `src/parsing/`: libclang 파싱과 translation unit 수집
- `src/analysis/`: 파생 model projection 분석
- `src/codegen/`: IR normalization, C ABI 생성, Go facade 생성
- `src/pipeline/`: 런타임 pipeline context
- `examples/`: 유지되는 end-to-end 샘플

내부 구조를 더 보고 싶다면 [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)를 보면 되지만, 실제 계약은 결국 코드와 CLI 동작입니다.

## 현재 지원 범위

- free function
- non-template class
- constructor / destructor
- deterministic overload disambiguation을 포함한 public method
- 지원되는 필드 타입에 대한 public struct field accessor (get / set)
- `int32`, `uint64`, `size_t` 같은 primitive / fixed-width alias
- `const char*`, `char*`, `std::string`, `std::string_view`
- fixed-size C 배열: `unsigned char[N]` (바이트 배열), primitive element 타입의 `T[N]`, model element 타입의 `Model[N]`
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

## 라이선스

[MIT](./LICENSE)
