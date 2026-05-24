# 0.43 Status: Backend-Agnostic Execution

Last updated: 2026-05-24

## Purpose

This file is the permanent implementation status log for the 0.43 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Started with the 0.43.0 executor-boundary scaffold.

0.43 depends on 0.41 deployment truth and 0.42 authority reconciliation so
execution backends can run against an explicit plan and safety report rather
than implicit installer state.

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

## Not Implemented Yet

- Full backend-neutral execution model.
- Separation between execution planning and the concrete local/IC backend.
- Backend-specific receipts mapped into the common deployment receipt model.
- Validation that backend behavior does not bypass deployment truth gates.

## Drift Log

- No implementation drift recorded yet.

## Release Bar

0.43 should not close until deployment execution can be represented and audited
independently of a single local installer backend, and at least one test or
harness path validates the same `DeploymentPlanV1` shape used by real
deployment execution.
