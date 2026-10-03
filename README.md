<p align="center">
  <img src="assets/canic-logo-hero.jpg" alt="Canic — user-friendly multi-canister management" width="800" />
</p>

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

The IC runs software in **canisters**: programs that contain both code and data.
Building one canister is straightforward. Once an application has several
canisters, you also need to decide how they are built, installed, connected,
funded, placed, observed, recovered, and safely changed together. Canic provides
one model and one toolchain for that work.

## Think Kubernetes, But For IC Canisters

<img src="assets/600x600/mechanic-tip.png" align="right" width="150" alt="The Canic mechanic presenting a tip" />

Kubernetes gives teams a consistent way to describe and operate applications
made from multiple containers. Canic plays a similar role for applications made
from multiple IC canisters.

<br clear="right" />

| Kubernetes concept | Canic concept |
| --- | --- |
| Container or Pod | Canister or Component |
| Deployment manifests | `canic.toml` plus a desired Fleet file |
| Container image | Versioned Wasm artifact with build evidence |
| Cluster control plane | Fleet Coordinator, Subnet Roots, and Wasm Stores |
| Scheduling and replica limits | Subnet placement, Groups, pools, and growth limits |
| Reconciliation | Reviewed `canic fleet ensure` plan and apply workflow |

Canic is not a Kubernetes port. Canisters combine code and persistent state,
run on a network you do not administer, and pay for computation with cycles.
Canic therefore emphasizes exact authority, bounded spending, reviewed changes,
and safe recovery after interrupted or uncertain IC calls.

## How Canic Works

```text
 Rust canister code            App configuration
                              (canic.toml)
          \                         /
           +------ canic build ----+
                       |
                       v
             Wasm artifacts + evidence
                       |
 Desired Fleet --------+-------- live IC observations
                       |
                       v
              plan -> review -> apply
                       |
                       v
                 Fleet on the IC
                       |
              Fleet Coordinator
                       |
          Root on each occupied Subnet
                 /             \
          Wasm Store     App Components
```

1. You write ordinary Rust canisters and describe their roles and allowed
   relationships in `canic.toml`.
2. `canic build` produces their Wasm files and evidence identifying exactly
   what was built.
3. A desired Fleet file selects the network, concrete deployment, funding, and
   placement limits for one running copy of the App.
4. `canic fleet ensure` compares that intent with the live IC and produces a
   plan without making paid changes.
5. The operator reviews the plan and applies its exact digest. Canic's
   management canisters then perform the approved work and retain the evidence
   needed for retry and recovery.

The Rust runtime can also be used by a single canister for lifecycle,
authentication, timers, calls, persistent-memory helpers, and monitoring. The
main payoff comes when several canisters must behave as one application.

## Start Here

<img src="assets/600x600/mechanic-help.png" align="right" width="125" alt="The Canic mechanic offering help" />

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

<br clear="right" />

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

Verify existing backup snapshots and restore the canisters in an application.
These tasks run from the operator's computer, so application canisters do not
receive access to local files or credentials. Creating fresh backups is
currently unavailable while its live topology safety check remains incomplete.

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

<img src="assets/600x600/mechanic-caution.png" align="right" width="130" alt="The Canic mechanic holding a caution sign" />

Canic is still pre-1.0, so releases may make breaking changes. Moving an
existing deployment to a new Canic release currently requires a clean
reinstallation rather than an in-place upgrade. Application data, canister
identities, and network layout are not guaranteed to carry across that
boundary. Canic must still account for **cycles**, the IC's units for paying
for computation, so they are not silently lost. Retry, backup, and recovery
within the same release remain supported.

<br clear="right" />

Read the [current implementation status](docs/status/current.md) for the exact
completed boundary rather than relying on a version-specific summary in this
landing page.

The repository is being opened for wider use; issues and pull requests are
currently limited to the core team.

## License

MIT. See [LICENSE](LICENSE).
