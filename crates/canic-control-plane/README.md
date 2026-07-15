# canic-control-plane

Lower-level root and `wasm_store` control-plane support crate for Canic.

Most downstream canister projects should use `canic` unless they are working
directly on root/bootstrap/store behavior and need the lower-level control-plane
types and helpers explicitly.

This crate exists to hold the shared control-plane runtime pieces used by:

- the `canic` facade when `control-plane` is enabled
- the canonical `canic-wasm-store` crate
- internal root/bootstrap orchestration support

## Feature Contract

Both published features are enabled by default because the ordinary package
supports the complete root/store control-plane contract.

| Feature | Default | Enables |
| --- | --- | --- |
| `root-control-plane` | Yes | Root-side runtime, workflow, view, bootstrap, publication, and template-management support; also enables `wasm-store-canister`. |
| `wasm-store-canister` | Yes | Store-side template upload, manifest, chunking, garbage-collection, and install APIs without the root runtime/workflow modules. |

Downstream roots should normally select the `canic` facade's `control-plane`
feature instead of depending on this crate directly. The canonical standalone
`wasm_store` package may use:

```toml
canic-control-plane = { version = "<version>", default-features = false, features = ["wasm-store-canister"] }
```

Selecting `root-control-plane` with default features disabled still enables
`wasm-store-canister`; there is no root-only feature combination that omits the
store-side contract.

See `../../README.md` for the broader workspace overview and use `canic` as the
default public entry surface unless you specifically need this crate.
