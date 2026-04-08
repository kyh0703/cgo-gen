# Callback Facade Support

## Goal

Extend the generator so function-pointer callback typedefs and facade APIs that consume them are first-class generation targets instead of declaration-level skips.

## Scope

- parse and preserve callback typedef declarations such as `typedef void (*EventCallback)(...)`
- allow facade declarations that accept callback typedef parameters to remain in IR
- generate Go-facing callback types as plain `func(...)`
- generate facade registration APIs that accept those Go callbacks
- generate the native bridge code needed to route native callback invocations into Go

## Non-goals

- generic support for arbitrary function-pointer values in every raw/model/facade context
- callback return values beyond the current supported `void` callback shape
- broad renderer consolidation such as merging `model.rs` and `facade.rs`

## Why

Some native APIs use named callback typedefs to simplify registration surfaces. The generator should preserve that API usability in Go instead of skipping declarations such as `SetEventCallback`.

## Design decisions

### 1. Callback typedefs are types, not skipped declarations

Named function-pointer typedefs should become explicit IR nodes so downstream generators can reason about them.

### 2. Go surface should accept `func(...)`

Generated Go APIs should not expose C function-pointer syntax. For a named callback typedef:

```c
typedef void (*EventCallback)(uint32_t appId, uint32_t eventId, const char* data, int32_t size);
```

the Go-facing shape should be:

```go
type EventCallback func(appID uint32, eventID uint32, data string, size int32)
```

The typedef name remains relevant internally for native bridge generation and API matching.

### 3. Callback registration APIs are the first supported callback use case

The first supported pattern is facade functions or methods that receive a named callback typedef parameter, for example `SetEventCallback(EventCallback cb)`.

### 4. Bridge generation owns callback plumbing

Generated output must handle:

- C trampoline entrypoints
- Go callback registry/dispatch
- registration and clearing behavior

### 5. Keep module boundaries intact

`model.rs` and `facade.rs` may share helper logic later, but callback support should be added without a speculative renderer merge.

## Success criteria

- callback typedef declarations appear in normalized IR instead of `support.skipped_declarations`
- facade APIs that consume named callback typedefs generate Go wrappers
- generated Go API accepts `func(...)`
- regression tests cover free-function callback registration and generated output shape
