# Current Status

Last updated: 2026-07-03

## Purpose

This is the compact handoff for new agent sessions. Read this first, then
inspect only the files needed for the current task. Detailed historical status
before this compaction is archived at
`docs/status/archive/2026-06-30-precompact.md`.

## Current Line

- The active line is `0.80.0` stable state migrations. Source of truth:
  `docs/design/0.80-stable-state-migrations/0.80-design.md`.

- The first post-0.80.0 working slice adds diagnostic state metadata surfaces:
  `canic state audit` and `canic state manifest`. Static Rust declarations
  cover the root canister family for canic-core stable-memory domains plus the
  retired root replay memory ID. The reports are diagnostic-only, render
  text/JSON with `schema_version = 1`, check duplicate memory IDs within a role,
  schema/storage declarations, record/snapshot naming, migration declarations,
  removed-state disposition, restore order, and post-upgrade invariant metadata.
  They do not read stable memory, run migrations, write generated manifests,
  write deployment truth, or mutate canisters. The 0.80.1 changelog entries are
  staged in the root ledger and detailed 0.80 notes.

- The `0.80.2` working slice expands the root-family manifest coverage to
  additional non-gated canic-core domains: subnet topology/state, cycle
  top-up/funding/refill records, intent store slots, canister pool, scaling
  registry, and directory registry. Ambiguous multi-memory log/cycle-tracker
  declarations and feature-gated sharding/blob-storage declarations remain
  deferred. The same slice now fails `canic state audit` if active state
  reclaims a memory ID declared by removed state, keeping retired IDs reserved
  unless a future explicit migration design handles them. The 0.80.2 changelog
  entries are staged in the root ledger and detailed 0.80 notes.

- The `0.80.3` and `0.80.4` slices add config-driven medic/build checks for
  runtime Canic feature gates implied by fleet auth settings, plus concise CI
  medic output and developer-owned Cargo.toml guidance. These are pushed.

- The `0.80.5` working slice returns to the stable-state design by summarizing
  `canic state audit` inside project-level medic as a diagnostic-only runtime
  readiness check. Medic maps the aggregate state-audit status into a single
  `state_audit_*` row and points operators to `canic state audit` for details;
  it does not inspect stable memory, run migrations, or take ownership of
  state-audit logic. The 0.80.5 changelog entries are staged in the root ledger
  and detailed 0.80 notes.

- The previous line was `0.79.12` declarative deployment plan. Source of truth:
  `docs/design/0.79-declarative-deployment-plan/0.79-design.md`.

- The first 0.79 slice is implemented: `canic deploy plan <deployment>` builds
  a deterministic, no-mutation `DeploymentPlanReport` from local project config
  by embedding the existing `DeploymentPlanV1`. It supports text output,
  `--json`, safe JSON `--out` writes, and hard-cut rejection of aliases,
  shorthand forms, `--apply`, `--write-truth`, `--evidence`, and `--force`.
  Missing installed deployment state is a warning/comparison gap, not a
  blocker; verified installed root state is surfaced as a report fact;
  unverified installed root state blocks the plan; malformed desired config
  blocks the plan. Already-available installed-state evidence now drives
  `comparison_status` to `compared`, `compared_with_warnings`, or
  `compared_with_drift`; missing installed state remains `not_available`.
  Invalid deployment target names are explicit blockers. Future-apply preview
  labels distinguish first-install `install_wasm` from known-canister
  `upgrade_wasm`. Medic next actions may point to
  `canic deploy plan`, but medic does not execute the planner.

- The 0.79.1 working slice tightens deploy-plan report facts: reports now
  surface deterministic config, topology, authority, artifact-set, and observed
  role-artifact facts that are already present in the embedded
  `DeploymentPlanV1`, without adding live observation, apply semantics, or
  mutation.

- The 0.79.2 working slice extends deploy-plan future-apply preview labels for
  configured pool expectations. Expected pool identities with no known
  canister id now emit `create_canister` preview labels such as
  `user_shards:user_shard`; these remain non-executed labels, not apply
  operation objects. Desired authority profiles with configured deployment
  controllers now also emit one deployment-scoped `set_controllers` preview
  label, with the same non-executed planning semantics.

