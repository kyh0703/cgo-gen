# simple-go-struct

generated Go struct를 사용하는 가장 단순한 예제입니다.

이 예제는 `simple-go`와 다릅니다.

- `simple-go`: generated C wrapper를 Go/cgo에서 사용하는 예제
- `simple-go-struct`: `go_structs`로 생성된 Go struct 자체를 사용하는 예제

즉 이 예제의 Go 파일은 **binding layer가 아니라 projection/data struct** 예제입니다.

## 흐름

1. `c-go`로 C++ header를 읽는다
2. wrapper 산출물과 함께 Go struct 파일을 생성한다
3. Go 코드에서 생성된 struct 타입을 직접 사용한다

## 사용법

```bash
make -C examples/simple-go-struct gen
make -C examples/simple-go-struct build
make -C examples/simple-go-struct run
```

## 기대 산출물

- `pkg/model/thing_model_wrapper.h`
- `pkg/model/thing_model_wrapper.cpp`
- `pkg/model/thing_model_wrapper.ir.yaml`
- `pkg/model/thing_model_wrapper.go`

## 참고

이 예제는 generated Go struct가 **컴파일되고 Go 코드에서 import/use 가능한지**를 보여주는 목적입니다.
자동 cgo method binding까지 생성하는 예제는 아닙니다.
