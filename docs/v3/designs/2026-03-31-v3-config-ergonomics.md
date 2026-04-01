---
feature: config-ergonomics
status: plan_ready
created_at: 2026-03-31T15:40:00+09:00
---

# Config Ergonomics

## Goal

- reduce required YAML boilerplate for common wrapper generation without changing the `libclang` parser backend.

## Context / Inputs

- Source docs:
  - `AGENTS.md`
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - users currently have to repeat the same header and include roots across `input.headers`, `files.model` / `files.facade`, and raw `clang_args`.
  - `Config::load` already resolves relative paths and output defaults, so config-level sugar can stay isolated in `src/config.rs`.
  - parsing and generation still depend on `libclang`; this slice should not replace parser infrastructure or compile-command handling.
- User brief:
  - improve user convenience as much as possible, keep `clang` support, and allow a mode where explicitly listed facade headers are treated as facade while remaining headers default to model.

## Plan Handoff

### Scope for Planning

- add config-level conveniences for `project_root`, header-directory expansion, and include-directory expansion into clang include args.
- support role inference where `files.facade` is explicit and omitted `files.model` means “all remaining headers are model”.
- update checked-in example config and config tests to prove the shorter YAML form works.

### Success Criteria

- users can point config at header directories instead of enumerating every header file.
- users can specify include directories without spelling raw `-I...` entries by hand.
- configs that declare only `files.facade` classify the remaining input headers as model.
- existing explicit `headers`, `files.model`, `files.facade`, and `clang_args` inputs remain valid.

### Non-Goals

- removing `libclang` or replacing parsing with GCC tooling.
- auto-classifying facade headers from naming patterns or AST heuristics.
- redesigning generator output layout or facade/model code generation semantics.

### Open Questions

- keep header-directory expansion recursive and limited to common header extensions unless fixtures show a narrower rule is safer.

### Suggested Validation

- `cargo test config`
- `cargo test example_simple_go_struct`
- `cargo test`

### Parallelization Hints

- Candidate write boundaries: config loading in `src/config.rs`; config-focused coverage in `tests/config.rs`; example YAML cleanup in `examples/simple-go-struct/config.yaml`.
- Shared files to avoid touching in parallel: `src/config.rs`, `tests/config.rs`.
- Likely sequential dependencies: config normalization lands before example/test rewrites.
