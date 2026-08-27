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
| Final release/tag | Validate the exact release commit, packages, artifacts, and tag path. | The maintainer-selected complete or eligible fast release flow passes. |

Each slice declares impact on runtime behavior, CLI/text, Candid, JSON/config,
stable state, package features, dependencies/lockfile, fixtures/generated
output, and release artifacts. Unstated impact is not permission to change it.

## Required Slice Gates

Use the narrowest checks that exercise changed files and behavior.

Documentation/audit-governance slices use only the guards that directly own
the changed documents, plus the changelog test when its surfaces change and a
whitespace check:

```text
bash scripts/ci/check-<affected-document>.sh
cargo test --locked -p canic --test changelog_governance -- --nocapture  # when applicable
git diff --check
```

Do not add the full dependency inventory, full-history secret scan,
workspace-wide tests, Clippy, broad PocketIC, package, or deployment gates to
an ordinary documentation slice unless that slice changes the corresponding
invariant or the maintainer explicitly requests them.

Rust code slices add targeted formatting, check/Clippy, and tests for the
changed package and behavior. Direct Cargo commands use `--locked` when
supported. Unexplained lockfile churn is a blocker.

Full release validation, not ordinary slice validation, includes:

```text
make validate
```

`make validate` explicitly composes formatting checks, repository invariants,
dependency and secret gates, the control-plane feature matrix, Cargo check,
Clippy, and the complete workspace test target. It collects every independent
failure within a cheap preflight barrier, admits a compile/lint barrier only
after preflight passes, and admits the complete test barrier only after check
and warning-denied Clippy pass. Within the complete workspace test target, all
ordinary suites finish first and any ordinary failure skips the serial
PocketIC suites. The primitive targets remain independently runnable and do
not invoke unrelated validation operations.

An eligible non-runtime patch may instead use:

```text
make patch-fast
```

This lane reuses the immutable published release receipt, rejects runtime,
build, package, protocol, generated and fixture changes, and runs targeted
release/dependency/locked-compile checks. It does not run workspace tests or
PocketIC and records `gate=fast` in the release marker.

## Required Local RC Gates

Before RC promotion, the maintainer selects or explicitly assigns the complete
gate, unless the patch satisfies the fast non-runtime boundary above:

```text
make validate
```

The maintainer records environment-specific gaps rather than treating an
unexecuted gate as a pass.

## Required CI Gates

The active workflow is the source of truth. Do not reproduce its step-by-step
command inventory here. The maintained outcome categories are:

- MSRV workspace checking for pull requests and `main`;
- pinned preflight, ShellCheck, formatting, lint, default-example, layering,
  feature, dependency, secret, audit, release-contract, and current-document
  checks for pull requests and `main`;
- separately reported ordinary and ordered serial PocketIC test lanes for pull
  requests and `main`, including Fleet deployment restore as the first case in
  the one-process internal harness plus per-suite PocketIC resource evidence;
  and
- the locked release workspace build for a `Release ...` commit on `main`.

CI also validates workflow syntax, installs ICP/Wasm helpers only in the lane
that exercises them, and runs the pinned full-history secret scanner with fully redacted findings. The
release-integrity guard checks security and release outcomes without freezing
job counts or step adjacency. Audit definitions must not claim a guard runs in
CI unless the current workflow contains it.

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

## Fleet Ensure Qualification

```text
cargo test --locked -p canic-host --lib \
  fleet_ensure::tests::governed_pocketic_toko_shaped_estate_converges_then_has_zero_effects \
  -- --exact --nocapture
```

This governed PocketIC journey is current desired-state qualification evidence.
It starts from an inconsistent multi-role estate, conserves cycles through
convergence and immediately repeats with zero mutation actions. Removed local
install and test-canister targets are not maintained release gates.

The two Canic-owned blob inventory gates are temporary product guards. They
remain required while Canic owns the embedded blob subsystem and retire with
a promoted standalone blob-service hard cut; historical inventory documents remain evidence
after their executable gate wiring is removed.

## Final Release And Artifact Gates

Final release accounting for production-source changes includes:

```text
make validate
cargo build --release --workspace --locked
make package
```

An eligible non-runtime patch substitutes `make patch-fast` for `make
validate`; package publication still uses the normal candidate and matching
package-set checks. The fast receipt must remain explicit in the status marker.

It also records the exact source commit/tree, lockfile/toolchain/features,
artifact checksums and provenance, package/install probes, supported
host/target matrix, and any authorized environment-specific validation.
Versioning, staging, commits, tags, pushes, publish and deployment require an
explicit maintainer instruction. An agent may execute them when that direct
authority is present.

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
