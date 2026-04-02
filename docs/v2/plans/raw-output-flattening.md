# Raw Output Flattening

## Goal
- Flatten generated native wrapper artifacts into the package root so `.go`, `.h`, `.cpp`, and `.ir.yaml` outputs are co-located under one `output.dir`.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-02-v2-raw-output-flattening.md`
- `docs/v2/research/references/config.md`

## Workspace
- Branch: `feat/v2-raw-output-flattening`
- Base: `master`
- Isolation: required
- Created by: `exec-plan` via `git-worktree`

## Task Graph
### Task T1
- Goal: replace `output.dir/raw/` assumptions in config and generation helpers with a flattened package-root output contract.
- Depends on:
  - none
- Write Scope:
  - `src/config.rs`
  - `src/generator.rs`
  - `src/facade.rs`
- Read Context:
  - `docs/v2/designs/2026-04-02-v2-raw-output-flattening.md`
  - current output path helpers and include rendering
- Checks:
  - `cargo test config`
  - `cargo test dir_only_generate`
- Parallel-safe: no

### Task T2
- Goal: update generation and compile-facing tests so the repository asserts the flattened layout instead of a `raw/` subdirectory layout.
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
  - updated output path behavior from T1
  - existing layout assertions
- Checks:
  - `cargo test dir_only_generate`
  - `cargo test facade_generate`
  - `cargo test compile_smoke`
  - `cargo test isaamaster_fixture`
- Parallel-safe: no

### Task T3
- Goal: align examples and architecture/config docs with the new co-located output contract.
- Depends on:
  - T2
- Write Scope:
  - `examples/simple-go-struct/README.md`
  - `examples/simple-go-struct/Makefile`
  - `examples/simple-go-struct/pkg/demo/native_sources.cpp`
  - `docs/ARCHITECTURE.md`
  - `docs/v2/research/references/config.md`
- Read Context:
  - flattened output behavior
  - example package layout
- Checks:
  - manual: example layout and includes match generated files
  - `cargo test compile_smoke`
- Parallel-safe: no

## Notes
- Keep the change limited to output layout and downstream package consumption. Do not broaden this plan into unrelated facade/model boundary work.
