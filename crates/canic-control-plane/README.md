# canic-control-plane

Lower-level Fleet Coordinator, Fleet Subnet Root and `wasm_store`
control-plane support crate for Canic.

Most downstream canister projects should use `canic` unless they are working
directly on root/bootstrap/store behavior and need the lower-level control-plane
types and helpers explicitly.

This crate exists to hold the shared control-plane runtime pieces used by:

- the `canic` facade when `control-plane` is enabled
- host-generated Coordinator, Root and Store canisters
- internal root/bootstrap orchestration support

## Feature Contract

All published features are enabled by default because the ordinary package
supports the complete Coordinator/root/store control-plane contract.

| Feature | Default | Enables |
| --- | --- | --- |
| `fleet-coordinator-canister` | Yes | Dedicated Fleet Coordinator lifecycle, canonical Fleet Registry state, and query APIs without root or Wasm Store runtime behavior. |
| `root-control-plane` | Yes | Root-side runtime, workflow, view, bootstrap, publication, and template-management support without Store-canister endpoints. |
| `wasm-store-canister` | Yes | Store-side template upload, manifest, chunking, garbage-collection, and install APIs without the root runtime/workflow modules. |

The host selects role features through the `canic` facade. A direct consumer
of Store implementation types may select:

```toml
canic-control-plane = { version = "<version>", default-features = false, features = ["wasm-store-canister"] }
```

The generated Fleet Coordinator package selects only
`fleet-coordinator-canister`; it does not compile App configuration or Root
runtime behavior.

Selecting `root-control-plane` with default features disabled compiles only the
root-side contract. Select `wasm-store-canister` independently for the sibling
Store canister package.

See `../../README.md` for the broader workspace overview and use `canic` as the
default public entry surface unless you specifically need this crate.
