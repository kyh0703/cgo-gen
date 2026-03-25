# Real SIL Model Header Onboarding Review Plan

## Why this plan exists

The current roadmap and status docs agree on the next practical step:

1. keep `files.model` as the sole semantic source of truth for Go-visible shared models
2. inspect verified real `iSiLib` IR/output instead of widening the boundary from guesswork
3. decide which additional SIL headers, if any, should be onboarded into `files.model`

The codebase already supports:

- raw/model/facade output separation
- model-aware facade routing
- raw-first handling for unknown reference/pointer model types
- declaration-level skipping for raw-unsafe by-value object declarations

The unresolved work is now a product and boundary decision backed by concrete real-SIL evidence, not another broad generator refactor.

Planning gaps discovered while preparing this plan:

- `/Users/kyh0703/Project/cgo-gen/docs/AGENTS.md` is missing in this repository layout
- `/Users/kyh0703/Project/cgo-gen/docs/PLANS.md` is missing in this repository layout
- `/Users/kyh0703/Project/cgo-gen/docs/references/exec-plan-template.md` is missing
- `/Users/kyh0703/Project/cgo-gen/docs/references/plan-quality-checklist.md` is missing
- `/Users/kyh0703/Project/cgo-gen/docs/references/docs-update-rules.md` is missing

This plan therefore uses the repository's existing completed exec-plan style as the local source of truth.

## Outcome

1. The repository has a durable, reviewable decision for which real SIL headers remain raw-only versus which become `files.model` onboarding candidates.
2. The decision is backed by concrete evidence from real `iSiLib` IR, generated raw output, generated Go output, and `support.skipped_declarations`.
3. The checked-in SIL example config and docs reflect the reviewed header classification policy.
4. If no new public model headers are justified, that is recorded explicitly rather than left as tribal knowledge.

## Scope

In scope:

- review of existing real-SIL evidence already described in durable docs
- regeneration of local real-SIL evidence if the environment is available
- classification of observed SIL headers into:
  - keep raw-only
  - candidate `files.model`
  - explicitly unsupported for now
- minimal config/doc/test updates required by the classification decision

Out of scope:

- broad facade helper work such as collection helpers or callback helpers
- widening Go-visible types beyond what the reviewed evidence justifies
- redesigning `files.model` / `files.facade` semantics
- committing machine-local include paths or environment-specific verification configs

## Design constraints

- `files.model` remains the only semantic source of truth for Go-visible shared model types.
- Real-SIL review must prefer the narrowest public boundary that still supports verified use cases.
- Raw-visible declarations are not automatically Go-visible.
- Unknown internal storage helpers such as `NsMap*` / `DsMap*` are not onboarding candidates by default.
- If a header is proposed as a new `files.model` candidate, the plan must require concrete evidence that:
  - the type is part of the stable shared contract
  - the current generator can project it safely or the missing support is narrowly defined
  - onboarding it does not force unrelated internal/native types into the Go boundary

## Files to read first

- `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`
- `/Users/kyh0703/Project/cgo-gen/docs/PRODUCT.md`
- `/Users/kyh0703/Project/cgo-gen/docs/roadmaps/current-roadmap.md`
- `/Users/kyh0703/Project/cgo-gen/docs/status/sil-conversion-status.md`
- `/Users/kyh0703/Project/cgo-gen/docs/design-docs/wrapping-package-plan.md`
- `/Users/kyh0703/Project/cgo-gen/configs/sil-wrapper.example.yaml`
- `/Users/kyh0703/Project/cgo-gen/src/config.rs`
- `/Users/kyh0703/Project/cgo-gen/src/model.rs`
- `/Users/kyh0703/Project/cgo-gen/src/facade.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/config.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/multi_header_generate.rs`
- `/Users/kyh0703/Project/cgo-gen/tests/facade_generate.rs`

## Execution status

- Strategy: direct execution
- Status: completed
- Ready for verify: yes
- Ready for finalize: yes
- Commit evidence: `a18173a678eaac1ae24f79065f7d259ee8bb21c7` (`docs: record sil model onboarding review`)

## Task 1: Reconstruct the exact header-classification decision surface

Owner: executor
Parallelizable: yes
Integration: this defines which real-SIL declarations need evidence before any config/doc change
Status: completed

