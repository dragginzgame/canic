# canic-host

Host-side build, install, fleet, and thin-root staging tooling for Canic workspaces.

## When to use it

Use this crate directly when you need:

- Canic build/install backend code in CI or local automation
- root staging from published backend APIs
- the lower-level host library surface without cloning the full repo

For normal local setup, use the tagged repo installer script:

```bash
curl -fsSL https://raw.githubusercontent.com/dragginzgame/canic/v0.36.5/scripts/dev/install_dev.sh | bash
```

That script bootstraps Rust when needed and installs the pinned internal
toolchain, the `canic` CLI, wasm/Candid utilities, and `icp` when missing.
This README documents the lower-level host library surface.

## What this crate is not

This crate is not a general deployment framework and it is not the main Canic
application facade. It owns host-side build/install/fleet/staging utilities
for standard Canic root/bootstrap/store flows. For normal operator use, prefer
`canic install` and other `canic` commands.

It is also separate from:

- `canic-backup`, which owns backup/restore manifests, journals, topology
  snapshots, backup layout validation, and restore planning.
- `canic-core` and `canic-control-plane`, which run inside canisters or provide
  canister-runtime support. `canic-host` runs on the operator machine and may
  call Cargo, `icp`, and the local filesystem.

Public thin-root flow:

- build visible canister artifacts through the backend builder used by install
- build the implicit bootstrap `wasm_store` through the same backend builder
- emit the root staging manifest under `.icp/<network>/canisters/root/`
- stage the ordinary fleet artifacts into `root`
- resume root bootstrap
- drive local root install, including one clean local `icp` restart attempt when `icp ping local` fails

Build profile selection:

- `CANIC_WASM_PROFILE=debug` builds raw debug wasm
- `CANIC_WASM_PROFILE=fast` builds the middle shrunk local/test/demo lane
- `CANIC_WASM_PROFILE=release` builds the shipping/install lane

If unset, backend builds default to `release`.

When the Rust workspace root and ICP CLI/project root differ, set both:

- `CANIC_WORKSPACE_ROOT` for Cargo, `canic.toml`, and canister manifests
- `CANIC_ICP_ROOT` for `icp.yaml`, `.icp`, and emitted artifacts

If canister crates live outside the default `fleets/` directory, host
discovery first tries Cargo workspace metadata. No extra config is needed when
package names follow `canister_<role>`, even in nested paths.

If you need to override discovery explicitly, set:

- `CANIC_CANISTERS_ROOT` for the canister crate root relative to `CANIC_WORKSPACE_ROOT`

or point `CANIC_CONFIG_PATH` at the real `canic.toml` path and host discovery
will infer the canister-manifest root from that config location.

For `canic install`, the implicit network default is always `local`; use
`--network <name>` for one command against another network. The public CLI
requires the fleet name as the first positional argument and uses
`fleets/<name>/canic.toml`.

If a package name does not follow `canister_<role>`, declare the role mapping
in `Cargo.toml`:

```toml
[package.metadata.canic]
role = "project_ledger"
```
