# Release Validation Matrix

This is the active inventory of Canic slice, CI, RC, and final-release gates.
General command, git, versioning, network, and release authority remains in
`docs/governance/ci-deployment.md`. Current line state remains in
`docs/status/current.md`; a dated release-line closeout owns the final verdict.

There is no standing RC-readiness audit or evergreen no-blocker conclusion.

## Scope

| Checkpoint | Purpose | Required outcome |
| --- | --- | --- |
| Slice closeout | Prove one bounded change and its invariant. | Targeted checks pass; diff and skipped wider gates are recorded. |
| Implementation closeout | Decide whether current accepted findings/slices are complete. | Required focused gates pass; remaining work is explicitly RC/final validation. |
| RC promotion | Account for the full maintained local/CI/package/environment matrix. | Every required gate passes or is assigned with a concrete limitation. |
| Final release/tag | Validate the exact release commit, packages, artifacts, and tag path. | Human-owned release flow and final gates pass. |

Each slice declares impact on runtime behavior, CLI/text, Candid, JSON/config,
stable state, package features, dependencies/lockfile, fixtures/generated
output, and release artifacts. Unstated impact is not permission to change it.

## Required Slice Gates

Use the narrowest checks that exercise changed files and behavior.

Documentation/audit-governance slices use applicable guards plus:

```text
bash scripts/ci/check-release-validation-matrix.sh
bash scripts/ci/check-release-integrity-contract.sh
bash scripts/ci/check-audit-method-catalog.sh
bash scripts/ci/check-recovery-runbooks.sh
bash scripts/ci/check-release-package-install-validation.sh
cargo test --locked -p canic --test changelog_governance -- --nocapture
make dependency-risk-gate
make gitleaks-scan
git diff --check
```

The changelog test applies when changelog/status/governance surfaces change.
Do not run workspace-wide tests, Clippy, broad PocketIC, package, or deployment
gates for an ordinary focused slice unless the maintainer explicitly requests
them or the changed invariant requires them.

Rust code slices add targeted formatting, check/Clippy, and tests for the
changed package and behavior. Direct Cargo commands use `--locked` when
supported. Unexplained lockfile churn is a blocker.

Full release validation, not ordinary slice validation, includes:

```text
cargo fmt --all -- --check
make fmt-check
make clippy
make test
```

## Required Local RC Gates

Before RC promotion, the maintainer runs or explicitly assigns:

```text
make fmt-check
bash scripts/ci/check-control-plane-feature-matrix.sh
make dependency-risk-gate
make gitleaks-scan
make clippy
make test
```

The maintainer records environment-specific gaps rather than treating an
unexecuted gate as a pass.

## Required CI Gates

The active workflow is the source of truth. At the current matrix it includes:

```text
cargo check --workspace --locked
bash scripts/ci/run-layering-guards.sh
bash scripts/ci/check-control-plane-feature-matrix.sh
bash scripts/ci/check-blob-storage-inventory-gate.sh
bash scripts/ci/check-blob-storage-cashier-inventory-gate.sh
bash scripts/ci/check-release-validation-matrix.sh
bash scripts/ci/check-release-integrity-contract.sh
bash scripts/ci/check-audit-method-catalog.sh
bash scripts/ci/check-recovery-runbooks.sh
bash scripts/ci/check-release-package-install-validation.sh
bash scripts/ci/check-dependency-risk-inventory.sh
bash scripts/ci/run-secret-scan.sh
make fmt-check
make clippy
make test-unit
cargo build -p canic --examples --locked
cargo build --release --workspace --locked
```

CI also validates workflow syntax, installs declared ICP/Wasm helpers, and
runs the pinned full-history secret scanner with fully redacted findings.
Audit definitions must not claim a guard runs in CI unless the current
workflow contains it.

The sole support declaration is the
[supported host and target matrix](../governance/supported-platforms.md). A
successful helper install on another platform is not release-support evidence.

## Focused Replay, Auth, And Cost Gates

When a slice touches or relies on replay/auth/cost behavior, select the exact
relevant commands, for example:

```text
cargo test --locked -p canic-core replay_policy --lib -- --nocapture
cargo test --locked -p canic-core --test cost_guard_boundary_guard -- --nocapture
cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture
make test-auth
make test-auth-chain-key
```

When stable memory or upgrade behavior changes, add focused ABI/storage and
PocketIC upgrade tests for that state owner. The current audit method and dated
report establish compatibility; a literal documentation guard does not.

When diagnostics change, assert typed causes internally and exact text, JSON,
or exit behavior only where it is a documented operator contract.

## Governance Gates

```text
cargo test --locked -p canic --test changelog_governance -- --nocapture
cargo test --locked -p canic --test workspace_manifest -- --nocapture
cargo test --locked -p canic --test release_index_guard -- --nocapture
cargo test --locked -p canic --test install_script_surface -- --nocapture
```

Use only the applicable focused governance test during development. Run the
full relevant set during release preparation or after changing those surfaces.

## Package And Install Gates

Package/install validation is RC/final-release work unless the current slice
changes packaging:

```text
make package
make test-installed-canic-cli
make test-packaged-downstream-cli
make test-packaged-downstream-wasm-store
```

These checks may require a clean worktree, isolated temporary package roots,
local caches, or authorized network access. A skipped gate records its owner,
reason, and target environment.

## Local ICP And Canister Gates

```text
make test-fleet-install
make test-canisters
```

These are maintainer-owned, environment-specific RC gates. They require the
named local ICP environment and must never target mainnet as an incidental
test default.

## Final Release And Artifact Gates

Final release accounting includes:

```text
cargo build --release --workspace --locked
make dependency-risk-gate
make gitleaks-scan
make package
```

It also records the exact source commit/tree, lockfile/toolchain/features,
artifact checksums and provenance, package/install probes, supported
host/target matrix, and any authorized environment-specific validation.
Versioning, staging, commits, tags, pushes, publish, and deployment remain
human-owned.

## Reporting Format

| Result | Meaning |
| --- | --- |
| `PASS` | Command ran and passed. |
| `FAIL` | Command ran and failed; retain the typed or exact command cause. |
| `BLOCKED` | Required authoritative evidence could not be produced. |
| `SKIPPED` | A non-required gate was intentionally not run, with reason/owner. |
| `NOT_APPLICABLE` | A conditional trigger is absent, with evidence. |

Do not translate `BLOCKED`, `SKIPPED`, or unavailable into `PASS`.

## Related Operation Docs

- [Release package and install validation](release-package-install-validation.md)
- [Recovery and retry runbooks](recovery-retry-runbooks.md)
- [0.56 v1 release probe inventory](0.56-v1-release-probes.md)
- [Installed CLI smoke](0.56-installed-cli-smoke.md)
- [Packaged downstream CLI](0.56-packaged-downstream-cli.md)
- [Packaged Wasm store](0.56-packaged-wasm-store.md)
