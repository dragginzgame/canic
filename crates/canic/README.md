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

## Default Surface

The default feature set is intentionally small:

- `metrics` - exports `canic_metrics` in ordinary builds unless you opt out

Disable default features in `Cargo.toml` when you need an even narrower facade
dependency.

## Optional Features

These features can also be selected explicitly when default features are off:

- `metrics`
- `control-plane` - enables root control-plane support
- `sharding` - enables sharding-oriented runtime support from `canic-core`
- `auth-root-canister-sig-create` - enables root canister-signature proof creation
- `auth-root-canister-sig-verify` - enables IC canister-signature proof verification
- `auth-issuer-canister-sig-create` - enables issuer canister-signature token proof creation
- `auth-issuer-canister-sig-verify` - enables issuer canister-signature token proof verification
- `auth-delegated-token-verify` - enables delegated-token verification, including
  root and issuer canister-signature verification

## Typical Use

Use `canic` in both `[dependencies]` and `[build-dependencies]` so the build
macros and runtime macros come from the same facade crate.

Each canister crate declares its role in package metadata:

```toml
[package.metadata.canic]
fleet = "demo"
role = "app"
```

Use `canic::build!("../canic.toml")` from `build.rs` and `canic::start!()` from
`lib.rs`. The `fleet` value must match `[fleet] name = "..."` in the selected
`canic.toml`. `role = "root"` selects the root lifecycle and root endpoint
bundle; ordinary roles select the non-root lifecycle and endpoint bundle.

This crate lives in the Canic workspace. See the workspace guide at
`../../README.md` for full setup, topology, and example canisters.
