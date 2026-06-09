<p align="center">
  <img src="assets/canic_logo.svg" alt="Canic logo" width="360" />
</p>

# Canic - Internet Computer Orchestration

[![Crates.io](https://img.shields.io/crates/v/canic.svg)](https://crates.io/crates/canic)
[![Docs.rs](https://docs.rs/canic/badge.svg)](https://docs.rs/canic)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.91.0-blue.svg)](Cargo.toml)
[![Internal Rust](https://img.shields.io/badge/internal%20rust-1.96.0-orange.svg)](rust-toolchain.toml)

Canic is a Rust toolkit and operator CLI for Internet Computer canister fleets.
It gives canister crates metadata-driven lifecycle macros, validated topology
config, stable-memory helpers, endpoint guards, thin-root artifact builds,
local fleet install, snapshot, backup, and restore workflows.

Install the published operator binary:

```bash
cargo install --locked canic-cli --version <same-version-as-canic>
canic --version
```

When working from this checkout:

```bash
make install
```

See [INSTALLING.md](INSTALLING.md) for the complete setup guide, local replica
notes, fleet management flow, path dependency setup, and backup/restore operator
walkthrough.

## Highlights

* **Lifecycle and build macros:** `canic::start!()` and `canic::build!` wire IC
  hooks, endpoint bundles, and compile-time config validation from
  `[package.metadata.canic] fleet = "..."` and `role = "..."`.
* **Role lifecycle:** ordinary managed canisters can be declared before
  topology placement, then explicitly attached before artifact builds or
  deployment truth.
* **Topology-aware config:** [CONFIG.md](CONFIG.md) covers `canic.toml`
  subnets, roles, singleton/replica/shard/instance placement, warm pools,
  scaling pools, sharding pools, and directory pools.
* **Delegated auth:** Root signs shard certificates, shards mint user tokens,
  and verifiers validate token plus embedded proof with local root/shard key
  material. See
  [AUTH_DELEGATED_SIGNATURES.md](docs/contracts/AUTH_DELEGATED_SIGNATURES.md).
* **Thin-root install flow:** The CLI stages ordinary child artifacts through
  the implicit `wasm_store` and keeps child artifacts out of the root Wasm. See
  [build-artifacts.md](docs/architecture/build-artifacts.md).
* **Passive adoption reports:** Existing and partial deployments can be
  inspected with read-only adoption profiles. Reports classify configured and
  observed resources, but recommendations are non-executed previews. See
  [adoption-profiles.md](docs/architecture/adoption-profiles.md).
* **Evidence and catalog reports:** Build provenance, deployment-check
  envelopes, policy gates, and the passive deployment catalog give operators a
  compact evidence flow without adding install, controller, registry, topology,
  or teardown authority. See
  [v1-readiness-checklist.md](docs/architecture/v1-readiness-checklist.md) and
  [v1-operator-walkthrough.md](docs/architecture/v1-operator-walkthrough.md).
* **NNS inspection:** The operator CLI can refresh and inspect cached public
  IC subnet, node, node-operator, and node-provider metadata from the NNS
  registry without mutating deployments or canisters.
* **Operator workflows:** The `canic` binary builds artifacts, manages local
  fleet configs and replica status, installs fleets, captures topology-aware
  snapshots, validates backup manifests, and drives guarded restore planning.

## Quick Start

For a copyable root-plus-two-children managed fleet, start with
[minimal-managed-fleet.md](docs/getting-started/minimal-managed-fleet.md).
For the compact setup checklist, use [INSTALLING.md](INSTALLING.md).

The short local loop from this checkout, using the checked-in `test` fleet, is:

```bash
canic status
canic replica start --background
canic install --profile fast test
canic info list test
canic info env test
canic info medic test
```

To inspect public IC NNS metadata:

```bash
canic nns subnet refresh
canic nns registry version
canic nns subnet list
canic nns data-center list
canic nns node list
canic nns node list --data-center <data-center-prefix>
canic nns node list --node-provider <node-provider-prefix>
canic nns node-provider list
canic nns node-operator list
canic nns topology refresh
canic nns topology summary
canic nns subnet info <subnet|canister|subnet-prefix|deployment-target>
```

Useful next reads:

* [INSTALLING.md](INSTALLING.md) - end-to-end installation and local operation.
* [docs/getting-started/local-academic-fleet.md](docs/getting-started/local-academic-fleet.md)
  - local named-target runbook for Canic, raw `icp`, sharding, metrics, and
  install/upgrade traps.
* [docs/architecture/v1-operator-walkthrough.md](docs/architecture/v1-operator-walkthrough.md)
  - compact pre-v1 build, evidence, policy, and catalog flow.
* [docs/architecture/v1-readiness-checklist.md](docs/architecture/v1-readiness-checklist.md)
  - compact v1-candidate commands, files, evidence outputs, and boundaries.
* [crates/canic-cli/README.md](crates/canic-cli/README.md) - operator command
  guide, including backup and restore.
* [crates/canic-host/README.md](crates/canic-host/README.md) - build profiles,
  split workspace/ICP roots, custom canister roots, and lower-level install
  commands.
* [TESTING.md](TESTING.md) - canister placement and test expectations.

## Repository Layout

The workspace keeps Rust crates under [crates/](crates/) and fleet fixtures under
[fleets/](fleets/). Detailed ownership and layering rules live in
[AGENTS.md](AGENTS.md).

* [crates/canic/](crates/canic/) - public facade crate, lifecycle/build macros,
  endpoint bundles, and protocol constants.
* [crates/canic-core/](crates/canic-core/) - shared canister runtime foundation:
  config, lifecycle, ingress limits, auth, storage, workflow, DTOs, and IDs.
* [crates/canic-macros/](crates/canic-macros/) - proc macros behind the public
  facade.
* [crates/canic-control-plane/](crates/canic-control-plane/) - root/control-plane
  runtime support built on `canic-core`.
* [crates/canic-wasm-store/](crates/canic-wasm-store/) - canonical implicit
  bootstrap `wasm_store` canister crate.
* [crates/canic-cli/](crates/canic-cli/) - published `canic` operator binary.
* [crates/canic-host/](crates/canic-host/) - host-side build, install, fleet,
  and thin-root staging library.
* [crates/canic-backup/](crates/canic-backup/) - backup/restore domain library.
* [crates/canic-testing-internal/](crates/canic-testing-internal/) and
  [crates/canic-tests/](crates/canic-tests/) - repo-only PocketIC harnesses and
  integration tests.
* [fleets/test/](fleets/test/) and [fleets/demo/](fleets/demo/) - local reference
  fleet configs.
* [canisters/](canisters/) - runnable canisters that are not Canic fleets.
* [scripts/](scripts/) - dev setup, CI, release, Wasm, and audit helpers.
* [docs/](docs/) and [.github/workflows/](.github/workflows/) - design notes,
  operational docs, audits, and CI.

## Architecture And Contracts

Canic follows the layering rules in [AGENTS.md](AGENTS.md): endpoints
authenticate and delegate, workflow orchestrates, policy decides, ops performs
approved state or platform actions, and model/storage own invariants.

Reference docs:

* [docs/architecture/README.md](docs/architecture/README.md)
* [docs/architecture/build-artifacts.md](docs/architecture/build-artifacts.md)
* [docs/architecture/authentication.md](docs/architecture/authentication.md)
* [docs/contracts/AUTH_DELEGATED_SIGNATURES.md](docs/contracts/AUTH_DELEGATED_SIGNATURES.md)
* [docs/contracts/ACCESS_ARCHITECTURE.md](docs/contracts/ACCESS_ARCHITECTURE.md)

## Development

Common local checks:

```bash
cargo fmt --all
make fmt-check
make check
make clippy
make test
```

[rust-toolchain.toml](rust-toolchain.toml) pins the internal toolchain so CI and
local builds stay in sync. Published crates declare MSRV `1.91.0` in
[Cargo.toml](Cargo.toml).

Follow [docs/governance/ci-deployment.md](docs/governance/ci-deployment.md) for
CI, git, deployment, and automation rules. Follow
[docs/governance/changelog.md](docs/governance/changelog.md) and
[CHANGELOG.md](CHANGELOG.md) for changelog policy.

## Project Status & Contributing

Canic is the successor to the internal ICU toolkit. The repository is in the
process of being opened for wider use; issues and PRs are currently limited to
the core team. Follow [AGENTS.md](AGENTS.md), [CONFIG.md](CONFIG.md), and
[scripts/ci/](scripts/ci/) for workflow expectations.

## License

MIT. See [LICENSE](LICENSE) for details.
