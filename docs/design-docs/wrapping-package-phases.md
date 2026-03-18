# Wrapping Package Execution Phases

## Phase 1 - Decide inputs
- lock the first `model` file set
- lock the first `facade` file set
- confirm include paths and compile environment

### Current status (2026-03-16)
- config support for `files.model` and `files.facade` is implemented
- headers cannot belong to both roles
- classified headers must also appear in `input.headers`
- `files.model` now directly drives Go enum/class model output
- `files.facade` now directly drives phase-1 free-function Go facade output
- next follow-up work should extend facade generation into model-mapped collection/callback helpers

## Phase 2 - Raw generation
- keep the current wrapper generator as the base layer
- ensure per-header raw wrapper generation is stable
- verify wrapper naming/output rules

## Phase 3 - Model generation
- map `model` files to shared Go output
- generate enums, typedefs, POD-like structs, and class projections
- keep the generated model layer free of business semantics

## Phase 4 - Facade generation
- map `facade` files to shared Go APIs
- make facade APIs consume raw/native output internally
- return shared generated models publicly

### Refined implementation order
1. phase-1 free-function facade (done)
2. single-model lifting from known model out-parameters
3. model-mapped collection helper generation
5. callback helper generation

## Phase 5 - Hardening
- add ownership and conversion policies
- add regression fixtures for representative SIL headers
- freeze stable public generation rules before wider IE rollout

## Phase 6 - Adoption
- let IE modules import the shared wrapping package
- keep module business logic separate
- prohibit direct raw/native access from module business code

## Next recommended starting point

When resuming work, start here:
1. inspect `iSiLib.h` methods that use known `files.model` classes as out-parameters
2. implement model-returning facade lifting for `bool Foo(..., Model&/* out)` -> `Foo(...) (Model, error)`
3. extend only already model-mapped APIs into collection helpers; do not treat pattern grouping as the primary source of truth