### 1.1 Extract the currently documented boundary rules

Read:

- `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`
- `/Users/kyh0703/Project/cgo-gen/docs/status/sil-conversion-status.md`
- `/Users/kyh0703/Project/cgo-gen/docs/roadmaps/current-roadmap.md`

Verification:

```bash
rg -n "files.model|raw-only|raw-first|skipped_declarations|iSiLib|IsAAMaster|NsMap|DsMap" docs/ARCHITECTURE.md docs/status/sil-conversion-status.md docs/roadmaps/current-roadmap.md
```

Expected result:

- one concise list of current accepted rules and already-verified examples

### 1.2 Extract the current checked-in SIL classification inputs

Read:

- `/Users/kyh0703/Project/cgo-gen/configs/sil-wrapper.example.yaml`

Verification:

```bash
Get-Content configs/sil-wrapper.example.yaml
```

Expected result:

- explicit list of headers currently checked in under `files.model` and `files.facade`

### 1.3 Record the review table in this plan

Create inside this plan before implementation:

- `Reviewed headers`
- `Evidence source`
- `Proposed classification`
- `Reason`

Expected result:

- the execution step can fill the table incrementally instead of keeping the decision in memory

Execution notes:

- documented boundary rules were confirmed from:
  - `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`
  - `/Users/kyh0703/Project/cgo-gen/docs/status/sil-conversion-status.md`
  - `/Users/kyh0703/Project/cgo-gen/docs/roadmaps/current-roadmap.md`
- checked-in SIL example config keeps a narrow boundary already:
  - `files.model`: `IsAAMaster.h`
  - `files.facade`: `IsAAUser.h`
- no checked-in config change was required to preserve the reviewed decision

## Task 2: Re-run or confirm real-SIL evidence for candidate headers

Owner: executor
Parallelizable: partial
Integration: depends on access to the local real SIL environment; if unavailable, fall back to durable evidence already captured in docs
Status: completed

### 2.1 Prepare a local verification config outside the committed example

Read:

- `/Users/kyh0703/Project/cgo-gen/configs/sil-wrapper.example.yaml`

Create locally outside git:

- a local config copy with actual include roots for the current machine

Suggested commands:

```bash
cargo run --bin c-go -- check --config <local-sil-config>
cargo run --bin c-go -- ir --config <local-sil-config> --format yaml > <local-ir-dump>
cargo run --bin c-go -- generate --config <local-sil-config> --dump-ir
```

Expected result:

- local real-SIL verification artifacts exist without editing committed example paths

### 2.2 Identify concrete candidate headers from real output

Inspect:

- generated IR
- generated `raw/`
- generated `model/`
- generated `facade/`
- `support.skipped_declarations`

Required evidence for each reviewed header:

- whether the header produces Go-visible model output today
- whether it only survives in raw output
- whether declarations are skipped due to raw-unsafe by-value usage
- whether transitive internals such as `NsMap*` / `DsMap*` appear

Expected result:

- each candidate header has a concrete evidence row rather than a guess

### 2.3 Stop broadening the surface unless a header passes all onboarding checks

Onboarding checks:

1. the header represents a stable shared business model rather than native storage plumbing
2. the current generator already produces or can narrowly be extended to produce safe Go-visible output
3. transitive internals do not force a wider Go boundary than intended

Expected result:

- candidate list stays intentionally small, possibly empty

Execution notes:

- no local real-SIL rerun was performed in this workspace because the repository does not include machine-specific IPRON include roots
- durable evidence already checked into:
  - `/Users/kyh0703/Project/cgo-gen/docs/status/sil-conversion-status.md`
  - `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`
  was used as the review source of truth
- based on that evidence, `IsCluster.h` and `IsCSTASession.h` remain raw-only and non-onboarded

## Task 3: Decide the checked-in classification change set

Owner: executor
Parallelizable: no
Integration: depends on Task 2 evidence
Status: completed

### 3.1 Choose one of two allowed outcomes

Allowed outcomes:

1. no new `files.model` onboarding candidates are justified yet
2. one narrowly scoped additional header is justified as a `files.model` candidate

Not allowed:

- bulk-onboarding multiple headers from weak evidence
- widening the boundary just because the raw layer can represent them

