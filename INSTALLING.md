# Installing Canic

This guide covers the normal operator setup and the smallest managed canister
shape. The short version is:

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

## ICP CLI Compatibility

Canic shells out to the installed `icp` binary for local replica and canister
operations. Canic releases that support the ICP CLI stable line require
`icp-cli >=1.2.0, <2.0.0`; the maintainer toolchain currently pins `1.2.0`.

ICP CLI requires custom connected-network definitions to declare an
explicit `root-key`. Canic's maintained project configuration uses the managed
local network and the built-in `ic` network, so no repository configuration
change is required. Downstream projects with custom connected networks must add
that key to their own ICP configuration.

Check the resolved binary and version:

```bash
which icp
icp --version
```

From a Canic checkout, install the checksum-bound pinned CLI:

```bash
bash scripts/ci/install-icp-cli.sh
```

The installer selects the declared Linux/macOS archive, verifies its
repository-owned SHA-256 identity before extraction, installs `icp` under the
Cargo binary directory, and rejects a mismatched reported version.

`icp network update` updates the local network launcher, such as
`icp-cli-network-launcher`, and does not replace the `icp` CLI binary itself.
If multiple `icp` binaries are installed, put Cargo's bin directory first on
`PATH`, or pass top-level `--icp /path/to/icp` for a single Canic command.

Password-protected ICP CLI PEM identities can cache session delegations so
operators do not re-enter the identity password for every command:

```bash
icp settings session-length 1h
icp identity reauth <identity-name> --duration 1h
```

Use `icp settings session-length disabled` if you need to turn session caching
off. These commands tune the operator's local ICP CLI identity session; they do
not change Canic canister auth or delegated-token behavior.

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

Each canister crate must also declare the Canic role it implements. This is the
single source of truth for both `canic::build!` and `canic::start!()`:

```toml
[package.metadata.canic]
app = "test"
role = "app"
```

Use `role = "root"` for the Fleet Subnet Root package. Ordinary Component and
potential-descendant roles use their configured App role name, such as `app`,
`hub`, or `registry`. The `app` metadata value must match the App source
identity from `[app].name`; it is not a live Fleet name.
Fleet Subnet Root canisters also need the `control-plane` feature on their
runtime `canic` dependency. When delegated-token material is enabled, root
issuers also need
`auth-root-canister-sig-create`; canisters that issue delegated tokens need
`auth-issuer-canister-sig-create`; endpoint verifiers need
`auth-delegated-token-verify`.

For a path checkout:

```toml
[dependencies]
candid = { version = "<version>", default-features = false }
canic = { path = "/path/to/canic/crates/canic" }
ic-cdk = "<version>"

[build-dependencies]
canic = { path = "/path/to/canic/crates/canic" }

[package.metadata.canic]
app = "test"
role = "app"
```

## Configure `build.rs`

Every Canic-managed canister crate has a small `build.rs`:

```rust
fn main() {
    canic::build!("../canic.toml");
}
```

The path is relative to the canister crate directory. A standalone probe with a
crate-local config can use:

```rust
fn main() {
    canic::build!("canic.toml");
}
```

## Minimal Canister Shapes

Every normal managed canister uses `canic::start!()`. Root vs non-root behavior
comes from `[package.metadata.canic] app = "..."` plus `role = "..."` and the
validated App config.

Non-root `lib.rs`:

```rust
#![expect(clippy::unused_async)]

use canic::prelude::*;

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

canic::finish!();
```

Root `lib.rs`:

```rust
#![expect(clippy::unused_async)]

canic::start!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

canic::finish!();
```

`start_local!()` is only for local/dev standalone canisters that synthesize a
minimal local environment. `start_wasm_store!()` is only for the canonical
`wasm_store` runtime.

Add application endpoints after `canic::start!()` and before `canic::finish!()`:

```rust
use canic::{Error, prelude::*};

#[canic_query]
fn health() -> Result<String, Error> {
    Ok("ok".to_string())
}
```

