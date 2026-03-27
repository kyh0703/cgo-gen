---
version: v2
status: completed
source: docs/v2/completed/2026-03-18-raw-first-unknown-model-filtering.md
---
# Raw-First Unknown-Model Filtering Plan

## Why this plan exists

The current workspace contains a temporary safety fix in `/Users/kyh0703/Project/cgo-gen/src/ir.rs` that skips whole declarations when a method or free function references an unknown non-classified model type. That stops generator aborts, but it also drops raw/C wrapper coverage for APIs that could still be represented as opaque-handle-based raw wrappers.

The intended behavior is stricter at the Go layer, not at the raw layer:

1. keep raw/C wrapper generation for unknown model reference/pointer types when the raw renderer can still express them safely
2. keep known-model lifting constrained to `files.model`
3. filter unknown-model declarations only from Go model/facade projection layers

Current grounding:
- `/Users/kyh0703/Project/cgo-gen/src/ir.rs` has `unknown_model_reason(...)`-based declaration skipping
- `/Users/kyh0703/Project/cgo-gen/src/generator.rs` already renders `model_reference` and `model_pointer` via opaque handles in `render_cpp_arg(...)`
- `/Users/kyh0703/Project/cgo-gen/src/facade.rs` already treats only known model projections as lift candidates
- current worktree also contains doc-sync edits plus the temporary `src/ir.rs` / `tests/facade_generate.rs` changes

Planning gaps discovered while preparing this plan:
- `/Users/kyh0703/Project/cgo-gen/docs/AGENTS.md` is missing
- `/Users/kyh0703/Project/cgo-gen/docs/PLANS.md` is missing

## Outcome

1. Raw IR and raw wrapper generation retain unknown model reference/pointer declarations when they are raw-safe.
2. Go model generation remains limited to `files.model`.
3. Go facade generation excludes unknown-model declarations instead of forcing declaration-level IR skips.
4. Verification proves the same declaration can survive in raw output while being absent from Go facade output.

## Scope

In scope:
- IR normalization behavior for unknown model reference/pointer types
- raw renderer support validation for unknown opaque-handle-style model types
- Go facade filtering rules for unknown model reference/pointer declarations
- regression tests that prove raw survives while Go facade filters
- doc updates for the new behavior

Out of scope:
- by-value unknown model arguments or returns beyond explicit follow-up notes
- real IPRON include-environment repair
- callback support
- collection helper generation
- raw/output directory layout refactors

## Design constraints

- `files.model` remains the only semantic source of truth for model-aware Go lifting.
- Unknown model reference/pointer types may be raw-safe, but they are not Go-model-safe by default.
- Any declaration that cannot be represented safely in raw output must still fail or be skipped explicitly with evidence.
- Keep diffs narrow and avoid rewriting the raw renderer if the existing `model_reference` / `model_pointer` path already covers the use case.

## Files to read first

- `/Users/kyh0703/Project/cgo-gen/src/ir.rs`
- `/Users/kyh0703/Project/cgo-gen/src/generator.rs`
- `/Users/kyh0703/Project/cgo-gen/src/facade.rs`
- `/Users/kyh0703/Project/cgo-gen/src/model.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/model_out_params.rs`
- `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`
- `/Users/kyh0703/Project/cgo-gen/docs/v2/research/roadmaps/current-roadmap.md`
- `/Users/kyh0703/Project/cgo-gen/docs/v2/research/status/sil-conversion-status.md`

## Task 1: Lock the intended raw-first behavior with regression tests

Owner: executor
Parallelizable: partial
Integration: tests define the target behavior before IR/facade changes land

### 1.1 Review the current temporary skip behavior

Read:
- `/Users/kyh0703/Project/cgo-gen/src/ir.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`

Commands:
```bash
rg -n "unknown_model_reason|unknown_model_type_name|unknown_model_candidate_name" /Users/kyh0703/Project/cgo-gen/src/ir.rs
rg -n "unknown-model|GetUnknown|ThingModel|Api" /Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs
```

Expected result:
- confirm that declaration skipping currently happens in IR normalization
- identify the existing regression that currently asserts declaration absence

### 1.2 Replace the current facade-only regression with a raw-first regression

