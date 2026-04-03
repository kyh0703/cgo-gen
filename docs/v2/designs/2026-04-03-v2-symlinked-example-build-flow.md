---
feature: symlinked-example-build-flow
status: plan_ready
created_at: 2026-04-03T10:02:10+09:00
---

# Symlinked Example Build Flow

## Goal

Add an example workflow that lets users build the checked-in Go examples even when the consuming build package lives outside the example directory tree, by linking the generated package in through a symbolic link.

## Context / Inputs
- Source docs:
  - `README.md`
  - `README.ko.md`
  - `examples/simple-go/README.md`
  - `examples/simple-go-struct/README.md`
- Existing system facts:
  - current examples assume the Go module and generated `pkg/...` tree live under the same example directory.
  - repo docs already explain symlinked external SDK input paths, but not symlinked example build package flows.
  - `simple-go` and `simple-go-struct` already have `Makefile`-based generate/build/run flows.
- User brief:
  - add a process for examples where the build package is not in the same directory, using a symbolic link to connect that package for build.

## Plan Handoff
### Scope for Planning
- Define one consistent external-build workflow for checked-in examples using symbolic links.
- Add the minimum helper tooling needed to create/remove the link safely on supported local environments.
- Update example docs and entry commands so users can run the symlinked build path without guessing layout details.

### Success Criteria
- At least one checked-in example documents and supports a symlinked external build-package flow end to end.
- The workflow makes it clear which generated package directory is linked and where an external consumer should build from.
- Existing in-place example build flow continues to work unchanged.

### Non-Goals
- General package publishing or module proxy workflows.
- Replacing the existing in-tree example build path.
- Supporting arbitrary cross-repo dependency management beyond the documented symlink flow.

### Open Questions
- Should the helper target cover both `simple-go` and `simple-go-struct`, or is one canonical example enough?
- Should the helper create only the package symlink, or also scaffold a minimal external app directory when missing?
- What is the minimum cross-platform contract we want to support for symlink creation in example tooling?

### Suggested Validation
- `cargo test`
- manual/example-oriented verification that the symlink helper creates the expected layout and does not break the existing build flow
- repository search confirming the new workflow is documented from the example entry points

### Parallelization Hints
- Candidate write boundaries:
  - example helper tooling and Makefile updates under `examples/...`
  - example documentation updates under `examples/.../README.md` and top-level README references if needed
- Shared files to avoid touching in parallel:
  - any shared helper script path used by both examples
  - example README files if the helper contract changes during implementation
- Likely sequential dependencies:
  - settle the symlink contract first, then wire Makefile targets and docs to the same shape
