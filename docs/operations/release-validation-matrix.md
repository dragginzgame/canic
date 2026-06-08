# Release Validation Matrix

This matrix is the durable release-validation inventory for Canic release work.

It documents existing repo checks and when they should run. It is intentionally
not named after a release line; release numbers belong in changelogs and status
docs, not in the operational validation entry point.

This matrix is linked from `docs/governance/ci-deployment.md` and guarded by CI
so it remains the active release-validation reference. The governance document
remains the general policy for command tiers, git boundaries, versioning,
release flow, network selection, and automation language rules.

Current release-line context: 0.62 is using this matrix for release durability,
upgrade confidence, and operator recovery work.

## Scope

Release validation distinguishes four checkpoints:

| Checkpoint | Purpose | Required outcome |
| --- | --- | --- |
| Slice close-out | Prove the current slice is scoped and clean. | Focused checks pass, diff is understood, and any skipped broader check is recorded. |
| Implementation close-out | Decide whether no more implementation/docs slices are needed for the current release line. | Required local and focused gates pass; remaining work is RC/full release validation only. |
| RC promotion | Decide whether the branch is ready for release-candidate handling. | Full local/CI matrix passes or every environment-specific gap is explicitly assigned. |
| Final release/tag | Validate the published release path. | Tag CI, release package, install, and smoke checks pass where applicable. |

Implementation close-out is not the same as RC promotion. A slice can close
without running every package/install gate, but RC promotion must account for
the full matrix.

## Scope Declaration

Every release-line slice must state whether it changes:

| Surface | Expected default |
| --- | --- |
| Runtime behavior | No |
| CLI behavior or text output | No |
| Candid | No |
| JSON/output formats | No |
| Cargo.toml or workspace dependency versions | No |
| Cargo.lock | No |
| Fixtures, snapshots, generated files, or package artifacts | No |

Any exception must name the concrete RC/release blocker or approved charter item
that justifies it.

## Required Slice Gates

Docs-only release-line slices should use docs-appropriate validation:

```text
cargo fmt --all -- --check
cargo test --locked -p canic --test changelog_governance -- --nocapture
bash scripts/ci/check-release-validation-matrix.sh
bash scripts/ci/check-upgrade-state-audit.sh
bash scripts/ci/check-recovery-runbooks.sh
git diff --check
```

If a docs-only slice does not touch changelog/status files, the changelog
governance test may be recorded as not applicable.

Code, test, tooling, or CI slices must also run the narrowest command that
exercises the touched invariant. If a change is cross-cutting, add the relevant
package or workspace gate from this matrix.

Direct Cargo validation commands should use `--locked` when the command
supports it. Makefile targets keep the repo's existing wrapper behavior.
Unexplained `Cargo.lock` churn is a blocker.

## Required Local RC Gates

These are the local gates a maintainer should run before RC promotion unless a
documented environment limitation assigns them to CI or final release
validation:

```text
make fmt-check
make clippy
make test
```

`make test` runs `make clippy` and `make test-unit`. Running `make clippy`
separately before `make test` is useful when reporting the matrix because it
separates lint failures from test failures.

## Required CI Gates

The GitHub Actions PR/main matrix currently includes:

```text
cargo check --workspace --locked
bash scripts/ci/run-layering-guards.sh
bash scripts/ci/run-forbidden-crypto-guards.sh
bash scripts/ci/check-release-validation-matrix.sh
bash scripts/ci/check-upgrade-state-audit.sh
bash scripts/ci/check-recovery-runbooks.sh
make fmt-check
make clippy
make test-unit
cargo build -p canic --examples --locked
```

The same CI job also installs and checks required helper tools including
`actionlint`, the ICP CLI, `ic-wasm`, and PocketIC.

The tag workflow currently includes:

```text
bash scripts/ci/run-forbidden-crypto-guards.sh
bash scripts/ci/check-release-validation-matrix.sh
bash scripts/ci/check-upgrade-state-audit.sh
bash scripts/ci/check-recovery-runbooks.sh
make fmt-check
make clippy
make test-unit
cargo build --release --workspace --locked
```