Use `#[canic_query]` and `#[canic_update]` for Canic-managed application
methods so endpoint dispatch, metrics, access checks, Candid export, and
payload inspection stay on the same path as the runtime bundle.

## Define An App

Create `apps/<app>/canic.toml`:

```toml
controllers = []

[app]
name = "test"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[component_specs.app]
component_role = "app"
maximum_instances = 1
topup = {}
```

Every role named in package metadata must exist in this App config.
Declared-only ordinary roles may compile before topology placement, but only
Component roles and potential descendants cataloged under
`[component_specs.*]` can be built as deploy artifacts or enter deployment
truth. `role = "root"` selects the Fleet Subnet Root lifecycle and endpoint
bundle; all other roles select the ordinary Fleet lifecycle and non-root
endpoint bundle.

The full schema lives in [`CONFIG.md`](CONFIG.md).

For a complete Fleet Subnet Root plus two-role Component example, see
[`docs/getting-started/minimal-managed-fleet.md`](docs/getting-started/minimal-managed-fleet.md).
For the compact v1-candidate command and evidence checklist, see
[`docs/architecture/v1-readiness-checklist.md`](docs/architecture/v1-readiness-checklist.md).

## Build And Install Locally

Check that `icp.yaml` contains the matching project config, start the local ICP
CLI replica, and provide a separate operator-owned Fleet input with exact
placement, admission, limit, and funding policy:

```bash
canic status
canic replica start --background
canic install test test-local \
  --fleet-input deployments/test-local.toml \
  --profile fast
```

The Fleet input is required. Its schema is documented in
[`docs/architecture/fleet-install-input.md`](docs/architecture/fleet-install-input.md).
The in-progress 0.100 installer currently verifies the Coordinator, every
planned Fleet Subnet Root, each root's local Store, and every root's Registry
`Joining` row, then stops before snapshot synchronization, acknowledgement,
activation, Component creation, and terminal Fleet-catalog publication.
`canic info list`, `canic info env`, Medic's live Fleet checks, backup, and
restore require that later terminal Fleet state.

Build one artifact without installing:

```bash
canic build test app --profile fast
```

For downstream repos where the Rust workspace and ICP project root differ, pass
both paths explicitly:

```bash
canic --environment local build \
  --workspace /path/to/cargo-workspace \
  --icp-root /path/to/icp-project \
  --config /path/to/cargo-workspace/apps/<app>/canic.toml \
  <app> root \
  --profile fast
```

When passing `--config` explicitly, prefer an absolute path. This keeps path
dependencies and build scripts from interpreting a relative config path from
their own crate directories.

For build profiles, split workspace/ICP roots, custom canister roots, role
metadata, and lower-level build/install commands, see
[`crates/canic-host/README.md`](crates/canic-host/README.md).

For downstream projects that use a named local ICP CLI target such as
`academic`, use
[`docs/getting-started/local-academic-fleet.md`](docs/getting-started/local-academic-fleet.md)
for the short runbook on `canic --environment ...`, raw `icp` target hygiene,
`canic info env` / `CANIC_ROOT`-style canister ID variables, sourced shell
helpers, sharded calls, metrics checks, and install versus upgrade decisions.

## App And Fleet Management

Use `canic app list` to list source Apps and `canic app config <app>` to inspect
declared config. Live commands continue to select an installed Fleet.

```bash
canic app config test
canic info list test
canic status
canic --environment local fleet list
canic app create demo --yes
canic app delete demo
```

Use `canic medic` when local project state, replica ownership, or a named
deployment does not look right:

```bash
canic medic
canic medic fleet test
```

For a terminal installed Fleet, use `canic info list <fleet>`,
`canic info env <fleet>`, and `canic medic fleet <fleet>` before changing
topology when local state looks wrong. `info list` shows live registered
Canisters, `info env` prints sourceable `CANIC_<ROLE>` canister ID exports, and
`app config` shows configured source intent. These live commands do not
reconstruct or bypass an incomplete 0.100 installation journal.

