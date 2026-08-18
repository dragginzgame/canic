<p align="center">
  <img src="assets/canic_logo.svg" alt="Canic logo" width="360" />
</p>

# Canic

[![Crates.io](https://img.shields.io/crates/v/canic.svg)](https://crates.io/crates/canic)
[![Docs.rs](https://docs.rs/canic/badge.svg)](https://docs.rs/canic)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.91.0-blue.svg)](Cargo.toml)
[![Internal Rust](https://img.shields.io/badge/internal%20rust-1.97.1-orange.svg)](rust-toolchain.toml)

Canic is a Rust toolkit and operator CLI for building and running Internet
Computer canister fleets. Its capabilities are deliberately separable: use the
runtime facade without Fleet installation, add authentication without scaling,
or use the host-side backup tools without giving application canisters access
to files, credentials, or operator authority.

## Start Here

Install the published operator CLI at the same version as the `canic` crate
used by your canisters:

```bash
cargo install --locked canic-cli --version <version>
canic --version
```

For a checkout of this repository:

```bash
make install
```

Then choose the path that matches what you are doing:

- **Build a first managed canister:**
  [Minimal managed Fleet](docs/getting-started/minimal-managed-fleet.md)
- **Install and operate Canic:** [Installing Canic](INSTALLING.md)
- **Configure roles and topology:** [Canic configuration](CONFIG.md)
- **Work on Canic itself:** [Contributor rules](AGENTS.md) and
  [testing guide](TESTING.md)

Canic uses the installed `icp` binary for replica, canister, snapshot, and
restore operations. Supported versions and upgrade guidance are maintained in
[INSTALLING.md](INSTALLING.md#icp-cli-compatibility).

[rust-toolchain.toml](rust-toolchain.toml) pins internal Rust `1.97.1`;
published crates declare MSRV `1.91.0` in [Cargo.toml](Cargo.toml).

## Features

Each feature has a short guide of its own. The guides explain the capability
and its authority boundary, then point to the detailed contracts and runbooks.

### Canister Runtime

Lifecycle and build macros, stable-memory helpers, timers, typed calls,
metrics, and configuration-derived runtime context for Rust canisters.

[Explore the canister runtime](docs/features/runtime/README.md)

### Authentication

Endpoint guards, delegated subject tokens, root-managed chain-key proof
renewal, issuer proofs, role attestation, and explicit caller/subject binding.

[Explore authentication](docs/features/authentication/README.md)

### Fleet Orchestration

Coordinator-backed Fleet installation, one root and Wasm Store per occupied
Subnet, qualified artifacts, registries, directories, and root-owned platform
effects.

[Explore Fleet orchestration](docs/features/fleet-orchestration/README.md)

### Scaling And Placement

Reusable Component Specs and Groups, bounded placement, service roles,
dynamic child trees, sharding pools, scaling pools, and reduction-only limits.

[Explore scaling and placement](docs/features/scaling-and-placement/README.md)

### Builds, Provenance, And Evidence

Role-aware Wasm builds, build provenance, passive deployment evidence,
policy gates, adoption reports, and network-scoped Fleet catalogs.

[Explore builds and evidence](docs/features/build-and-evidence/README.md)

### Backup And Restore

Topology-aware canister snapshots, manifests, checksums, resumable download
journals, verified restore plans, and guarded execution from the host CLI.

[Explore backup and restore](docs/features/backup-and-restore/README.md)

### Blob Storage

Optional runtime APIs and operator tooling for product blob storage, with the
non-billing integration kept separate from Cashier-backed billing support.

[Explore blob storage](docs/features/blob-storage/README.md)

### Operations And Diagnostics

App and Fleet setup, network trust, local replicas, installation, status,
inspection, medic checks, recovery guidance, and automation-friendly output.

[Explore operations and diagnostics](docs/features/operations/README.md)

The complete feature index is in [docs/features](docs/features/README.md).

## Demo Fleets

**[Prequel Wars](https://github.com/dragginzgame/prequel-wars)** is the external
stateful demonstration and downstream proving ground for Canic. It maps game
planets to application Subnets and exercises managed IcyDB Components, direct
scoped ingress, retirement evidence, reusable estates and a bounded Galactic
War Room Fleet overview.

The game stays in its own repository. Canic owns only generic infrastructure,
lifecycle, authorization, retirement and observatory contracts; it does not
vendor game code or make canonical infrastructure depend on the demo.

## Core Vocabulary

- An **App** is checked-in source and configuration.
- A **Fleet** is one installed instance of an App on one network.
- A **workspace** is the local checkout containing configuration and operator
  state; it is not a deployment identity.
- A **Component Spec** is a reusable blueprint. A concrete Component is one
  deployed occurrence with its own identity, root, state, and limits.
- A **Fleet Subnet Root** owns lifecycle effects for Components on its physical
  Subnet. The Fleet Coordinator owns Fleet-wide planning and publication.

See [CONFIG.md](CONFIG.md) for the complete configuration vocabulary and
[fleet-install-input.md](docs/architecture/fleet-install-input.md) for the
separate operator-owned installation input.

## Repository Map

- [crates/canic](crates/canic/) — public canister facade
- [crates/canic-core](crates/canic-core/) — shared runtime, models, policy, and
  protocols
- [crates/canic-control-plane](crates/canic-control-plane/) — root,
  Coordinator, and Store runtime support
- [crates/canic-cli](crates/canic-cli/) — published `canic` operator binary
- [crates/canic-host](crates/canic-host/) — host-side build, install, and Fleet
  orchestration
- [crates/canic-backup](crates/canic-backup/) — backup and restore domain
  contracts
- [apps](apps/) — reference App configurations and canister packages
- [docs](docs/) — architecture, contracts, operations, designs, and audits

Detailed ownership and dependency rules live in [AGENTS.md](AGENTS.md).

## Status

Canic is pre-1.0. Release transitions are reinstall-only unless an explicit
design says otherwise; same-release retry, backup, and recovery remain durable
operational contracts. Read the
[current implementation status](docs/status/current.md) for the exact completed
boundary rather than relying on a version-specific summary in this landing
page.

The repository is being opened for wider use; issues and pull requests are
currently limited to the core team.

## License

MIT. See [LICENSE](LICENSE).
