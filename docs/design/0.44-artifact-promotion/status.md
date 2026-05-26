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
- Promotion readiness artifacts now carry deterministic readiness digests over
  their target plan link, status, role rows, blockers, and warnings, so
  archived readiness reports reject stale pre-plan drift directly.
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
- Promotion transform evidence artifacts now carry deterministic evidence
  digests over their metadata and nested transform, so archived transform
  evidence rejects stale wrapper or transform drift directly.
- Added host-owned passive text rendering for `PromotionPlanTransformEvidenceV1`
  that explicitly reports no execution occurred.
- Added `PromotionArtifactIdentityReportV1` to separate role source locator
  kind from artifact identity kind before promotion planning consumes role
  sources.
- Promotion artifact identity reports now group roles by deterministic artifact
  identity key so operator output can show when distinct source locators resolve
  to the same sealed or source/build identity.
- Promotion artifact identity reports now include validated summary counters for
  role count, identity group count, shared identity groups, digest-pinned roles,
  source/build roles, and deferred identities so dedupe semantics are explicit
  report data rather than presentation-only grouping.
- Promotion artifact identity reports now carry deterministic report digests
  over status, summary, role identity rows, identity groups, and blockers, so
  archived identity reports reject stale grouping or summary drift directly.
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
- Build materialization evidence now carries deterministic evidence digests
  over the recipe, materialization input, materialization result, computed
  input digest, and consistency flags. Materialization identity reports and
  source-build transform links preserve that digest beside the evidence ID.
- Added passive `RolePromotionPolicyV1` and `PromotionPolicyCheckV1` so
  promotion can report role policy decisions before execution, including the
  distinction between roles that must reuse sealed bytes and roles that may
  rebuild only when byte-identical output is later proven.
- Promotion policy checks now carry deterministic check digests over their
  status, role decisions, and blockers, so archived policy reports reject stale
  decision drift directly.
- Added passive CLI report surfacing for role promotion policy checks through
  `canic deploy promote inspect policy --request <file>`, with JSON output by
  default and host-owned text output through `--format text`.
- Added passive CLI report surfacing for promotion readiness and artifact
  identity reports through
  `canic deploy promote inspect readiness --request <file>` and
  `canic deploy promote inspect artifact-identity --request <file>`, with JSON
  output by default and host-owned text output through `--format text`.
- Promotion readiness can now optionally fold role promotion policy blockers
  into the same passive `PromotionReadinessV1` artifact, while keeping the
  standalone policy check available for separate operator reports.
- Source/build promotion transforms can now opt into validated materialization
  evidence links, recording the evidence ID, materialization evidence digest,
  target materialization input digest, and materialized output digests in the
  role transform summary.
- Passive promotion transforms now carry a deterministic promotion-plan lineage
  digest over the target plan ID, promoted plan ID, promoted plan, and role
  summaries. Validation rejects stale lineage digests.
- Added passive CLI report surfacing for promoted-plan transforms and
  transform-evidence wrappers through
  `canic deploy promote inspect transform --request <file>` and
  `canic deploy promote inspect transform-evidence --request <file>`, with
  JSON output by default and host-owned text output through `--format text`.
- Added passive CLI report surfacing for target execution lineage through
  `canic deploy promote inspect target-lineage --request <file>`, with JSON
  output by default and host-owned text output through `--format text`.
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
- Added passive wasm-store artifact identity reports derived from staging
  receipts, preserving role locators, transport, chunk publication counts, and
  verified postcondition facts without querying `wasm_store`.
- Passive wasm-store artifact identity reports now carry deterministic report
  digests over staged role locators, transport, chunk facts, verified
  postconditions, status, and blockers, so archived staged artifact identity
  reports reject stale staging drift directly.
- Added passive wasm-store catalog verification reports that compare staged
  wasm-store promotion identity against supplied catalog observations and
  report missing catalog entries, artifact mismatches, or chunk-count
  mismatches without querying `wasm_store` or executing promotion. Role-level
  catalog observations carry deterministic digests so archived catalog
  evidence cannot drift silently.
- Passive wasm-store catalog verification reports now carry deterministic
  verification digests over the wasm-store identity report link, role
  observations, status, and blockers, so archived catalog verification
  artifacts reject stale catalog-observation drift directly.
- Added passive CLI report surfacing for staged wasm-store identity and
  supplied catalog verification reports through
  `canic deploy promote inspect wasm-store-identity --request <file>` and
  `canic deploy promote inspect catalog-verification --request <file>`, with
  JSON output by default and host-owned text output through `--format text`.
