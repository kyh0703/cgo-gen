# Model-Aware Facade Routing Design

**Date:** 2026-03-17

## Goal

Refactor facade generation so it performs a clear, reusable routing decision based only on explicit type information in the normalized signature:

- if a known model type from `files.model` appears in the supported model out-param position, treat the facade method as model-mapped
- otherwise treat it as a regular API method

The immediate goal is to make this routing explicit, testable, and reusable without inferring behavior from method names, source implementation, or SIL-specific conventions.

## Current Context

The current code in `src/facade.rs` already supports two different generated Go shapes for facade class methods:

1. general methods returning primitive / string-like values
2. lifted methods where the final parameter is a known model out-param (`Model&` / `Model*`) and the Go API returns `(Model, error)`

That behavior works today, but the classification logic is mixed into collection and rendering flow. The next work should not jump directly into iterator or list helper generation. First, the generator needs a stable model-aware routing layer that is generic across inputs.

## Approved Design Principles

### 1. Type-based only

Routing decisions must be based only on explicit signature information already present in the IR and config.

Allowed signals:
- `files.model`
- known model projections derived from `files.model`
- explicit normalized IR parameter kinds such as `model_reference` and `model_pointer`
- explicit C++ type names attached to normalized IR nodes

### 2. No source inference

The generator must not inspect source implementation details or infer runtime behavior from how a method appears to work internally.

### 3. No name-based behavior inference

Method names such as `List`, `Next`, `Select`, `Find`, `Enum`, or similar must not determine whether an API is treated as collection-oriented or model-oriented.

Names may still be used only for Go symbol rendering after classification is already decided.

### 4. Generic behavior

The routing logic must remain generic for future non-SIL headers. It must not hardcode SIL-specific model names, method names, or source file assumptions.

## Scope

### In scope
- refactor facade analysis so routing is explicit before rendering
- classify facade class methods into two buckets:
  - general API methods
  - model-mapped methods
- keep current supported model-mapped output shape for known model out-params:
  - `bool Foo(..., Model& out)` -> `Foo(...) (Model, error)`
  - `bool Foo(..., Model* out)` -> `Foo(...) (Model, error)`
- add tests that lock type-based routing behavior
- update docs to reflect that this phase is about model-aware routing, not collection inference

### Out of scope
- iterator helper generation
- `[]Model` collection helper generation
- callback helper generation
- name-driven lifting such as `List*`, `Next*`, `Select*`
- source-driven or behavior-driven inference
- automatic support for unknown model types
- broad free-function redesign beyond preserving current behavior

## High-Level Architecture

The facade pipeline should be split into two conceptual phases.

### Phase A: analysis

Analyze facade functions and methods first, producing an intermediate description of what each renderable item is.

For facade class methods, analysis should answer:
- is this method renderable at all?
- if renderable, is it a general API method?
- if renderable, is it a model-mapped method backed by a known model projection?

This phase owns classification only. It should not generate Go strings.

### Phase B: rendering

Render Go code from the analyzed structures.

Rendering should consume already-decided classifications and should not repeat routing logic except for simple formatting concerns.

## Proposed Internal Decomposition

The exact type names can vary, but the design should separate the following responsibilities inside `src/facade.rs`.

### 1. Facade class analysis

Introduce an analysis step for each facade class that gathers an analyzed facade artifact before rendering. That artifact should carry:
- constructor
- destructor
- renderable general methods
- renderable model-mapped methods

The current `FacadeClass` structure can evolve into a more analysis-oriented shape, or a new analyzed structure can be introduced before rendering.

### 2. Method classification

Create a classification path that distinguishes:
- `GeneralApiMethod`
- `ModelMappedMethod`

Classification rules for this phase:
- if the method matches the current supported lifted-method pattern and its out-param resolves to a known model projection, classify as `ModelMappedMethod`
- otherwise, if it matches the existing primitive/string general-method support, classify as `GeneralApiMethod`
- otherwise, do not render it

