<p align="center">
  <img src="assets/canic_logo.svg" alt="Canic logo" width="360" />
</p>

# Canic

[![Crates.io](https://img.shields.io/crates/v/canic.svg)](https://crates.io/crates/canic)
[![Docs.rs](https://docs.rs/canic/badge.svg)](https://docs.rs/canic)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.91.0-blue.svg)](Cargo.toml)
[![Internal Rust](https://img.shields.io/badge/internal%20rust-1.99.0-orange.svg)](rust-toolchain.toml)

<p align="center">
  <img src="assets/canic-hero.jpg" alt="The Canic mechanic holding a canister beside a network of connected canisters" width="800" />
</p>

Canic helps Rust developers build and operate applications on the Internet
Computer (IC). The IC runs applications in **canisters**: programs that contain
both code and data, much like backend services that run directly on the
network.

Canic provides a Rust library that runs inside those canisters and a
command-line tool for managing them from your computer.

Use Canic when you want to:

- handle startup, persistent data, scheduled tasks, communication between
  canisters, and health information;
- control who can use an application's methods;
- build and deploy an application made up of one or many canisters; or
- manage growth, backups, restores, and troubleshooting from one tool.

You can use only the parts you need. A single canister can use Canic's Rust
helpers on their own. A larger application can also use Canic to coordinate
many canisters running across different parts of the IC network.

## Start Here

<p align="center">
  <img src="assets/1400x600/canic-start-here.jpg" alt="The Canic mechanic consulting a map beside signs for building, configuring, and operating an application" width="700" />
</p>

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

| Goal | Start here |
| --- | --- |
| Build and deploy a small Canic application | [First managed application](docs/getting-started/minimal-managed-fleet.md) |
| Install the command-line tools and prerequisites | [Installing Canic](INSTALLING.md) |
| Describe an application's canisters and layout | [Canic configuration](CONFIG.md) |
| Learn what each part of Canic does | [Feature guides](docs/features/README.md) |
| Contribute to Canic | [Contributor rules](AGENTS.md) and [testing guide](TESTING.md) |

Some Canic commands use the `icp` command-line program behind the scenes. It
starts a local IC network for development and performs low-level canister,
backup snapshot, and restore operations. Supported versions and upgrade
guidance are maintained in [INSTALLING.md](INSTALLING.md#icp-cli-compatibility).

[rust-toolchain.toml](rust-toolchain.toml) pins internal Rust `1.99.0`;
published crates declare MSRV `1.91.0` in [Cargo.toml](Cargo.toml).

## Features

Canic is a collection of features rather than an all-or-nothing framework.
Each feature has a short guide that explains what it does, what access it
needs, and where to find the detailed reference material.

### Canister Runtime

Reusable Rust tools that run inside a canister. They handle startup and state
restoration, data that must survive restarts, scheduled tasks, safe calls to
other canisters, health information, and application configuration.

[Explore the canister runtime](docs/features/runtime/README.md)

### Authentication

Controls for deciding who may call an application's public methods and what
they may do. Canic can also verify when one service is acting for a user or
another service without confusing their identities.

[Explore authentication](docs/features/authentication/README.md)

### Deploying An Application

A running copy of a Canic application is called a **Fleet**. Canic can build
the application's canisters, check that the intended code is being used, place
them on the IC network, and coordinate their installation.

[Explore Fleet orchestration](docs/features/fleet-orchestration/README.md)

### Growing An Application

Describe reusable kinds of canisters, how many are allowed to run, and where
they may run. Larger applications can divide work or data among more canisters
and add capacity within limits set by the operator.

[Explore scaling and placement](docs/features/scaling-and-placement/README.md)

### Verifiable Builds

Build canister programs in WebAssembly (Wasm), record exactly how they were
built, and compare that evidence before deployment. This helps operators check
that the code they intend to run is the code that was produced.

[Explore builds and evidence](docs/features/build-and-evidence/README.md)

### Backup And Restore

Create and restore snapshots of the canisters in an application. These tasks
run from the operator's computer, so application canisters do not receive
access to local files or credentials.

[Explore backup and restore](docs/features/backup-and-restore/README.md)

### File Storage

Optional APIs let an application store large pieces of data, such as uploaded
files, without every application implementing that machinery itself. Storage
can be used with or without a separate billing integration.

[Explore blob storage](docs/features/blob-storage/README.md)

### Developer And Operator Tools

Set up an application, run a local IC network, build canisters, inspect their
state, troubleshoot problems, and safely bring a deployed application to its
declared configuration.

[Explore operations and diagnostics](docs/features/operations/README.md)

Browse the [documentation index](docs/README.md) for guides, contracts and
operator runbooks.

## Reference Apps

- [Demo App](apps/demo/canic.toml) — a small example that shows how an
  application can divide work among several canisters.
- [Test App](apps/test/canic.toml) — a larger example used to test lookup,
  data distribution, growth, and network placement.

See the [App guide](apps/README.md) for their source packages, build commands,
and local workflow. Canic generates the internal management canisters needed
by the selected configuration.

## Core Vocabulary

- An **App** is the source code and configuration stored in a local checkout.
- A **Fleet** is one running copy of an App on one IC network.
- A **workspace** is the local folder containing the App and Canic's operator
  files. It does not identify a deployed application.
- A **Component Spec** is a reusable blueprint for one kind of application
  canister. A **Component** is one deployed canister created from that
  blueprint, with its own identity, data, and limits.
- A **Component Group** collects related blueprints. Each deployment of a Group
  has its own size and rules about where its canisters may run.
- A **Fleet service** is a stable name for a selected set of deployed
  Components. Application code can target the service without choosing a
  particular canister itself.
- A **Subnet** is one part of the IC network. A **Fleet Subnet Root** is a
  Canic management canister that performs approved actions for the Fleet's
  Components on that Subnet.
- The **Fleet Coordinator** is the management canister that coordinates plans
  and shared information for the whole Fleet.

See [CONFIG.md](CONFIG.md) for the App vocabulary and
[Fleet ensure](docs/features/operations/fleet-ensure.md) for the separate
operator-owned desired Fleet contract.

## Repository Map

- [crates/canic](crates/canic/) — the public Rust library used by application
  canisters
- [crates/canic-core](crates/canic-core/) — shared internal behavior and data
  types
- [crates/canic-control-plane](crates/canic-control-plane/) — implementation
  of Canic's management canisters
- [crates/canic-cli](crates/canic-cli/) — the published `canic` command-line
  program
- [crates/canic-host](crates/canic-host/) — building and deployment behavior
  that runs on the operator's computer
- [crates/canic-backup](crates/canic-backup/) — backup and restore behavior
- [crates/canic-testing-internal](crates/canic-testing-internal/) — internal
  test support and deployment scenarios
- [crates/canic-tests](crates/canic-tests/) — runtime and integration tests
- [apps](apps/) — reference App configurations and canister packages
- [docs](docs/) — architecture, contracts, operations, designs, and audits

Detailed ownership and dependency rules live in [AGENTS.md](AGENTS.md).

## Status

<p align="center">
  <img src="assets/1400x600/canic-pre-1-0.jpg" alt="The Canic mechanic working beside a partially assembled rack of canisters" width="700" />
</p>

Canic is still pre-1.0, so releases may make breaking changes. Moving an
existing deployment to a new Canic release currently requires a clean
reinstallation rather than an in-place upgrade. Application data, canister
identities, and network layout are not guaranteed to carry across that
boundary. Canic must still account for **cycles**, the IC's units for paying
for computation, so they are not silently lost. Retry, backup, and recovery
within the same release remain supported.

Read the [current implementation status](docs/status/current.md) for the exact
completed boundary rather than relying on a version-specific summary in this
landing page.

The repository is being opened for wider use; issues and pull requests are
currently limited to the core team.

## License

MIT. See [LICENSE](LICENSE).
