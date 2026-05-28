# Build Artifacts

This is the canonical vocabulary for Canic build outputs and generated canister
interfaces after the 0.33 ICP CLI hard cut.

## Project State

- `.icp/` is the local ICP CLI project state root.
- `.canic/` is Canic operator state. Deployment-target install state lives under
  `.canic/<network>/deployments/<deployment>.json`.

## Canister Build Contract

Every Canic-managed canister package declares its runtime role in Cargo
metadata:

```toml
[package.metadata.canic]
role = "app"
```

The package role is the single source of truth for the build and startup
macros. `canic::build!("<path-to-canic.toml>")` validates that the metadata role
exists in the fleet config and emits the compile-time role/config environment
consumed by `canic::start!()`.

`role = "root"` emits the root build cfgs and selects the root lifecycle and
root endpoint bundle. Every other configured role selects the non-root
lifecycle and endpoint bundle. There is no separate public root startup macro.

## Canister Artifacts

- Direct Cargo canister builds emit raw wasm under
  `target/wasm32-unknown-unknown/<profile>/canister_<name>.wasm`.
- Canic release artifacts are gzip-compressed `.wasm.gz` files owned by the
  Canic build scripts.
- ICP CLI-visible canister artifacts and generated Candid sidecars live under
  `.icp/<environment>/canisters/<role>/`.
- Generated Candid interfaces use
  `.icp/<environment>/canisters/<role>/<role>.did`.

## Audit Usage

Recurring audits should use this vocabulary instead of restating tool-specific
paths inline. When an audit needs a concrete local environment path, use
`local` unless the report preamble names another environment.
