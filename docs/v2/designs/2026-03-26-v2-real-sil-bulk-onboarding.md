# v2 Real SIL Bulk Onboarding

## Goal

Define the next major milestone that moves `c-go` from narrow, header-by-header SIL verification toward a real integrated SIL wrapping package built from the actual `D:/Project/IPRON/IE/SIL` source tree.

The immediate target is:

- treat `iSiLib.h` as the primary facade header
- use `iSiLib.cpp` as the real native integration/compile reference
- review `Is*.h` headers as candidate shared model inputs
- generate raw/model/facade outputs against the real SIL tree
- verify that the generated output can be wired into a real end-to-end integration path

This milestone is about establishing a durable bulk-onboarding strategy and an integration contract, not blindly forcing every parsed type into the Go public surface.

## Context / Inputs

Current repository state:

- `c-go` already supports `files.model` / `files.facade` classification.
- Raw/model/facade output layers already exist.
- Real SIL verification has been performed only on a narrow checked-in slice.
- Current durable docs intentionally keep `IsAAMaster.h` as the only verified checked-in `files.model` header.

Relevant local documents:

- `docs/ARCHITECTURE.md`
- `docs/v2/designs/PRODUCT.md`
- `docs/v2/research/roadmaps/current-roadmap.md`
- `docs/v2/research/status/sil-conversion-status.md`
- `docs/v2/designs/wrapping-package-plan.md`
- `docs/v2/designs/wrapping-package-phases.md`
- `docs/v2/completed/2026-03-25-real-sil-model-header-onboarding-review.md`

Relevant external/local source surface:

- `D:/Project/IPRON/IE/SIL/iSiLib.h`
- `D:/Project/IPRON/IE/SIL/iSiLib.cpp`
- `D:/Project/IPRON/IE/SIL/iSiLib-inl.h`
- `D:/Project/IPRON/IE/SIL/Is*.h`

Observed scale:

- the real SIL directory contains one primary `iSiLib` facade surface plus a large set of `Is*.h` headers
- the count of `Is*.h` headers is already large enough that this is a boundary-definition problem, not a patch-sized extension

## Problem Statement

The current implementation and docs are intentionally conservative:

- only a narrow real-SIL path is checked in as verified
- `files.model` onboarding is deliberately restricted
- raw-visible declarations are not automatically promoted to Go-visible shared models

That is correct for safety, but it leaves the next practical goal unresolved:

- how to use the actual `IPRON/IE/SIL` tree as the source of truth
- how to classify the real facade boundary around `iSiLib`
- how to decide which `Is*.h` headers become shared models, which remain raw-only, and which must stay unsupported for now
- how to prove the generated output can participate in a real integration flow instead of fixture-only verification

Without a major design pass, there is a high risk of taking the wrong shortcut:

- treating all `Is*.h` headers as public shared models by naming convention alone
- widening the Go public boundary to include internal/native-only storage details
- conflating raw parse success with safe model onboarding
- attempting real integration without a stable ownership and build contract

## Options Considered

### Option 1: Bulk-classify every `Is*.h` header as `model` immediately

Pros:

- fastest path to broad output coverage
- simple rule to explain

Cons:

- naming-based onboarding is too coarse
- likely to widen the Go boundary with internal-only or unstable types
- would mix verified shared contract models with raw-only helper/storage types
- likely to create many partial or misleading generated models before the ownership rules are stable

### Option 2: Keep the current narrow checked-in model set and continue only header-by-header review

Pros:

- safest boundary control
- minimal semantic risk

Cons:

- too slow for the stated goal of real SIL onboarding
- does not produce a practical integration path over the actual SIL tree
- keeps the repository in a perpetual exploratory state

### Option 3: Use `iSiLib` as the facade anchor, bulk-review `Is*.h` headers under explicit tiers, and verify with a real integration slice

Pros:

- keeps one clear operational surface for facade work
- allows scale without pretending all `Is*.h` headers are equally safe as public models
- preserves the existing raw-first architecture
- turns the next milestone into a verifiable onboarding framework rather than a one-off guess

Cons:

- requires explicit classification rules and review tables
- needs a local integration environment and build contract
- more planning overhead before implementation

## Recommended Option

Choose Option 3.

The major milestone should be defined as:

1. `iSiLib.h` is the primary facade source of truth.
2. `iSiLib.cpp` is the primary native compile/integration reference.
3. `Is*.h` onboarding is reviewed in bulk, but not auto-promoted wholesale.
4. Each reviewed header must land in one of these tiers:
   - shared model candidate
   - raw-only for now
   - unsupported for now
5. The first end state is not "all SIL types are public Go models".
   The first end state is "the real SIL tree can generate and integrate through a stable, reviewable boundary".

This keeps the architecture coherent with the current raw-first design while acknowledging that the next step is materially larger than a patch or minor feature extension.

## Scope Decision

In scope:

- define the real-SIL major onboarding strategy
- treat `iSiLib.h` / `iSiLib.cpp` as the primary facade/integration anchor
- inventory and review the `Is*.h` model candidate set
- define classification rules for:
  - shared model
  - raw-only
  - unsupported
- add a checked or locally reproducible real-SIL config path
- verify generate flow against the real SIL tree
- verify one real integration path that proves generated output can compile and link in practice
- document ownership boundaries for generated wrappers, generated Go APIs, and native build dependencies

Out of scope for this major design doc:

- making every `Is*.h` header Go-visible in one pass
- callback helper completion for every callback in `iSiLib`
- collection helper completion for every iterator/list pattern in one milestone
- business-logic abstraction for IE modules
- forcing unsupported internal storage types into the public Go model layer

## Plan Handoff

`plan` should break this major milestone into implementation phases with hard verification gates.

Suggested plan skeleton:

1. Real SIL environment contract
   - define include roots, compile arguments, and any machine-local config handling
   - verification: `cargo run --bin c-go -- check` succeeds against a local real-SIL config

2. Facade anchor stabilization
   - make `iSiLib.h` the primary reviewed facade surface
   - account for `iSiLib-inl.h` and real include-cycle/alias/callback issues
   - verification: `ir` and `generate --dump-ir` succeed for the real facade slice

3. Bulk model review table
   - inventory `Is*.h` headers and classify each into:
     - shared model candidate
     - raw-only
     - unsupported for now
   - verification: the review table is checked into docs and reflected in config/test decisions

4. First bounded onboarding slice
   - onboard only the first reviewed subset that satisfies the public-model rules
   - do not onboard the whole directory by naming convention alone
   - verification: generated `model/` output is stable and regression-covered for that subset

5. Real integration slice
   - compile generated raw wrappers with the real native sources needed for the first facade/model path
   - validate one end-to-end generated integration path
   - verification: documented build/run procedure succeeds in the local integration environment

6. Boundary hardening
   - record unsupported and raw-only reasons explicitly
   - ensure skipped declarations and internal native types do not silently leak into Go-visible output
   - verification: docs, config examples, and regression tests agree on the reviewed boundary

Initial success criteria for the milestone:

- the repository has a durable reviewed classification strategy for the real SIL tree
- `iSiLib` is established as the facade anchor in docs and implementation planning
- at least one real integrated generation path is verified against the actual SIL sources
- the public Go model boundary remains intentional and reviewable rather than name-driven
