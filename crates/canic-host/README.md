# canic-host

Host-side build, install, fleet, and release-set tooling for Canic workspaces.

## When to use it

Use this crate directly when you need:

- Canic build/install backend code in CI or local automation
- root release-set staging from published backend APIs
- the lower-level host library surface without cloning the full repo

For normal local setup, use the tagged repo installer script:

```bash
curl -fsSL https://raw.githubusercontent.com/dragginzgame/canic/v0.31.2/scripts/dev/install_dev.sh | bash
```

That script bootstraps Rust when needed and installs the pinned internal
toolchain, the `canic` CLI, wasm/Candid utilities, and `dfx` when missing.
This README documents the lower-level host library surface.

## What this crate is not

This crate is not a general deployment framework and it is not the main Canic
application facade. It owns host-side build/install/fleet/release utilities
for standard Canic root/bootstrap/store flows. For normal operator use, prefer
`canic build`, `canic install`, and other `canic` commands.

It is also separate from:

- `canic-backup`, which owns backup/restore manifests, journals, topology
  snapshots, preflight checks, and restore planning.
- `canic-core` and `canic-control-plane`, which run inside canisters or provide
  canister-runtime support. `canic-host` runs on the operator machine and may
  call Cargo, `dfx`, and the local filesystem.

Public thin-root flow:

- build visible canister artifacts through the backend builder used by `canic build`
- build the implicit bootstrap `wasm_store` through the backend builder used by `canic build wasm_store`
- emit `.dfx/<network>/canisters/root/root.release-set.json`
- stage the ordinary release set into `root`
- resume root bootstrap
- drive local root install, including one clean local `dfx` restart attempt when `dfx ping local` fails

Build profile selection:

- `CANIC_WASM_PROFILE=debug` builds raw debug wasm
- `CANIC_WASM_PROFILE=fast` builds the middle shrunk local/test/demo lane
- `CANIC_WASM_PROFILE=release` builds the shipping/install lane

If unset, backend builds default to `release`.

When the Rust workspace root and DFX/project root differ, set both:

- `CANIC_WORKSPACE_ROOT` for Cargo, `canic.toml`, and canister manifests
- `CANIC_DFX_ROOT` for `dfx.json`, `.dfx`, and emitted artifacts

If canister crates live outside the default `canisters/` directory, host
discovery first tries Cargo workspace metadata. No extra config is needed when
package names follow `canister_<role>`, even in nested paths.

To inspect the local install target list from `canic.toml`, prefer the main
CLI:

```bash
canic release-set targets
```

That command prints `root` first, then ordinary roles from the subnet that owns `root`. It excludes the implicit bootstrap `wasm_store`. To point at a specific config path:

```bash
canic release-set targets --config path/to/canic.toml
```

If you need to override discovery explicitly, set:

- `CANIC_CANISTERS_ROOT` for the canister crate root relative to `CANIC_WORKSPACE_ROOT`

or point `CANIC_CONFIG_PATH` at the real `canic.toml` path and host discovery
will infer the canister-manifest root from that config location.

For `canic install`, the project default is `canisters/canic.toml`. If that
file is missing and multiple nested `canic.toml` files exist, the command
prints a choices table and requires `--config <path>` instead of guessing.

If a package name does not follow `canister_<role>`, declare the role mapping
in `Cargo.toml`:

```toml
[package.metadata.canic]
role = "project_ledger"
```
