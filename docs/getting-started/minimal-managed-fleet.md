# Minimal Managed Fleet

This guide shows the smallest Canic-managed shape that exercises the
root-owned Component-tree model: one Fleet Subnet Root manages a `hub`
Component, and that Component asks the root to create one direct `registry`
child.
Use this as the reference before adapting a product canister layout.

This guide tracks the current Canic scaffold shape. For new fleets, prefer
`canic app create <name>` and keep all `canic` dependencies on the same
release as the installed `canic` CLI. The current schema uses
`[app].name`, flat Component role catalogs, bounded descendants, `topup`, and
`canic::finish!()`.

The root executes lifecycle, topology, and artifact effects. It does not proxy
ordinary application methods. Each registered node owns its direct children
logically and asks the root to perform admitted cycle, creation, and
installation effects. Those children may make the same request in turn, so
runtime trees may have several levels even though the Spec's potential-Wasm
catalog is flat. Callers resolve application Canister IDs from a
revision-bound Component Directory or an application-owned Placement Index
and call them directly.

## Layout

```text
apps/example/
├── canic.toml
├── root/
│   ├── Cargo.toml
│   ├── build.rs
│   └── src/lib.rs
├── hub/
│   ├── Cargo.toml
│   ├── build.rs
│   └── src/lib.rs
└── registry/
    ├── Cargo.toml
    ├── build.rs
    └── src/lib.rs
```

Every canister package must declare the Canic role it implements. The role must
resolve to a declared role in `canic.toml`:

```toml
[package.metadata.canic]
app = "example"
role = "hub"
```

If you use `--profile fast` in local Canic commands, define the Cargo profile
in the workspace root:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = "symbols"
debug = false
panic = "abort"
overflow-checks = false
incremental = false

[profile.fast]
inherits = "release"
lto = false
codegen-units = 16
incremental = false
```

## ICP Project Config

Local managed installs use `icp.yaml` plus `.icp/` state. Do not copy old
`dfx.json` or `canister_ids.json` files just to start a new local Canic fleet.

Add matching canister and environment entries for the fleet roles:

```yaml
canisters:
  - name: root
    build:
      steps:
        - type: script
          commands:
            - canic build example root --profile fast
  - name: hub
    build:
      steps:
        - type: script
          commands:
            - canic build example hub --profile fast
  - name: registry
    build:
      steps:
        - type: script
          commands:
            - canic build example registry --profile fast

environments:
  - name: example
    network: local
    canisters: [root, hub, registry]
```

## Fleet Config

Declare one Component Spec with one top-level Component role and a flat catalog
of its potential descendant roles. Do not use a flat `[[canisters]]` list or
nest child tables to express runtime parentage.

```toml
controllers = []

[app]
name = "example"

[roles.root]
kind = "root"
package = "root"

[roles.hub]
kind = "canister"
package = "hub"

[roles.registry]
kind = "canister"
package = "registry"

[component_specs.main]
component_role = "hub"
maximum_instances = 1
topup = {}

[component_specs.main.children.registry]
kind = "singleton"
topup = {}

[component_specs.main.spawn_grants.hub.registry]
maximum_instances_per_parent = 1
```

## Build Scripts

Each canister crate needs the same small `build.rs`. The path is relative to
the canister crate directory, so adjust it if your layout differs.

```rust
fn main() {
    canic::build!("../canic.toml");
}
```

If your canisters are nested more deeply, pass the real relative path, for
example `../../canic.toml`.

## Root Canister

The root crate needs Canic's `control-plane` feature. Add
`auth-root-canister-sig-create` only when the fleet issues role attestations.
Add `auth-issuer-canister-sig-create` to canisters that issue delegated tokens,
and `auth-delegated-token-verify` to endpoint verifiers.

```toml
[package.metadata.canic]
app = "example"
role = "root"

[dependencies]
candid = "<version>"
canic = { version = "<same-version-as-canic-cli>", features = ["auth-root-canister-sig-create", "control-plane"] }
ic-cdk = "0.20"

