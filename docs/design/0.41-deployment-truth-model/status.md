# 0.41 Status: Deployment Truth Model

Last updated: 2026-05-22

## Purpose

This file is the permanent implementation status log for the 0.41 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Active implementation is underway.

0.41 has moved beyond design preparation into the host-side deployment truth
model, local observation layer, read-only operator JSON surfaces, and the first
narrow current-install artifact gate.

## Implemented

- Local deployment truth now treats the implicit bootstrap `wasm_store` as part
  of the passive role set. Plans expect it, local artifact manifests and
  inventories observe its `.wasm.gz` file when present, and missing bootstrap
  store artifacts remain typed gaps rather than installer-side mutation.
- Installed child canister inventory now opportunistically enriches
  subnet-registry role observations with read-only live status, controllers,
  and module hashes. Failed child status reads stay typed observation gaps and
  the registry-derived role fact remains available.
- Deployment diffs now warn on extra observed non-pool canister roles so
  unexpected registry/live topology appears in operator reports without
  blocking current installer continuation.
- Duplicate observed canisters for a planned non-pool role also surface as
  extra canister warnings instead of being hidden by the expected role name.
- Local deployment truth plans and inventories now populate
  `deployment_manifest_digest` from the observed root release-set manifest file
  when it exists. Missing manifests remain typed assumptions or observation
  gaps instead of installer authority.
- Local deployment truth plans and inventories now populate canonical runtime
  config digests from the parsed `ConfigModel`, keeping raw config SHA-256 as
  separate local consistency evidence.
- Local deployment truth identities now include stable set digests for planned
  authority, expected/observed topology, artifact sets, and pool identities
  where those passive facts are available.
- Local deployment inventories now map live subnet-registry role entries into
  observed canister facts. Registry-derived observations satisfy role
  existence and module-hash evidence without pretending controller authority
  was observed.
- Current install now persists additional deployment receipts for release-set
  manifest emission, successful root canister resolution, local artifact build,
  the IC-mutating root install/funding/staging/bootstrap phases, and observed
  `wait_ready` evidence, plus the final local install-state write. The build
  receipt now carries role-scoped artifact outcomes for configured build targets
  when those roles are present in the deployment truth plan.
- Current-install deployment truth gates now treat every
  `SafetyReportV1.hard_failures` entry as a blocker instead of maintaining a
  hand-picked blocker-code allowlist. Warnings remain report-only.
- Current-install deployment truth gates now persist the lightweight
  `DeploymentReceiptV1` artifact-gate receipt as machine-readable JSON under
  `.canic/<network>/deployment-receipts/<fleet>/` before any installer mutation.
- `canic deploy resume-report <fleet>` can now discover the latest persisted
  local deployment receipt automatically; `--receipt <file>` remains available
  for explicit comparisons.
- Added passive pool-canister comparison to deployment truth diffs. Planned
  pool identities now produce `pool_diff` entries, missing concrete pool
  canisters or mismatched pool IDs block, unsafe observed pool control classes
  block, and undeclared observed pool canisters warn without changing installer
  execution.
- Tightened passive verifier-readiness diffs so required role epochs are
  compared against observed epochs: stale observed epochs block and missing
  required role-epoch observations warn.
- Local deployment plans now populate `expected_pool` from configured
  scaling, sharding, and directory pool identities, so pool expectations appear
  in passive deployment truth reports instead of staying empty.
- Local deployment inventory can now map installed fleet registry entries into
  `observed_pool` for configured pool roles. Ambiguous role-to-pool mappings
  are reported as observation gaps rather than guessed.
- Added receipt-aware deployment truth comparison for resume reporting. It
  evaluates plan, inventory, and prior receipt identity together, reports
  blockers for mismatched plans, roots, failed commands, or unverified
  postconditions, and only marks phases resumable after live truth and receipt
  postconditions agree.
- Current-install deployment truth gates now construct and print a lightweight
  `DeploymentReceiptV1` with explicit `Complete` or `FailedBeforeMutation`
  operation status for the artifact materialization gate.
- Added read-only `canic deploy resume-report <fleet> --receipt <file>` to
  print passive `ResumeSafetyV1` JSON from the current deployment truth check
  and a prior `DeploymentReceiptV1`, without resuming or mutating state.
- Extended local deployment truth plans with installed root identity from
  `.canic` state. The plan now records the current root trust anchor and
  concrete expected root canister when local install state exists, and the
  current-install safety gate blocks when that expected root is missing from
  observed inventory.
- Fresh local deployment truth plans now record missing install state as an
  explicit non-blocking plan assumption, and deployment truth reports surface
  plan assumptions as warning findings.
