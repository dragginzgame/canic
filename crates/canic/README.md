# canic

Facade crate that re-exports the main Canic stack for canister projects:

- endpoint and lifecycle macros
- core runtime/types
- curated IC CDK helpers
- stable-memory helpers under `canic::memory`

Most downstream canister projects should start here instead of reaching for
lower-level crates directly.

Use the explicit module paths for the larger bundled surfaces:

- `canic::api::*` for runtime APIs
- `canic::cdk::*` for curated IC CDK helpers
- `canic::memory::*` for stable-memory helpers and macros

## Default surface

The default feature set includes:

- `metrics` — exports `canic_metrics` in ordinary builds unless you opt out

If you want a narrower facade dependency, disable default features in your
`Cargo.toml`.

## Optional features

- `control-plane` — enable root/`wasm_store` control-plane support
- `sharding` — enable sharding-oriented runtime support from `canic-core`
- `auth-crypto` — enable crypto-backed auth/runtime helpers from `canic-core`

## Typical use

Use `canic` in both `[dependencies]` and `[build-dependencies]` so the build
macros and runtime macros come from the same facade crate.

This crate lives in the Canic workspace. See the workspace guide at
`../../README.md` for full setup, topology, and example canisters.
