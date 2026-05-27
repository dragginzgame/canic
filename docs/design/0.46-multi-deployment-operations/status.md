# 0.46 Status: Multi-Deployment Operations

Last updated: 2026-05-26

## Purpose

This file is the permanent implementation status log for the 0.46 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

In progress.

0.46 depends on deployment truth, promotion, and external lifecycle state so
multiple deployment targets can be compared and operated without conflating
template identity with live deployment identity.

The next required correction is the deployment-target local state hard cut:
fleet templates remain reusable topology inputs, deployments become the only
live target state, and old fleet-named install state must not be used as
deployment truth.

## Implemented

- Added the first passive cross-deployment comparison artifact:
  `DeploymentComparisonReportV1`.
- Added `deployment_comparison_report_from_checks(...)` and
  `validate_deployment_comparison_report(...)` so two existing
  `DeploymentCheckV1` artifacts can be compared without querying live state or
  mutating deployments.
- The comparison report records check/plan/inventory digests for both sides
  and keeps normalized categories for identity, trust-domain, artifact, module
  hash, embedded config, authority, pool, verifier readiness, and external
  lifecycle evidence.
- External lifecycle comparison preserves the 0.45 handoff boundary:
  control-class evidence is compared as evidence, and supplied observation,
  consent, reported action, and completion artifacts are not flattened into
  live deployment truth.
- Added passive text rendering and host tests for drift detection, digest
  validation, and no-execution rendering.
- Added `canic deploy compare --left <file> --right <file>` as the first
  0.46 operator command. It reads two archived `DeploymentCheckV1` JSON
  artifacts, emits `DeploymentComparisonReportV1` JSON by default or passive
  text with `--format text`, and does not query live state or mutate
  deployments.

## Not Implemented Yet

- Deployment-target local state under
  `.canic/<network>/deployments/<deployment>.json`.
- Exact deployment target identity enforcement for supplied-plan install.
- Old `.canic/<network>/fleets/<fleet>.json` state refusal with recovery
  guidance.
- Explicit operator deployment registration, such as
  `canic deploy register <deployment> --fleet-template <fleet> --root
  <principal>`, with minimal state and root verification status.
- Passive comparison beyond the current two-check artifact command.
- Live inventory crawling for comparison inputs.

## Drift Log

- The first implementation slice starts with direct comparison of existing
  `DeploymentCheckV1` artifacts instead of introducing a broader deployment
  catalog first. This keeps the 0.46 release bar anchored to comparison and
  drift reporting before group/catalog UX grows.
- A 0.41-0.45 implementation audit found that deployment-target terminology
  had moved ahead of the local state model: live install state was still
  fleet-named. 0.46 is now explicitly course-corrected so the hard-cut
  deployment-target state slice lands before group/catalog work continues.

## Release Bar

0.46 should not close until Canic can distinguish fleet templates from
deployment targets in local state and operator-visible workflows.

The minimum identity bar is:

- fleet templates are reusable desired topology, not live state;
- deployments are concrete live targets with their own local state path;
- old fleet-named local state is refused, not silently read;
- supplied-plan install requires exact deployment target identity;
- any recovery path is explicit operator registration, not automatic
  migration.
- unverified registered roots cannot authorize mutation, and read-only
  `deploy check` must not silently rewrite verification state.
