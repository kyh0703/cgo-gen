# Model-Aware Facade Routing Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor facade generation so known model out-params are routed through an explicit model-mapped analysis path, while all other supported facade methods remain on the regular API path without name-based or source-based inference.

**Architecture:** Keep raw wrapper generation unchanged and refactor only the facade Go generation pipeline. Split `src/facade.rs` into an analysis phase that classifies facade methods from IR + known model projections and a rendering phase that consumes the analyzed result. Preserve existing Go output shapes for supported cases and lock the routing rule with regression tests before updating docs.

**Tech Stack:** Rust, cargo test, libclang-backed parser, Go facade string generation, markdown docs

---

## Preflight Notes

- The accepted spec is `docs/superpowers/specs/2026-03-17-model-aware-facade-routing-design.md`.
- The current workspace hit a macOS runtime error while running tests because `libclang.dylib` was not found. Before any `cargo test` step in this plan, run:

```bash
export DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}"
```

Then confirm the shell can see the runtime library with (adjust the Xcode path if your machine differs):

```bash
ls /Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/libclang.dylib
```
- Keep scope tight: do **not** add iterator helpers, collection helpers, callback helpers, or name-based routing in this plan.

## File Map

**Primary implementation**
- Modify: `src/facade.rs` — split method classification from rendering; introduce analyzed facade structures; preserve existing supported output shapes.

**Regression tests**
- Modify: `tests/facade_generate.rs` — add type-based routing tests and keep existing model-mapped generation coverage.
- Modify: `tests/multi_header_generate.rs` — extend only if needed to prove multi-header routing still respects `files.model` and `files.facade`.

**Docs**
- Modify: `README.md` — describe this phase as model-aware routing cleanup, not collection inference.
- Modify: `.context/roadmap.md` — replace the immediate-next wording with the approved routing-focused milestone.
- Modify: `.context/architecture.md` — remove collection-oriented language for this slice and align the narrative with routing-only scope.

**Spec reference**
- Reference only: `docs/superpowers/specs/2026-03-17-model-aware-facade-routing-design.md`

### Task 1: Lock the routing rule with failing facade tests

**Files:**
- Modify: `tests/facade_generate.rs:144-230`
- Test: `tests/facade_generate.rs`

- [ ] **Step 1: Add a failing test for “known model missing means no model-mapped routing”**

```rust
#[test]
fn does_not_lift_methods_without_known_model_out_params() {
    // Build a fixture where method names look like lookups,
    // but no files.model entry exists for the signature.
    // Assert the generated Go output does not contain `(ThingModel, error)`.
}
```

- [ ] **Step 2: Add a failing test for “known model outside the final supported out-param position does not lift”**

```rust
#[test]
fn does_not_lift_when_known_model_is_not_the_final_supported_out_param() {
    // Put a known model somewhere else in the signature and assert
    // the method is not routed to the model-mapped renderer.
}
```

- [ ] **Step 3: Run just the new tests and confirm they fail for the right reason**

Run:
```bash
cargo test does_not_lift_methods_without_known_model_out_params --test facade_generate -- --nocapture
cargo test does_not_lift_when_known_model_is_not_the_final_supported_out_param --test facade_generate -- --nocapture
```

Expected:
- FAIL because current routing logic is still mixed into ad-hoc collection/render flow and the new assertions are not yet satisfied.

- [ ] **Step 4: Re-run the existing lifted-method regression to establish the safety baseline**

Run:
```bash
cargo test lifts_known_model_out_param_methods_into_model_returning_facade_methods --test facade_generate -- --nocapture
```

Expected:
- Existing test behavior is understood before refactoring `src/facade.rs`.

- [ ] **Step 5: Commit the test-only red state**

```bash
git add tests/facade_generate.rs
git commit -m "test: lock facade routing rules"
```

### Task 2: Introduce explicit facade analysis structures

**Files:**
- Modify: `src/facade.rs:14-150`
- Test: `tests/facade_generate.rs`

- [ ] **Step 1: Replace or supplement the current `FacadeClass` / `LiftedMethod` model with analyzed structures**

```rust
#[derive(Debug)]
struct AnalyzedFacadeClass<'a> {
    go_name: String,
    handle_name: String,
    constructor: &'a IrFunction,
    destructor: &'a IrFunction,
    general_methods: Vec<&'a IrFunction>,
    model_mapped_methods: Vec<ModelMappedMethod<'a>>,
}

#[derive(Debug)]
struct ModelMappedMethod<'a> {
    function: &'a IrFunction,
    model: KnownModelProjection,
}
```

- [ ] **Step 2: Extract a helper that analyzes one method without rendering it**

```rust
fn classify_facade_method(
    config: &Config,
    function: &IrFunction,
) -> Option<AnalyzedMethod<'_>> {
    // Return GeneralApi, ModelMapped, or None.
}
```

- [ ] **Step 3: Keep the classification rule narrow and spec-aligned**

```rust
// ModelMapped only when:
// - liftable_method_supported(function)
// - final param is model_out_param(function)
// - config.known_model_projection(...) resolves
// - model constructor / destructor support exists
```

- [ ] **Step 4: Rebuild `collect_facade_classes(...)` around the analyzed structures**

Run:
```bash
cargo test lifts_known_model_out_param_methods_into_model_returning_facade_methods --test facade_generate -- --nocapture
```

Expected:
- Still failing or partially passing is acceptable while rendering is mid-refactor, but the file should compile.

- [ ] **Step 5: Commit the analysis-layer refactor**

```bash
git add src/facade.rs
git commit -m "refactor: separate facade method analysis"
```