Named-fleet commands default to the local ICP CLI environment. Pass top-level
`--environment <name>` for one command against another configured ICP CLI
environment. Nonlocal targets must be managed externally.

The local ICP CLI replica does not persist canister state across stop/start. If
`canic status` shows a local Fleet as `lost`, its recorded Canisters are gone
from the restarted local replica; rerun
`canic install <app> <fleet> --fleet-input <path>` with the exact intended
operator input to recreate it.

App configs live under project-root `apps/`. Commands launched from nested
directories discover the outer project root and keep ICP project config plus
`.icp/` and `.canic/` state there.

## Backup And Restore

Show installed canisters:

```bash
canic --environment local info list test
canic --environment local info list test --subtree app
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
Preparing the same pristine documents again is idempotent. A conflicting plan
or a journal containing progress is preserved and rejected rather than
overwritten. Rerun `restore run` to reconcile interrupted work after the prior
command tree has ended.

```bash
canic restore prepare 1 --require-verified --require-restore-ready
canic restore status 1 --require-no-attention
canic restore run 1 --execute --max-steps 1 --require-no-attention
canic restore status 1 --require-complete --require-no-attention
```

See [`crates/canic-cli/README.md`](crates/canic-cli/README.md) for the fuller
operator guide.

## Generated State

The Fleet Coordinator, Fleet Subnet Root, and Wasm Store are separately
qualified infrastructure artifacts. Ordinary Component and descendant
artifacts stay outside the root Wasm, are built once as the Fleet-wide
application union, and are projected into an exact admitted release set for
each root. Visible canister Candid files are generated under
`.icp/local/canisters/<role>/<role>.did`. The checked-in exception is
`crates/canic-wasm-store/wasm_store.did`, the canonical interface for the
implicit root-local Store.

Local builds extract Candid from a debug Wasm and may embed public
`candid:service` metadata into the local Wasm for inspection. Builds targeting
`ICP_ENVIRONMENT=ic` skip `.did` generation and Candid metadata embedding so
production Wasm artifacts do not carry local interface metadata.

Canic-managed Candid includes both application methods and Canic runtime
methods such as readiness, metadata, topology, and management endpoints. When
migrating a non-Canic canister, compare the application surface separately from
Canic-owned methods.

## First Install Troubleshooting

- If `canic.toml` uses `[[canisters]]`, rewrite it as one
  `[component_specs.<name>]` Component plus any direct
  `[component_specs.<name>.children.<role>]` tables. Canic validates the flat
  Component shape and bounded instance ceilings.
- If a lifecycle macro reports
  `__canic_missing_finish_macro_add_canic_finish_at_end_after_all_endpoints`,
  add `canic::finish!()` at the end of the canister crate root after custom
  endpoint definitions.
- If a child cannot find its config at build time, check the path passed to
  `canic::build!`; it is relative to the canister crate directory.
- If the root canister does not compile or bootstrap delegated-auth material,
  confirm the runtime dependency enables `control-plane` plus the delegated-auth
  features required by that role, such as `auth-root-canister-sig-create` for
  root proof issuance, `auth-issuer-canister-sig-create` for delegated-token
  issuance, and `auth-delegated-token-verify` for protected endpoint
  verification.
- Each canister crate must declare its App-scoped role with
  `[package.metadata.canic] app = "<app>"` and `role = "<role>"`.
- If a fresh 0.100 install stops after verifying each root-local Store and
  `Joining` Registry row, that is the current explicit
  snapshot-synchronization boundary, not a successful terminal Fleet. Inspect
  the install error and implementation status; do not use a legacy single-root
  Registry path to bypass it.
- If a test manually installs one root and its application canisters, it is not
  validating the current managed-Fleet path. A current PocketIC journey must
  start at the Coordinator, install all planned roots and Stores, join them
  through the Fleet Registry, and resolve application Canisters from
  Component Registry/Directory authority.
