# Remove Include Dirs Config

## Goal
- Public config and implementation stop exposing `input.include_dirs` and rely on authored `input.clang_args` `-I...` tokens for include-path input instead.

## References
- docs/STATE.md
- docs/ROADMAP.md
- docs/ARCHITECTURE.md
- docs/v2/designs/2026-04-03-v2-remove-include-dirs-config.md
- docs/v2/designs/2026-04-03-v2-go-package-metadata.md
- src/config.rs
- src/compiler.rs

## Workspace
- Branch: feat/v2-remove-include-dirs-config
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: Remove `input.include_dirs` parsing/normalization from config loading, verify that `input.clang_args` still covers the supported include-path behavior, and update config tests to the reduced surface.
- Depends on:
  - none
- Write Scope:
  - src/config.rs
  - tests/config.rs
- Read Context:
  - docs/v2/designs/2026-04-03-v2-remove-include-dirs-config.md
  - docs/v2/designs/2026-04-03-v2-go-package-metadata.md
  - src/compiler.rs
- Checks:
  - cargo test config
  - cargo test tu_based_parsing
- Parallel-safe: no

### Task T2
- Goal: Remove `input.include_dirs` from checked-in examples, fixtures, and current public docs so the documented config contract matches T1.
- Depends on:
  - T1
- Write Scope:
  - README.md
  - README.ko.md
  - configs/...
  - examples/...
  - tests/fixtures/...
  - docs/v2/research/...
  - docs/v2/designs/2026-04-03-v2-go-package-metadata.md
- Read Context:
  - src/config.rs
  - docs/v2/designs/2026-04-03-v2-remove-include-dirs-config.md
- Checks:
  - cargo test
  - rg -n "input\\.include_dirs" README.md README.ko.md configs examples tests docs/v2/research
- Parallel-safe: no

## Notes
- Preserve relative `-I...`, `-I <path>`, and `-isystem` normalization through `input.clang_args`; do not remove that behavior.
- If any checked-in config needs include roots after this change, express them explicitly in `input.clang_args`.
