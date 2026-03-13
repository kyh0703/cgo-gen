# c-go

`c-go`는 보수적인 C++ 헤더 subset을 파싱해서 C ABI wrapper를 생성하는 Rust CLI입니다.

현재 기준 동작은 다음과 같습니다.

- 입력: 하나의 YAML config
- 처리 단위: `input.headers`에 들어 있는 **각 헤더 파일별**
- 출력: 헤더별로 별도의 wrapper 세트 생성

예를 들어:

- `IsAAMaster.h`
- `IsAAUser.h`

를 같은 config에 넣으면:

- `is_aa_master_wrapper.h/.cpp/.ir.yaml/.go`
- `is_aa_user_wrapper.h/.cpp/.ir.yaml/.go`

처럼 **헤더별 산출물**이 나옵니다.

---

## What it does

주어진 YAML config로:

1. 대상 C++ 헤더를 `libclang`으로 파싱
2. 내부 IR로 정규화
3. C ABI wrapper 파일 생성

생성 스타일:

- 클래스는 opaque handle 기반
- 메서드/함수는 flat C symbol로 노출
- cgo 친화적인 C/C++ wrapper 생성
- 필요 시 Go struct projection 생성

---

## Commands

### Generate wrapper files

```bash
cargo run --bin c-go -- generate --config path/to/wrapper.yaml --dump-ir
```

### Print IR only

```bash
cargo run --bin c-go -- ir --config path/to/wrapper.yaml --format yaml
```

### Check parseability without generating files

```bash
cargo run --bin c-go -- check --config path/to/wrapper.yaml
```

---

## YAML config

예시:

```yaml
version: 1

input:
  headers:
    - /absolute/path/to/src/IE/SIL/IsAAMaster.h
    - /absolute/path/to/src/IE/SIL/IsAAUser.h
  clang_args:
    - -std=c++11
    - -x
    - c++
    - -I/absolute/path/to/CORE/inc
    - -I/absolute/path/to/src/IE/inc
    - -I/absolute/path/to/src/IE/SIL
    - -I/absolute/path/to/src/LIB/inc

output:
  dir: ./pkg/sil

filter:
  classes:
    - IsAAMaster
    - IsAAUser
  methods:
    - IsAAMaster::*
    - IsAAUser::*

go_structs:
  - IsAAMaster
  - IsAAUser

naming:
  prefix: sil
  style: preserve

policies:
  string_mode: c_str
  enum_mode: c_enum
  unsupported:
    templates: error
    stl_containers: skip
    exceptions: error
```

기본 예제 파일:

```text
configs/sil-wrapper.example.yaml
```

---

## Output naming rules

기본적으로 사용자가 `output.header`, `output.source`, `output.ir`를 매번 직접 지정할 필요는 없습니다.

### 기본 규칙

각 헤더 파일 stem 기준으로 자동 생성됩니다.

- `IsAAMaster.h` → `is_aa_master_wrapper.h`
- `IsAAMaster.h` → `is_aa_master_wrapper.cpp`
- `IsAAMaster.h` → `is_aa_master_wrapper.ir.yaml`
- `IsAAMaster.h` → `is_aa_master_wrapper.go`

### multi-header config

`input.headers`가 여러 개인 경우:

- config는 1개
- 산출물은 헤더 수만큼 여러 세트 생성

### explicit output names

단일 헤더일 때는 `output.header/source/ir`를 직접 지정할 수 있습니다.

하지만 multi-header 생성에서는 파일 충돌을 피하기 위해 **기본 자동 파일명 규칙을 사용하는 것을 전제**합니다.

### Go package name

Go package명은 `output.dir`의 마지막 디렉토리명을 기준으로 결정됩니다.

예:

- `output.dir: ./pkg/sil` → `package sil`

---

## SIL-focused usage

일반적인 사용 흐름:

1. `configs/sil-wrapper.example.yaml` 복사
2. 헤더 경로와 include path 수정
3. 필요한 클래스/메서드 filter 지정
4. `generate` 실행
5. 생성된 wrapper 파일을 cgo 프로젝트에서 사용

예를 들어 `output.dir: ./pkg/sil`이고 headers가 두 개면:

```text
pkg/sil/is_aa_master_wrapper.h
pkg/sil/is_aa_master_wrapper.cpp
pkg/sil/is_aa_master_wrapper.ir.yaml
pkg/sil/is_aa_master_wrapper.go

pkg/sil/is_aa_user_wrapper.h
pkg/sil/is_aa_user_wrapper.cpp
pkg/sil/is_aa_user_wrapper.ir.yaml
pkg/sil/is_aa_user_wrapper.go
```

---

## Current scope

잘 동작하는 범위:

- free functions
- non-template classes
- constructors / destructors
- simple public methods
- simple enums
- common typedef aliases such as `NPCSTR`, `uint32`, `int32`

아직 제한이 있는 범위:

- templates
- 광범위한 STL container 지원
- overload-safe naming
- namespace가 다른 동일 leaf class 이름 충돌 처리
- 프로젝트별 수동 facade API 생성

---

## Notes for cgo users

이 도구는 wrapper layer만 생성합니다.

실제 컴파일/링크 플래그는 소비하는 Go/cgo 프로젝트에서 직접 지정해야 합니다.

예:

- include path: `#cgo CXXFLAGS: -I...`
- link flags: `#cgo LDFLAGS: ...`

즉:

- `c-go`는 wrapper 파일 생성 담당
- 소비 프로젝트는 그 파일을 컴파일/링크 담당

---

## Test

```bash
cargo test
```

---

## Examples

- `examples/simple-cpp`: 최소 C++ fixture
- `examples/simple-go`: generated wrapper를 Go/cgo에서 빌드/실행하는 최소 예제