Tag CI also runs workflow linting and helper-tool setup.

## Focused Replay, Auth, And Cost Gates

These gates should be run during 0.62 close-out and before RC promotion because
they directly protect the 0.61 replay/auth/cost boundary:

```text
bash scripts/ci/run-auth-trust-chain-guards.sh
cargo test --locked -p canic-core replay_policy --lib -- --nocapture
cargo test --locked -p canic-core --test cost_guard_boundary_guard -- --nocapture
cargo test --locked -p canic-core --test delegated_auth_hard_cut_guard -- --nocapture
```

If the slice touches stable memory, replay receipt state, or upgrade/state
compatibility, also run the relevant focused stable-state gates:

```text
bash scripts/ci/check-upgrade-state-audit.sh
cargo test --locked -p canic-core --test stable_memory_abi_guard -- --nocapture
cargo test --locked -p canic-core storage::stable::replay --lib -- --nocapture
```

If the slice touches internal-call authorization or protected call boundaries,
also run:

```text
cargo test --locked -p canic-core --test protected_internal_call_guard -- --nocapture
```

If the slice touches operator recovery wording or retry/runbook expectations,
also run:

```text
bash scripts/ci/check-recovery-runbooks.sh
```

## Governance Gates

Changelog and workspace governance checks are:

```text
cargo test --locked -p canic --test changelog_governance -- --nocapture
cargo test --locked -p canic --test workspace_manifest -- --nocapture
cargo test --locked -p canic --test release_index_guard -- --nocapture
cargo test --locked -p canic --test install_script_surface -- --nocapture
```

Use `changelog_governance` for any changelog/status/governance slice. Use
`workspace_manifest`, `release_index_guard`, and `install_script_surface` during
RC or release-preparation validation, and for focused slices that touch package
metadata, release index behavior, install URLs, or script/version surfaces.

## Package And Install Gates

Package and install checks are RC/final-release gates rather than ordinary
docs-slice gates:

```text
make test-installed-canic-cli
make test-packaged-downstream-cli
make test-packaged-downstream-wasm-store
make package
```

These checks exercise installed binaries, packaged downstream crate resolution,
the special packaged `wasm_store` wrapper path, and publishable package
creation. They may require a clean worktree, local Cargo cache state, or network
access depending on the environment. If they are not run locally, the RC audit
must record where they will be run.

## Local ICP And Canister Gates

The local ICP/canister gates are environment-specific:

```text
make test-fleet-install
make test-canisters
```

Run these before RC promotion when the local ICP CLI, local replica, and
canister build environment are available. If they are skipped, the RC audit must
record the reason and identify the CI/manual environment that covers them.

## Broad Workspace Gates

The broad Cargo equivalents are:

```text
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
cargo build --release --workspace --locked
```

Prefer the Makefile gates for normal local validation because they include the
repo's wrapper behavior and deterministic workspace test sequencing. Use direct
Cargo gates when debugging or when CI reports them directly.

## Reporting Format

Release-readiness reports should classify each command as:

| Result | Meaning |
| --- | --- |
| PASS | Command ran and passed. |
| FAIL | Command ran and failed; include the failing package/test/target. |
| SKIPPED | Command was intentionally not run; include the reason and owner. |
| NOT APPLICABLE | Gate does not apply to the slice surface. |

Do not treat an unrun broad gate as an implementation blocker for a docs-only
slice. Do treat it as required accounting before RC promotion.

## Related Operation Docs

The package/install gates above are documented in:

- [Recovery and retry runbooks](recovery-retry-runbooks.md)
- [Upgrade and state compatibility audit](upgrade-state-compatibility-audit.md)
- [0.56 v1 release probe inventory](0.56-v1-release-probes.md)
- [Installed CLI smoke](0.56-installed-cli-smoke.md)
- [Packaged downstream CLI](0.56-packaged-downstream-cli.md)
- [Packaged wasm store](0.56-packaged-wasm-store.md)
