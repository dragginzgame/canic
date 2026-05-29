# 0.47 Status: Verified Deployment-Target Registration

Last updated: 2026-05-29

## Purpose

This file is the permanent implementation status log for the 0.47 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Closed with documented caveats.

0.47 closed the registered-root recovery caveat from 0.46. Registered
deployment roots still begin as `not_verified`; registration remains recovery,
not verification. A registered root can be promoted to `verified` only by
explicit root verification against bound deployment-truth evidence.

0.47 intentionally did not add broad live deployment verification, live
inventory crawling, deployment catalog/group/readiness UX,
teardown/test-deployment lifecycle, or root rotation.

## Implemented

- Added explicit `DeploymentRootObservationV1` evidence to deployment truth
  inventory, including deployment target, network, fleet template, observed
  root principal, observed canister ID, observation source, controller facts,
  module hash, status, and role assignment source.
- Added passive `DeploymentRootVerificationRequestV1` and
  `DeploymentRootVerificationReportV1` artifacts for inspecting whether a
  source `DeploymentCheckV1` contains acceptable root evidence.
- Kept passive root inspection read-only. `canic deploy root inspect` can
  report satisfied evidence but cannot update local state or emit a mutation
  receipt.
- Added `canic deploy root verify <deployment> --from-check <file>` as the
  explicit state transition from `not_verified` to `verified`.
- Required root verification to validate source check identity, root evidence,
  observed-root source, digest shape, check-row consistency, and report
  consistency before local-state mutation.
- Required accepted root evidence to come from deployment-truth
  `IcpCanisterStatus` observation. Local-state-only root observations remain
  honest inventory but cannot promote trust.
- Wrote verified root state through a guarded compare-and-swap helper so stale
  local state prevents promotion.
- Emitted `DeploymentRootVerificationReceiptV1` only after a successful
  guarded local-state write, or as an explicit no-op receipt for same-root
  re-verification.
- Rejected verified-root replacement attempts.
- Preserved and validated source report status, source report source,
  requested timestamp, observed-root canister ID, previous verification state,
  passive state transition, and local-state digest transitions in receipts.
- Kept `deploy check`, `deploy compare`, and passive root inspection read-only
  through source-guard coverage.

## Deferred Beyond 0.47

- General `canic deploy verify` profiles.
- Live inventory crawling for arbitrary deployment verification.
- Wall-clock freshness or observation-age policy.
- Metrics, protected-call, and runtime profile verification.
- Deployment group/catalog/readiness UX.
- Root rotation or root replacement.
- Generalized verification evidence embedded inside install state.
- Teardown/test-deployment orchestration.

## Drift Log

- The implementation stayed narrow: it verifies registered deployment roots
  from existing deployment-truth check artifacts rather than introducing a
  broad live verification system.
- Offline/local deployment checks can still produce honest root inventory, but
  only live `IcpCanisterStatus` observed-root evidence can satisfy promotion.
- Root verification is a local trust-state transition, not an IC/controller
  mutation and not a full deployment health verdict.

## Release Bar

Closed. An explicitly registered deployment root can be promoted from
`not_verified` to `verified` only by an explicit root-verification command
using bound deployment-truth evidence, and mismatched, stale, legacy, passive,
operator-asserted, or incomplete evidence fails closed without authorizing
mutation.
