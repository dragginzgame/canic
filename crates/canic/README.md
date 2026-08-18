# canic

Facade crate that re-exports the main Canic stack for canister projects:

- endpoint and lifecycle macros
- core runtime/types
- stable-memory helpers under `canic::memory`

Most downstream canister projects should start here instead of reaching for
lower-level crates directly.

Use the explicit module paths for the larger bundled surfaces:

- `canic::api::*` for runtime APIs
- `canic::dto::*` for public wire and value types
- `canic::memory::*` for stable-memory helpers and macros

## Crate Boundary

Use `canic` from configured canister role packages and Canic's host-generated
built-in infrastructure wrappers. Each package must declare its own direct,
normal runtime dependency on `canic`.

Shared runtime libraries must not depend on `canic`. Keep their domain logic
framework-independent; role packages and IC adapters depend directly on
upstream crates such as `candid`, `ic-cdk`, or `ic-stable-structures` for
generic IC types and APIs. This keeps every role package's runtime graph to
one direct path to Canic.

## Feature Contract

The default feature set is empty. Select every Cargo-gated runtime capability
required by the role; generated metrics are derived from role configuration,
not selected through a facade feature.

| Feature | Default | Enables |
| --- | --- | --- |
| `control-plane` | No | Root control-plane bootstrap and Wasm publication APIs without Store-canister endpoints. |
| `fleet-coordinator-canister` | No | The dedicated canonical Fleet Coordinator lifecycle and Fleet Registry API. Configured application roles should not enable it. |
| `wasm-store-canister` | No | The canonical `wasm_store` canister API used by generated/bootstrap store packages. Ordinary application roles should not enable it. |
| `blob-storage` | No | Non-billing blob-storage status and gateway-administration runtime APIs/endpoints. |
| `blob-storage-billing` | No | Cashier-backed blob-storage billing, funding, and readiness support; also enables `blob-storage`. |
| `sharding` | No | Sharding placement, storage, metrics, and lifecycle support from `canic-core`. |
| `auth-chain-key-ecdsa` | No | Chain-key ECDSA validation and cryptographic support used by delegated-auth proof flows. |
| `auth-chain-key-root-sign` | No | Root-managed chain-key delegation-batch signing; also enables `auth-chain-key-ecdsa`. |
| `auth-root-canister-sig-create` | No | Root canister-signature proof creation for role attestation. |
| `auth-root-canister-sig-verify` | No | Root canister-signature proof verification for role attestation. |
| `auth-issuer-canister-sig-create` | No | Issuer canister-signature token-proof creation. |
| `auth-issuer-canister-sig-verify` | No | Issuer canister-signature token-proof verification. |
| `auth-delegated-token-verify` | No | Delegated-token verification, including required chain-key and issuer-signature verification support. |

The `control-plane` feature is the normal root-role selection. The narrower
`fleet-coordinator-canister` and `wasm-store-canister` features exist for
Canic-owned infrastructure packages; neither is an alternate application or
root configuration.

## Config-Driven Auth Features

Some `canic.toml` auth settings require matching runtime `canic` features in
the role crate's `[dependencies]`. Add these to the runtime dependency, not
only `[build-dependencies]`.

| Config setting | Role crate that needs the feature | Required runtime `canic` feature |
| --- | --- | --- |
| `auth.role_attestation_cache = true` on a non-root canister | that non-root role | `auth-root-canister-sig-verify` |
| any non-root role uses `auth.role_attestation_cache = true` | root role | `auth-root-canister-sig-create` |
| `auth.delegated_token_issuer = true` | that issuer role | `auth-issuer-canister-sig-create`, `auth-delegated-token-verify` |
| `auth.delegated_token_verifier = true` | that verifier role | `auth-delegated-token-verify` |

Run `canic medic project --ci` for concise fail-only diagnostics, or
`canic medic project --json` for automation-friendly check rows such as
`role_required_canic_feature_missing`.

## Typical Use

Use `canic` in both `[dependencies]` and `[build-dependencies]` so the build
macros and runtime macros come from the same facade crate.

Each canister crate declares its role in package metadata:

```toml
[package.metadata.canic]
app = "demo"
role = "app"
```

Use `canic::build!("../canic.toml")` from `build.rs` and `canic::start!()` from
`lib.rs`. The `app` value must match `[app].name` in the selected
`canic.toml`. `role = "root"` selects the root lifecycle and root endpoint
bundle; ordinary roles select the non-root lifecycle and endpoint bundle.

## Inter-Canister Calls

Use the Canic call builder when application code benefits from concise Candid
encoding, typed public errors, and Canic's inter-canister call metrics:

```rust
use candid::Principal;
use canic::prelude::Call;

async fn read_count(target: Principal) -> Result<u64, canic::Error> {
    Call::bounded_wait(target, "read_count")
        .execute_candid()
        .await
}
```

`Call::unbounded_wait` is also available, as are `with_arg`, `with_args`,
`with_raw_args`, `with_cycles`, and `execute_candid_tuple`. Use `execute`
directly when the response must be retained or decoded later. This is an
ordinary IC call builder; it does not replace Canic's protected capability RPC
used for framework-owned creation, upgrade, placement, recycling, or cycle
operations.

## Application Timers

`timer!` schedules one asynchronous invocation. `timer_interval!` schedules
the next invocation only after the current future completes, so interval work
cannot overlap itself. Both return a typed `Result` containing an opaque,
single-owner handle:

```rust
use canic::prelude::*;
use std::time::Duration;

let handle = timer_interval!(Duration::from_secs(30), refresh_cache)
    .expect("schedule cache refresh");
canic::api::timer::cancel(handle).expect("cancel cache refresh");
```

Cancellation consumes the handle. A pending invocation is cleared; a running
invocation is allowed to finish but cannot rearm. Guarded timer macros and raw
CDK timer access are not part of the maintained facade. Labels are validated
by the shared `ic-timers` runtime and are limited to 64 UTF-8 bytes. Dropping a
handle deliberately detaches caller control and does not cancel the timer.
Call `handle.detach()` when that loss of control is intentional and should be
visible in the source.

Every linked owner must resolve the same exact `ic-timers` package ID; two
versions create two canister-local registries rather than one inventory.
Timer performance observations bracket the complete accepted shared-runtime
callback path: registry acceptance, consumer work, completion accounting, and
successor binding. They are operational callback costs, not isolated
application-function benchmarks. A transient `RemoveWhenStopped` declaration
that reaches terminal state is removed from the shared inventory, so no final
status or performance sample survives after removal; retained declarations
preserve their normally completed samples.
Authority snapshots currently reject any timer claim outside Canic custody;
scheduled 0.104 owns the synchronous lifecycle participant, direct native
`ic-timers` adoption guide and combined Canic/IcyDB qualification. The current
timer macros remain the published surface until that hard cut is implemented.

This crate lives in the Canic workspace. See the workspace guide at
`../../README.md` for full setup, topology, and example canisters.
