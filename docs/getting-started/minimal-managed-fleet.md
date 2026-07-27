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
catalog is flat. Callers resolve application Canister IDs from topology and
call them directly.

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

Build and install the fleet locally:

```bash
canic status
canic replica start --background
canic install --profile fast example example-local
canic info list example-local
```

Build one role without installing:

```bash
canic build example hub --profile fast
```

If you pass `--workspace`, `--icp-root`, or `--config` explicitly, use absolute
paths for the explicit roots and config file.

`canic info list example` shows the root and managed children. If it only shows
`root`, the root canister has been reserved but the managed tree is not fully
installed yet; run `canic medic fleet example` and reinstall the local
fleet if the local replica was restarted.

## Testing Shape

A managed-fleet PocketIC test should validate the same path as local install:

1. Install the root with root init arguments.
2. Stage the ordinary child release set.
3. Resume root bootstrap.
4. Wait for root and child `canic_ready`.
5. Query `canic_subnet_registry` on root to resolve the child canister ID.
6. Call the child method directly.

Installing root, hub, and registry manually in the same PocketIC instance only
tests individual Canic lifecycle adapters. It does not test that root creates,
registers, and manages the fleet.

## Candid Surface

Canic-managed canisters expose application methods plus Canic runtime,
metadata, readiness, and management methods. When comparing an old non-Canic
canister to a Canic-managed rewrite, compare the application surface separately
from Canic-owned methods.

Local builds extract `.did` files from debug Wasm artifacts. Production
`ICP_ENVIRONMENT=ic` builds intentionally skip Candid extraction and embedded
`candid:service` metadata so deployed Wasm artifacts stay smaller.
