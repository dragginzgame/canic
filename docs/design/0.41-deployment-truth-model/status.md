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
- Add post-build materialization checks to the current build/install path.
- Emit lightweight `DeploymentReceiptV1` records from existing installer phases.
- Compare plan, inventory, and receipt during install/resume.
- Gate mutating installer operations on `SafetyReportV1`.
- Expose an operator-facing deployment truth report through CLI/host commands.

## Drift Log

- The design says 0.41 should make the installer refuse unsafe states. Current
  implementation is intentionally earlier: model, local observation, and passive
  reporting exist, but installer refusal is not wired yet.
- Artifact evidence now distinguishes release-set payload hashes from observed
  local file hashes. The original design called for artifact truth, but this
  implementation makes source semantics explicit before using hashes as safety
  authority.

## Release Bar

0.41 should not close until Canic can produce a deployment truth report from the
current install/build inputs and use at least the first safety gate before a
mutating installer operation.
