# Roadmap

## Current baseline
- Rust CLI skeleton
- YAML config loader
- IR emission
- native wrapper generation
- fixture tests
- libclang parser backend
- richer type mapping
- overload collision detection
- stronger diagnostics
- per-header generation support
- class projection support for selected getter/setter models

## Next plan

### Phase 1 - file classification
- [x] add file-level `model` / `facade` classification to config
- [x] validate that classified files are also present in `input.headers`
- [x] reject overlapping `model` + `facade` classification for the same header
- [x] treat file classification as the first source of generation intent for current Go projection gating
- [ ] propagate file classification deeper into dedicated model/facade generation pipelines

### Phase 2 - raw wrapper stabilization
- keep raw wrapper generation as the base output layer
- stabilize per-header native wrapper naming and layout
- separate source headers from generated wrapper headers in planning/config

### Phase 3 - shared Go model generation
- [x] generate shared Go enums from model files
- [ ] generate typedef mappings from model files
- generate shared Go DTOs from model files
- generate class-to-model projections from selected model-class headers

### Phase 4 - shared Go facade generation
- [x] generate phase-1 common Go free-function APIs from facade files
- [x] support primitive/bool/string return handling in phase-1 facade output
- [x] reject Go export collisions for namespaced facade functions
- [x] type-driven single-model facade lifting from `iSiLib`-style out-params
- [ ] model-mapped collection facade generation
- [ ] callback helper generation
- [ ] make facade output depend on shared generated models instead of raw/native values

### Phase 5 - verification and rollout
- add representative SIL fixture coverage for file-classified generation
- verify model/facade separation in generated outputs
- prepare the shared wrapping package for IE module adoption

## Current checkpoint (2026-03-16)

Completed in code:
- config-level `files.model` / `files.facade`
- generation-time role lookup per scoped header
- dedicated Go model rendering module separated from raw wrapper generation
- enum model emission for `model`-classified headers
- model-class auto projection for `IsAAMaster`-style getter/setter headers
- `files.model` as the sole semantic source of truth for model output routing
- phase-1 facade Go wrapper generation for supported free functions
- bool/string/c_string facade return support with regression tests
- facade export collision detection with regression tests
- model out-param recognition in IR/raw wrapper generation
- `bool Foo(..., Model&/* out)` -> `Foo(...) (Model, error)` facade lifting with regression tests
- regression tests for classification loading, validation, and multi-header behavior

Immediate next target:
- separate model-aware facade routing from rendering and lock the type-based rule with regression tests

Detailed next steps:
1. split facade method analysis from Go rendering in `src/facade.rs`
2. classify facade class methods by explicit signature only:
   - known supported model out-param -> model-mapped API
   - otherwise supported primitive/string method -> general API
3. keep `files.model` as the sole semantic source of truth for model-aware routing
4. add regression coverage for:
   - `Model&` out-param
   - `Model*` out-param
   - names that look like lookup/list APIs but have no known model type
   - known model type outside the final supported out-param position
5. defer iterator/list helper inference until after routing cleanup is stable
