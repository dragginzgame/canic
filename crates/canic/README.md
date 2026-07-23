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

Use `canic` only from configured canister role packages. Each role package
must declare its own direct, normal runtime dependency on `canic`.

Shared runtime libraries must not depend on `canic`. Keep their domain logic
framework-independent; role packages and IC adapters depend directly on
upstream crates such as `candid`, `ic-cdk`, or `ic-stable-structures` for
generic IC types and APIs. This keeps every role package's runtime graph to
one direct path to Canic.

## Feature Contract

The default feature set contains only `metrics`. Disable default features when
you need a narrower facade dependency, then select every runtime capability
required by the role.

| Feature | Default | Enables |
| --- | --- | --- |
| `metrics` | Yes | The standard `canic_metrics` endpoint bundle. |
| `control-plane` | No | Root control-plane bootstrap and Wasm publication APIs; also enables `wasm-store-canister`. |
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
`wasm-store-canister` feature exists for the canonical store canister package;
it is not an alternate root control-plane configuration.

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
fleet = "demo"
role = "app"
```

Use `canic::build!("../canic.toml")` from `build.rs` and `canic::start!()` from
`lib.rs`. The `fleet` value must match `[fleet] name = "..."` in the selected
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
        .execute()
        .await?
        .candid()
}
```

`Call::unbounded_wait` is also available, as are `with_arg`, `with_args`,
`with_raw_args`, and `with_cycles`. This is an ordinary IC call builder; it
does not replace Canic's protected capability RPC used for framework-owned
creation, upgrade, placement, recycling, or cycle operations.

## Application Timers

`timer!` schedules one asynchronous invocation. `timer_interval!` schedules
the next invocation only after the current future completes, so interval work
cannot overlap itself. Both return an opaque, single-owner handle:

```rust
use canic::prelude::*;
use std::time::Duration;

let handle = timer_interval!(Duration::from_secs(30), refresh_cache);
assert!(canic::api::timer::cancel(handle));
```

Cancellation consumes the handle. A pending invocation is cleared; a running
invocation is allowed to finish but cannot rearm. Guarded timer macros and raw
CDK timer access are not part of the maintained facade.

This crate lives in the Canic workspace. See the workspace guide at
`../../README.md` for full setup, topology, and example canisters.
