# Symlinked Example Build Flow

## Goal
- Checked-in examples support a documented symlink-based build flow for consumers whose build package lives outside the example directory tree.

## References
- docs/STATE.md
- docs/ROADMAP.md
- docs/ARCHITECTURE.md
- docs/v2/designs/2026-04-03-v2-symlinked-example-build-flow.md
- examples/simple-go/README.md
- examples/simple-go/Makefile
- examples/simple-go-struct/README.md
- examples/simple-go-struct/Makefile

## Workspace
- Branch: feat/v2-symlinked-example-build-flow
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: Add the minimum helper tooling and Makefile entrypoints needed to create or refresh a symlinked external build-package layout without breaking the existing in-tree example flow.
- Depends on:
  - none
- Write Scope:
  - examples/...
- Read Context:
  - docs/v2/designs/2026-04-03-v2-symlinked-example-build-flow.md
  - examples/simple-go/Makefile
  - examples/simple-go-struct/Makefile
- Checks:
  - cargo test
- Parallel-safe: no

### Task T2
- Goal: Document the symlinked external build flow from the example README entry points so users can follow one clear setup/build/run sequence.
- Depends on:
  - T1
- Write Scope:
  - examples/simple-go/README.md
  - examples/simple-go-struct/README.md
  - README.md
  - README.ko.md
- Read Context:
  - docs/v2/designs/2026-04-03-v2-symlinked-example-build-flow.md
  - examples/...
- Checks:
  - rg -n "symlink|symbolic link|mklink|New-Item -ItemType SymbolicLink|ln -s" examples README.md README.ko.md
- Parallel-safe: no

## Notes
- Preserve the existing `make ... gen/build/run` flow for users who keep the package in-tree.
- Keep the symlink helper narrow: link the generated package into an external consumer layout, not a full dependency management system.
