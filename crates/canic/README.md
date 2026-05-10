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

The pre-1.0 default feature set includes the standard Canic runtime bundle:

- `metrics` — exports `canic_metrics` in ordinary builds unless you opt out
- `control-plane` — enables root/`wasm_store` control-plane support
- `sharding` — enables sharding-oriented runtime support from `canic-core`
- `auth-crypto` — enables crypto-backed auth/runtime helpers from `canic-core`

Disable default features in `Cargo.toml` when you need a narrower facade
dependency and want to opt out of the standard runtime bundle.

## Optional features

These features can also be selected explicitly when default features are off:

- `metrics`
- `control-plane`
- `sharding`
- `auth-crypto`

## Typical use

Use `canic` in both `[dependencies]` and `[build-dependencies]` so the build
macros and runtime macros come from the same facade crate.

This crate lives in the Canic workspace. See the workspace guide at
`../../README.md` for full setup, topology, and example canisters.
