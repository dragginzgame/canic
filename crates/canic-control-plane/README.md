# canic-control-plane

Lower-level root and `wasm_store` control-plane support crate for Canic.

Most downstream canister projects should use `canic` unless they are working
directly on root/bootstrap/store behavior and need the lower-level control-plane
types and helpers explicitly.

This crate exists to hold the shared control-plane runtime pieces used by:

- the `canic` facade when `control-plane` is enabled
- the canonical `canic-wasm-store` crate
- internal root/bootstrap orchestration support

See `../../README.md` for the broader workspace overview and use `canic` as the
default public entry surface unless you specifically need this crate.
