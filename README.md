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

files:
  model:
    - /absolute/path/to/src/IE/SIL/IsAAMaster.h
  facade:
    - /absolute/path/to/src/IE/SIL/IsAAUser.h

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

## File classification

`files.model` / `files.facade`로 헤더 역할을 나눌 수 있습니다.

- `model`: shared Go model 후보 헤더
- `facade`: raw wrapper는 생성하지만 Go model projection은 만들지 않는 헤더

현재 기준으로는:

- **model 여부는 오직 `files.model`로만 판단합니다.**
- `model` 헤더는 enum 기반 Go model 파일을 생성할 수 있습니다.
- `model` 헤더에 `IsAAMaster` 같은 getter/setter 기반 model class가 있으면 Go struct projection도 함께 생성할 수 있습니다.
- `facade` 헤더는 현재 phase 1 기준으로 primitive/bool/string 반환 free function용 Go facade API를 생성합니다.
- known model out-param을 쓰는 facade class method는 `(Model, error)` 형태의 Go facade method로 승격할 수 있습니다.
- namespaced facade free function이 같은 Go export 이름으로 충돌하면 생성 단계에서 명시적으로 실패합니다.

## Current implementation snapshot

2026-03-16 기준 구현 상태:

- raw C ABI wrapper 생성
- `files.model` / `files.facade` 파일 분류
- model 헤더용 Go enum 생성
- model 헤더용 getter/setter class projection 생성
- facade 헤더용 phase-1 Go facade 생성
  - free function only
  - primitive parameter 지원
  - primitive / bool / `std::string` / `const char*` 반환 지원
- facade class method의 known model out-param lifting
  - `bool Foo(..., Model&/* out)` -> `Foo(...) (Model, error)`

아직 미구현:

- typedef alias 기반 Go model 생성
- POD/DTO struct 생성
- iterator/list facade helper 생성
- callback facade 생성
- model-mapped collection facade helper

## Current design decisions

현재 facade 설계는 추상적인 이름 분류보다 **실제 SIL 호출 surface**를 기준으로 잡습니다.

- 기준 surface:
  - `src/IE/SIL/iSiLib.h`
  - `iSiLib-ini.h`는 현재 로컬 트리에서 아직 확인되지 않음
- facade의 목표는 raw wrapper를 하나 더 노출하는 것이 아니라 **사용 가능한 Go SDK 형태**로 올리는 것
- model 여부는 계속 `files.model`이 유일한 semantic source of truth
- facade 여부는 계속 `files.facade`가 유일한 semantic source of truth

### facade best-practice direction

다음 단계의 facade는 이름(`Select`, `List`, `Next`)보다 **타입 기반**으로 승격합니다.

예:

- `bool GetAAMaster(uint32 id, IsAAMaster& out)`
- `bool GetAAMaster(NPCSTR digit, IsAAMaster* out)`

위와 같이 `files.model`에 등록된 model type이 out-parameter로 나타나면,
이를 **model-returning facade 후보**로 본다.

초기 기본 shape:

```go
func GetAAMaster(...) (IsAAMaster, error)
```

현재 합의:

- 기본 자동 승격 shape는 `(Model, error)`
- `(Model, bool, error)`는 기본값으로 쓰지 않음
- 이유:
  - C++ `bool` 반환이 not-found인지 generic failure인지 자동으로 단정하기 어렵기 때문

### routing direction

현재 단계의 facade 확장은 **collection 의미 추론**이 아니라 **model-aware routing 정리**를 우선한다.

- known model type이 supported out-param 위치에 명시적으로 나타나는 API만 model-mapped facade 후보로 본다
- known model type이 없으면 기존 지원 범위 안에서 일반 API로 유지한다
- method/function 이름(`List`, `Next`, `Select`)만으로 collection/helper 승격을 결정하지 않는다
- source 구현을 보고 동작을 추론하지 않는다

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
3. `files.model` / `files.facade` 역할만 지정
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
- iterator/list/callback facade
- facade의 shared-model 반환 변환

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
- `examples/simple-go-struct`: generated Go struct(projection)를 import/use 하는 최소 예제
