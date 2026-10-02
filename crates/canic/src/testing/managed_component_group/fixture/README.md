# Managed Component Group Root fixture

`sharding_root_stub.wasm` is the host-only allocation peer embedded by the
public `canic::testing` Component-tree fixture. It is built from
`canisters/test/sharding_root_stub` and is never linked into a deployed Canic
runtime because the complete testing module is disabled for `wasm32` targets.

Refresh it and its structured provenance from the workspace root with:

```text
cargo run --locked --offline -p canic-testing-internal --example refresh_embedded_root
```

Run this only while other repository validation is idle. The refresh records the
selected Cargo inputs, lock, tool identities and artifact hash in
[`scripts/dev/managed-root-fixture.json`](../../../../../../scripts/dev/managed-root-fixture.json).
Evidence lives outside the producer's dependencies so it cannot hash itself.

The internal builder copies the current Git-listed files (including untracked
source additions) into invocation-owned scratch. Only in that copy, the fixture
workspace uses synthetic package version `0.0.0`; matching local dependency
requirements follow it. External locked packages must remain exactly unchanged.
This is a test-only producer identity, not the version of any deployed Canic
package. Rust source names are remapped before compilation so local workspace,
Cargo-cache and toolchain paths do not leak into Wasm panic strings. The producer
recipe is itself a watched input. Both original and producer lock hashes are
retained, and scratch is removed on return; the shared artifact cache survives.

The governed managed Component-tree PocketIC journey checks this evidence before
installing the peer, then validates the production allocation and acknowledgement
protocol. Unchanged selected inputs reuse the recorded proof. Changed inputs
require a content-addressed build and exact byte comparison; metadata-only changes
that produce identical Wasm pass without rewriting tracked files. Tests never
refresh the fixture automatically. Ordinary release-version changes therefore
reuse the same source proof instead of invalidating the peer after validation.

The explicit cross-directory/version regression can be run with:

```text
cargo test --locked --offline -p canic-testing-internal --lib embedded_root::tests::embedded_peer_reproduces_across_paths_and_release_versions -- --exact --ignored
```

It performs real Wasm builds, changes only private snapshot versions, and checks
that a broken source edit still refuses qualification. It does not claim identical
output across different compiler releases or host architectures.

The peer retains allocation replies until host settlement has installed the child
and synchronized its parent's exact allocation identity. On-demand calls must be
submitted before host settlement and awaited afterward.
