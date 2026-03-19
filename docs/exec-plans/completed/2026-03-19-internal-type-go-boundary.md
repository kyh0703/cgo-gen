# Internal-Type Go Boundary Plan

## Why this plan exists

The latest direction is that types such as `NsMap` should be treated as internal/native-side types, not as part of the generated Go-facing SDK contract.

That direction matches the current layered design:

1. raw/C wrapper coverage may remain broader when the raw layer can safely express the declaration
2. Go-facing model/facade output must stay constrained to `files.model`
3. internal or non-onboarded SIL types should not leak into generated Go APIs just because they appear in facade signatures

The currently in-progress raw-first work already covers unknown reference/pointer model types well enough for this rule at the Go boundary. The unresolved part is the follow-up policy for raw-unsafe by-value unknown types and the real `iSiLib` verification pass that will reveal concrete examples such as `NsMap`.

Planning gaps discovered while preparing this plan:
- `/Users/kyh0703/Project/cgo-gen/docs/AGENTS.md` is missing
- `/Users/kyh0703/Project/cgo-gen/docs/PLANS.md` is missing
- `/Users/kyh0703/Project/cgo-gen/docs/references/exec-plan-template.md` is missing
- `/Users/kyh0703/Project/cgo-gen/docs/references/plan-quality-checklist.md` is missing
- `/Users/kyh0703/Project/cgo-gen/docs/references/docs-update-rules.md` is missing
- `NsMap` does not appear in the repository today; treat it as a real-SIL example from the external IPRON header surface, not as an in-repo fixture name

## Outcome

1. Generated Go output never exposes unknown internal/non-model SIL types such as `NsMap`.
2. Unknown by-value model-like types no longer abort whole header generation implicitly; they are either explicitly skipped with evidence or supported through a narrowly justified raw representation rule.
3. Real `iSiLib` verification produces concrete evidence for which declarations were kept in raw output, which were filtered from Go output, and which were skipped as raw-unsafe.
4. Docs clearly state that `files.model` remains the only semantic source of truth for Go-visible model types.

## Scope

In scope:
- policy for unknown by-value argument and return types
- IR normalization behavior for raw-unsafe internal/non-model declarations
- regression coverage for “Go must not know this type”
- real `iSiLib` verification flow using the correct local include roots
- documentation updates after the policy is locked

Out of scope:
- onboarding additional model headers into `files.model` unless verification proves they should be public
- callback support
- iterator/collection helper generation
- repairing the external IPRON source tree beyond temporary local verification steps

## Design constraints

- `files.model` remains the only source of truth for Go-visible shared model types.
- Unknown reference/pointer types may stay in raw output only when the raw layer can express them with opaque handles.
- Unknown by-value types are not Go-safe by default.
- Internal/native helper types are not a reason to widen the Go SDK surface.
- Any skip policy must leave evidence in IR or diagnostics so the missing declaration is explainable.

## Files to read first

- `/Users/kyh0703/Project/cgo-gen/src/ir.rs`
- `/Users/kyh0703/Project/cgo-gen/src/facade.rs`
- `/Users/kyh0703/Project/cgo-gen/src/generator.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/function_pointer_skip.rs`
- `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`
- `/Users/kyh0703/Project/cgo-gen/docs/roadmaps/current-roadmap.md`
- `/Users/kyh0703/Project/cgo-gen/docs/status/sil-conversion-status.md`
- `/Users/kyh0703/Project/cgo-gen/configs/sil-wrapper.example.yaml`

## Execution status

- Strategy: direct execution
- Ready for finish: yes
- Commit evidence: none yet in this workspace

### Completed verification in this turn

