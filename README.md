<p align="center">
  <img src="https://raw.githubusercontent.com/dragginzgame/shared-assets/main/canic/canic-readme-header.svg" alt="Canic — user-friendly multi-canister management" width="100%">
</p>

<!-- helper-navigation:start -->
<p align="center">
  <a href="https://github.com/dragginzgame/canic"><img src="https://raw.githubusercontent.com/dragginzgame/shared-assets/main/icons/canic.svg" width="18" height="18" alt=""> <strong>canic</strong></a>
  &nbsp;&middot;&nbsp;
  <a href="https://github.com/dragginzgame/icydb"><img src="https://raw.githubusercontent.com/dragginzgame/shared-assets/main/icons/icydb.svg" width="18" height="18" alt=""> <strong>icydb</strong></a>
  &nbsp;&middot;&nbsp;
  <a href="https://github.com/dragginzgame/ic-timers"><img src="https://raw.githubusercontent.com/dragginzgame/shared-assets/main/icons/ic-timers.svg" width="18" height="18" alt=""> <strong>ic-timers</strong></a>
  &nbsp;&middot;&nbsp;
  <a href="https://github.com/dragginzgame/ic-memory"><img src="https://raw.githubusercontent.com/dragginzgame/shared-assets/main/icons/ic-memory.svg" width="18" height="18" alt=""> <strong>ic-memory</strong></a>
  &nbsp;&middot;&nbsp;
  <a href="https://github.com/dragginzgame/ic-query"><img src="https://raw.githubusercontent.com/dragginzgame/shared-assets/main/icons/ic-query.svg" width="18" height="18" alt=""> <strong>ic-query</strong></a>
  &nbsp;&middot;&nbsp;
  <a href="https://github.com/dragginzgame/ic-backup"><img src="https://raw.githubusercontent.com/dragginzgame/shared-assets/main/icons/ic-backup.svg" width="18" height="18" alt=""> <strong>ic-backup</strong></a>
  &nbsp;&middot;&nbsp;
  <a href="https://github.com/dragginzgame/ic-blob-storage"><img src="https://raw.githubusercontent.com/dragginzgame/shared-assets/main/icons/ic-blob-storage.svg" width="18" height="18" alt=""> <strong>ic-blob-storage</strong></a>
  &nbsp;&middot;&nbsp;
  <a href="https://github.com/dragginzgame/ic-testkit"><img src="https://raw.githubusercontent.com/dragginzgame/shared-assets/main/icons/ic-testkit.svg" width="18" height="18" alt=""> <strong>ic-testkit</strong></a>
</p>
<!-- helper-navigation:end -->

# Canic

[![Crates.io](https://img.shields.io/crates/v/canic.svg)](https://crates.io/crates/canic)
[![Docs.rs](https://docs.rs/canic/badge.svg)](https://docs.rs/canic)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.91.0-blue.svg)](Cargo.toml)
[![Internal Rust](https://img.shields.io/badge/internal%20rust-1.99.0-orange.svg)](rust-toolchain.toml)

Canic is an application framework and orchestration system for Rust applications
on the Internet Computer (IC).

> **If your IC application uses more than one canister, you should be using
> Canic.**

The IC runs software in **canisters**, which contain both code and data. Once an
application has several canisters, someone must build, connect, fund, place,
observe, recover, and safely change them together. Canic provides one model and
one toolchain for that work.

Think of Canic as playing a Kubernetes-like role for IC applications: you
describe the application, build versioned artifacts, review the intended
changes, and operate the deployed system as a whole. Canic is designed around
IC-specific concerns such as persistent state, cycles, canister authority, and
uncertain network effects.

## How It Works

<p align="center">
  <a href="assets/canic-build-deploy.jpg">
    <img src="assets/canic-build-deploy.jpg" alt="Canic build and deployment workflow from Rust canister code and App configuration through build evidence, desired Fleet planning, review, and apply" width="650" />
  </a>
</p>

The first stage stays under operator control. `canic build` produces the Wasm
and evidence for the exact App configuration. `canic fleet ensure` then combines
those artifacts, the desired Fleet, and live IC observations into a plan without
making paid changes.

Applying the reviewed plan digest is the boundary between local intent and
IC-side effects. After that approval, Canic's management canisters perform only
the work admitted by that plan and retain the evidence needed for safe retry.

<p align="center">
  <a href="assets/canic-ic-fleet.jpg">
    <img src="assets/canic-ic-fleet.jpg" alt="IC Fleet structure with a Fleet Coordinator, one Root and Wasm Store on each occupied Subnet, and application Components" width="650" />
  </a>
</p>

The Coordinator manages the Fleet-wide view. Each occupied Subnet has a Root
that performs approved work for its local application Components and uses its
Wasm Store for qualified installation artifacts.

[Read the complete model and terminology](docs/getting-started/how-canic-works.md).

## Start Here

| Goal | Guide |
| --- | --- |
| Understand Canic's model | [How Canic works](docs/getting-started/how-canic-works.md) |
| Install the tools | [Installing Canic](INSTALLING.md) |
| Build a small managed application | [First managed application](docs/getting-started/minimal-managed-fleet.md) |
| Configure canisters and their layout | [Canic configuration](CONFIG.md) |
| Explore all documentation | [Documentation index](docs/README.md) |

Install the published CLI at the same version as the `canic` Rust crate used by
your canisters:

```bash
cargo install --locked canic-cli --version <version>
```

## Pick The Parts You Need

Canic is not an all-or-nothing framework. Each canister role enables only the
runtime capabilities it needs, and operator tools remain outside the canisters.
You can add authentication without scaling, or use host-side diagnostics and
recovery tools without giving application canisters access to local files or
credentials.
Managed lifecycle and endpoints require their Fleet bindings. The explicit
standalone-local lifecycle supports local development and testing.

| Capability | What it does |
| --- | --- |
| [Runtime](docs/features/runtime/README.md) | Startup, persistent memory, timers, typed calls, and monitoring |
| [Authentication](docs/features/authentication/README.md) | Protect methods and keep caller and user identities separate |
| [Fleet orchestration](docs/features/fleet-orchestration/README.md) | Plan, review, deploy, retry, and recover multi-canister applications |
| [Scaling and placement](docs/features/scaling-and-placement/README.md) | Control how applications grow and where canisters may run |
| [Build evidence](docs/features/build-and-evidence/README.md) | Produce Wasm and record exactly how it was built |
| [Backup and restore](docs/features/backup-and-restore/README.md) | Verify existing backups and perform same-release recovery |
| [Blob storage](docs/features/blob-storage/README.md) | Compose the independent blob service as an application |
| [Operations](docs/features/operations/README.md) | Set up, inspect, diagnose, and operate a Fleet |

Example applications live under [apps](apps/README.md). Contributors should
read [AGENTS.md](AGENTS.md) and [TESTING.md](TESTING.md).

## Status

Canic is pre-1.0. Releases may make breaking changes, and moving between current
releases requires a clean reinstall rather than an in-place upgrade. Same-release
retry and recovery remain supported, and controlled cycles must still be
conserved.

Read the [current implementation status](docs/status/current.md) for the exact
completed boundary. Issues and pull requests are currently limited to the core
team while the repository is prepared for wider use.

## License

MIT. See [LICENSE](LICENSE).
