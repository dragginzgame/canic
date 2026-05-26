# 0.45 Status: External Lifecycle

Last updated: 2026-05-26

## Purpose

This file is the permanent implementation status log for the 0.45 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Started.

0.45 now has the first passive lifecycle-authority projection over existing
deployment truth, lifecycle plan partitioning, and derived proposal/receipt
evidence. External or user-owned lifecycle flows remain explicit report data;
no consent delivery, external execution, or install mutation path has landed.

## Implemented

- `LifecycleAuthorityReportV1` and `LifecycleAuthorityV1` model the
  role/canister lifecycle authority projection for a `DeploymentCheckV1`.
- Lifecycle authority reports carry deterministic report digests and validation
  checks for required IDs, duplicate subjects, count drift, and stale digest
  drift.
- `lifecycle_authority_report_from_check(...)` consumes existing
  `CanisterControlClassV1` classifications from plans and inventory. It does
  not reclassify controller ownership, query IC state, mutate deployment state,
  or create an external lifecycle execution path.
- Lifecycle authority rows report direct deployment-authority lifecycle,
  external proposal/execution, verify-external-completion, observe-only, and
  blocked modes, plus verification requirements that later proposal/receipt
  surfaces can cite.
- `ExternalLifecyclePlanV1` partitions lifecycle rows into directly executable,
  externally proposed, and blocked upgrades. It carries a deterministic plan
  digest plus residual exposure and protected-call implications.
- Lifecycle plan validation checks required IDs, duplicate subjects, status
  consistency, stale digest drift, and optional source-check linkage against
  the `DeploymentCheckV1` it claims to derive from.
- `ExternalUpgradeProposalReportV1` and `ExternalUpgradeProposalV1` model the
  first passive proposal artifacts for externally actionable lifecycle rows.
  Proposal reports are derived from `ExternalLifecyclePlanV1` and bind current
  observed module/config facts, target role artifact/config facts, root trust
  anchor, authority profile identity, consent requirements, proposal/lifecycle
  digests, and allowed authorization modes without granting consent or
  attempting execution.
- Proposal reports carry deterministic report digests and validation checks for
  required IDs, nested proposal digests, duplicate proposal subjects, and
  directly controlled rows accidentally appearing as external proposals. They
  can also be validated against their source lifecycle plan and deployment
  truth check to reject stale archived proposal evidence.
- Blocked lifecycle rows are reported as blocked subjects instead of producing
  executable-looking proposals.
- `ExternalUpgradeReceiptV1` models pending, refused, delegated, and
  externally executed lifecycle outcomes. Receipt validation checks structural
  consistency only; live inventory remains the source of truth for completion.
- Receipt validation now rejects stale receipt digests while preserving
  semantic checks for refused-but-verified and missing-observation claims.
- Passive text renderers exist for lifecycle authority reports, lifecycle
  plans, proposal reports, and external completion receipts. They explicitly
  report `mode: passive` and `execution: none`.
- JSON shape and projection coverage pins deployment-controlled,
  user-controlled, and unknown-unsafe lifecycle authority behavior, plus the
  first external proposal and receipt artifact shapes.

## Not Implemented Yet

- Consent and operator handoff workflow.
- Safe upgrade/install boundaries for externally controlled canisters.
- Live re-inventory integration for external lifecycle verification.

## Drift Log

- The first implementation slice follows the 0.42.14 handoff constraint:
  lifecycle authority is projected from existing `CanisterControlClassV1`
  observations instead of introducing a second user/external classification
  model.

## Release Bar

0.45 should not close until Canic can represent user-owned or externally
controlled lifecycle states without pretending Canic has unilateral authority.
