# 0.41 Status: Deployment Truth Model

Last updated: 2026-05-22

## Purpose

This file is the permanent implementation status log for the 0.41 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Active implementation is underway.

0.41 has moved beyond design preparation into the passive host-side deployment
truth model and local observation layer. It has not yet reached installer
gating.

## Implemented

- Added passive `canic-host::deployment_truth` V1 DTOs for deployment plans,
  inventories, receipts, diffs, safety reports, role artifacts, canister control
  classifications, verifier-readiness observations, and phase postconditions.
- Split `canic-host::deployment_truth` into focused modules:
  - `mod.rs`: public exports and schema version.
  - `model.rs`: passive V1 DTOs.
  - `observe.rs`: read-only local observation.
  - `report.rs`: diff and safety-report classification.
  - `tests.rs`: focused host-side coverage.
- Added read-only local inventory collection from configured fleet roles, local
  install-state root identity, and materialized `.wasm.gz` artifacts.
- Added read-only local role artifact manifest collection from configured roles
  and materialized artifacts.
- Added read-only local deployment plan construction from resolved fleet config
  and the local role artifact manifest.
- Added a read-only local deployment check wrapper that returns plan,
  inventory, diff, and safety report together.
- Added a read-only current-install preflight helper that adapts
  `InstallRootOptions` into the local deployment truth check without mutating
  deployment state.
- Added `canic deploy plan|inventory|check <fleet>` as read-only
  operator-facing commands that emit local deployment truth JSON. They are
  report surfaces, not executor replacements.
- Wired the first current-install safety gate after build and before
  manifest/install/stage continuation. The gate blocks missing configured role
  artifacts while leaving broader live-inventory warnings report-only.
- Captured missing config, artifact roots, release-set manifests, and role
  artifacts as typed observation gaps instead of installer errors.
- Preserved release-set payload hashes with `ReleaseSetManifest` source
  metadata.
- Added observed local `.wasm.gz` file SHA-256 evidence with
  `ObservedFileDigest` source metadata.
- Added passive diff/report generation for missing artifacts, unsafe control
  classes, identity mismatches, canonical config drift, unobserved verifier
  readiness, and observation gaps.
- Surfaced observed artifact file hashes as informational
  `artifact_file_sha256` evidence instead of comparing them as release-set
  payload hashes.

## Not Implemented Yet

- Build a real `DeploymentPlanV1` from resolved config/build intent.
- Extend `DeploymentInventoryV1` with live IC observations such as controllers,
  installed module hashes, and canister status.
- Extend post-build materialization checks beyond missing configured role
  artifacts.
- Emit lightweight `DeploymentReceiptV1` records from existing installer phases.
- Compare plan, inventory, and receipt during install/resume.
- Gate mutating installer operations on broader `SafetyReportV1` findings.
- Expose an operator-facing deployment truth report through CLI/host commands.

## Drift Log

- The first installer refusal gate is intentionally narrow. It blocks missing
  configured role artifacts after build, but broader authority, live canister,
  and resume-safety findings remain report-only until live inventory improves.
- Artifact evidence now distinguishes release-set payload hashes from observed
  local file hashes. The original design called for artifact truth, but this
  implementation makes source semantics explicit before using hashes as safety
  authority.

## Release Bar

0.41 should not close until the current install path has at least one more
operator-facing validation pass confirming the new report and artifact gate work
on realistic local install inputs.