[build-dependencies]
canic = "<same-version-as-canic-cli>"
```

```rust
#![expect(clippy::unused_async)]

canic::start!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

canic::finish!();
```

## Child Canister

Child canisters declare their role in Cargo metadata and use Canic endpoint
macros for application methods.

```toml
[package.metadata.canic]
app = "example"
role = "hub"

[dependencies]
candid = "<version>"
canic = "<same-version-as-canic-cli>"
ic-cdk = "0.20"

[build-dependencies]
canic = "<same-version-as-canic-cli>"
```

```rust
#![expect(clippy::unused_async)]

use candid::Principal;
use canic::{Error, prelude::*};
use ic_cdk::api::msg_caller;

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query]
fn whoami_query() -> Result<Principal, Error> {
    Ok(msg_caller())
}

#[canic_update]
fn whoami_update() -> Result<Principal, Error> {
    Ok(msg_caller())
}

canic::finish!();
```

Use the same `lib.rs` shape for `registry`; set its role in that crate's
`Cargo.toml` instead:

```toml
[package.metadata.canic]
app = "example"
role = "registry"
```

## Install And Inspect

Create a separate operator Fleet input using the exact local application
Subnet principal. The complete document shape and public-IC selectors are in
[`fleet-install-input.md`](../architecture/fleet-install-input.md).

Then exercise the current local installation boundary:

```bash
canic status
canic replica start --background
canic install example example-local --fleet-input deployments/example-local.toml --profile fast
```

The 0.100 implementation currently creates and verifies the Coordinator, the
planned Fleet Subnet Roots, each root's exact local Store, and every root's
Registry `Joining` row, private snapshot candidate and Coordinator
acknowledgement, then atomically commits and independently verifies the
complete Coordinator Registry as `Active`. Every root remains
runtime-`Prepared` while installation atomically activates and independently
verifies every exact matching Registry Mirror/Fleet Directory. Installation
then stops before Component creation, runtime activation and terminal
Fleet-catalog publication.
`canic info list example-local` becomes applicable only after that Fleet
reaches the terminal catalog boundary.

Build one role without installing:

```bash
canic build example hub --profile fast
```

If you pass `--workspace`, `--icp-root`, or `--config` explicitly, use absolute
paths for the explicit roots and config file.

For a terminal installed Fleet, `canic info list example-local` shows its
registered application Canisters. The planned
`canic info subnets example-local [--json]` command will instead report exact
Fleet-owned Canister counts grouped by occupied physical Subnet. That Subnet
inventory is a required 0.100 closeout surface and is not available in the
current CLI.

## Testing Shape

A managed-fleet PocketIC test should validate the same path as local install:

1. Compile and freeze the complete Component Topology and Fleet install plan.
2. Install and verify the Coordinator with exact Fleet Registry genesis.
3. Install every planned Fleet Subnet Root with its protected authority.
4. Stage each admitted release set and bootstrap exactly one verified local
   Store per root.
5. Join every root through the Coordinator Fleet Registry.
6. Synchronize the final snapshot and activate roots only after Store, Mirror,
   and Directory evidence agree.
7. Create the admitted `hub` Component through root-owned Component Registry
   authority.
8. Have `hub` request `registry` through the exact compiled spawn grant.
9. Resolve the child from the revision-bound Component Directory and call its
   application method directly.

The current implementation reaches step 5 and deliberately stops before step
6. Installing one root, `hub`, and `registry` manually in the same PocketIC
instance only tests individual lifecycle adapters; it does not validate the
Coordinator-anchored managed-Fleet journey.

## Candid Surface

Canic-managed canisters expose application methods plus Canic runtime,
metadata, readiness, and management methods. When comparing an old non-Canic
canister to a Canic-managed rewrite, compare the application surface separately
from Canic-owned methods.

Local builds extract `.did` files from debug Wasm artifacts. Production
`ICP_ENVIRONMENT=ic` builds intentionally skip Candid extraction and embedded
`candid:service` metadata so deployed Wasm artifacts stay smaller.
