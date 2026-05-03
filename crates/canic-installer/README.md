# canic-installer

Published installer and release-set tooling for downstream Canic workspaces.

## When to use it

Use this crate directly when you need:

- Canic build/install binaries in CI or local automation
- root release-set staging from a published tool package
- the downstream installer surface without cloning the full repo

For normal local setup, use the tagged repo installer script:

```bash
curl -fsSL https://raw.githubusercontent.com/dragginzgame/canic/v0.30.3/scripts/dev/install_dev.sh | bash
```

That script bootstraps Rust when needed and installs the pinned internal toolchain, `canic-installer`, wasm/Candid utilities, and `dfx` when missing. This README documents the installed-binary surface.

## What this crate is not

This crate is not a general deployment framework and it is not the main Canic application facade. It owns the published build/install/release utilities for standard Canic root/bootstrap/store flows.

Public thin-root flow:

- build visible canister artifacts through `canic-build-canister-artifact`
- build the implicit bootstrap `wasm_store` through `canic-build-wasm-store-artifact`
- emit `.dfx/<network>/canisters/root/root.release-set.json`
- stage the ordinary release set into `root`
- resume root bootstrap
- drive local root install, including one clean local `dfx` restart attempt when `dfx ping local` fails

Typical installed binaries:

- `canic-build-canister-artifact`
- `canic-build-wasm-store-artifact`
- `canic-emit-root-release-set-manifest`
- `canic-list-install-targets`
- `canic-stage-root-release-set`
- `canic-install-root`

Build profile selection:

- `CANIC_WASM_PROFILE=debug` builds raw debug wasm
- `CANIC_WASM_PROFILE=fast` builds the middle shrunk local/test/demo lane
- `CANIC_WASM_PROFILE=release` builds the shipping/install lane

If unset, installer/build binaries default to `release`.

When the Rust workspace root and DFX/project root differ, set both:

- `CANIC_WORKSPACE_ROOT` for Cargo, `canic.toml`, and canister manifests
- `CANIC_DFX_ROOT` for `dfx.json`, `.dfx`, and emitted artifacts

If canister crates live outside the default `canisters/` directory, the installer first tries Cargo workspace metadata. No extra config is needed when package names follow `canister_<role>`, even in nested paths.

To inspect the local install target list from `canic.toml`:

```bash
canic-list-install-targets
```

That command prints `root` first, then ordinary roles from the subnet that owns `root`. It excludes the implicit bootstrap `wasm_store`. To point at a specific config path:

```bash
canic-list-install-targets path/to/canic.toml
```

If you need to override discovery explicitly, set:

- `CANIC_CANISTERS_ROOT` for the canister crate root relative to `CANIC_WORKSPACE_ROOT`

or point `CANIC_CONFIG_PATH` at the real `canic.toml` path and the installer will infer the canister-manifest root from that config location.

If a package name does not follow `canister_<role>`, declare the role mapping
in `Cargo.toml`:

```toml
[package.metadata.canic]
role = "project_ledger"
```
