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

The governed managed Component-tree PocketIC journey checks this evidence before
installing the peer, then validates the production allocation and acknowledgement
protocol. Unchanged selected inputs reuse the recorded proof. Changed inputs
require a content-addressed build and exact byte comparison; metadata-only changes
that produce identical Wasm pass without rewriting tracked files. Tests never
refresh the fixture automatically.

The peer retains allocation replies until host settlement has installed the child
and synchronized its parent's exact allocation identity. On-demand calls must be
submitted before host settlement and awaited afterward.