Modify:
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`

Add or rewrite one focused test so the same fixture proves:
- raw header contains a wrapper symbol for a method with an unknown `T&` or `T*`
- raw source contains the wrapper implementation
- generated Go facade does **not** contain that method
- a known-model method in the same class still lifts correctly

Suggested fixture shape:
```cpp
class ThingModel { ... };
class UnknownThing { ... };
class Api {
public:
    Api() = default;
    ~Api() = default;
    int Count() const;
    bool GetThing(int id, ThingModel& out);
    bool GetUnknown(int id, UnknownThing& out);
};
```

Expected result:
- the test fails against the current temporary declaration skip because the raw wrapper symbol for `GetUnknown` is absent

### 1.3 Add a direct IR assertion if needed

Modify only if the previous test is not specific enough:
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`

Target:
- assert that `GetUnknown` still appears in IR functions after the intended change
- assert that Go facade output omits it

Command:
```bash
env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" \
    CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" \
    DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" \
    cargo test --test facade_generate -- --nocapture
```

Expected result:
- a red test that specifically shows raw output must survive while Go facade filtering still applies

## Task 2: Move unknown-model filtering out of IR declaration skip

Owner: executor
Parallelizable: no
Integration: must compile before downstream renderer work

### 2.1 Narrow the IR-level skip to only raw-unsafe cases

Modify:
- `/Users/kyh0703/Project/cgo-gen/src/ir.rs`

Current problem:
- `unknown_model_reason(...)` runs before normalization finishes and drops declarations that could still be expressed in raw output

Target behavior:
- do not skip declarations just because a parameter or return type is an unknown model reference/pointer
- keep skipping function-pointer declarations and any truly raw-unsafe forms

Suggested substeps:
1. identify which parts of `unknown_model_reason(...)` cover reference/pointer versus by-value cases
2. remove or narrow the pre-normalization skip path for unknown `T&` / `T*`
3. preserve explicit failure or skip for raw-unsafe by-value unknown model types if the raw renderer cannot express them

Commands:
```bash
rg -n "normalize_method|normalize_function|unknown_model_reason" /Users/kyh0703/Project/cgo-gen/src/ir.rs
cargo check
```

Expected result:
- `normalize_method(...)` and `normalize_function(...)` no longer discard raw-safe unknown model reference/pointer declarations
- build still passes

### 2.2 Preserve opaque type collection for unknown reference/pointer models

Modify if needed:
- `/Users/kyh0703/Project/cgo-gen/src/ir.rs`

Check:
- `collect_referenced_opaque_types(...)`
- normalized `IrType.kind` / `handle` values produced by `normalize_type_with_canonical(...)`

Expected result:
- unknown model reference/pointer declarations still populate `opaque_types`
- raw header generation can emit `typedef struct UnknownThingHandle UnknownThingHandle;`

Command:
```bash
cargo test raw_unknown_model_reference --test facade_generate -- --nocapture
```

Expected result:
- the new red test moves from "missing wrapper symbol" toward the intended raw-preserved shape

## Task 3: Filter unknown models only in Go facade analysis

Owner: executor
Parallelizable: no
Integration: depends on IR preserving the declaration

### 3.1 Make facade analysis distinguish known versus unknown model out-params

Modify:
- `/Users/kyh0703/Project/cgo-gen/src/facade.rs`

Target behavior:
- `model_out_param(...)` may still see both known and unknown model refs/pointers
- `model_projection_for_out_param(...)` continues to return only known model projections
- declarations with unknown model refs/pointers are excluded from Go facade generation instead of being lifted or rendered as general APIs

Suggested substeps:
1. add a small helper that detects "has model ref/pointer but no known projection"
2. keep returning `None` from facade classification for those declarations
3. leave known-model lifting and primitive/string general API behavior unchanged

Commands:
```bash
rg -n "classify_facade_method|model_projection_for_out_param|model_out_param|general_method_supported" /Users/kyh0703/Project/cgo-gen/src/facade.rs
env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" \
    CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" \
    DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" \
    cargo test --test facade_generate -- --nocapture
```

Expected result:
- `GetUnknown(...)` remains absent from `.go`
- `GetThing(...)` still lifts into `(ThingModel, error)`
- primitive/string general APIs still render

