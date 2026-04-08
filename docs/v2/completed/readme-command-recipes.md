# README Command Recipes

## Goal
- Add a focused command-recipes section to the top-level READMEs so users can copy/paste the most common `cgo-gen` flows from fenced `bash` blocks.

## References
- `docs/STATE.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/v2/designs/2026-04-03-v2-readme-command-recipes.md`
- `README.md`
- `README.ko.md`

## Workspace
- Branch: feat/v2-readme-command-recipes
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: Define the final command sets and insert an English command-recipes section that matches the current CLI and example layout without changing existing meaning.
- Depends on:
  - none
- Write Scope:
  - `README.md`
- Read Context:
  - `docs/v2/designs/2026-04-03-v2-readme-command-recipes.md`
  - `README.md`
- Checks:
  - `cargo run --bin cgo-gen -- --help`
  - `cargo run --bin cgo-gen -- generate --help`
  - `rg -n "Command Recipes|go-module|examples/simple-go-struct" README.md`
- Parallel-safe: no

### Task T2
- Goal: Mirror the command-recipes structure in the Korean README, keeping the same command coverage and path examples.
- Depends on:
  - T1
- Write Scope:
  - `README.ko.md`
- Read Context:
  - `docs/v2/designs/2026-04-03-v2-readme-command-recipes.md`
  - `README.md`
  - `README.ko.md`
- Checks:
  - `rg -n "명령어|go-module|examples/simple-go-struct" README.ko.md`
- Parallel-safe: no

### Task T3
- Goal: Run a narrow verification pass so the added commands still match the checked-in CLI surface and example paths.
- Depends on:
  - T2
- Write Scope:
  - none
- Read Context:
  - `README.md`
  - `README.ko.md`
  - `examples/simple-go`
  - `examples/simple-go-struct`
- Checks:
  - `cargo run --bin cgo-gen -- check --config examples/simple-go/config.yaml`
  - `cargo test`
- Parallel-safe: no

## Notes
- Keep the change documentation-only.
- Prefer one compact section instead of spreading extra examples across multiple places.
- Include the latest public flow for `generate --go-module <module-path>` in the command recipes.
