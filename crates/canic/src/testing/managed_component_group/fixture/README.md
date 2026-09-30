# Managed Component Group Root fixture

`sharding_root_stub.wasm` is the host-only allocation peer embedded by the
public `canic::testing` Component-tree fixture. It is built from
`canisters/test/sharding_root_stub` and is never linked into a deployed Canic
runtime because the complete testing module is disabled for `wasm32` targets.

Rebuild it from the workspace root with:

```text
cargo build --locked -p sharding_root_stub --target wasm32-unknown-unknown --profile fast
```

Then copy
`target/wasm32-unknown-unknown/fast/sharding_root_stub.wasm` over the adjacent
fixture. The governed managed Component-tree PocketIC journey validates the
embedded Wasm through the production allocation and acknowledgement protocol.
The peer retains allocation replies until host settlement has installed the child
and synchronized its parent's exact allocation identity. On-demand calls must be
submitted before host settlement and awaited afterward.

Current SHA-256:
`f2aaa3bafcd7b4e8c45d8e5e3725283e03d5dd7602758c8ead4346e3c9871d6b`.
