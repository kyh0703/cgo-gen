# Architecture

`cgo-gen` turns a conservative subset of C/C++ headers into a stable raw C ABI plus optional Go wrappers emitted into the same output package.

## Core flow

1. Load YAML config.
2. Resolve headers, directories, compile commands, and extra clang args.
3. Parse selected translation units through libclang.
4. Normalize supported declarations into IR.
5. Generate raw native wrapper output.
   - C ABI wrapper headers and sources for classes and free functions.
   - bridge helpers for strings, callbacks, and other supported interop cases.
6. Generate Go-facing wrapper output when the IR can be rendered safely.
   - handle-backed model wrappers for supported object types.
   - facade functions and methods for supported free functions and class APIs.

## Layer responsibilities

### Raw layer
- Bridges C++ declarations to a stable C ABI.
- Owns `*_wrapper.h` / `*_wrapper.cpp` artifacts.
- Handles constructors, destructors, overload-safe symbol suffixing, and native ownership edges.

### Go layer
- Emits `*_wrapper.go` beside the generated native files.
- Preserves handle-backed object identity instead of inventing detached DTO copies.
- Hides C string ownership, callback bridge plumbing, and native calling details behind Go-friendly helpers.

## Runtime boundaries

The implementation separates persisted configuration, runtime pipeline state, analysis, and rendering:

- `Config` stores only user-authored file configuration.
- `PipelineContext` owns runtime-derived state such as scoped headers, resolved clang args, and analyzed model metadata.
- `domain::kind` owns serialized IR enums such as `IrTypeKind`, `IrFunctionKind`, and `FieldAccessKind`.
- `domain::model_projection` owns shared projection structures reused across analysis and rendering.
- `parsing` owns libclang parsing and translation-unit collection.
- `analysis` owns derived model projection analysis.
- `codegen` owns IR normalization plus raw and Go rendering.

The important boundary is that parsing and normalization operate on `PipelineContext`, not on raw config alone. Renderers consume analyzed state instead of recomputing pipeline facts ad hoc.

## Design principle

Generated output should expose a stable interoperability boundary, not business logic. The generator should keep native ownership and calling semantics explicit while still producing wrappers that downstream packages can use directly.

## Facade routing

Facade generation is intentionally type-driven:

- if an API uses a known supported model type directly in a pointer or reference position
- and that usage is safe to preserve as a live native-backed handle
- then the Go layer should prefer a wrapper that accepts that handle-backed model directly

Examples of the desired shape:

- `bool LoadThing(..., ThingRecord& out)` -> `LoadThing(..., out *ThingRecord) bool`
- `bool LoadThing(..., ThingRecord* out)` -> `LoadThing(..., out *ThingRecord) bool`

This keeps native mutability and lifetime attached to the same handle instead of fabricating detached return values.

## Current implementation note

- Raw native wrapper generation is implemented and remains the stable base layer.
- Generated wrapper files and Go files now emit together under `output.dir/`.
- Supported free functions and class methods can render Go wrappers for primitive, string, callback, and known-model handle cases.
- Unknown object reference or pointer declarations can remain in raw output as opaque handles when the raw layer can express them safely.
- The same unknown declarations stay filtered out from Go-facing layers unless they map to a supported model path.
- Raw-unsafe by-value object declarations are skipped at declaration level and recorded in `support.skipped_declarations`.
- Known-model Go helpers enforce non-nil handles for required references and allow nil where pointer semantics permit it.
- Overloaded raw symbols and Go exports are disambiguated deterministically from parameter signatures.
- Namespaced facade functions that would collide in exported Go names are rejected during generation.
- The current source layout follows the implemented domain split under `src/parsing/`, `src/codegen/`, `src/analysis/`, `src/domain/`, and `src/pipeline/`.
