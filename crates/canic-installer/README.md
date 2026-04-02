# canic-installer

Published installer and release-set tooling for downstream Canic workspaces.

This crate owns the public thin-root build and staging path:

- build visible canister artifacts through `canic-build-canister-artifact`
- build the hidden bootstrap store through `canic-build-wasm-store-artifact`
- emit `.dfx/<network>/canisters/root/root.release-set.json`
- stage the ordinary release set into `root`
- resume root bootstrap
- drive the local root install flow against an already running `dfx` replica

Typical installed binaries:

- `canic-build-canister-artifact`
- `canic-build-wasm-store-artifact`
- `canic-emit-root-release-set-manifest`
- `canic-stage-root-release-set`
- `canic-install-root`
- `canic-install-reference-topology`

When the Rust workspace root and the DFX/project root differ, set both:

- `CANIC_WORKSPACE_ROOT` for Cargo, `canic.toml`, and canister manifests
- `CANIC_DFX_ROOT` for `dfx.json`, `.dfx`, and emitted artifacts
