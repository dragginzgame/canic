# Release Package and Install Validation

This checklist is the durable package, install, artifact, and smoke-test
validation reference for Canic release work.

It documents existing repo targets and when they should run. It is
intentionally not named after a release line; release numbers belong in
changelogs and status docs, not in the operational validation entry point.

Current release-line context comes from `docs/status/current.md`. This file is
the canonical package/install validation reference for current release work.

<img src="../../assets/256x256/mechanic-notes.png" align="left" width="96" alt="The Canic mechanic holding a package-validation checklist" />

**Maintainer outcome:** account for package construction, installed and
downstream smoke tests, artifact verification, and environment-owned gates
without turning this checklist into release authority.

<br clear="left" />

## At A Glance

| Need | Section |
| --- | --- |
| Select the package/install gates | [Existing Package And Install Gates](#existing-package-and-install-gates) |
| Verify promoted artifacts | [Artifact Verification Expectations](#artifact-verification-expectations) |
| Assign environment-owned work | [Environment And Ownership](#environment-and-ownership) |
| Distinguish slice, RC, and release work | [Release Flow Boundary](#release-flow-boundary) |
| Record candidate results | [Required RC Gates](#required-rc-gates) |

## Scope

This checklist covers:

- publishable crate package creation;
- installed CLI smoke validation, including current shipped operator command
  surfaces;
- packaged downstream CLI validation, including current shipped operator
  command surfaces;
- packaged downstream Canister macro/Candid and `wasm_store` bootstrap validation;
- current desired-state Fleet convergence qualification;
- release artifact verification expectations;
- environment-specific gate ownership;
- the boundary between implementation slices and maintainer-directed release flow.

This checklist does not change runtime behavior, Candid, CLI output,
JSON/output formats, package manifests, dependencies, lockfiles, fixtures,
snapshots, generated artifacts, package artifacts, or release package contents.

## Related Evidence

- [Release validation matrix](release-validation-matrix.md) assigns package
  and install gates to slice, RC, or final-release validation.
- Current implementation-closeout state belongs to `docs/status/current.md`;
  the final verdict belongs to a dated release-line closeout.

## Existing Package and Install Gates

These gates already exist in the repo. This checklist classifies them; it does
not add new release behavior.

| Gate | Command | Release question | When to run |
| --- | --- | --- | --- |
| Publishable crate package | `make package` | Can the workspace produce publishable package archives through `cargo package` from a clean worktree? | RC/final release. |
| Installed CLI smoke | `make test-installed-canic-cli` | Does an installed `canic` binary run the maintained v1 readiness smoke and current retained operator CLI checks without using `target/debug/canic` or repository state? | RC/final release when local Cargo install is available. |
| Packaged downstream CLI | `make test-packaged-downstream-cli` | Can an installed CLI from extracted packages build a consumer, deploy a small Fleet on PocketIC, recover interrupted calls and replay with zero effects using public JSON? | Explicit package qualification and RC/final release with the pinned PocketIC/toolchain and local Cargo cache. |
| Packaged downstream Canister, managed-App testing and wasm store | `make test-packaged-downstream-wasm-store` | Can an ordinary typed Canister use packaged `build!`, `start!` and `finish!`, can an isolated host consumer compile the published `canic::testing` managed/standalone facade, and can both `wasm_store` bootstrap paths build outside the repository package graph? | RC/final release when Wasm/Cargo package support is available. |
| Release workspace build | `cargo build --release --workspace --locked` | Does the release build shape compile with the locked resolver? | Release-commit `main` CI and RC validation. |
| Fleet ensure qualification | governed `canic-host` PocketIC journey in `make validate` | Can `canic fleet ensure` conserve cycles while converging an inconsistent estate, then repeat with zero mutation effects? | RC/final release. |

The retained probe details remain documented in:

- [0.56 v1 release probe inventory](0.56-v1-release-probes.md)
- [Installed CLI smoke](0.56-installed-cli-smoke.md)
- [Packaged downstream CLI](0.56-packaged-downstream-cli.md)
- [Packaged wasm store](0.56-packaged-wasm-store.md)

These documents retain their original filenames; their maintained procedures
describe the current probes. This checklist owns package/install gate accounting.

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
- Installed CLI smoke covers command availability and input validation. The
  packaged CLI journey also builds real Wasm, exercises Fleet recovery through
  public JSON and compares deployed artifact hashes on disposable PocketIC.
  Neither proof mutates a live IC environment.
- Packaged Canister proof must compile one typed endpoint through packaged
  `build!`, `start!` and `finish!` at the MSRV with warnings denied, extract
  that endpoint from local Wasm and prove IC Wasm omits the local export.
- Packaged `wasm_store` proof must build the generated infrastructure entrypoint
  from extracted current packages, verifying the selected facade and dependencies
  resolve from package contents rather than repository crate paths.
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
| Fleet ensure PocketIC qualification | Runs inside the maintained workspace test graph and requires the pinned PocketIC server. | RC validation owner or CI. |
| Release versioning targets | Complete releases use `make patch`, `make minor`, or `make major`; eligible non-runtime patches may use `make patch-fast`. Staging, candidate verification, commit, immutable tag and atomic push remain the same. | Maintainer or explicitly authorized agent. |

If a package/install gate is not run locally, the RC audit must record:

- the command;
- the reason it was skipped;
- where it will run;
- who owns the result;
- whether the gap blocks RC promotion or final release only.

## Release Flow Boundary

Release mutation and publication require an explicit maintainer instruction.
An automated agent may execute that instruction without a second confirmation.
Absent that direct authority, it must not change release versions, install
URLs, package versions, workspace dependency versions or release-script
defaults.

Complete release flow remains:

```text
make patch
make minor
make major
make release-stage
make release-commit
make release-push
```

An eligible non-runtime patch may substitute `make patch-fast` for `make
patch`, or use the one-shot `make release-patch-fast`. That lane skips broad
tests and PocketIC but records an explicit fast validation receipt.

The package/install validation gates are release-readiness evidence. They do
not authorize an automated version bump, tag, publish, or package-artifact
commit.

## Required RC Gates

Use these gates when validating package/install readiness before RC promotion
or final release, assigning environment-specific gates when needed:

```text
bash scripts/ci/check-release-validation-matrix.sh
make package
make test-installed-canic-cli
make test-packaged-downstream-cli
make test-packaged-downstream-wasm-store
cargo build --release --workspace --locked
```

The full validation graph owns the governed Fleet ensure PocketIC journey. It
must cover current desired-state convergence and an immediate zero-mutation
second run; removed local install commands are not release evidence.

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

This checklist does not issue a release verdict. Record each required gate as
`PASS`, `FAIL`, `BLOCKED`, `SKIPPED`, or `NOT_APPLICABLE` in the dated
release-line closeout, including the owner and target environment for every
unexecuted environment-specific gate.

## Continue From Here

- [Review the release validation matrix](release-validation-matrix.md)
- [Review CI and deployment governance](../governance/ci-deployment.md)
- [Follow recovery and retry procedures](recovery-retry-runbooks.md)
- [Browse Operations](README.md)
- [Browse all documentation](../README.md)
- [Back to the main README](../../README.md)
