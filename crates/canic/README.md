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
| `internal-test-fixtures` | No | Repository qualification helpers that are excluded from product-role builds and grant no runtime capability. |

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

Run `canic build <app> <role>` to validate the selected role contract and its
required runtime features through the maintained generated build path.

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

Application canisters depend on the exact workspace-compatible `ic-timers`
release and own their native registrations directly. Canic intentionally
provides no timer macro, handle or cancellation facade:

```rust
use ic_timers::{
    DeclarationLifetime, OnceContext, TimerCompletion, TimerDirective,
    TimerIdentity, TimerRunResult, TimerSchedule, register_once,
};
use std::time::Duration;

let timer = register_once(
    TimerIdentity::try_new("my-app", "cache", "refresh")?,
    DeclarationLifetime::RemoveWhenStopped,
    |_context: OnceContext| async {
        refresh_cache().await;
        TimerRunResult::new(TimerCompletion::success(1), TimerDirective::Stop)
    },
)?;
timer.ensure_scheduled(TimerSchedule::After(Duration::from_secs(30)))?;
```

Keep the native registration when later cancellation or reconciliation is
required. Dropping it detaches caller control without cancelling the timer.
Provider inventory is volatile observation, not durable application demand.
Use the paired `lifecycle_participant(init = ..., post_upgrade = ...)`
declaration on `canic::start!` to reconstruct application-owned volatile
registrations synchronously after Canic restoration and before deferred hooks.

Every linked owner must resolve the same exact `ic-timers` package ID; two
versions create two canister-local registries rather than one inventory.
Timer performance observations bracket the complete accepted shared-runtime
callback path: registry acceptance, consumer work, completion accounting, and
successor binding. They are operational callback costs, not isolated
application-function benchmarks. A transient `RemoveWhenStopped` declaration
that reaches terminal state is removed from the shared inventory, so no final
status or performance sample survives after removal; retained declarations
preserve their normally completed samples.
Authority snapshots currently reject any timer claim outside Canic custody.
See the maintained
[native timer adoption guide](../../docs/features/runtime/native-timers.md)
for exact dependency, custody, lifecycle reconstruction and qualification
rules. Combined Canic/IcyDB qualification remains a separate 0.104 proof.

This crate lives in the Canic workspace. See the workspace guide at
`../../README.md` for full setup, topology, and example canisters.
