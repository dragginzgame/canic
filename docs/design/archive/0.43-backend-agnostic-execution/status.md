# 0.43 Status: Backend-Agnostic Execution

Last updated: 2026-05-25

## Purpose

This file is the permanent implementation status log for the 0.43 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Started with the 0.43.0 executor-boundary scaffold.

0.43 depends on 0.41 deployment truth and 0.42 authority reconciliation so
execution backends can run against an explicit plan and safety report rather
than implicit installer state.

0.43 is closed at `0.43.8`. The closeout report is
[docs/audits/reports/2026-05/2026-05-25/0.43-closeout.md](../../audits/reports/2026-05/2026-05-25/0.43-closeout.md).

## Implemented

- `DeploymentExecutionContextV1`, `DeploymentExecutorBackendV1`, and
  `DeploymentExecutorCapabilityV1` model the backend, roots, artifact roots,
  and capability evidence that execution receipts can carry.
- `DeploymentReceiptV1` now has optional execution context metadata. Existing
  generic receipt builders populate it as `None`, while the current install
  flow attaches current-CLI execution context before persisting its deployment
  truth receipts.
- Added a minimal `DeploymentExecutor` trait plus current-CLI capability
  helpers and deterministic missing-capability reporting.
- Added a concrete current-CLI executor wrapper and routed current-install
  execution context construction through that executor object.
- Current install now checks that the selected backend advertises the
  capabilities required by the existing install phases before those phases
  begin mutating deployment state.
- Added a passive `DeploymentExecutionPreflightV1` model that consumes
  `DeploymentPlanV1`, `SafetyReportV1`, `AuthorityReconciliationPlanV1`, and
  executor capabilities to produce a ready/blocked execution gate without
  running backend operations.
- Added validation helpers for `DeploymentExecutionPreflightV1` artifacts,
  including source-`DeploymentCheckV1` identity checks, status/blocker
  consistency, required/missing capability consistency, and schema/provenance
  guards. Current-install preflight paths validate the artifact before
  returning read-only readiness or writing the `execution_preflight` receipt.
- Added host tests pinning the `DeploymentExecutionPreflightV1` JSON field
  shape and enum strings before any public CLI surface is promoted.
- Current-install deployment truth receipts now record workspace root, ICP
  root, artifact roots, backend, and backend capability evidence as metadata
  on the existing receipt shape.
- Added `StagingReceiptV1` and `ArtifactTransportV1` as the typed artifact
  staging evidence shape anticipated by the executor design. Current install
  now derives richer `stage_release_set` phase evidence from the release-set
  manifest, including role, artifact identity, wasm-store transport,
  prepared chunk hashes, published chunk counts, and verified postconditions,
  without changing current installer mutation behavior.
- Added shared receipt status classification for generic deployment receipt
  construction. Failed receipts now derive `FailedBeforeMutation`,
  `FailedAfterMutation`, or `PartiallyApplied` from command result plus
  role-phase evidence unless a caller explicitly supplies a more specific
  status for phases that are not role-representable yet.
- Receipt-aware resume safety now rejects persisted receipts whose claimed
  execution status contradicts the command result and role-phase evidence,
  preventing stale or hand-edited execution receipts from overstating resume
  safety.
- Added a narrow `TestkitPreflightContext` and a plan-shape preflight
  test proving the testkit harness path consumes the same
  `DeploymentPlanV1`, `SafetyReportV1`, authority reconciliation plan, and
  phase list as the current CLI executor. This deliberately is not a full
  test harness execution backend in `canic-host`.
- Routed current-install root wasm installation, root funding, and
  `stage_release_set` through narrow operation values that own phase evidence
  and execution calls. This preserves current installer behavior while moving
  those phases closer to the executor operation boundary.
- Routed current-install root bootstrap resume and readiness waiting through
  narrow operation values that own phase evidence and execution calls. This
  preserves current installer behavior while reducing the remaining ad hoc
  activation closure wiring before the executor boundary is fully separated.
- Routed current-install configured artifact builds through a narrow operation
  value that owns build-target evidence, role names, and the existing build
  call. This preserves current build behavior while keeping pre-activation
  phase evidence on the same operation boundary as later install phases.
- Routed current-install root canister resolution through a narrow operation
  value that owns root-target evidence and the existing root lookup/create
  call. This preserves current canister creation behavior while keeping the
  pre-build receipt phase on the same operation boundary as later phases.
- Routed current-install release-set manifest emission through a narrow
  operation value that owns manifest-path evidence and the existing manifest
  writer call. This preserves current manifest output while keeping the
  pre-activation manifest phase on the same operation boundary as later phases.
- Aligned current-install execution preflight phase evidence with the actual
  deployment-truth receipt phases emitted by the installer, replacing the older
  coarse phase list with receipt-level phase names.
- Added a private current-install phase-operation runner, so activation phases
  now execute through a common phase/action/evidence boundary instead of
  manually wiring each operation into `run_phase`.
- Added source-guard coverage proving current-install activation phases use
  the operation runner and run only after deployment-truth and execution
  preflight gates are recorded.

## Not Implemented Yet

- Full backend-neutral execution model.
- Full separation between execution planning and the concrete local/IC backend.
- Backend-specific mutating operation receipts mapped into the common
  deployment receipt model.
- Validation that future non-current-install backends do not bypass deployment
  truth gates.

## Drift Log

- No release-blocking implementation drift recorded.
- 0.43 deliberately closes as an internal current-install execution boundary,
  not as a full backend-neutral executor, controller apply surface, new
  backend, or public execution receipt JSON contract.

## Release Bar

0.43 closed once deployment execution could be represented and audited
independently of a single local installer backend, at least one test or harness
path validated the same `DeploymentPlanV1` shape used by real deployment
execution, and current-install activation phases were mediated by the private
operation runner after deployment-truth/preflight readiness was recorded.