Expected result:

- a single explicit decision with rationale

### 3.2 If the decision is “no new header,” update only docs/status evidence

Modify if needed:

- `/Users/kyh0703/Project/cgo-gen/docs/status/sil-conversion-status.md`
- `/Users/kyh0703/Project/cgo-gen/docs/roadmaps/current-roadmap.md`

Expected result:

- docs say the review was completed and no additional onboarding was approved yet

### 3.3 If the decision is “one new header,” keep the code/config diff narrow

Modify only as needed:

- `/Users/kyh0703/Project/cgo-gen/configs/sil-wrapper.example.yaml`
- regression tests directly affected by header classification expectations

Verification:

```bash
cargo test --test config -- --nocapture
cargo test --test multi_header_generate -- --nocapture
cargo test --test facade_generate -- --nocapture
```

Expected result:

- config/test changes prove only the approved header classification delta

Execution notes:

- chosen outcome: no new `files.model` onboarding candidate is justified yet
- checked-in classification remains intentionally narrow
- only doc/status updates were required

## Task 4: Update durable docs to reflect the reviewed decision

Owner: executor
Parallelizable: yes after Task 3
Integration: docs must reflect the actual classification decision, not open-ended intent
Status: completed

Docs to update:

- `/Users/kyh0703/Project/cgo-gen/README.md`
- `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`
- `/Users/kyh0703/Project/cgo-gen/docs/roadmaps/current-roadmap.md`
- `/Users/kyh0703/Project/cgo-gen/docs/status/sil-conversion-status.md`
- `/Users/kyh0703/Project/cgo-gen/docs/exec-plans/active/2026-03-25-real-sil-model-header-onboarding-review.md`

Required doc changes:

- record the reviewed header decision clearly
- keep `files.model` as the sole semantic source of truth
- distinguish raw-only survivability from Go-visible onboarding
- note any remaining blocker if the local real-SIL environment could not be reproduced in this workspace

Verification:

```bash
rg -n "files.model|raw-only|Go-visible|onboard|iSiLib|IsAAMaster|NsMap|DsMap" README.md docs/ARCHITECTURE.md docs/roadmaps/current-roadmap.md docs/status/sil-conversion-status.md
```

Expected result:

- the reviewed boundary policy is obvious from durable docs without external context

## Task 5: Verification and lifecycle handoff

Owner: executor
Parallelizable: no
Integration: final proof before verify handoff
Status: completed

### 5.1 Run targeted repository verification for any checked-in change

Run:

```bash
cargo check
cargo test --test config -- --nocapture
cargo test --test multi_header_generate -- --nocapture
cargo test --test facade_generate -- --nocapture
```

Expected result:

- all affected repository checks pass

### 5.2 If a local real-SIL environment is available, capture the command evidence

Run if possible:

```bash
cargo run --bin c-go -- check --config <local-sil-config>
cargo run --bin c-go -- ir --config <local-sil-config> --format yaml > <local-ir-dump>
cargo run --bin c-go -- generate --config <local-sil-config> --dump-ir
```

Expected result:

- verify notes can cite exact real-SIL evidence rather than a paraphrase

### 5.3 Update readiness and handoff fields in this plan

Before verify handoff, fill:

- `Status`
- `Ready for verify: yes`
- `Verification completed in this turn`
- `Handoff to verify`

Before finalize handoff, fill:

- `Ready for finalize: yes`
- `Handoff to finalize`

Expected result:

- this plan acts as the active state document for `execute -> verify -> finalize`

### Verification completed in this turn

- `cargo fmt`
- `cargo check`
- `cargo test --test config -- --nocapture`
- `cargo test --test multi_header_generate -- --nocapture`
- `cargo test --test facade_generate -- --nocapture`

### Docs updated in this turn

- `/Users/kyh0703/Project/cgo-gen/README.md`
- `/Users/kyh0703/Project/cgo-gen/docs/ARCHITECTURE.md`
- `/Users/kyh0703/Project/cgo-gen/docs/roadmaps/current-roadmap.md`
- `/Users/kyh0703/Project/cgo-gen/docs/status/sil-conversion-status.md`
- `/Users/kyh0703/Project/cgo-gen/docs/exec-plans/active/2026-03-25-real-sil-model-header-onboarding-review.md`