- The 0.79.3 working slice extends deploy-plan future-apply preview labels for
  root and child registration. Expected canisters and configured pool
  identities without known ids now emit `register_root` or `register_child`
  labels alongside create/install labels; these remain report-only planning
  labels, not apply instructions. The same slice reserves the
  `unsupported.*` assumption namespace for desired shapes outside the 0.79
  planner contract so those become explicit `unsupported` diagnostics instead
  of generic blockers or warnings. The 0.79.3 changelog entries are staged in
  the root ledger and detailed 0.79 notes.

- The 0.79.4 working slice extends deploy-plan future-apply preview labels to
  include `verify_readiness` when the embedded `DeploymentPlanV1` already
  carries verifier-readiness requirements or expected role epochs, and surfaces
  the same expectation as a `verifier_readiness_expectation_resolved` report
  fact. Reports also name resolved expected canister inventory when role config
  is available. This remains non-executed and does not add live observation or
  mutation. The 0.79.4 changelog entries are staged in the root ledger and
  detailed 0.79 notes.

- The 0.79.5 working slice continues deploy-plan report visibility by
  surfacing fleet-template, expected controller-set, role-artifact inventory,
  expected pool-inventory, and root trust-anchor facts already present in
  `DeploymentPlanV1`. These are passive `verified_facts` only and do not add
  live observation, deployment truth writes, or apply semantics. The 0.79.5
  changelog entries are staged in the root ledger and detailed 0.79 notes.

- The 0.79.6 working slice aligns deploy-plan text output with the stable
  report model by rendering schema version, command identity, and each
  diagnostic source. Future-apply preview lines now also render explicit
  label, subject, and status fields. This is output-only provenance; it does
  not alter JSON shape, plan construction, comparison, observation, deployment
  truth, or mutation behavior.

- The 0.79.7 working slice continues deploy-plan report-contract hardening by
  keeping the text-output parity changes from the post-0.79.6 branch and
  adding focused tests for the documented exit-code contract: planned and
  warning reports exit successfully, while blocked and unsupported reports
  return `PlanBlocked` with exit code 1. The same slice now smoke-tests
  `canic deploy plan help` and `canic deploy plan --help` so the planning
  command's safety-contract help remains reachable through the shared CLI
  help path. Deploy-plan coverage also pins the command help's no-mutation /
  JSON `--out` wording and the deterministic diagnostic sort order used by
  report arrays. Stable report command, preview phase, and preview status
  strings are centralized to reduce contract drift. This guards the report
  contract without changing plan construction, output schema, observation,
  deployment truth, apply behavior, or mutation semantics. The 0.79.6 and
  0.79.7 changelog entries are staged in the root ledger and detailed 0.79
  notes.

- The 0.79.8 working slice has started with a report-layer cleanup:
  `proposed_operations()` now returns sorted and deduplicated operation labels
  so repeated desired-plan inputs cannot duplicate future-apply preview lines.
  Stable severity, category, source, operation-label, and known assumption-key
  strings are also centralized so report construction, status derivation,
  assumption classification, and tests share the same serialized values.
  Diagnostic sorting now uses an explicit severity rank instead of relying on
  lexical string order, and the public JSON report test pins the complete
  sorted future-apply preview array. This does not change the embedded
  `DeploymentPlanV1`, plan construction, observation, deployment truth, apply
  behavior, or mutation semantics. The 0.79.8 changelog entries are staged in
  the root ledger and detailed 0.79 notes.

- The 0.79.9 working slice has started by adding report-only
  `upload_artifact` future-apply preview labels for each resolved
  `DeploymentPlanV1.role_artifacts` entry. The labels remain non-executed
  planning output and do not add apply operation objects, artifact registration,
  deployment truth writes, live observation, or mutation semantics. Public JSON
  and text-renderer coverage now pins the label while continuing to reject
  apply-safety wording such as `will upload`. Plans with artifact diagnostics
  now also include the top-level next action
  `run canic build or provide a build profile with resolved artifacts`. The
  same slice surfaces passive `build_profile_resolved`, `plan_id_resolved`,
  `runtime_variant_resolved`, and `planner_version_resolved` verified facts
  already present in the command options or embedded `DeploymentPlanV1`.
  The 0.79.9 changelog entries are staged in the root ledger and detailed
  0.79 notes.

