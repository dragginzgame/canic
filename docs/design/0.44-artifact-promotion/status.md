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
- Added `promoted_deployment_plan_from_inputs(...)` as a pure plan
  transformation helper. It applies validated sealed-wasm artifact identity to
  selected target roles while preserving target authority/trust-domain fields;
  source/build promotion leaves target materialization output in the target
  plan.
- Added `PromotionPlanTransformV1` and
  `promoted_deployment_plan_transform_from_inputs(...)` so passive promotion
  reports can carry the promoted plan plus role-scoped before/after artifact
  identity, embedded-config change, and target materialization preservation
  facts.
- Added host-owned passive text rendering for `PromotionPlanTransformV1`.
- Added validation for archived `PromotionPlanTransformV1` artifacts,
  including schema, identity fields, promoted-plan linkage, duplicate roles,
  role presence, role summary consistency, and transform flag consistency.
- Added `PromotionPlanTransformEvidenceV1` as a passive provenance wrapper for
  validated promotion transforms, with evidence ID, generated-at metadata, and
  validation that rechecks the nested transform.
- Added host-owned passive text rendering for `PromotionPlanTransformEvidenceV1`
  that explicitly reports no execution occurred.
- Added `PromotionArtifactIdentityReportV1` to separate role source locator
  kind from artifact identity kind before promotion planning consumes role
  sources.
- Promotion artifact identity reports now group roles by deterministic artifact
  identity key so operator output can show when distinct source locators resolve
  to the same sealed or source/build identity.
- Added host-owned passive text rendering for
  `PromotionArtifactIdentityReportV1`.
- Added passive `BuildRecipeIdentityV1`, `BuildMaterializationInputV1`, and
  `BuildMaterializationResultV1` DTOs so source/build promotion can record the
  reusable build recipe, target-specific materialization input, and concrete
  output as separate evidence objects.
- Added validation for source/build materialization identity fields, including
  required IDs, builder/toolchain selectors, config digests, and output digest
  shape.
- Added `BuildMaterializationEvidenceV1` to link a recipe, materialization
  input, and materialization result with computed input-digest evidence and
  explicit passive text rendering.
- Added passive `RolePromotionPolicyV1` and `PromotionPolicyCheckV1` so
  promotion can report role policy decisions before execution, including the
  distinction between roles that must reuse sealed bytes and roles that may
  rebuild only when byte-identical output is later proven.
- Promotion readiness can now optionally fold role promotion policy blockers
  into the same passive `PromotionReadinessV1` artifact, while keeping the
  standalone policy check available for separate operator reports.
- Source/build promotion transforms can now opt into validated materialization
  evidence links, recording the evidence ID, target materialization input
  digest, and materialized output digests in the role transform summary.
- Passive promotion transforms now carry a deterministic promotion-plan lineage
  digest over the target plan ID, promoted plan ID, promoted plan, and role
  summaries. Validation rejects stale lineage digests.
- Receipt-backed promotion artifact sources now require a source receipt
  lineage digest, and non-receipt sources reject that field.
- Added passive target execution lineage artifacts that bind a validated
  promotion transform to a validated target execution preflight and explicitly
  record that no execution has occurred.
- Added a passive artifact promotion plan envelope that ties together
  readiness, artifact identity, promoted-plan transform, and optional target
  execution lineage artifacts without becoming an execution shortcut.
- Added target-check validation for artifact promotion plans, proving the
  promoted plan and execution preflight match the deployment truth check that
  would gate later execution.

## Not Implemented Yet

- Promotion execution receipts and richer provenance.
- Integration with wasm-store artifact identity.
- Artifact identity dedupe semantics beyond passive report grouping.
- CLI/operator report surfaces for source/build materialization identity.
- CLI/report surfaces for role promotion policy checks.

## Drift Log

- No implementation drift recorded yet.

## Release Bar

0.44 should not close until promoted artifacts carry enough provenance to be
checked against deployment truth before use.
