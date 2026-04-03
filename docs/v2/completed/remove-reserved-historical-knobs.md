# Remove Reserved Historical Knobs

## Goal
- Public config contract and implementation agree on which keys are supported by removing leftover reserved or historical knobs from docs and config parsing.

## References
- docs/STATE.md
- docs/ROADMAP.md
- docs/ARCHITECTURE.md
- docs/v2/designs/2026-04-03-v2-remove-reserved-historical-knobs.md
- docs/v2/research/references/config.md

## Workspace
- Branch: feat/v2-remove-reserved-historical-knobs
- Base: master
- Isolation: required
- Created by: exec-plan via git-worktree

## Task Graph
### Task T1
- Goal: Trace the actual public config surface, remove obsolete parser fields for `project_root` and inactive `policies.*`, and update config-focused tests/fixtures to the reduced schema.
- Depends on:
  - none
- Write Scope:
  - src/config.rs
  - tests/...
  - configs/...
  - examples/...
- Read Context:
  - docs/v2/designs/2026-04-03-v2-remove-reserved-historical-knobs.md
  - README.md
  - README.ko.md
- Checks:
  - cargo test
- Parallel-safe: no

### Task T2
- Goal: Remove the reserved/historical knob section and other now-incorrect public documentation so README and versioned docs reflect the supported config contract from T1.
- Depends on:
  - T1
- Write Scope:
  - README.md
  - README.ko.md
  - docs/ARCHITECTURE.md
  - docs/v2/research/references/config.md
  - docs/v2/research/status/...
- Read Context:
  - src/config.rs
  - docs/v2/designs/2026-04-03-v2-remove-reserved-historical-knobs.md
- Checks:
  - rg -n "Reserved Or Historical Knobs|project_root|string_mode|enum_mode|unsupported\\.templates|unsupported\\.stl_containers|unsupported\\.exceptions|files\\.model|files\\.facade" README.md README.ko.md docs/ARCHITECTURE.md docs/v2/research
- Parallel-safe: no

## Notes
- Treat `files.model` and `files.facade` as removed from the current public config contract unless T1 finds an active loader path that still consumes them.
