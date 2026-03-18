# Architecture

`c-go` now targets a two-layer output model built on top of the existing native wrapper pipeline.

## Core flow

1. Load YAML config.
2. Parse selected C/C++ headers.
3. Classify input files by role.
   - `model`: files that define shared data models, enums, typedefs, or class-to-model projections.
   - `facade`: files that define shared operational APIs, lifecycle entrypoints, iterators, callbacks, or service-style functions.
4. Normalize parsed declarations into IR.
5. Generate raw native wrapper output.
   - C ABI wrapper headers/sources for C++ classes and functions.
   - raw type/data bridge artifacts when needed.
6. Generate upper Go-facing shared output.
   - shared Go models from `model` files.
   - shared Go facade APIs from `facade` files.
7. Downstream IE process modules consume the shared Go package and keep business logic outside the generated wrapping layer.

## Layer responsibilities

### Raw layer
- Lowest-level generated wrapper output.
- Bridges C++ to stable C ABI.
- Owns `wrapper.h` / `wrapper.cpp` style artifacts.
- Hides constructors, destructors, overload flattening, and C++-specific details.

### Shared model layer
- Built from files classified as `model`.
- Produces shared Go structs, enums, typedef mappings, and class projections.
- Represents the common data contracts IE modules should import and reuse.

### Shared facade layer
- Built from files classified as `facade`.
- Produces common Go functions/helpers that return shared Go models.
- Hides raw iteration, callbacks, native error codes, and native calling conventions.

## Design principle

The generated wrapping package is not the place for business logic. Its job is to expose a stable, reusable shared SDK over the native SIL surface so DCM/HTD/other IE modules can share the same models and common APIs.

## Facade design direction

Facade design is now anchored to the **actual SIL call surface**, not abstract naming alone.

Primary reference surface:
- `src/IE/SIL/iSiLib.h`
- `iSiLib-ini.h` should also be included once its local path is confirmed

The key principle is **type-driven facade lifting**:

- if a facade API fills a known model type from `files.model`
- and that model appears directly in the signature as an out-parameter
- then the wrapper layer should prefer generating a Go API that returns the shared model directly

Examples of desired lifting:
- `bool GetAAMaster(..., IsAAMaster& out)` -> `GetAAMaster(...) (IsAAMaster, error)`
- `bool GetAAMaster(..., IsAAMaster* out)` -> `GetAAMaster(...) (IsAAMaster, error)`

Why `Model, error` instead of `Model, bool, error` by default:
- the C++ `bool` is not guaranteed to mean `found/not found`
- it may also mean generic success/failure
- therefore the safer default shape is `Model, error`

For the current facade slice, the design should prefer **model-aware routing first**:
- if the API is tied to a known shared model type in the supported out-param position, it can be routed to model-mapped facade generation
- if it is not mapped to a known model type, it should remain a regular API when otherwise supported
- pattern naming alone should not be treated as the primary decision source
- source implementation details must not be used to infer higher-level helper behavior

## Current implementation note (2026-03-16)

- Raw native wrapper generation is implemented and remains the current stable base layer.
- File-level classification config now exists via `files.model` and `files.facade`.
- A dedicated Go model rendering path now exists beside raw wrapper generation.
- Current classification effect is still intentionally partial:
  - model/facade semantic classification is determined only by explicit config (`files.model`, `files.facade`).
  - `model` headers can emit Go enum models and auto-project `IsAAMaster`-style getter/setter classes into Go structs.
  - `facade` headers now generate phase-1 Go facade wrappers and still do not emit Go model files.
  - the base supported facade surface is primitive-parameter free functions with primitive/bool/string returns.
  - as a current type-driven extension, facade class methods that fill known `files.model` types via `Model&` / `Model*` out-params can be lifted into `Model, error` Go methods.
  - namespaced facade functions that would collide in Go export names are rejected during generation.
- Typedef/DTO model generation, model-mapped collection facade generation, callback facade generation, and richer type-driven facade lifting beyond the first out-param pattern are not implemented yet. The current implementation focus is routing cleanup, not collection helper inference.
