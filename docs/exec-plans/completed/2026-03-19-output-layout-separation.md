# Output Layout Separation Plan

## Why this plan exists

The repository now classifies headers by role with `files.model` and `files.facade`, but generated artifacts still land flat under one `output.dir`.

That leaves the conceptual layer split implemented in behavior only:

1. raw wrapper headers/sources/IR are generated
2. model Go output is generated only for `files.model`
3. facade Go output is generated only for `files.facade`

But the physical output layout still mixes those artifacts together.

The next step is to align output structure with the existing layer model so generated files are easier to inspect and downstream consumers can reason about raw/model/facade ownership directly from paths.

## Outcome

1. Generated native wrapper artifacts land under `output.dir/raw/`.
2. Generated Go model artifacts land under `output.dir/model/`.
3. Generated Go facade artifacts land under `output.dir/facade/`.
4. Generated Go files still compile with correct relative `#include` paths back to raw headers.
5. Existing role behavior remains unchanged; this is an output-layout change, not a routing semantic change.

## Scope

In scope:
- output path helpers in config/generator
- Go file emission paths for model/facade output
- generated cgo include paths to raw wrapper headers
- regression updates for path expectations
- docs for new layout

Out of scope:
- changing `files.model` / `files.facade` semantics
- moving raw generation behind a separate command
- introducing separate Go module/package names per layer
- changing runtime linking behavior

## Design constraints

- raw wrapper generation remains the base layer and source of cgo includes.
- Go model and Go facade files must continue to share one Go package name unless explicitly redesigned later.
- Multi-header generation must stay deterministic.
- Diffs should stay narrow: only output layout and include-path plumbing should change.

## Files to read first

- `/Users/kyh0703/Project/cgo-gen/src/config.rs`
- `/Users/kyh0703/Project/cgo-gen/src/generator.rs`
- `/Users/kyh0703/Project/cgo-gen/src/model.rs`
- `/Users/kyh0703/Project/cgo-gen/src/facade.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/config.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/pipeline.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/isaamaster_fixture.rs`
- `/Users/kyh0703/Project/cgo-gen/README.md`
- `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`

## Execution status

- Strategy: direct execution
- Ready for finish: yes
- Commit evidence: none yet in this workspace

### Verification completed in this turn

- `cargo fmt --check`
- `cargo check`
- `env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

### Landed layout

- raw wrapper artifacts now emit under `output.dir/raw/`
- Go model artifacts now emit under `output.dir/model/`
- Go facade artifacts now emit under `output.dir/facade/`
- generated facade cgo preambles include raw headers via `../raw/<header>`

## Task 1: Add output-path helpers for raw/model/facade

Owner: executor
Parallelizable: no
Integration: all emitters should use one consistent path contract
Status: done

### 1.1 Define layer-specific output helpers

Modify:
- `/Users/kyh0703/Project/cgo-gen/src/config.rs`

Target behavior:
- raw artifacts resolve under `output.dir/raw`
- model Go artifacts resolve under `output.dir/model`
- facade Go artifacts resolve under `output.dir/facade`
- raw header/source/ir filenames stay per-header stable

Verification:
```bash
cargo check
```

Expected result:
- config exposes enough helpers that generator/model/facade code no longer hardcodes a flat `output.dir`

## Task 2: Route generated files into the new layout

Owner: executor
Parallelizable: no
Integration: depends on Task 1 helpers
Status: done

### 2.1 Write raw artifacts into `raw/`

Modify:
- `/Users/kyh0703/Project/cgo-gen/src/generator.rs`

Expected result:
- `.h`, `.cpp`, `.ir.yaml` emit under `output.dir/raw`

### 2.2 Write Go model/facade artifacts into `model/` and `facade/`

Modify:
- `/Users/kyh0703/Project/cgo-gen/src/generator.rs`
- `/Users/kyh0703/Project/cgo-gen/src/model.rs`
- `/Users/kyh0703/Project/cgo-gen/src/facade.rs`

Expected result:
- model `.go` files emit under `output.dir/model`
- facade `.go` files emit under `output.dir/facade`
- generated cgo `#include` directives point back to raw headers with valid relative paths

Verification:
```bash
cargo check
env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" \
    CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" \
    DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" \
    cargo test
```

Expected result:
- full repository tests still pass after layout move

## Task 3: Update regression expectations for layout-sensitive tests

Owner: executor
Parallelizable: no
Integration: depends on Task 2
Status: done

Modify:
- `/Users/kyh0703/Project/cgo-gen/tests/config.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/pipeline.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/isaamaster_fixture.rs`
- any other failing path assertion tests

Expected result:
- tests assert the new `raw/ model/ facade/` file locations explicitly

## Task 4: Update docs to reflect physical layout separation

Owner: executor
Parallelizable: yes after Task 2
Integration: docs should reflect landed layout, not intent only
Status: done

Docs to update:
- `/Users/kyh0703/Project/cgo-gen/README.md`
- `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`
- `/Users/kyh0703/Project/cgo-gen/docs/roadmaps/current-roadmap.md`
- `/Users/kyh0703/Project/cgo-gen/docs/exec-plans/active/2026-03-19-output-layout-separation.md`

Expected result:
- docs distinguish conceptual layer routing from physical output layout and show the new directory tree

## Task 5: Final verification and handoff readiness

Owner: executor
Parallelizable: no
Integration: final proof before finish handoff
Status: done

Verification:
```bash
cargo fmt --check
cargo check
env SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" \
    CPLUS_INCLUDE_PATH="$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1" \
    DYLD_FALLBACK_LIBRARY_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib:/Applications/Xcode.app/Contents/Frameworks:${DYLD_FALLBACK_LIBRARY_PATH}" \
    cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Finish entry criteria:
- raw/model/facade files emit into separate directories
- generated Go includes still resolve raw headers
- tests pass
- docs reflect new layout
- active plan can be marked `Ready for finish: yes`
