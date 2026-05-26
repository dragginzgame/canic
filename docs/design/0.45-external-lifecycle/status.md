# 0.45 Status: External Lifecycle

Last updated: 2026-05-26

## Purpose

This file is the permanent implementation status log for the 0.45 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Started.

0.45 now has passive lifecycle-authority projection over existing deployment
truth, lifecycle plan partitioning, derived proposal/receipt/pending/check/
handoff evidence, critical-fix residual exposure reporting, structural
external verification reporting, verification policies, and supplied-observation
verification checks. External or user-owned lifecycle flows remain explicit
report data; no consent delivery, external execution, live re-inventory, or
install mutation path has landed.

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
  plans, proposal reports, external completion receipts, pending reports,
  critical-fix reports, and external verification reports. They explicitly
  report `mode: passive` and `execution: none`.
- `ExternalLifecyclePendingReportV1` summarizes unresolved external lifecycle
  work from the lifecycle plan and proposal report, including direct,
  pending, and blocked counts; pending proposal links; residual exposure; and
  digest validation.
- `CriticalExternalFixReportV1` summarizes directly patchable roles,
  externally blocked roles, dependency-blocked roles, required external
  actions, protected-call implications, residual exposure, and operator next
  steps from lifecycle pending evidence without claiming deployment
  completion.
- `ExternalUpgradeConsentEvidenceV1` separates reported consent/action state
  from completion verification. It links a proposal/receipt pair, records
  consent state, reporter, consent requirements, allowed authorization modes,
  and a deterministic evidence digest, and remains passive structural evidence.
  `canic deploy external inspect consent --request <file>` exposes it under an
  advanced inspection namespace rather than as a top-level lifecycle workflow.
- `ExternalUpgradeVerificationReportV1` packages a validated
  proposal/receipt pair into a digest-pinned passive verification artifact.
  It records the verification result, source proposal/receipt digests, notes,
  and whether fresh live inventory remains required.
- `ExternalUpgradeVerificationPolicyV1` makes required live-inventory
  postconditions explicit before an externally reported lifecycle action can
  be treated as complete. It records source proposal digests, required
  verification facts, expected module/config facts, protected-call readiness
  requirements, and passive status text.
- `ExternalUpgradeVerificationCheckV1` evaluates supplied observation facts
  against an `ExternalUpgradeVerificationPolicyV1`, recording per-requirement
  expected/observed values and a verified/mismatch result without querying live
  inventory itself.
- `ExternalLifecycleCheckV1` summarizes lifecycle plan, proposal, and pending
  evidence into one passive status artifact with direct, pending, blocked, and
  residual-exposure counts, source artifact digests, summary text, and next
  actions.
- `ExternalLifecycleHandoffV1` packages pending external proposals into
  passive operator coordination instructions with proposal/check/pending
  digests, consent channel/subject facts, target verification facts, blocked
  subjects, residual exposure, and deterministic handoff validation.
- `canic deploy external plan <fleet>`, `proposals <fleet>`,
  `check <fleet>`, `handoff <fleet>`, `pending <fleet>`, and
  `critical-fix <fleet>` expose local deployment-truth external lifecycle
  artifacts as JSON by default or passive text with `--format text`.
- `canic deploy external verify --request <file>` reads an
  `ExternalUpgradeVerificationReportRequest` JSON file and emits a passive
  `ExternalUpgradeVerificationReportV1` without live lookup, consent delivery,
  external execution, install, or mutation.
- `canic deploy external inspect verification-policy --request <file>` reads
  an `ExternalUpgradeVerificationPolicyRequest` JSON file and emits a passive
  `ExternalUpgradeVerificationPolicyV1` without live lookup, consent delivery,
  external execution, install, or mutation.
- `canic deploy external inspect verification-check --request <file>` reads an
  `ExternalUpgradeVerificationCheckRequest` JSON file and emits a passive
  `ExternalUpgradeVerificationCheckV1` from supplied observation facts without
  live lookup, consent delivery, external execution, install, or mutation.
- JSON shape and projection coverage pins deployment-controlled,
  user-controlled, and unknown-unsafe lifecycle authority behavior, plus the
  first external proposal, receipt, consent evidence, verification request,
  and verification report artifact shapes.

## Not Implemented Yet

- Consent delivery and external action execution workflow.
- Safe upgrade/install boundaries for externally controlled canisters.
- Live re-inventory integration for external lifecycle verification. Current
  verification reports and checks use supplied structural evidence only.

## Drift Log

- The first implementation slice follows the 0.42.14 handoff constraint:
  lifecycle authority is projected from existing `CanisterControlClassV1`
  observations instead of introducing a second user/external classification
  model.

## Release Bar

0.45 should not close until Canic can represent user-owned or externally
controlled lifecycle states without pretending Canic has unilateral authority.