- The 0.79.10 working slice has started by surfacing passive
  `config_path_resolved` and `network_resolved` verified facts from the
  deploy-plan invocation and embedded plan identity. These mirror existing
  top-level report fields and do not change plan construction, comparison,
  observation, deployment truth, apply behavior, or mutation semantics. The
  0.79.10 changelog entries are staged in the root ledger and detailed 0.79
  notes.

- The 0.79.11 working slice has started by adding a report-only
  `apply_policy` future-apply preview label when the desired authority profile
  already includes controller policy expectations. The label remains
  non-executed planning output and does not add apply operation objects,
  controller mutation, deployment truth writes, live observation, or mutation
  semantics. Text output also now prints each preview label's `phase` field so
  the human renderer mirrors the JSON `ProposedOperationLabel` shape more
  closely, and the future-apply section header names rows as non-executed
  proposed-operation labels. Command help now documents the same
  preview-label boundary. The 0.79.11 changelog entries are staged in the root
  ledger and detailed 0.79 notes.

- The 0.79.12 working slice has started by tightening the deploy-plan
  evidence/truth boundary in command help: JSON output is explicitly described
  as `DeploymentPlanReport`, not an evidence envelope, deployment truth, or
  authorization to mutate. Report-renderer coverage also pins that actual
  text/JSON reports do not include those truth/evidence/authorization claims
  or apply-safety wording. The 0.79.12 changelog entries are staged in the
  root ledger and detailed 0.79 notes.

- The previous line was `0.78.0` top-level medic preflight. Source of truth:
  `docs/design/0.78-top-level-medic-preflight/0.78-design.md`.

- The first 0.78 slice is implemented: `canic medic` is the top-level
  diagnostic surface with project and explicit deployment scopes, a
  `schema_version = 1` report model, text/JSON renderers, deterministic
  status/category ordering, and hard-cut rejection of old/shorthand forms.
  The old `canic info medic` route is removed from active CLI dispatch.

- Existing deployment-scoped diagnostics are being preserved under
  `canic medic deployment <deployment>`, including targeted
  `--blob-storage <canister-or-role>` and
  `--auth-renewal <issuer-principal>` checks.

- The post-0.78.2 working tree adds passive project-config quality checks to
  `canic medic project`: discovered roles now report
  `role_package_metadata_present` / `role_package_metadata_missing`, and
  declared-only roles report `declared_role_not_deployable` without running
  Cargo or mutating project state.

- The same working tree adds deployment-truth receipt completeness checks to
  `canic medic deployment <deployment>`: complete succeeded receipts report
  `deployment_truth_complete`, missing/unfinished receipts warn as
  `deployment_truth_incomplete`, and partial post-mutation receipts fail.

- Missing deployment-target medic runs now emit exact-match project-config hints
  when the requested deployment name matches a known fleet template
  (`fleet_name_deployment_name_conflated`) or role
  (`role_name_deployment_name_conflated`).

- Deployment-scoped medic also smoke-checks installed deployment registry
  observation through the existing resolver, emitting
  `deployment_registry_observed`, `deployment_registry_empty`,
  `deployment_registry_unavailable`, or `deployment_registry_not_evaluated`
  before targeted blob-storage/auth diagnostics.

- Targeted blob-storage medic failures now keep the stable target-resolution
  codes promised by the 0.78 design: `blob_storage_target_missing`,
  `blob_storage_target_ambiguous`, and `blob_storage_target_not_blob_storage`.

