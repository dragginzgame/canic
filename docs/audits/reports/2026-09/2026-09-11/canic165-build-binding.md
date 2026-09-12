# CANIC-165 complete-build fixture binding

Date: 2026-09-11. This continues the accepted Canic-only FP2 implementation;
it is not a release or minor closeout audit.

## Result

Complete App builds now discover package-declared fixture sources, retain them
before Cargo, and bind their canonical child manifest into the complete release.
The required `fixture_artifact_manifest_sha256` field also covers empty fixture
sets. Its current v1 schema changes through the pre-1.0 hard cut.

Source fingerprints cover configuration, attached package manifests, source
documents and payload files. Explicitly declared files under normally excluded
directories participate in the build cache. Metadata is inspected without an
additional Cargo process. Input verification runs even when complete-build
reuse is unavailable; before finalization it also checks the retained descriptors
against the initial byte observation. Cache reuse validates retained content
against the current configured selection and preserves exact release identity.

The transient-change test demonstrates why a final source-file comparison alone
is insufficient: a different payload can be copied and then the original source
restored. Retained descriptor comparison rejects that substitution.

Existing no-fixture Fleet generation remains valid. Fixture-bearing generation
and Canic installation argument compilation reject until delivery is connected,
preventing a declared prerequisite from being silently skipped. Source metadata
and local paths do not enter runtime configuration. The internal Fleet fixture
builder now persists the required empty fixture manifest.

The [source/build contract](../../../../features/build-and-evidence/fixture-artifacts.md)
documents package metadata, source JSON fields, path/transport bounds and the
current delivery boundary.

## Targeted evidence

- [42 host tests](canic165-build-binding-evidence/host-tests.log) pass in 11.94s
  after 36.41s compilation. They cover fixture artifacts/configuration, transient
  source substitution, complete manifest binding, build-reuse input discovery,
  first-build dependency records and existing Fleet generation authority.
- [25 CLI build tests](canic165-build-binding-evidence/cli-tests.log) pass in
  0.01s after 1m43s compilation. These qualify the current command surface; they
  do not claim an end-to-end production Wasm build or deployment.
- [Host, CLI and internal testing library/test Clippy](canic165-build-binding-evidence/clippy.log)
  passes with warnings denied in 14.38s.
- Scoped formatting, diff whitespace and changed-document link checks pass.
  [Affected source hashes](canic165-build-binding-evidence/source.sha256) record
  the final implementation checkpoint.

Commands use `ICP_ENVIRONMENT=local RUSTC_WRAPPER= CARGO_NET_OFFLINE=true`:

```sh
cargo test --locked -p canic-host --lib -- release_set::fixture release_set::current canister_build::reuse fleet_ensure::generate::tests --test-threads=1
cargo test --locked -p canic-cli --lib build::tests
cargo clippy --locked --keep-going -p canic-host -p canic-cli -p canic-testing-internal --lib --tests -- -D warnings
```

No broad gate or PocketIC runtime journey ran. This extension changes host build,
cache and release-manifest authority; Store Candid and endpoint authorization
remain at the earlier qualified checkpoint. No dependency, version, publication,
deployment or sibling-repository mutation was performed.

## Remaining work

Reviewed remote publication must preserve Root-only source registration and
provide a content-scoped host upload path without a Root payload relay. Root
must then derive grants from verified installation intent and connect importer
receipts to placement/dispatch and deployment completion. Initial parent/child
ordering, later Shards, funding/backoff and reference retirement remain in FP2.

CANIC-165 and the combined worktree are not push-ready. The changelog remains
unassigned under root Unreleased; package versions are unchanged. Continue the
[accepted design](../../../working/canic165-fixture-provisioning/design.md)
without a closeout-audit prerequisite.
