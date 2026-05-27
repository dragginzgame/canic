# 0.46 Status: Multi-Deployment Operations

Last updated: 2026-05-27

## Purpose

This file is the permanent implementation status log for the 0.46 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

In progress.

0.46 depends on deployment truth, promotion, and external lifecycle state so
multiple deployment targets can be compared and operated without conflating
template identity with live deployment identity.

The deployment-target local state hard cut is underway: fleet templates remain
reusable topology inputs, deployments are the only live target state, and old
fleet-named install state is refused as deployment truth.

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
- Moved local install state to
  `.canic/<network>/deployments/<deployment>.json`; persisted state now records
  `deployment_name`, `fleet_template`, `created_at_unix_secs`,
  `updated_at_unix_secs`, and `root_verification`.
- Deployment truth now reads deployment-target state by deployment name, and
  supplied-plan install requires exact deployment identity instead of accepting
  a fleet-template fallback.
- Legacy `.canic/<network>/fleets/<fleet>.json` state is rejected with explicit
  recovery guidance instead of being projected into deployment truth.
- Added `canic deploy register <deployment> --fleet-template <fleet> --root
  <principal> --allow-unverified` as the explicit operator recovery path for
  known live roots. Registration writes minimal state only, marks the root
  `not_verified`, and cannot make the root trusted deployment authority.
- Stale deployment-target state that still contains the old duplicate `fleet`
  field or pre-cut `installed_at_unix_secs` timestamp field fails closed.
- Unverified registered roots now produce an install safety blocker instead of
  an ordinary plan-assumption warning, so recovery state cannot authorize
  mutation before explicit verification evidence is recorded.
- Legacy fleet-state recovery guidance now keeps deployment target and
  fleet-template identity separate by requiring the operator to supply the
  owning `<fleet-template>` explicitly.
- Added source-guard coverage keeping `canic deploy check` and host
  deployment-truth check/preflight paths read-only, so checks cannot silently
  update `root_verification`.
- Deployment comparison now preserves blocked/warning input check status, so a
  comparison between matching unsafe `DeploymentCheckV1` artifacts cannot be
  rendered as safe just because no cross-target drift was found.
- Comparison report validation now requires each archived target to retain its
  deployment name and network, keeping comparison artifacts aligned with the
  deployment-target identity hard cut.
- Comparison now treats stale or tampered input `DeploymentCheckV1` diff/report
  content as a hard failure before summarizing drift, so archived check
  artifacts cannot hide unsafe state by carrying a forged safe report.
- `canic deploy compare` help and text-rendering coverage now make that
  revalidation boundary visible to operators.
- Release commits now use a release-index guard that rejects staged
  non-release files and partially staged release files before creating the
  release commit/tag. This keeps code slices separate from version-only release
  commits.

## Not Implemented Yet

- Passive comparison beyond the current two-check artifact command.
- Live inventory crawling for comparison inputs.
- Verified-root registration that can write `root_verification = "verified"`.

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
