# Raw Output Flattening

## Goal
- Flatten generated wrapper artifacts into the package root so generated `.go`, `.h`, `.cpp`, and `.ir.yaml` outputs can be consumed as one co-located cgo package without a `raw/` subdirectory.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-02-v2-raw-output-flattening.md`
- `docs/v2/research/status/sil-conversion-status.md`

## Workspace
- Branch: feat/v2-raw-output-flattening
- Base: master
- Isolation: required
- Created by: `exec-plan` via `git-worktree`

## Task Graph
### Task T1
- Goal: replace `output.dir/raw/` assumptions in config and generation helpers with a flattened package-root output contract, including generated include path rendering.
- Depends on:
  - none
- Write Scope:
  - `src/config.rs`
  - `src/generator.rs`
  - `src/facade.rs`
- Read Context:
  - `docs/ARCHITECTURE.md`
  - `docs/v2/designs/2026-04-02-v2-raw-output-flattening.md`
  - existing output path helpers and facade include rendering
- Checks:
  - `cargo test config`
  - `cargo test dir_only_generate`
- Parallel-safe: no

### Task T2
- Goal: update generation and compile-facing tests to assert the flattened file layout and valid package-local include/compile behavior.
- Depends on:
  - T1
- Write Scope:
  - `tests/dir_only_generate.rs`
  - `tests/facade_generate.rs`
  - `tests/model_out_params.rs`
  - `tests/pipeline.rs`
  - `tests/compile_smoke.rs`
  - `tests/isaamaster_fixture.rs`
  - `tests/config.rs`
- Read Context:
  - `src/config.rs`
  - `src/generator.rs`
  - `src/facade.rs`
  - current output-layout assertions
- Checks:
  - `cargo test dir_only_generate`
  - `cargo test facade_generate`
  - `cargo test compile_smoke`
  - `cargo test isaamaster_fixture`
- Parallel-safe: no

### Task T3
- Goal: update examples and architecture/config docs so the checked-in usage model matches the flattened output contract and no longer relies on `raw/`-specific guidance.
- Depends on:
  - T1
  - T2
- Write Scope:
  - `examples/simple-go-struct/README.md`
  - `examples/simple-go-struct/Makefile`
  - `examples/simple-go-struct/pkg/demo/native_sources.cpp`
  - `docs/ARCHITECTURE.md`
  - `docs/v2/research/references/config.md`
- Read Context:
  - `docs/v2/designs/2026-04-02-v2-raw-output-flattening.md`
  - example package layout
  - updated generator behavior from T1
- Checks:
  - manual: example file tree and include examples match generated layout
  - `cargo test compile_smoke`
- Parallel-safe: no

## Notes
- Keep the change scoped to output layout and consumption path. Do not broaden the public model boundary or auto-fix unrelated raw wrapper quality issues in this plan.
