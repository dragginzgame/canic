# canic-host

Host-side build, install, deployment, App-source, and multi-root staging
tooling for Canic workspaces.

## When to use it

Use this crate directly when you need:

- Canic build/install backend code in CI or local automation
- Coordinator/root/Store staging from published backend APIs
- the lower-level host library surface without cloning the full repo

For normal local setup, prefer the root
[`INSTALLING.md`](../../INSTALLING.md) guide. From a Canic checkout, use the
maintainer setup target:

```bash
make install-dev
```

That path requires Rustup and Cargo, then installs the pinned internal
toolchain, the `canic` CLI, wasm/Candid utilities, and checksum-bound external
executables.
This README documents the lower-level host library surface.

## What this crate is not

This crate is not a general deployment framework and it is not the main Canic
application facade. It owns host-side build/install/Fleet/staging utilities
for standard Canic Coordinator/root/Store flows. For normal operator use, prefer
the installed `canic` CLI and the compact v1 workflow documented in
`docs/architecture/v1-operator-walkthrough.md`; use install commands only for
the local managed-fleet flows that document them explicitly.

It is also separate from:

- `canic-backup`, which owns backup/restore manifests, journals, topology
  snapshots, backup layout validation, and restore planning.
- `canic-core` and `canic-control-plane`, which run inside canisters or provide
  canister-runtime support. `canic-host` runs on the operator machine and may
  call Cargo, `icp`, and the local filesystem.

Current 0.100 installation flow:

- compile the App's complete Component Topology and resolve the required
  operator Fleet input before effects
- build and freeze the exact Fleet Coordinator, Fleet Subnet Root, and Wasm
  Store infrastructure artifacts
- build one topology-qualified application artifact union and project an exact
  admitted release set for every planned root
- journal, create, install, and independently verify the Coordinator first
- journal, create, install, and independently verify every planned Fleet
  Subnet Root
- stage each root's exact release set, bootstrap one root-local Store, and
  verify its live catalog independently
- register and independently verify every root as Registry `Joining`
- stage and independently verify the exact all-`Joining` snapshot and
  Coordinator acknowledgement at every root
- atomically commit and independently verify the complete Coordinator
  Registry as all-`Active`
- atomically activate and independently verify every root's exact all-`Active`
  Registry Mirror and Registry-derived Fleet Directory
- keep every root runtime-`Prepared` and stop before Component creation,
  runtime activation, or terminal Fleet-catalog publication

The local driver permits one clean local `icp` restart attempt when
`icp ping local` fails. Exact journals own same-release interruption recovery;
the host does not fall back to the removed single-root installer.

Build profile selection:

- `canic build <app> <role> --profile debug` builds raw debug wasm
- `canic build <app> <role> --profile fast` builds the middle shrunk local/test/demo lane
- `canic build <app> <role> --profile release` builds the shipping/install lane

If omitted, CLI builds default to `release`.

When the Rust workspace root and ICP CLI/project root differ, pass
`--workspace`, `--icp-root`, and `--config` to `canic build`. The low-level
`build_artifact` example takes those three paths after its role and profile.

If canister crates live outside the default `apps/` directory, host
discovery first tries Cargo workspace metadata. Every Canic-managed canister
package must declare the App-scoped role it implements in Cargo metadata:

```toml
[package.metadata.canic]
app = "project"
role = "project_ledger"
```

For `canic install`, the implicit environment default is always `local`; use
`--environment <name>` for one command against another environment. The public
CLI requires `canic install <app> <fleet> --fleet-input <path>`, uses
`apps/<app>/canic.toml` for reusable App topology, and reads concrete
placement, admission, limit, and funding policy only from the separate Fleet
input.

Canonical network identity is trust-derived rather than environment-derived.
IC mainnet profiles resolve from Canic's compiled DER root key. Pre-existing
local and connected profiles resolve only through the exact root key and
enrollment record under `.canic/networks/<canonical-network-id>/`; the
environment profile is revalidated as a non-authoritative lookup on every
resolution.
