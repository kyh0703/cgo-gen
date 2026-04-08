---
feature: remove-include-dirs-config
status: plan_ready
created_at: 2026-04-03T09:54:17+09:00
---

# Remove Include Dirs Config

## Goal

Remove `input.include_dirs` from the public config surface if it is only a thin compatibility layer over `input.clang_args` `-I...` handling.

## Context / Inputs
- Source docs:
  - `README.md`
  - `README.ko.md`
  - `docs/v2/research/references/config.md`
  - `docs/v2/designs/2026-04-03-v2-go-package-metadata.md`
- Existing system facts:
  - `src/config.rs` still parses `input.include_dirs`, resolves those paths, and prepends them into `input.clang_args`.
  - `src/compiler.rs` ultimately consumes `config.input.clang_args`.
  - recent docs already state that exported metadata uses raw `input.clang_args` as the source of truth and ignores `input.include_dirs`.
- User brief:
  - review whether the YAML include-path knob is redundant with `CFlags`/`-I` usage and remove it if that redundancy is real.

## Plan Handoff
### Scope for Planning
- Verify that `input.include_dirs` has no distinct runtime behavior beyond producing `-I...` tokens in `input.clang_args`.
- Remove `input.include_dirs` from config parsing and checked-in config/docs if that verification holds.
- Update tests so include paths are expressed through `input.clang_args` only.

### Success Criteria
- The public config contract no longer documents or accepts `input.include_dirs`.
- Existing supported include-path behavior remains available through `input.clang_args`.
- Checked-in examples, fixtures, and tests pass after the config surface reduction.

### Non-Goals
- Redesigning `compile_commands.json` support.
- Changing fallback platform include discovery.
- Broad cleanup of unrelated historical config keys.

### Open Questions
- Do any checked-in tests rely on `input.include_dirs` semantics beyond ordering before existing `clang_args`?
- Should the migration explicitly preserve `input.include_dirs` ordering by requiring users to author `-I...` first in `input.clang_args`?

### Suggested Validation
- Targeted config-loading tests for include-path normalization and env expansion.
- Full `cargo test`.
- Repository search confirming `input.include_dirs` no longer appears in current public docs/config references.

### Parallelization Hints
- Candidate write boundaries:
  - parser/config changes under `src/` and affected tests
  - docs/example config updates under `README*`, `tests/fixtures/`, `examples/`, `docs/v2/research/`
- Shared files to avoid touching in parallel:
  - `src/config.rs`
  - `tests/config.rs`
- Likely sequential dependencies:
  - prove removal is behavior-preserving first, then update fixtures/docs to the new contract