- `cargo fmt --check`
- `cargo check`
- `env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

### Real-SIL evidence captured in this turn

- `full iSiLib check`: `/tmp/cgo-gen-real-sil-wrapper.yaml` with local IPRON include roots now succeeds
- `full iSiLib ir`: `cargo run --bin c-go -- ir --config /tmp/cgo-gen-real-sil-wrapper.yaml --format yaml > /tmp/cgo-gen-real-sil-wrapper.ir.yaml` succeeds
- `full iSiLib generate`: `cargo run --bin c-go -- generate --config /tmp/cgo-gen-real-sil-wrapper.yaml --dump-ir` succeeds and writes `/tmp/cgo-gen-real-sil-out/`
- `representative subset generate`: `/tmp/cgo-gen-real-sil-subset.yaml` and `/tmp/iSiLib_subset.h` successfully generated wrappers and IR from real SIL headers
- `subset boundary proof`: `GetAAMaster(IsAAMaster&)` lifted into Go, while `GetCluster(IsCluster&)` and `GetCSTASession(IsCSTASession&)` survived in raw output only
- `transitive internal-type evidence`: real SIL classes such as `IsCluster` and `IsCSTASession` include `NsMap*` internals (`IsCluster.h`, `IsCSTASession.h`), but those types did not need to surface in generated Go output
- `full iSiLib skip evidence`: `/tmp/cgo-gen-real-sil-out/i_si_lib_wrapper.ir.yaml` records 8 skipped declarations, limited to function-pointer and raw-unsafe by-value object cases
- `full iSiLib overload evidence`: raw symbols are disambiguated deterministically (`sil_iSiLib_GetAAMaster__uint32_model_ref_isaamaster_mut`, `sil_iSiLib_Init__int32_model_ptr_uint32_bool_mut`) and Go facade overloads are rendered as distinct methods such as `GetAAMasterUint32(...)` and `GetAAMasterString(...)`

### Active blocker

- no known blocker remains for full local `iSiLib` parse/IR/generate verification
- remaining product decision is policy, not mechanics: decide which additional SIL headers should become explicit `files.model` onboarding candidates

## Task 1: Lock the public-boundary rule with regression tests

Owner: executor
Parallelizable: partial
Integration: tests must define the intended policy before IR skip behavior changes
Status: done

### 1.1 Review the current failure mode for unknown by-value types

Read:
- `/Users/kyh0703/Project/cgo-gen/src/ir.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`

Commands:
```bash
rg -n "normalize_type_with_canonical|normalize_type\\(|raw_safe_model_handle_name|unsupported C\\+\\+ type" /Users/kyh0703/Project/cgo-gen/src/ir.rs
rg -n "UnknownThing|GetUnknown|ThingModel" /Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs
```

Expected result:
- confirm that unknown by-value types still surface as `unsupported C++ type in v1`
- identify where the current code aborts generation instead of recording an explicit skip

### 1.2 Add a focused regression for by-value internal types

Modify:
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`
- or create `/Users/kyh0703/Project/cgo-gen/tests/unknown_model_policy.rs` if isolation is cleaner

Fixture target:
- one known model type from `files.model`
- one unknown internal type such as `UnknownThing`
- one facade class that mixes:
  - a supported known-model out-param method
  - an unsupported unknown by-value method
  - an unsupported unknown by-value return if needed

Required assertions:
- generation succeeds overall for the header
- raw/header/Go output still includes the supported known-model path
- unsupported by-value declaration is absent from raw output and absent from Go output
- IR support metadata records the skipped declaration and reason

Expected result:
- a failing regression against the current behavior that proves we need declaration-level skip handling rather than header-level abort

### 1.3 Preserve the already-decided raw/reference behavior

Read or extend:
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`

Command:
```bash
env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" \
    CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" \
    DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" \
    cargo test --test facade_generate -- --nocapture
