# Product

`c-go` is a Rust CLI that turns a conservative subset of C++ APIs into a generated C ABI surface (`wrapper.h` and `wrapper.cpp`). The generated header is intended to be consumed by `c-for-go`, not by humans writing C manually.

## Goals
- Parse selected C++ headers.
- Normalize them into a stable intermediate representation (IR).
- Generate a Go-friendly C ABI wrapper layer.

## Non-goals
- Generate Go bindings directly.
- Support the entire C++ type system in v1.
