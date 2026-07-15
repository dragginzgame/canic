# canic-host

Host-side build, install, deployment, fleet-template, and thin-root staging tooling for Canic workspaces.

## When to use it

Use this crate directly when you need:

- Canic build/install backend code in CI or local automation
- root staging from published backend APIs
- the lower-level host library surface without cloning the full repo

For normal local setup, prefer the root
[`INSTALLING.md`](../../INSTALLING.md) guide or use the tagged repo installer
script directly:

```bash
curl -fsSL https://raw.githubusercontent.com/dragginzgame/canic/v0.92.2/scripts/dev/install_dev.sh | bash
```

That script bootstraps Rust when needed and installs the pinned internal
toolchain, the `canic` CLI, wasm/Candid utilities, and `icp` when missing.
This README documents the lower-level host library surface.

## What this crate is not

This crate is not a general deployment framework and it is not the main Canic
application facade. It owns host-side build/install/fleet/staging utilities
for standard Canic root/bootstrap/store flows. For normal operator use, prefer
the installed `canic` CLI and the compact v1 workflow documented in
`docs/architecture/v1-operator-walkthrough.md`; use install commands only for
the local managed-fleet flows that document them explicitly.

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

- `canic build <fleet> <role> --profile debug` builds raw debug wasm
- `canic build <fleet> <role> --profile fast` builds the middle shrunk local/test/demo lane
- `canic build <fleet> <role> --profile release` builds the shipping/install lane

If omitted, CLI builds default to `release`.

When the Rust workspace root and ICP CLI/project root differ, pass
`--workspace`, `--icp-root`, and `--config` to `canic build`. The low-level
`build_artifact` example takes those three paths after its role and profile.

If canister crates live outside the default `fleets/` directory, host
discovery first tries Cargo workspace metadata. Every Canic-managed canister
package must declare the fleet-scoped role it implements in Cargo metadata:

```toml
[package.metadata.canic]
fleet = "project"
role = "project_ledger"
```

For `canic install`, the implicit network default is always `local`; use
`--network <name>` for one command against another network. The public CLI
requires the fleet name as the first positional argument and uses
`fleets/<name>/canic.toml`.