```

Expected result:
- existing unknown reference/pointer raw-first behavior keeps passing
- the new red test isolates only the by-value gap

## Task 2: Convert by-value unknown-model failures into explicit declaration-level skips

Owner: executor
Parallelizable: no
Integration: this is the main behavior change and must stabilize before real-SIL verification
Status: done

### 2.1 Introduce an explicit skip reason helper for raw-unsafe unknown types

Modify:
- `/Users/kyh0703/Project/cgo-gen/src/ir.rs`

Target behavior:
- distinguish:
  - raw-safe unknown reference/pointer types
  - raw-unsafe unknown by-value argument/return types
- provide a stable skip reason instead of a generic normalization abort for the latter

Suggested substeps:
1. add a helper near `function_pointer_reason(...)` that inspects method/function return and params for raw-unsafe unknown by-value forms
2. keep the helper narrow; do not broaden support to templates, STL containers, or already-unsupported categories
3. make the reason mention the concrete declaration/type that was skipped

Expected result:
- unsupported by-value internal types have an explicit, reproducible skip reason

### 2.2 Skip whole declarations instead of aborting the entire header

Modify:
- `/Users/kyh0703/Project/cgo-gen/src/ir.rs`

Target behavior:
- `normalize_method(...)` and `normalize_function(...)` record a `SkippedDeclaration`
- generation continues for the rest of the header
- constructors/destructors and supported methods still normalize normally

Commands:
```bash
cargo check
env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" \
    CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" \
    DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" \
    cargo test --test facade_generate -- --nocapture
