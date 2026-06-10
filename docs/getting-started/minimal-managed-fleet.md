# Minimal Managed Fleet

This guide shows the smallest Canic-managed shape that exercises the real
fleet model: one root canister creates and registers two singleton child
canisters. Use this as the reference before adapting a product canister layout.

This guide tracks the current Canic scaffold shape. For new fleets, prefer
`canic fleet create <name>` and keep all `canic` dependencies on the same
release as the installed `canic` CLI. The current schema uses `app_index`,
`[fleet]`, subnet canister tables, `topup`, and `canic::finish!()`; legacy
aliases are intentionally not documented here.

The root manages lifecycle, topology, and artifact staging. It does not proxy
ordinary application methods. After install, callers resolve child canister IDs
from the root registry and call the child canisters directly.

## Layout

```text
fleets/example/
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
fleet = "example"
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
incremental = true
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

Declare canisters under subnet tables. Do not use a flat `[[canisters]]` list.

```toml
controllers = []
app_index = []

[fleet]
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

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.hub]
kind = "service"
topup = {}

[subnets.prime.canisters.registry]
kind = "service"
topup = {}
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
`auth-root-canister-sig-create` when the fleet enables delegated-token root
proof issuance, and `auth-threshold-ecdsa-public-key` while root still
certifies threshold-ECDSA shard public keys. Add `auth-threshold-ecdsa-sign` to
canisters that sign shard tokens, and `auth-delegated-token-verify` to endpoint
verifiers.

```toml
[package.metadata.canic]
fleet = "example"
role = "root"

[dependencies]
candid = "<version>"
canic = { version = "<same-version-as-canic-cli>", features = ["auth-root-canister-sig-create", "auth-threshold-ecdsa-public-key", "control-plane"] }
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
fleet = "example"
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

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query]
fn whoami_query() -> Result<Principal, Error> {
    Ok(canic::cdk::api::msg_caller())
}

#[canic_update]
fn whoami_update() -> Result<Principal, Error> {
    Ok(canic::cdk::api::msg_caller())
}

canic::finish!();
```

Use the same `lib.rs` shape for `registry`; set its role in that crate's
`Cargo.toml` instead:

```toml
[package.metadata.canic]
fleet = "example"
role = "registry"
```

## Install And Inspect

Build and install the fleet locally:

```bash
canic status
canic replica start --background
canic install --profile fast example
canic info list example
```

Build one role without installing:

```bash
canic build example hub --profile fast
```

If you pass `--workspace`, `--icp-root`, or `--config` explicitly, use absolute
paths for the explicit roots and config file.

`canic info list example` shows the root and managed children. If it only shows
`root`, the root canister has been reserved but the managed tree is not fully
installed yet; run `canic info medic example` and reinstall the local fleet if
the local replica was restarted.

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