### Task 3: Rewire rendering to consume analyzed classifications

**Files:**
- Modify: `src/facade.rs:153-760`
- Test: `tests/facade_generate.rs`

- [ ] **Step 1: Update `render_go_facade(...)` to render from analyzed classes instead of in-line classification**

```rust
let classes = collect_facade_classes(config, ir)?;
let contents = render_go_facade_file(config, &functions, &classes);
```

- [ ] **Step 2: Rename rendering helpers to reflect routing boundaries**

```rust
fn render_general_api_method(...)
fn render_model_mapped_method(...)
```

- [ ] **Step 3: Update include/model-mapper collection helpers to use analyzed model-mapped methods only**

```rust
fn collect_used_models(classes: &[AnalyzedFacadeClass<'_>]) -> Vec<KnownModelProjection>
```

- [ ] **Step 4: Verify the Go output shape remains unchanged for existing supported lifted methods**

Run:
```bash
cargo test lifts_known_model_out_param_methods_into_model_returning_facade_methods --test facade_generate -- --nocapture
cargo test generates_go_facade_for_bool_and_string_returns --test facade_generate -- --nocapture
```

Expected:
- PASS for existing supported facade generation behavior.

- [ ] **Step 5: Verify the new negative-routing tests now pass**

Run:
```bash
cargo test does_not_lift_methods_without_known_model_out_params --test facade_generate -- --nocapture
cargo test does_not_lift_when_known_model_is_not_the_final_supported_out_param --test facade_generate -- --nocapture
```

Expected:
- PASS because routing is now explicitly type-based and conservative.

- [ ] **Step 6: Commit the rendering rewire**

```bash
git add src/facade.rs tests/facade_generate.rs
git commit -m "refactor: route facade generation by analyzed method kind"
```

### Task 4: Confirm multi-header behavior and preserve free-function stability

**Files:**
- Modify: `tests/multi_header_generate.rs:113-258` (only if coverage is missing)
- Test: `tests/multi_header_generate.rs`

- [ ] **Step 1: Inspect whether existing multi-header tests already prove the accepted rule**

Check:
```bash
rg -n "files\.model|files\.facade|wrapper.go|model" tests/multi_header_generate.rs
```

Expected:
- Determine whether current coverage is enough to prove that `files.model` remains the sole model-routing source of truth.

- [ ] **Step 2: If coverage is insufficient, add one focused regression test**

```rust
#[test]
fn facade_routing_still_depends_on_model_header_classification_in_multi_header_generation() {
    // Assert a facade header only gets model-mapped output when the model type
    // comes from a header classified in files.model.
}
```

- [ ] **Step 3: Re-run the targeted multi-header regression**

Run:
```bash
cargo test multi_header --test multi_header_generate -- --nocapture
```

Expected:
- PASS with no regression to per-header generation behavior.

- [ ] **Step 4: Re-run the simple free-function facade test to ensure non-model routing still works**

Run:
```bash
cargo test generates_go_facade_for_simple_free_function_header --test facade_generate -- --nocapture
```

Expected:
- PASS; free-function facade support remains unchanged.

- [ ] **Step 5: Commit any coverage additions**

```bash
git add tests/multi_header_generate.rs tests/facade_generate.rs
git commit -m "test: preserve facade routing across multi-header generation"
```

### Task 5: Update docs to match the approved scope

**Files:**
- Modify: `README.md:124-193`
- Modify: `.context/roadmap.md:36-78`
- Modify: `.context/architecture.md`

- [ ] **Step 1: Update README language from collection direction to routing direction for this phase**

```md
- known model out-param methods are routed through model-mapped facade generation
- methods without known model types remain regular APIs when otherwise supported
- this phase does not add iterator/list inference
```

- [ ] **Step 2: Update roadmap immediate-next wording**

```md
Immediate next target:
- separate model-aware facade routing from rendering and lock it with regression tests
```

- [ ] **Step 3: Update `.context/architecture.md` so this slice is described as routing cleanup, not collection helper work**

- [ ] **Step 4: Run a quick diff review and markdown sanity check**

Run:
```bash
git diff -- README.md .context/roadmap.md .context/architecture.md
```

Expected:
- Docs describe routing cleanup and type-based mapping only.

- [ ] **Step 5: Commit the documentation updates**

```bash
git add README.md .context/roadmap.md .context/architecture.md
git commit -m "docs: clarify model-aware facade routing scope"
```

### Task 6: Final verification sweep

**Files:**
- Modify: none expected
- Test: `tests/facade_generate.rs`, `tests/multi_header_generate.rs`

- [ ] **Step 1: Run the focused regression suite for this feature**

Run:
```bash
cargo test --test facade_generate -- --nocapture
cargo test --test multi_header_generate -- --nocapture
```

Expected:
- PASS for the new routing-focused coverage and the existing facade generation coverage.

- [ ] **Step 2: If the shell environment has libclang configured, run the broader project suite**

Run:
```bash
cargo test
```

Expected:
- PASS, or if the environment still lacks `libclang.dylib`, record that as an environment blocker rather than a code failure.

- [ ] **Step 3: Review the final diff for scope discipline**

Run:
```bash
git diff --stat -- src/facade.rs tests/facade_generate.rs tests/multi_header_generate.rs README.md .context/roadmap.md .context/architecture.md
```

Expected:
- Changes remain limited to facade routing, tests, and docs.

- [ ] **Step 4: Create the final implementation handoff commit if needed**

```bash
git add src/facade.rs tests/facade_generate.rs tests/multi_header_generate.rs README.md .context/roadmap.md .context/architecture.md
git commit -m "feat: clarify model-aware facade routing"
```
