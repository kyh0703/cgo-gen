---
feature: raw-output-flattening
status: plan_ready
created_at: 2026-04-02T11:05:00+09:00
---

# Raw Output Flattening

## Goal

Flatten generated native wrapper artifacts into the same package output directory as generated Go files so downstream cgo packages can consume the library as one co-located output set instead of a split `raw/` subdirectory.

## Context / Inputs

- Source docs:
  - `docs/ARCHITECTURE.md`
  - `docs/v2/designs/wrapping-package-plan.md`
  - `docs/v2/research/status/sil-conversion-status.md`
- Existing system facts:
  - raw native artifacts currently emit under `output.dir/raw/`
  - generated Go files currently emit under `output.dir/`
  - current examples rely on `native_sources.cpp`-style aggregation when raw `.cpp` files stay under `raw/`
  - downstream PSC integration succeeded through a narrow manual bridge, not through full generated package consumption
- User brief:
  - split `raw/` output is inconvenient for the intended shared library consumption model
  - the long-term package should be consumable with generated artifacts co-located in one package folder

## Plan Handoff

### Scope for Planning

- change generator output paths so generated wrapper `.h`, `.cpp`, `.ir.yaml`, and generated Go files can be emitted into one package directory
- update include rendering and generated cgo/header references to stay valid after flattening
- update tests, examples, and checked assumptions that currently expect `raw/` subdirectory output
- verify the flattened output remains buildable for the existing example flow and does not silently break current raw/native ownership rules

### Success Criteria

- generator no longer hardcodes `output.dir/raw/` as the emitted location for generated wrapper artifacts for this feature slice
- generated Go files include the flattened wrapper headers with valid paths
- example or regression coverage proves package-local `.cpp` files are visible to cgo without requiring a `raw/` subdirectory
- docs and tests reflect the new co-located output contract consistently

### Non-Goals

- making every currently generated SIL wrapper semantically safe for public consumption in one pass
- solving all raw-only versus Go-visible boundary decisions
- introducing DTO/business-layer abstractions for downstream IE modules
- broad refactors unrelated to output layout and build consumption

### Open Questions

- whether IR dump files should also flatten into the package root or remain separated from `.go/.h/.cpp`
- whether config needs an explicit compatibility switch or whether the flattened layout replaces the current default immediately

### Suggested Validation

- `cargo test dir_only_generate`
- `cargo test facade_generate`
- `cargo test compile_smoke`
- manual inspection of generated example output paths and include directives

### Parallelization Hints

- Candidate write boundaries:
  - `src/config.rs`, `src/generator.rs`, `src/facade.rs`
  - tests asserting output paths and include locations
  - example fixtures and README snippets that document generated layout
- Shared files to avoid touching in parallel:
  - `docs/ARCHITECTURE.md`
  - any shared config/output path helpers used by multiple renderers
- Likely sequential dependencies:
  - output path helper changes before test fixture and example expectation updates
