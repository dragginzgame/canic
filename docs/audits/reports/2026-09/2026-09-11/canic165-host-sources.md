# CANIC-165 host fixture sources

Date: 2026-09-11. Scope: continued Canic-only FP2 implementation under the
[accepted design](../../../working/canic165-fixture-provisioning/design.md).
This is an implementation checkpoint, not a release or minor closeout audit.

## Implemented

`canic-host::release_set::fixture` compiles application-authored, independently
decodable chunk files without interpreting database rows or altering their
order. Each payload uses the existing 1 MiB transport bound. The existing Root
release-manifest byte ceiling bounds local manifest reads; no new application
instruction threshold is introduced.

The canonical child manifest binds the release ID, Component topology digest,
application role, descriptor and content ID. Identical sources can serve multiple
roles; content identity excludes release identity, paths and future target
Principals. Host and Store now use the same control-plane content compiler,
including validation of the full serialized Store command envelope.

Artifact persistence rejects non-relative paths and observed file/directory
symlinks, verifies each bounded payload against its descriptor, and commits
payload files before the manifest. Exact existing files are never overwritten.
Interrupted copying resumes from retained files; changed source selections
conflict, and finalized releases cannot acquire or repair missing artifacts.
Publication reads retained artifacts, so mutable or removed authored files
cannot change an already selected payload.

The publication planner accepts an authenticated source observation and returns
one preparation or upload intent, or complete. It validates content ID, chunk
count, prefix byte count and the completion flag. Only an explicit `NotFound`
requests preparation; other Store errors propagate. It owns neither network
execution nor a competing persistent upload cursor.

## Qualification

- [Seven host and six Store native cases](canic165-host-source-evidence/native.log)
  pass. Coverage includes role/content/release separation, chunk order, exact
  retry, interrupted copying, finalized-artifact refusal, source and retained
  tampering, role/path/envelope bounds and false completion/error propagation.
  The final run took 52.32s to compile, then 0.41s and 0.17s for the test groups.
- [Host/control-plane library and test Clippy](canic165-host-source-evidence/clippy.log)
  passes with warnings denied, as does the exact
  [composition integration target](canic165-host-source-evidence/composition-clippy.log).
- The [Root-only feature compile](canic165-host-source-evidence/root-check.log)
  passes: content compilation does not require Store state ownership.
- Scoped formatting and diff whitespace checks pass. The final
  [affected source hashes](canic165-host-source-evidence/source.sha256) include
  the subsequent import grouping and module-documentation cleanup.

Commands use `ICP_ENVIRONMENT=local RUSTC_WRAPPER= CARGO_NET_OFFLINE=true`:

```sh
cargo test --locked -p canic-host -p canic-control-plane --lib -- release_set::fixture ops::fixture_store::tests --test-threads=1
cargo clippy --locked -p canic-host -p canic-control-plane --lib --tests -- -D warnings
cargo clippy --locked -p canic-tests --test icydb_lifecycle_composition -- -D warnings
cargo check --locked -p canic-control-plane --lib --no-default-features --features root-control-plane
```

No PocketIC runtime journey was rerun: this extension changes host artifact
handling and extracts the existing Store validator into a shared owner. No
canister endpoint authorization or Candid shape changed. The previous
[retained Store journey](canic165-store.md) remains a separate checkpoint.

## Remaining integration

The new API is not yet called by `canic build` or Fleet Ensure. Config source
discovery, pre-build input fingerprinting, complete-release digest binding,
reviewed remote publication and its bounded publisher authority remain next.
A fixture child manifest by itself grants no runtime authority.

Root-derived grants, importer/receipt delivery, Prepared-Root parent/child
ordering, later Shards, funding/backoff and terminal reference retirement remain
in FP2. CANIC-165 and the combined worktree are not push-ready because these
requirements remain unimplemented. The separately qualified .15 operator
corrections are unchanged. No version, Git publication, deployment, broad gate,
dependency upgrade or sibling-repository mutation ran.
