---
feature: scrub-sil-artifacts
status: plan_ready
created_at: 2026-04-08T15:06:00+09:00
---

# Scrub SIL Artifacts

## Goal

Remove checked-in SIL-specific configuration, tests, and documentation traces so the repository no longer carries company-source-specific SIL examples or naming.

## Context / Inputs

- Source docs:
  - `docs/STATE.md`
  - `docs/ROADMAP.md`
  - `docs/ARCHITECTURE.md`
- Existing system facts:
  - `configs/sil-wrapper.example.yaml` is a checked-in example config with company-source include paths and SIL-specific naming
  - `tests/config.rs` still loads that checked-in SIL example directly as regression coverage
  - multiple docs under `docs/v2/` record real-SIL onboarding evidence, concrete internal header names, and local validation commands
  - some tests and docs still use SIL-prefixed generated symbol examples even when the underlying behavior is generic
- User brief:
  - `SIL 검증용으로 쓰는거 지워줘 회사소스라 남으면안돼`
  - `응 다지워야돼`

## Plan Handoff

### Scope for Planning

- delete the checked-in SIL example config and any direct test coverage that depends on it
- scrub repository docs that mention real-SIL onboarding, internal SIL headers, company-specific type names, or SIL-specific generated output paths
- replace remaining SIL-specific test literals with neutral generic names when the coverage is still needed for behavior regression
- keep the scrub focused on checked-in repository content; do not widen into unrelated product or architecture refactors

### Success Criteria

- no checked-in file under the repo references `configs/sil-wrapper.example.yaml`
- no checked-in doc retains real-SIL onboarding notes, internal SIL header/type names, or company-specific local validation examples
- remaining regression tests still cover the intended generic behavior without SIL-specific naming
- repo-wide search for the targeted SIL/company-source identifiers returns no remaining checked-in hits

### Non-Goals

- changing generator behavior beyond the smallest rename or test cleanup needed to keep generic coverage
- deleting unrelated historical docs that do not contain SIL/company-source traces
- adding new abstraction or new config features while performing the scrub

### Open Questions

- whether any remaining `sil` string is now purely generic test data or still a company-source trace that should be neutralized
- whether architecture docs should keep a product-level mention of a shared SDK over a native surface after the SIL-specific wording is removed

### Suggested Validation

- `rg -n -i "sil|iSiLib|IsAAMaster|IsCluster|IsCSTASession|SetHACallback|HACallback" .`
- `cargo test --test config`
- `cargo test facade_generate`
- `cargo test`

### Parallelization Hints

- Candidate write boundaries:
  - `configs/` and tests that directly load the SIL example config
  - generic regression tests that still use SIL-prefixed symbol literals
  - docs under `docs/ARCHITECTURE.md`, `docs/v2/designs/`, `docs/v2/research/`, and `docs/v2/completed/`
- Shared files to avoid touching in parallel:
  - `docs/ARCHITECTURE.md`
  - any single doc file rewritten wholesale for scrub
  - `tests/facade_generate.rs`
- Likely sequential dependencies:
  - identify the exact checked-in SIL/company-source patterns first, then delete/replace code references, then scrub docs, then verify with repo-wide search and targeted cargo tests