- Added passive source/build materialization identity reports that aggregate
  validated materialization evidence by role and group roles by materialized
  output identity.
- Source/build materialization identity reports now carry deterministic report
  digests over their role evidence, output groups, status, and blockers, so
  archived materialization identity reports reject stale output grouping drift
  directly.
- Added passive CLI report surfacing for source/build materialization identity
  reports through
  `canic deploy promote inspect materialization-identity --request <file>`,
  with JSON output by default and host-owned text output through
  `--format text`.
- Added passive artifact promotion provenance reports that link a promotion
  plan to readiness, artifact identity, transform, target execution lineage,
  wasm-store identity, wasm-store catalog verification, and materialization
  identity report IDs without claiming execution.
- Promotion provenance reports now cite wasm-store catalog verification reports
  by both ID and digest, making supplied catalog-observation provenance
  linkage digest-pinned rather than ID-only.
- Promotion provenance validates linked wasm-store catalog verification against
  the same wasm-store identity report and turns mismatched or unknown catalog
  evidence into blockers rather than treating it as live artifact truth.
  Role-level provenance rows also preserve the catalog observation digest, and
  provenance blocks locator drift between the identity report and the supplied
  catalog verification.
- Role-level provenance rows now also preserve the materialization evidence
  digest for source/build roles, keeping materialization references
  digest-pinned when provenance is inspected without the full materialization
  report loaded.
- Promotion execution receipt wrappers now preserve the role-level catalog
  observation and materialization evidence digests from provenance, keeping
  receipt evidence tied to the same archived catalog/materialization artifacts
  without making the receipt a live catalog or build proof.
- Artifact promotion plan envelopes now carry a deterministic plan digest over
  the plan linkage, readiness, artifact identity, transform, optional target
  execution lineage, and blocker set, so archived plans reject stale plan-body
  drift.
- Promotion provenance reports now cite the artifact promotion plan by both ID
  and digest, making plan/provenance linkage digest-pinned rather than ID-only.
- Promotion provenance reports now carry a deterministic provenance digest over
  linkage fields, role evidence, blockers, and the passive execution boundary,
  so archived provenance artifacts reject stale drift in their own report
  contents.
- Added passive CLI report surfacing for artifact promotion plan envelopes
  through `canic deploy promote plan --request <file>`, readiness checks
  through `canic deploy promote check --request <file>`, and transform diffs
  through `canic deploy promote diff --request <file>`, with JSON output by
  default and host-owned text output through `--format text`.
- Added passive CLI report surfacing for provenance reports through
  `canic deploy promote inspect provenance --request <file>`, keeping
  DTO-level provenance under the advanced inspection namespace.
- Promotion execution receipt wrappers now carry the provenance report digest
  alongside the provenance report ID, making receipt/provenance linkage
  digest-pinned rather than ID-only.
- Promotion execution receipt wrappers also carry the artifact promotion plan
  digest alongside the plan ID, keeping receipt/plan linkage digest-pinned even
  when receipts are inspected without the full provenance artifact loaded.
- Promotion provenance reports now cite optional wasm-store identity reports by
  both ID and digest, making staged artifact provenance linkage digest-pinned
  rather than ID-only.
- Promotion provenance reports now cite optional materialization identity
  reports by both ID and digest, making source/build provenance linkage
  digest-pinned rather than ID-only.
- Promotion execution receipt wrappers now also carry a deterministic execution
  receipt digest over receipt linkage, nested deployment receipt data, and role
  evidence, so archived receipt artifacts reject stale receipt drift.
- Added passive artifact promotion execution receipt wrappers that link a
  validated promotion provenance report to an existing deployment receipt,
  preserve promoted-plan lineage, and surface role-level execution evidence
  without introducing a separate promotion executor. Execution receipt wrappers
  require ready promotion provenance, so blocked passive provenance cannot be
  represented as a promotion execution artifact. They also require the nested
  deployment receipt role evidence to match the promotion provenance role set.
- Added passive CLI report surfacing for artifact promotion execution receipt
  wrappers through
  `canic deploy promote inspect execution-receipt --request <file>`, with JSON
  output by default and host-owned text output through `--format text`.

## Not Implemented Yet

- Execution-path emission of promotion execution receipts.
- Live `wasm_store` catalog lookup. Catalog verification now exists for
  supplied observations, but no live catalog reader is wired yet.
- Artifact identity dedupe policy decisions beyond passive summary/grouping.

## Drift Log

- No implementation drift recorded yet.

## Release Bar

0.44 should not close until promoted artifacts carry enough provenance to be
checked against deployment truth before use.
