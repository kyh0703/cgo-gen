# Architecture

`c-go` now targets a raw C ABI layer plus a shared Go package built on top of the native wrapper pipeline.

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
   - handle-backed Go model wrappers from `model` files.
   - shared Go facade APIs from `facade` files in the same Go package.
7. Downstream IE process modules consume the shared Go package and keep business logic outside the generated wrapping layer.

## Layer responsibilities

### Raw layer
- Lowest-level generated wrapper output.
- Bridges C++ to stable C ABI.
- Owns `wrapper.h` / `wrapper.cpp` style artifacts.
- Emits physical files under `output.dir/`.
- Hides constructors, destructors, overload-safe symbol suffixing, and other C++-specific details.

### Shared model layer
- Built from files classified as `model`.
- Produces handle-backed Go wrappers, enums, typedef mappings, and class projections.
- Emits physical files under `output.dir/`.
- Represents the common native-backed model contracts IE modules should import and reuse.

### Shared facade layer
- Built from files classified as `facade`.
- Produces common Go functions/helpers that operate on shared Go model wrappers.
- Emits physical files under `output.dir/`.
- Hides raw iteration, callbacks, native error codes, and native calling conventions.

## Design principle

The generated wrapping package is not the place for business logic. Its job is to expose a stable, reusable shared SDK over the native SIL surface so DCM/HTD/other IE modules can share the same models and common APIs.

## Facade design direction

Facade design is now anchored to the **actual SIL call surface**, not abstract naming alone.

Primary reference surface:
- `src/IE/SIL/iSiLib.h`
- `iSiLib-ini.h` should also be included once its local path is confirmed

The key principle is **type-driven facade routing**:

- if a facade API fills a known model type from `files.model`
- and that model appears directly in the signature as an out-parameter
- then the wrapper layer should prefer generating a Go API that accepts the shared model wrapper directly

Examples of desired routing:
- `bool GetAAMaster(..., IsAAMaster& out)` -> `GetAAMaster(..., out *IsAAMaster) bool`
- `bool GetAAMaster(..., IsAAMaster* out)` -> `GetAAMaster(..., out *IsAAMaster) bool`

Why wrapper-first `*Model` + raw `bool`:
- the native object must stay mutable through the original handle
- preserving the wrapper shape avoids reverse DTO-to-handle reconstruction
- the raw `bool` semantics stay explicit instead of being reinterpreted by the generator

For the current facade slice, the design now applies **model-aware routing first**:
- if the API is tied to a known shared model type, it can be routed to handle-backed facade generation
- if it is not mapped to a known model type, it should remain a regular API when otherwise supported
- pattern naming alone should not be treated as the primary decision source
- source implementation details must not be used to infer higher-level helper behavior

## Current implementation note (2026-03-19)

- Raw native wrapper generation is implemented and remains the current stable base layer.
- File-level classification config now exists via `files.model` and `files.facade`.
- A dedicated Go model rendering path now exists beside raw wrapper generation.
- Current classification effect is still intentionally partial:
  - model/facade semantic classification is determined only by explicit config (`files.model`, `files.facade`).
  - `model` headers can emit Go enum models and auto-project `IsAAMaster`-style getter/setter classes into handle-backed Go wrappers.
  - `facade` headers now generate phase-1 Go facade wrappers and still do not emit Go model files.
  - generated wrapper and Go files now emit together under `output.dir/`.
  - the base supported facade surface includes primitive/string free functions plus known-model `Model&` / `Model*` params routed as `*Model` wrappers.
  - facade class methods preserve raw `bool`/primitive/string returns instead of lifting known-model out-params into DTO-style return values.
  - unknown non-classified model reference/pointer declarations can now remain in raw wrapper output as opaque handles when the raw renderer can express them safely.
  - the same unknown model declarations are still filtered out from Go facade/model projection layers unless they map to `files.model`.
  - raw-unsafe by-value object declarations are now skipped at declaration level and recorded in `support.skipped_declarations` instead of aborting the whole header.
  - known-model Go helpers now enforce `Model&` as non-nil live handles and allow `Model*` to pass `nil` through when requested.
  - overloaded raw wrapper symbols are now disambiguated deterministically from parameter signatures instead of aborting normalization.
  - overloaded Go facade exports are also disambiguated for renderable methods such as `GetAAMasterUint32(...)` versus `GetAAMasterString(...)`.
  - namespaced facade functions that would collide in Go export names are rejected during generation.
- Typedef/DTO model generation, model-mapped collection facade generation, callback facade generation, and richer type-driven facade lifting beyond the first out-param pattern are not implemented yet.
- Review note (2026-03-25): the current durable real-SIL evidence still supports keeping `IsAAMaster.h` as the only verified checked-in `files.model` path. Raw-visible types such as `IsCluster` and `IsCSTASession` remain non-onboarded until a narrower public-model case is proven.
