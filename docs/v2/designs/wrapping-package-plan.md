# Wrapping Package Plan

## Objective

Build a shared wrapping package over the IE native SIL surface so multiple Go-based IE process modules can reuse the same:
- native wrapper layer
- shared Go models
- shared Go facade APIs

Business logic stays in each process module. The wrapping package owns only common native integration and common data/API translation.

## Input classification strategy

Start with file-level classification.

```yaml
files:
  model:
    - SIL/sil_types.h
    - SIL/sil_data.h
    - SIL/IsAAMaster.h
    - SIL/IsAAUser.h

  facade:
    - SIL/iSiLib.h
    - SIL/sil_wrapper.h
```

### Meaning
- `model`: generate shared Go types or class projections.
- `facade`: generate shared Go functions that call the raw layer and return shared Go types.

This is intentionally file-first. Symbol-level overrides can be added later only if a mixed file becomes a real problem.

## Target output shape

### 1. Raw native output
Generated first.

Examples:
- `sil_wrapper.h`
- `sil_wrapper.cpp`
- `is_aa_master_wrapper.h`
- `is_aa_master_wrapper.cpp`
- `is_aa_user_wrapper.h`
- `is_aa_user_wrapper.cpp`

Purpose:
- flatten C++ classes/functions into C ABI
- provide stable interop boundary
- support later Go generation

### 2. Shared Go model output
Generated from `model` files.

Examples:
- shared enum/type definitions
- `Webhook`
- `WebhookEvent`
- `AAMaster`
- `AAUser`

Purpose:
- define common data contracts for IE modules
- centralize projection rules
- avoid repeated DTO/model duplication in each module

### 3. Shared Go facade output
Generated from `facade` files.

Examples:
- `Init`
- `Clear`
- `IsInit`
- `SetHACallback`
- `NextWebhook`
- `ListWebhooks`

Purpose:
- hide native/raw details
- return shared Go models
- centralize common native access patterns

## Revised facade strategy

Facade generation should now follow the real `iSiLib` usage surface rather than an abstract taxonomy first.

Reference:
- `src/IE/SIL/iSiLib.h`
- `iSiLib-ini.h` should be folded into the same review once its local path is confirmed

### Core rule

If a facade API uses a known shared model type directly in its out-parameter, the generator should lift it into a model-returning Go facade.

Examples:
- `bool GetAAMaster(..., IsAAMaster& out)`
- `bool GetAAMaster(..., IsAAMaster* out)`

Desired generated shape:

```go
func GetAAMaster(...) (IsAAMaster, error)
```

Defaulting to `(Model, error)` is intentional:
- the native `bool` is not assumed to mean `found/not found`
- it may encode generic success/failure instead

### Collection helpers should be model-mapped first

Collection/list/iterator helpers should not be introduced from pattern grouping alone.

Instead:
- known shared model mapping is the first gate
- if an API is clearly mapped to a known model type, it can be lifted further as a collection/helper candidate
- if it is not model-mapped, it stays a regular API

## Responsibility boundary

### Wrapping package owns
- native wrapper generation
- common Go model generation
- common Go facade generation
- raw/native-to-Go mapping
- callback bridge handling
- iterator/list helper handling
- error/result normalization

### IE process modules own
- business rules
- workflow orchestration
- module-specific domain models derived from shared models
- process-specific interpretation of shared SIL data

## Plan stages

### Stage 0 - classification baseline
- finalize the file-level `model` / `facade` list
- confirm source-of-truth headers
- identify generated-vs-source headers that should not become primary input by mistake

### Stage 1 - raw wrapper baseline
- generate raw wrapper files for facade/class inputs
- validate compileability of generated wrapper headers/sources
- establish file naming rules per input header

### Stage 2 - shared model generation
- generate Go enums/typedef mappings from `sil_types.h`
- generate Go DTOs from `sil_data.h`
- generate class projection models from `IsAAMaster.h`, `IsAAUser.h`
- define stable naming rules for generated Go models

### Stage 3 - shared facade generation
- generate lifecycle APIs (`Init`, `Clear`, `IsInit`)
- generate iterator-based facade helpers (`NextWebhook`, `ListWebhooks`)
- generate callback registration APIs
- ensure facade functions return shared Go models instead of raw/native values

### Stage 4 - mapping and ownership rules
- standardize string conversion rules
- standardize bool/enum/result handling
- define allocation/free ownership boundaries
- define handle-vs-value projection rules

### Stage 5 - verification
- verify raw wrapper compile success
- verify generated Go output shape
- verify model/facade layer boundaries
- run fixture-based regression tests for representative SIL headers

## Initial success criteria

- file-level classification drives generation without manual symbol picking
- `model` files generate reusable shared Go types
- `facade` files generate reusable shared Go APIs
- raw/native details do not leak into downstream IE business modules
- at least one iterator-style SIL API is exposed as a Go-friendly facade returning a shared model

## Implementation checkpoint (2026-03-16)

### Done
- file-level classification is now implemented in config:

```yaml
files:
  model:
    - path/to/model.hpp
  facade:
    - path/to/facade.hpp
```

- classification paths are resolved relative to the config file just like `input.headers`
- classified files must also exist inside `input.headers`
- the same header cannot be both `model` and `facade`
- in multi-header generation, current Go projection output is emitted only for `model` headers
- `facade` headers still participate in raw wrapper generation

### Not done yet
- typedef alias-based model output is not implemented yet
- DTO/POD-style model output is not implemented yet
- model-mapped collection facade generation is not implemented yet
- callback facade generation is not implemented yet
- facade APIs do not yet lift `iSiLib`-style model out-params into shared generated model returns

### Resume-from-here plan
1. keep `files.model` / `files.facade` as the source of truth
2. extend model generation beyond current enum/class projection:
   - enums
   - typedef aliases
   - POD-like DTOs where possible
3. extend facade generation beyond current phase-1 free-function coverage:
   - single-model lifting from `Model&` / `Model*` out-params
   - model-mapped collection helper generation
   - callback registration helpers

### Guardrails for the next session
- do not remove or bypass raw wrapper generation; it remains the base layer
- prefer file-level routing first; avoid symbol-level overrides unless a mixed-header case forces it
