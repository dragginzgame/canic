# 0.44 Status: Artifact Promotion

Last updated: 2026-05-25

## Purpose

This file is the permanent implementation status log for the 0.44 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Started with passive artifact-source modeling.

0.44 depends on deployment truth and backend-agnostic receipts so artifact
promotion can prove what was built, uploaded, installed, and promoted.

## Implemented

- Added `RoleArtifactSourceV1` and source-kind DTOs for role-scoped promotion
  artifact sources.
- Added validation for digest-pinned executable override inputs, lowercase
  sha256 digest shape, required locators, and receipt-backed artifact source
  eligibility.
- Receipt-backed artifact sources currently accept only deployment receipt or
  staging receipt evidence. Authority dry-run receipts/evidence are not
  representable as artifact sources.
- Added the first passive `PromotionReadinessV1` model, with role-scoped
  source identity, target wasm/config identity, byte/config identity
  comparisons, blockers, warnings, and restage-required reporting.
- Added validation for archived `PromotionReadinessV1` artifacts, including
  schema, identity fields, status/blocker consistency, duplicate roles, digest
  shape, restage state, and finding severity checks.
- Added host-owned passive text rendering for `PromotionReadinessV1`.
- Added JSON shape coverage for the initial promotion source/input/readiness
  DTOs and semantic coverage distinguishing source/build target-config changes
  from sealed-wasm embedded-config mismatch.
- Added `check_promotion_readiness(...)` as the host-owned passive entry point
  that builds and validates readiness from a target plan plus role promotion
  inputs.

## Not Implemented Yet

- Full artifact promotion plan model.
- Promotion receipts and provenance.
- Full promotion safety checks across deployment targets.
- Integration with wasm-store artifact identity.
- Source locator kind versus artifact identity dedupe/report semantics.
- Full source/build materialization environment identity, including target,
  linker, deterministic-build mode, wasm optimization, and compression
  identity.
- Explicit role policy distinction between "must use sealed bytes" and
  "rebuild allowed only if byte-identical output is proven."
- Promotion plan/source receipt/target execution lineage identity.

## Drift Log

- No implementation drift recorded yet.

## Release Bar

0.44 should not close until promoted artifacts carry enough provenance to be
checked against deployment truth before use.
