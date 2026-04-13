# canic-installer

Published installer and release-set tooling for downstream Canic workspaces.

## Who should use this crate directly

Use this crate directly when you need:

- the installed Canic release/build binaries in CI or local automation
- root release-set staging or install flows from a published tool package
- the canonical downstream installer/build surface without cloning the full repo

If you just want a normal local setup, prefer the tagged install script below.
That is still the simplest entry path for most users.

For the full local setup path, prefer the shared tagged installer script from the
Canic repo:

```bash
curl -fsSL https://raw.githubusercontent.com/dragginzgame/canic/v0.27.4/scripts/dev/install_dev.sh | bash
```

That script bootstraps Rust when needed, installs the pinned toolchain,
`canic-installer`, the required wasm/Candid utilities, and `dfx` when it is
missing. This crate README documents the thinner installed-binary surface below.

## What this crate is not

This crate is not a general deployment framework and it is not the main Canic
application facade. It owns the published build/install/release utilities for
the standard Canic topology, especially root/bootstrap/store flows.

This crate owns the public thin-root build and staging path:

- build visible canister artifacts through `canic-build-canister-artifact`
- build the hidden bootstrap store through `canic-build-wasm-store-artifact`
- emit `.dfx/<network>/canisters/root/root.release-set.json`
- stage the ordinary release set into `root`
- resume root bootstrap
- drive the local root install flow, including one clean local `dfx` restart attempt when `dfx ping local` fails

Typical installed binaries:

- `canic-build-canister-artifact`
- `canic-build-wasm-store-artifact`
- `canic-emit-root-release-set-manifest`
- `canic-list-install-targets`
- `canic-stage-root-release-set`
- `canic-install-root`
- `canic-install-reference-topology`

Build-profile selection is explicit:

- `CANIC_WASM_PROFILE=debug` builds raw debug wasm
- `CANIC_WASM_PROFILE=fast` builds the middle shrunk local/test/demo lane
- `CANIC_WASM_PROFILE=release` builds the shipping/install lane

If unset, installer/build binaries default to `release`.

When the Rust workspace root and the DFX/project root differ, set both:

- `CANIC_WORKSPACE_ROOT` for Cargo, `canic.toml`, and canister manifests
- `CANIC_DFX_ROOT` for `dfx.json`, `.dfx`, and emitted artifacts

If canister crates live somewhere other than the default `canisters/`
directory, the installer first tries to discover them from Cargo workspace
metadata. Zero extra config is needed when package names still follow the
normal `canister_<role>` convention, even if manifests live in nested paths.

If you need the local install target list from `canic.toml` directly,
`canic-installer` now exposes it as one public CLI:

```bash
canic-list-install-targets
```

That command prints one install target per line for the single subnet that owns
`root`, including `root` first and then the ordinary roles for that subnet.
The hidden bootstrap `wasm_store` is still excluded. To point at a specific
config path explicitly:

```bash
canic-list-install-targets path/to/canic.toml
```

If you need to override discovery explicitly, set:

- `CANIC_CANISTERS_ROOT` for the canister crate root relative to `CANIC_WORKSPACE_ROOT`

or point `CANIC_CONFIG_PATH` at the real `canic.toml` path and the installer
will infer the canister-manifest root from that config location.

If a package name does not follow `canister_<role>`, declare the role mapping
in `Cargo.toml`:

```toml
[package.metadata.canic]
role = "project_ledger"
```
