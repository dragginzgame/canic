# Current Status

Last updated: 2026-07-01

## Purpose

This is the compact handoff for new agent sessions. Read this first, then
inspect only the files needed for the current task. Detailed historical status
before this compaction is archived at
`docs/status/archive/2026-06-30-precompact.md`.

## Current Line

- The active line is `0.78.0` top-level medic preflight. Source of truth:
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

- Continue 0.78 by tightening remaining preflight slices around broader medic
  readiness checks selected from the 0.78 design.

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