- The 0.78.4 medic readiness slice classifies invalid targeted
  auth-renewal issuers as
  `auth_renewal_issuer_invalid` before treating other auth-renewal failures as
  `auth_renewal_drift_fail`, and by distinguishing missing ICP CLI binaries as
  `icp_cli_missing` instead of the generic `icp_cli_incompatible`. It also
  keeps `local_network_implicit` / `local_network_explicit` project-only so
  deployment medic relies on its deployment-scoped network check instead of
  emitting duplicate network diagnostics. Blob-storage target resolution now
  follows the 0.78 design order by treating principal text as a canister ID
  before falling back to role names. The same released slice updates active
  `canic install` collision guidance to point at
  `canic medic deployment <deployment>` instead of the removed
  `canic info medic <deployment>` route, removes the same retired `info medic`
  leaf from top-level global ICP/network option forwarding, and keeps medic
  subcommand help usage-only:
  `canic medic project help` and `canic medic deployment help` render medic
  usage instead of entering project/deployment report construction, including
  when medic-local flags such as `--json` appear around the subcommand. The
  same slice wraps unbroken long diagnostic values within `MEDIC_REPORT_WIDTH`.

- The 0.78.5 slice retargets the auth-renewal installed/packaged CLI proof
  helper from the removed `canic info medic` route to
  `canic medic deployment <deployment> --auth-renewal <issuer>`, makes the
  fixture satisfy deployment medic's project-level preconditions, and asserts
  the current medic `auth_renewal_drift_warn` output shape.

- 0.77 completed the wasm-footprint feature-boundary line, including
  chain-key/root-publication feature splitting and local DTO replacements for
  helper crate fan-in. Current dependency work may include local
  `ic-memory` surface adjustments; preserve those edits if present.

- 0.76 bridge-free delegated auth is closed. Delegated-token `RootProof` is
  chain-key-only: `RootProof::IcChainKeyBatchSignatureV1`. The old
  bridge-backed canister-signature delegated root-proof renewal path is
  historical documentation only, not public runtime/API/CLI code or active auth
  stable state.

## Open Work

- Continue 0.80 by expanding Rust-authored state declarations beyond the first
  root-family slice, then add more precise `*Data` snapshot declarations and
  migration coverage metadata. Do not add migration execution, stable-memory
  inspection, state dump/explore commands, generated manifest writes, runtime
  introspection endpoints, or mutation semantics.

- Before release preparation, run the focused gates for touched surfaces and
  broaden to the release matrix as needed. Do not assign a new patch version or
  change Cargo package versions unless the maintainer explicitly asks for
  release preparation.

## Useful Validation

Focused 0.78 medic validation:

```text
cargo test --locked -p canic-cli medic
cargo test --locked -p canic-cli status
cargo test --locked -p canic-host deployment_truth --lib
```

Broader CLI validation after command-surface edits:

```text
cargo test --locked -p canic-cli
```

Retained auth validation when a change touches live delegated-auth behavior:

```text
cargo check --locked -p canic-core -p canic
cargo test --locked -p canic-core chain_key --lib
cargo test --locked -p canic-core chain_key_batch --lib
cargo test --locked -p canic-core workflow::runtime::auth --lib
cargo test --locked -p canic --test protocol_surface
cargo check --locked -p canic-tests --test root_suite
```

Focused aliases added for ordinary local iteration:

```text
make test-auth
make test-auth-chain-key
make test-cli
make test-runtime-fast
```

When PocketIC is available and the change touches live 0.76 auth behavior,
run:

```text
POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic \
  cargo test --locked -p canic-tests --test root_suite auth_076 -- --nocapture --test-threads=1
```

## Standing Constraints

- Preserve dirty worktree state and keep edits scoped.
- Do not change Cargo versions, workspace dependency versions, release script
  defaults, install URLs, or matching lockfile package versions unless the
  maintainer explicitly requests a version bump or release-preparation change.
- Follow `docs/governance/ci-deployment.md` for command, git, versioning, and
  release boundaries.
- Follow `docs/governance/changelog.md`; ordinary development goes under root
  `CHANGELOG.md` `Unreleased` when release notes are warranted.
- Follow `docs/governance/code-hygiene/README.md`; use directory modules with
  `mod.rs`, keep DTOs passive, and keep dependency direction
  `endpoints -> workflow -> policy -> ops -> model/storage`.
