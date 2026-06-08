# Release Package and Install Validation

This checklist is the durable package, install, artifact, and smoke-test
validation reference for Canic release work.

It documents existing repo targets and when they should run. It is
intentionally not named after a release line; release numbers belong in
changelogs and status docs, not in the operational validation entry point.

Current release-line context: 0.62 is using this checklist for release
durability and RC/final-release accounting.

## Scope

This checklist covers:

- publishable crate package creation;
- installed CLI smoke validation;
- packaged downstream CLI validation;
- packaged downstream `wasm_store` wrapper validation;
- local ICP install/canister validation;
- release artifact verification expectations;
- environment-specific gate ownership;
- the boundary between implementation slices and human-owned release flow.

This checklist does not change runtime behavior, Candid, CLI output,
JSON/output formats, package manifests, dependencies, lockfiles, fixtures,
snapshots, generated artifacts, package artifacts, or release package contents.

## Related Evidence

- [RC readiness audit](rc-readiness-audit.md) records whether implementation
  slicing is closed and which package/install gates remain RC or final-release
  validation work.

## Existing Package and Install Gates

These gates already exist in the repo. This checklist classifies them; it does
not add new release behavior.

| Gate | Command | Release question | When to run |
| --- | --- | --- | --- |
| Publishable crate package | `make package` | Can the workspace produce publishable package archives through `cargo package` from a clean worktree? | RC/final release. |
| Installed CLI smoke | `make test-installed-canic-cli` | Does an installed `canic` binary run the maintained v1 readiness smoke without using `target/debug/canic` or repository state? | RC/final release when local Cargo install is available. |
| Packaged downstream CLI | `make test-packaged-downstream-cli` | Can packaged Canic crates resolve and run current downstream CLI/read-only commands without repository crate paths? | RC/final release when local Cargo cache/toolchain support is available. |
| Packaged downstream wasm store | `make test-packaged-downstream-wasm-store` | Can the special packaged downstream `wasm_store` bootstrap wrapper build from packaged Canic crates outside the repository package graph? | RC/final release when Wasm/Cargo package support is available. |
| Release workspace build | `cargo build --release --workspace --locked` | Does the release build shape compile with the locked resolver? | Tag CI and RC validation. |
| Local fleet install | `make test-fleet-install` | Can the full local test/reference topology install with the configured local ICP environment? | RC validation when local ICP/PocketIC/canister build prerequisites are available. |
| Local canister tests | `make test-canisters` | Can the local canister install/test flow run end to end? | RC validation when local ICP/PocketIC/canister build prerequisites are available. |

The retained probe details remain documented in:

- [0.56 v1 release probe inventory](0.56-v1-release-probes.md)
- [Installed CLI smoke](0.56-installed-cli-smoke.md)
- [Packaged downstream CLI](0.56-packaged-downstream-cli.md)
- [Packaged wasm store](0.56-packaged-wasm-store.md)

Those older docs are retained historical probe inventories. This checklist is
the current non-versioned package/install validation entry point.

## Artifact Verification Expectations

RC and final release reports should account for these artifact expectations:

- `make package` must run from a clean worktree because the target depends on
  `ensure-clean`.
- Package validation must not leave committed package artifacts, generated
  files, fixtures, snapshots, or lockfile churn.
- Packaged downstream proofs must resolve through temporary package roots, not
  repository crate paths.
- Installed CLI proof must execute the temporary installed binary, not
  `target/debug/canic`.
- Packaged `wasm_store` proof must exercise the generated wrapper path and
  verify the generated wrapper uses packaged Canic sibling crates instead of
  repository crate paths.
- Release build validation should use locked resolver commands where the
  command supports it.
- Any checksum, reproducibility, or artifact-signing requirement belongs to
  RC/final release accounting unless a maintainer explicitly promotes it into a
  release-blocking implementation task.

## Environment and Ownership

Package/install gates may be expensive or environment-specific.

| Gate family | Environment notes | Owner |
| --- | --- | --- |
| `make package` | Requires a clean worktree and may write under `target/package`. | RC/final release owner. |
| Installed CLI smoke | Installs into a temporary root and isolates `HOME`, `CARGO_HOME`, `CARGO_TARGET_DIR`, and `TMPDIR` under the proof root. | RC/final release owner or CI environment with local install support. |
| Packaged downstream probes | Use package archives and temporary downstream roots; they intentionally reuse caller Cargo/Rust caches for offline package execution. | RC/final release owner or CI environment with package cache support. |
| Local ICP/canister gates | Require local ICP CLI, local replica/canister build environment, and can take longer than ordinary docs-slice validation. | RC validation owner or dedicated local/CI environment. |
| Release versioning targets | `make patch`, `make minor`, `make major`, `make release-stage`, `make release-commit`, and `make release-push` are human-owned release flow. | Maintainer only. |

If a package/install gate is not run locally, the RC audit must record:

- the command;
- the reason it was skipped;
- where it will run;
- who owns the result;
- whether the gap blocks RC promotion or final release only.

## Release Flow Boundary

Automated agents must not change release versions, install URLs, package
versions, workspace dependency versions, or release-script default versions
during ordinary development slices.

Human-owned release flow remains:

```text
make patch
make minor
make major
make release-stage
make release-commit
make release-push
```

The package/install validation gates are release-readiness evidence. They do
not authorize an automated version bump, tag, publish, or package-artifact
commit.

## Required RC Gates

Use these gates when validating package/install readiness before RC promotion
or final release, assigning environment-specific gates when needed:

```text
bash scripts/ci/check-release-package-install-validation.sh
make package
make test-installed-canic-cli
make test-packaged-downstream-cli
make test-packaged-downstream-wasm-store
cargo build --release --workspace --locked
make test-fleet-install
make test-canisters
```

The local ICP/canister gates may be assigned to CI or a dedicated RC
environment when too expensive or environment-specific for an ordinary docs
slice.

## Non-Goals

- No runtime behavior change.
- No Candid change.
- No CLI output change.
- No JSON/output format change.
- No dependency or lockfile change.
- No package manifest change.
- No generated artifact change.
- No package artifact commit.
- No release version bump.
- No publish or tag operation.
- No new packaging system.

## Outcome Summary

Release blockers: none found in this checklist.

The current package/install validation inventory is sufficient to close 0.62
implementation work after the [RC readiness audit](rc-readiness-audit.md)
records its final verdict. Remaining work belongs to assigned package/install
gate execution, final release accounting, or focused defect handling if a
concrete release blocker is found.
