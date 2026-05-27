# Installing Canic

This guide covers the normal operator setup and the first local fleet install.
The short version is:

```bash
cargo install --locked canic-cli --version <same-version-as-canic>
canic --version
```

When working from this checkout:

```bash
make install
```

For the full maintainer toolchain, including ICP CLI, wasm/Candid tools, and
repo helper binaries:

```bash
make install-dev
```

## Install The Operator CLI

Install the published operator binary with Cargo:

```bash
cargo install --locked canic-cli
canic --version
```

Pinned downstream projects should install the same `canic-cli` version as their
`canic` crate dependency:

```bash
cargo install --locked canic-cli --version <same-version-as-canic>
```

From a checkout, install the local CLI:

```bash
make install
```

The installed binary is named `canic`.

## Add Canic To Canister Crates

Inside each canister crate that uses Canic:

```bash
cargo add canic
cargo add candid ic-cdk
cargo add canic --build
```

`canic` is needed in `[dependencies]` for runtime macros and
`[build-dependencies]` for `build.rs`. The `candid` and `ic-cdk` dependencies
must also be visible to the canister crate because CDK attributes and Candid
export expand against those crate names.

For a path checkout:

```toml
[dependencies]
candid = { version = "<version>", default-features = false }
canic = { path = "/path/to/canic/crates/canic" }
ic-cdk = "<version>"

[build-dependencies]
canic = { path = "/path/to/canic/crates/canic" }
```

## Configure `build.rs`

Root canister:

```rust
fn main() {
    canic::build_root!("../canic.toml");
}
```

Child canister:

```rust
fn main() {
    canic::build!("../canic.toml");
}
```

Standalone probe:

```rust
fn main() {
    canic::build_standalone!("sandbox_minimal");
}
```

## Bootstrap A Canister

In `lib.rs`:

```rust
use canic::ids::CanisterRole;
use canic::prelude::*;

const APP: CanisterRole = CanisterRole::new("app");

canic::start!(APP); // use canic::start_root!() for root

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

canic::finish!();
```

Root canisters use `canic::start_root!()` and root hook signatures:

```rust
canic::start_root!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

canic::finish!();
```

Use `#[canic_query]` and `#[canic_update]` for Canic-managed application
methods so endpoint dispatch, metrics, access checks, Candid export, and
payload inspection stay on the same path as the runtime bundle.

## Define A Fleet

Create `fleets/<fleet>/canic.toml`:

```toml
[fleet]
name = "test"

[subnets.prime]
auto_create = ["app"]
subnet_index = ["app"]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
topup = {}
```

The full schema lives in [`CONFIG.md`](CONFIG.md).

For a complete root-plus-two-children example, see
[`docs/getting-started/minimal-managed-fleet.md`](docs/getting-started/minimal-managed-fleet.md).

## Build And Install Locally

Check that `icp.yaml` contains the matching project config, start the local ICP
CLI replica, install the fleet, then query the deployed root registry:

```bash
canic status
canic replica start --background
canic install --profile fast test
canic info list test
```

Build one artifact without installing:

```bash
canic build --profile fast app
```

For downstream repos where the Rust workspace and ICP project root differ, pass
both paths explicitly:

```bash
canic --network local build \
  --profile fast \
  --workspace /path/to/cargo-workspace \
  --icp-root /path/to/icp-project \
  --config /path/to/cargo-workspace/fleets/<fleet>/canic.toml \
  root
```

When passing `--config` explicitly, prefer an absolute path. This keeps path
dependencies and build scripts from interpreting a relative config path from
their own crate directories.

For build profiles, split workspace/ICP roots, custom canister roots, role
metadata, and lower-level build/install commands, see
[`crates/canic-host/README.md`](crates/canic-host/README.md).

## Fleet Management

Use `canic fleet list` to list config-defined fleets. Use
`canic config <fleet>` for declared config, and pass `<fleet>` as the first
argument to deployed-fleet commands.

```bash
canic config test
canic info list test
canic status
canic --network local fleet list
canic fleet create demo --yes
canic fleet delete demo
```

Use `canic medic` when local project state, replica ownership, or a named fleet
does not look right:

```bash
canic medic test
```

Named-fleet commands default to the local ICP CLI environment. Pass top-level
`--network <name>` for one command against another configured ICP CLI
environment. Nonlocal targets must be managed externally.

The local ICP CLI replica does not persist canister state across stop/start. If
`canic status` shows a local fleet as `lost`, the recorded root canister is
gone from the restarted local replica; run `canic install <fleet>` to recreate
the local deployment.

Fleet configs live under project-root `fleets/`. Commands launched from nested
directories discover the outer project root and keep ICP project config plus
`.icp/` and `.canic/` state there.

## Backup And Restore

Show installed canisters:

```bash
canic --network local info list test
canic --network local info list test --subtree app
```

Create and verify a topology-aware backup:

```bash
canic backup create test
canic backup list
canic backup verify 1
```

Restore work is backup-row and journal driven. `restore prepare 1` writes the
default plan and apply journal inside the backup layout, `restore status 1`
checks progress and gates, and `restore run 1 --execute` advances the durable
journal through upload, stop, snapshot load, start, and verification
operations.

```bash
canic restore prepare 1 --require-verified --require-restore-ready
canic restore status 1 --require-no-attention
canic restore run 1 --execute --max-steps 1 --require-no-attention
canic restore status 1 --require-complete --require-no-attention
```

See [`crates/canic-cli/README.md`](crates/canic-cli/README.md) for the fuller
operator guide.

## Generated State

`root` embeds only the bootstrap `wasm_store.wasm.gz`; ordinary child releases
stay outside `root` and are staged after install. Visible canister Candid files
are generated under `.icp/local/canisters/<role>/<role>.did`. The checked-in
exception is `crates/canic-wasm-store/wasm_store.did`, the canonical interface
for the implicit bootstrap `wasm_store` crate.

Canic-managed Candid includes both application methods and Canic runtime
methods such as readiness, metadata, topology, and management endpoints. When
migrating a non-Canic canister, compare the application surface separately from
Canic-owned methods.

## First Install Troubleshooting

- If `canic.toml` uses `[[canisters]]`, rewrite it under
  `[subnets.<name>.canisters.<role>]`; Canic validates the subnet-shaped schema.
- If a lifecycle macro reports
  `__canic_missing_finish_macro__add_canic_finish_at_end_after_all_endpoints`,
  add `canic::finish!()` at the end of the canister crate root after custom
  endpoint definitions.
- If a child cannot find its config at build time, check the path passed to
  `canic::build!`; it is relative to the canister crate directory.
- If the root canister does not compile or bootstrap delegated-auth material,
  confirm the runtime dependency enables the `auth-crypto` and `control-plane`
  features.
- If host discovery cannot map a crate to a role, use a package name like
  `canister_hub` or declare `[package.metadata.canic] role = "hub"`.
- If `canic info list <fleet>` only shows `root`, the managed children were not
  fully installed or the local replica lost state. Run `canic medic <fleet>` and
  reinstall the local fleet.
- If a test manually installs root and child canisters, it is not validating the
  managed fleet path. A managed-fleet test should let root create/register
  children and then resolve them from `canic_subnet_registry`.