### 3. Model mapping lookup

Known model lookup must continue to come from `Config::known_model_projection(...)`, which is built from headers explicitly classified in `files.model`.

That keeps `files.model` as the sole semantic source of truth for model-aware routing.

### 4. Rendering boundary

Rendering functions should become narrower and more explicit, for example:
- render analyzed facade class shell
- render general API method
- render model-mapped method

The important part is the boundary, not the final function names.

## Detailed Routing Rules

### Rule 1: model-mapped routing

Treat a facade class method as model-mapped only when all of the following are true:
- the method is currently liftable by the existing supported pattern
- the return type is the currently supported success flag shape (`bool` in normalized primitive form)
- the final parameter is a normalized known model out-param
- that out-param resolves to a known model projection from `files.model`
- the resolved model projection has the required wrapper constructor and destructor symbols for current handle-based mapping

If any of those checks fail, the method must not be classified as model-mapped.

### Rule 2: general API routing

Treat a facade class method as a general API method when:
- it is not classified as model-mapped
- it matches existing supported general method constraints

This preserves current primitive / string-oriented behavior.

### Rule 3: no extra meaning

Even if multiple methods mention the same model type, this phase must not infer collection semantics, iteration semantics, or lifecycle semantics from that fact.

The result is only:
- model-mapped API
- regular API

Nothing more.

## Data Flow

1. `generator::prepare_config` builds known model projections from `files.model`
2. `facade::render_go_facade` inspects only headers classified as `Facade`
3. facade analysis classifies class methods using IR signature data plus known model projections
4. rendering consumes the analyzed result
5. generated Go output remains the same for already-supported cases, but the internal routing is cleaner and better isolated

## Error Handling

This phase should preserve existing failure behavior where possible.

Examples:
- facade class with renderable methods but no constructor wrapper: error
- facade class with renderable methods but no destructor wrapper: error
- constructor parameters not yet supported: error
- model-mapped method with missing allocation support in the known projection: not model-mapped; if no other supported path applies, it is omitted from rendering

The key point is to avoid inventing new inference-based fallback behavior.

## Testing Strategy

Add or update regression tests to prove the routing rules.

### Required tests

1. **Known model out-param remains model-mapped**
   - input method with supported `Model&` and `Model*` out-param shapes
   - expected Go output returns `(Model, error)`

2. **No known model means no model-mapped routing**
   - input method names may resemble lookup or collection patterns
   - if no known model type is present in the supported out-param position, the generator must not produce model-mapped output
   - such methods stay on the general API path only when they independently satisfy existing general-method support rules

3. **Names do not control routing**
   - method names like `ListThing`, `NextThing`, or similar should not cause special lifting by themselves

4. **Known model outside the supported out-param position does not lift**
   - if a known model type appears somewhere else in the signature, that alone must not trigger model-mapped routing

5. **Multi-header classification still holds**
   - `files.model` drives known model projection availability
   - `files.facade` drives facade generation
   - the routing behavior must remain consistent under multi-header generation

## Documentation Updates

Update docs so the roadmap and README describe this phase accurately:
- this step strengthens model-aware facade routing
- this step does not yet add collection helpers
- `files.model` remains the only semantic source of truth for model mapping

## Acceptance Criteria

The work is complete when all of the following are true:

1. facade method classification is separated from Go rendering
2. known model type presence in the supported signature shape routes methods to the model-mapped path
3. absence of a known model type leaves methods on the general API path only when otherwise supported by existing general-method rules
4. no routing decision depends on method names or source implementation inference
5. existing `(Model, error)` lifted output continues to work for supported out-param cases
6. regression tests cover the type-based routing rule explicitly
7. docs describe the phase as routing cleanup and model-aware mapping, not collection inference

## Deferred Follow-Up

Once this routing layer is stable, a later spec can decide whether any iterator or collection helper should exist. That later work must still preserve the same core rule: known model types may enable richer APIs, but names and source inference are not enough on their own.