- Current-install gate output now prefixes findings with stable source labels
  (`plan`, `inventory`, or `diff`) and subjects, making plan assumptions
  distinguishable from live observation gaps.
- Current-install artifact receipts now include role-scoped materialization
  evidence. Each configured role records whether its artifact was verified or
  failed, while the deployment truth check remains the gate authority.
- Wired configured deployment controllers into the local deployment truth plan.
  Controller drift checks now compare live root status against `canic.toml`
  authority intent instead of only synthetic test plans.
- Promoted the current-install deployment truth gate beyond missing artifacts:
  materialized artifact digest drift and observable controller-authority drift
  now block before manifest emission, install, or staging.
- Blocked current-install deployment truth gates now print their summary,
  receipt postcondition, and machine-readable blocker codes before returning
  the install error.
- Deployment truth gate errors and warning output now include finding codes so
  failed current installs remain scriptable without parsing prose.
- Added controller authority comparison to the deployment truth diff. Live
  root controllers must include the expected authority profile controllers;
  authority-profile overlaps block as unsafe; undeclared live controllers warn;
  declared staging and emergency controllers are recognized as intentional
  authority instead of unexplained drift.
- Corrected the config identity model after the latest design shift: raw local
  config SHA-256 values are now raw evidence only, not
  `deployment_manifest_digest`. Raw config drift still blocks as a local
  consistency finding until canonical resolved-config digests are implemented.
- Started live inventory expansion for installed roots. When local install
  state identifies a root canister, deployment truth attempts a read-only ICP
  status observation and records live controllers, module hash, and status when
  available. Failed live reads are typed observation gaps.
- Added installed module-hash comparison to the normalized diff so planned
  role module identity can be checked against live root status observations.
- Aligned `DeploymentReceiptV1` with the revised partial-execution design by
  adding operation status and role-scoped phase receipt fields. Current
  installer receipts still populate this lightly; richer per-role outcomes
  remain future execution work.
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
- Added `canic deploy diff <fleet>` and `canic deploy report <fleet>` as
  direct read-only JSON views for the normalized diff and safety report.
- Added local deployment config SHA-256 evidence to the deployment truth plan
  and inventory. The normalized diff now blocks raw config digest mismatch as
  local consistency evidence without treating it as canonical deployment
  manifest identity.
- Made `canic deploy check <fleet>` return a failing exit status for blocked
  `SafetyReportV1` output while keeping `plan`, `inventory`, `diff`, and
  `report` as read-only JSON inspection surfaces.
- Tightened local artifact consistency checks so plan-observed and
  inventory-observed `.wasm.gz` file digests for the same role must agree.
- Wired the first current-install safety gate after build and before
  manifest/install/stage continuation. The gate blocks missing configured role
  artifacts while leaving broader live-inventory warnings report-only.
- Added lightweight `materialize_artifacts` phase receipt construction for the
  current-install artifact gate. The receipt records verified postcondition
  evidence, but it is not persisted and does not replace live check authority.
- Clarified the cross-line design contract that deployment execution is not
  atomic. Receipts must be able to express partial application, per-role
  outcomes, and resume evidence without promising automatic rollback.
- Clarified the promotion design split between sealed wasm promotion and
  source/build promotion, with source/build recipe identity kept separate from
  target-specific materialization input and target materialization result.
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

- Extend `DeploymentPlanV1` beyond resolved local config/build intent with
  fuller authority, concrete pool capacity, and live-runtime expectations.
- Extend `DeploymentInventoryV1` beyond root status, local `wasm_store`
  artifact evidence, subnet-registry role observations, and opportunistic
  per-child IC status with richer authority/readiness state.
- Compute richer role-scoped embedded config digests after the promotion and
  execution layers expose target materialization identities. Raw config
  SHA-256 remains diagnostic/local consistency evidence only.
- Persist or surface `DeploymentReceiptV1` records from remaining installer
  phases beyond the current artifact, root-canister, build, manifest, install,
  funding, staging, bootstrap resume, wait-ready, and install-state receipts.
- Extend role-scoped phase receipt outcomes beyond the build-artifacts phase
  once installer phases can mutate multiple roles or canisters.
- Persist or discover richer prior deployment receipts beyond latest local
  artifact-gate receipt lookup.

## Drift Log

- The installer refusal gate now blocks every hard failure from
  `SafetyReportV1`, but warnings remain report-only until live inventory is
  broad enough to make warning classes consistently actionable.
- Artifact evidence now distinguishes release-set payload hashes from observed
  local file hashes. The original design called for artifact truth, but this
  implementation makes source semantics explicit before using hashes as safety
  authority.

## Release Bar

0.41 should not close until the current install path has at least one more
operator-facing validation pass confirming the new report and artifact gate work
on realistic local install inputs.