### 3.2 Confirm Go model projection stays tied to `files.model`

Read:
- `/Users/kyh0703/Project/cgo-gen/src/model.rs`
- `/Users/kyh0703/Project/cgo-gen/src/generator.rs`

Expected result:
- no model projection logic should be added for unknown models
- unknown reference/pointer declarations do not leak into generated Go model structs

## Task 4: Verify raw output explicitly

Owner: executor
Parallelizable: yes with Task 5 doc drafting after code stabilizes
Integration: validates the design goal directly

### 4.1 Add assertions on generated raw files

Modify:
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`

Assertions to add:
- generated `.h` contains the wrapper declaration for the unknown-model method
- generated `.cpp` contains the wrapper definition using `reinterpret_cast<UnknownThing*>` or `*reinterpret_cast<UnknownThing*>`
- generated `.go` does not contain the same method

Expected result:
- the test proves "raw survives, Go filters"

### 4.2 Run targeted tests

Commands:
```bash
env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" \
    CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" \
    DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" \
    cargo test --test facade_generate -- --nocapture

env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" \
    CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" \
    DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" \
    cargo test recognizes_known_model_out_params_in_facade_wrappers --test model_out_params -- --nocapture
```

Expected result:
- raw-preservation regression passes
- known-model lifting regression still passes

## Task 5: Run project verification

Owner: executor
Parallelizable: no
Integration: final gate before docs lifecycle updates

Commands:
```bash
cargo fmt --check
cargo check
env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" \
    CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" \
    DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" \
    cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Expected result:
- full repository verification passes

## Task 6: Update docs to match the new layering rule

Owner: executor
Parallelizable: after Task 3 is stable
Integration: docs should describe the landed behavior, not the temporary skip

Docs to update:
- `/Users/kyh0703/Project/cgo-gen/README.md`
- `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`
- `/Users/kyh0703/Project/cgo-gen/docs/v2/research/roadmaps/current-roadmap.md`
- `/Users/kyh0703/Project/cgo-gen/docs/v2/research/status/sil-conversion-status.md`

Update goals:
- clarify that unknown model reference/pointer declarations can survive in raw wrappers
- clarify that Go facade/model generation remains conservative and `files.model`-driven
- note that by-value unknown model types are still a separate follow-up if not covered in this change

Commands:
```bash
rg -n "unknown model|raw|facade|files.model|skip" /Users/kyh0703/Project/cgo-gen/README.md /Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md /Users/kyh0703/Project/cgo-gen/docs/v2/research/roadmaps/current-roadmap.md /Users/kyh0703/Project/cgo-gen/docs/v2/research/status/sil-conversion-status.md
git diff -- /Users/kyh0703/Project/cgo-gen/README.md /Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md /Users/kyh0703/Project/cgo-gen/docs/v2/research/roadmaps/current-roadmap.md /Users/kyh0703/Project/cgo-gen/docs/v2/research/status/sil-conversion-status.md
```

Expected result:
- docs explain the raw-first layering and do not describe declaration-level unknown-model skip as the intended end state

## Task 7: Plan lifecycle cleanup

Owner: executor
Parallelizable: final step only
Integration: keeps active plan inventory accurate

Steps:
1. after implementation and verification, move this file to `/Users/kyh0703/Project/cgo-gen/docs/v2/completed/` if that directory exists, or create it as part of doc lifecycle cleanup if the team wants completed-plan tracking
2. if `/Users/kyh0703/Project/cgo-gen/docs/AGENTS.md` and `/Users/kyh0703/Project/cgo-gen/docs/PLANS.md` are still absent, note that future planner runs will continue to rely on repository-root guidance and ad hoc plan discovery

Expected result:
- active plans contain only unfinished work

## Risks and follow-up notes

- The current raw renderer is already suitable for reference/pointer cases, but not necessarily for by-value unknown model types. Treat by-value unknown model arguments and returns as a separate follow-up unless the implementation proves they can be expressed safely without broadening risk.
- Real IPRON verification is still blocked by upstream include-path gaps and the `iSiLib.h` / `iSiLib-inl.h` include cycle. This plan intentionally focuses on cgo-gen layering behavior, not on repairing the external source tree.