```

Expected result:
- headers no longer fail wholesale because of one by-value internal type
- skipped declarations appear in IR support metadata

## Task 3: Verify the Go boundary rule against real `iSiLib`

Owner: executor
Parallelizable: partial after Task 2 lands
Integration: depends on declaration-level skip behavior being stable
Status: done

### 3.1 Prepare a local real-SIL config without changing the checked-in example

Read:
- `/Users/kyh0703/Project/cgo-gen/configs/sil-wrapper.example.yaml`

Create locally outside git or under a temporary ignored path:
- a copy of the SIL config with actual local IPRON include roots

Suggested command template:
```bash
cargo run -- generate --config /absolute/path/to/local-sil-wrapper.yaml --dump-ir
```

Expected result:
- generation runs against the real local `iSiLib` environment without editing committed example paths

Actual result:
- created `/tmp/cgo-gen-real-sil-wrapper.yaml`
- local include roots were resolved well enough for `cargo run --bin c-go -- check --config /tmp/cgo-gen-real-sil-wrapper.yaml` to succeed end to end

### 3.2 Capture concrete evidence for internal/non-model declarations

Inspect:
- generated `*.ir.yaml`
- generated raw wrapper header/source
- `support.skipped_declarations`

Command template:
```bash
cargo run -- check --config /absolute/path/to/local-sil-wrapper.yaml
cargo run -- ir --config /absolute/path/to/local-sil-wrapper.yaml --format yaml
```

Expected result:
- identify which concrete declarations mention internal types such as `NsMap`
- confirm one of these outcomes for each declaration:
  - raw survives and Go filters it
  - declaration is explicitly skipped as raw-unsafe by-value

Actual result:
- full `iSiLib` evidence capture now succeeds:
  - `check` summary: `ok: 2 headers, 2 classes, 0 functions, 0 enums, 512 abi functions`
  - `ir` output written to `/tmp/cgo-gen-real-sil-wrapper.ir.yaml`
  - generated artifacts written under `/tmp/cgo-gen-real-sil-out/`
- full real output confirms:
  - raw header contains disambiguated overloaded wrapper symbols
  - Go facade renders disambiguated overloads for renderable methods, for example `GetAAMasterUint32(...)` and `GetAAMasterString(...)`
  - unsupported/internal model references such as `GetCluster(...)` and `GetCSTASession(...)` remain raw-only
  - `support.skipped_declarations` contains 8 explainable skips, limited to function pointers and raw-unsafe by-value object declarations
- representative real-SIL subset evidence was captured with `/tmp/iSiLib_subset.h` and `/tmp/cgo-gen-real-sil-subset.yaml`
- generated raw header kept:
  - `sil_iSiLibSubset_GetAAMaster(..., IsAAMasterHandle* ...)`
  - `sil_iSiLibSubset_GetCluster(..., IsClusterHandle* ...)`
  - `sil_iSiLibSubset_GetCSTASession(..., IsCSTASessionHandle* ...)`
- generated Go facade exposed:
  - `GetAAMaster(...) (IsAAMaster, error)`
- generated Go facade omitted:
  - `GetCluster(...)`
  - `GetCSTASession(...)`
- subset IR recorded `IsCluster&` and `IsCSTASession&` as raw `model_reference` handles without projecting them into Go
- source inspection confirmed transitive internal collection types under the real SIL classes:
  - `IsCluster.h` includes `NsMapInt.h`
  - `IsCSTASession.h` includes `NsMapStr.h`
  - `IEMemory.h` defines many `DsMap*` / `NsMap*` aliases for SIL storage

### 3.3 Decide whether any “internal” types were misclassified and should actually be onboarded

Read:
- generated IR and skipped declaration output from 3.2
- `/Users/kyh0703/Project/cgo-gen/docs/design-docs/wrapping-package-plan.md`

Expected result:
- document a small allowlist of truly public shared model headers if needed
- otherwise keep internal types out of `files.model`

Current decision:
- no new onboarding candidate was proven in this turn
- keep `files.model` limited to already-verified shared model headers such as `IsAAMaster.h`
- defer any broader onboarding decision until the generated full `iSiLib` IR/output has been reviewed declaration by declaration

## Task 4: Update durable docs after the policy is proven

Owner: executor
Parallelizable: yes after Task 3 evidence is collected
Integration: docs should reflect the final verified rule, not a guess
Status: done

Docs to update:
- `/Users/kyh0703/Project/cgo-gen/README.md`
- `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`
- `/Users/kyh0703/Project/cgo-gen/docs/roadmaps/current-roadmap.md`
- `/Users/kyh0703/Project/cgo-gen/docs/status/sil-conversion-status.md`
- `/Users/kyh0703/Project/cgo-gen/docs/exec-plans/active/2026-03-19-internal-type-go-boundary.md`

Required doc changes:
- state that Go-visible model/facade types are gated by `files.model`
- state that unknown internal reference/pointer types may stay raw-only
- state that unknown by-value internal types are explicitly skipped until a safe representation exists
- record real-SIL examples if `NsMap` or similar types are confirmed during verification

Commands:
```bash
rg -n "files.model|unknown model|raw-first|by-value|iSiLib|skip" /Users/kyh0703/Project/cgo-gen/README.md /Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md /Users/kyh0703/Project/cgo-gen/docs/roadmaps/current-roadmap.md /Users/kyh0703/Project/cgo-gen/docs/status/sil-conversion-status.md
```

Expected result:
- docs explain the boundary cleanly enough that “Go does not need to know `NsMap`” is an explicit project rule, not just tribal knowledge

## Task 5: Final verification and plan lifecycle

Owner: executor
Parallelizable: no
Integration: final proof before moving the plan to completed
Status: done

Commands:
```bash
cargo check
env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" \
    CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" \
    DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" \
    cargo test
```

Expected result:
- all tests pass
- targeted regression proves the new skip policy
- real-SIL verification evidence is captured separately if the external environment is available

Completion notes:
- once implementation and docs are complete, move this file to `docs/exec-plans/completed/`
- final report should call out:
  - changed files
  - simplifications made at the Go boundary
  - any remaining risk around real external include environments

## Open questions to resolve during implementation

- Should unknown by-value returns and unknown by-value params use the same skip reason text, or should they be distinguished for easier triage?
- Should recorded skip reasons reference `files.model` explicitly, or stay phrased in raw-safety terms only?
- If real `iSiLib` verification shows that `NsMap` is actually part of the stable business contract, should that trigger model-header onboarding or a facade-specific manual policy?
