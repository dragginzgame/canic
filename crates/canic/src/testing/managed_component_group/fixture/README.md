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

Current SHA-256:
`fc2c6d931733e9946464059f39066d580c8f259a0d20862a17ee2f11be2e982f`.
