# Dependency roles and deployment admission

Canic records its reviewed Shared Tooling revision in
[the snapshot](../../.shared-tooling.snapshot). Shared Tooling owns the
[role definitions](../../rules/cargo-dependencies.md#dependency-roles-and-deployment-checks).
Catalog labels describe use here, not registry publication or build success.
The selected IC tool catalog advances ICP CLI from 1.6.0 to 1.7.0; the extractor
remains explicitly selected at 0.1.6. Existing tool installations are retained.

## Classification

The production family is `canic`, `canic-backup`, `canic-cli`, `canic-contracts`,
`canic-control-plane`, `canic-core`, `canic-host`, `canic-macros` and
`canic-blob-service`. Demo/test Apps, framework probes and internal test crates are
validation packages. The two independent Blob consumers are validation workspaces.

The root catalog's tests/tooling-only entries are `canic-blob-service`, `canic-cli`,
and `canic-testing-internal`. The first two are production packages,
but these catalog entries are consumed only by validation packages/dev edges.
A deployment root is not its own dependency. Normal/build dependencies needed by
production, including supported optional features and transitive uses, are labelled
production. In particular, `ic-testkit` is used by Host's supported `local-fleet`
feature; its name does not make it a test-only dependency here. Renamed dependencies
must be compared using their Cargo package identity, not the catalog key.

## Formatting and release checks

The Canic formatter uses `cargo sort --workspace --grouped` for write and check
modes, including both independent Blob workspaces. The release-candidate guard
uses that same ordering for current/prepared source and its exact archived release
commit. The independent workspaces retain their existing table-order selection.
Hooks continue to delegate to the formatter. Neither check changes dependency
versions, lockfiles or release versions.

## Reviewed graph evidence — 2026-10-18

Locked, offline `cargo tree --edges normal,build` inspection passed for all nine
production packages, for default and all-feature selections. Runtime packages
used `wasm32-unknown-unknown`; Host, CLI, Backup and Macros used the native host.
No tests/tooling-only catalog identity occurred below the selected root in these
18 graphs. Direct normal, build, development and target-specific declarations were
also reviewed. Evidence is retained in `/tmp/canic-shared-024/graphs.json` and the
adjacent tree files. This is graph inspection, not compilation qualification;
all-features inspection does not prove every feature combination builds.

## Missing enforcement

**The labels are not automatically enforced at deployment.** Follow-up:
[Canic #602](https://github.com/dragginzgame/canic/issues/602). Current role-contract
admission checks protected Canic ownership paths and runtime Memory/Timer identities.
The Wasm crypto closure gate checks its selected crypto identities. Neither reads
these role labels nor excludes all test-only packages through transitive normal
and build edges. Formatting and snapshot integrity do not supply that enforcement.

The remaining consumer work belongs in Host's selected build/deployment admission:
retain a machine-readable exclusion policy, inspect the exact locked target and
feature graph (including renamed and transitive build dependencies), and bind that
result to the successful build's source/lock/selection and final artifact digest.
Cover both generated infrastructure and application builds; inspect a cache reuse
against its retained evidence as well. Do not treat this broad graph review as a
receipt for a later build or deployed artifact. Registry availability is a separate
check. Until this is implemented, repeat exact graph inspection for each deployment.