### Code updated in this turn

- `/Users/kyh0703/Project/cgo-gen/src/compiler.rs`

### Execution notes

- initial targeted verification exposed a pre-existing Windows `libclang` include-path gap
- `src/compiler.rs` was updated narrowly to:
  - normalize Windows verbatim paths before passing include paths to `libclang`
  - add the header parent include path automatically
  - add a Windows fallback builtin clang include directory when available
- this change was required to make the repository's targeted facade-generation checks pass in the current workspace

## Reviewed headers

| Header | Evidence source | Proposed classification | Reason |
| --- | --- | --- | --- |
| `IsAAMaster.h` | durable docs + checked-in example config | keep `files.model` | already verified shared model path and the only checked-in public-model classification |
| `iSiLib.h` | durable docs describing real-SIL verification surface | keep facade-only status | real facade verification surface, not a shared model header |
| `IsCluster.h` | durable real-SIL evidence in `docs/status/sil-conversion-status.md` | keep raw-only for now | raw-visible, but not yet justified as a Go-visible shared model and carries transitive internal storage concerns |
| `IsCSTASession.h` | durable real-SIL evidence in `docs/status/sil-conversion-status.md` | keep raw-only for now | raw-visible, but not yet justified as a Go-visible shared model and carries transitive internal storage concerns |

## Verify entry criteria

- reviewed header table is filled with concrete evidence
- any checked-in config/test/doc changes are complete
- targeted repository checks pass
- local real-SIL verification status is recorded as either:
  - completed with command evidence
  - blocked by environment with exact blocker text

## Handoff to verify

- Execution completed with a narrow checked-in decision: no additional `files.model` header was approved.
- Reviewed evidence came from durable docs plus the checked-in SIL example config; no local real-SIL rerun was possible in this workspace because machine-specific include roots are not present in the repository.
- Targeted verification completed:
  - `cargo fmt`
  - `cargo check`
  - `cargo test --test config -- --nocapture`
  - `cargo test --test multi_header_generate -- --nocapture`
  - `cargo test --test facade_generate -- --nocapture`
- Updated docs:
  - `README.md`
  - `docs/ARCHITECTURE.md`
  - `docs/roadmaps/current-roadmap.md`
  - `docs/status/sil-conversion-status.md`
- Additional code fix required by verification:
  - `src/compiler.rs`
- Remaining risk:
  - a future local real-SIL rerun could still justify one narrowly scoped additional model header, but the current checked-in evidence does not justify that change yet

## Review summary

- Blocking issues: none
- Verified outcome:
  - checked-in classification remains intentionally narrow with no new `files.model` header approved
  - durable docs now record that `IsCluster` and `IsCSTASession` stay raw-only until a narrower public-model case is proven
  - Windows `libclang` parsing is more robust in this workspace because include paths are normalized and fallback builtin include discovery is added
- Residual risk:
  - local real-SIL reruns still depend on machine-specific IPRON include roots outside the repository

## Commit evidence

- `a18173a678eaac1ae24f79065f7d259ee8bb21c7` - `docs: record sil model onboarding review`

## Finalize entry criteria

- verify review has no unresolved blocker for the chosen narrow classification decision
- the active plan records final decision, evidence, and any remaining follow-up
- the plan is ready to move to `docs/exec-plans/completed/`

## Handoff to finalize

- Verify completed with no blocking issues.
- Commit recorded:
  - `a18173a678eaac1ae24f79065f7d259ee8bb21c7` - `docs: record sil model onboarding review`
- Finalize should:
  - keep this plan under `docs/exec-plans/completed/`
  - preserve the reviewed decision that no additional checked-in `files.model` header is approved yet
  - keep the Windows `libclang` include-path robustness fix as part of this completed review slice

## Close summary

- Close path: close complete + follow-up milestone needed
- Closed scope:
  - durable docs now record the reviewed SIL model-header boundary
  - checked-in policy stays conservative with `IsAAMaster.h` as the only verified checked-in `files.model` path
  - Windows `libclang` include-path handling is stabilized for the repository's targeted facade-generation checks
- Follow-up milestone:
  - if a future local real-SIL rerun provides narrower public-model evidence, start a new plan to review exactly one additional candidate header at a time
