# ✨ ICU – Internet Computer Utilities

Internal Use Only — This repository is private and intended solely for use by authorized team members. Do not distribute or share outside the organization. For access or questions, contact the maintainers.

**ICU** (Internet Computer Utilities) is a Rust framework that simplifies the development and management of multi-canister systems on the DFINITY **Internet Computer (IC)**. It provides a set of utilities and macros to coordinate multiple canisters (smart contracts) working together, making it easier to create complex canister-based dapps that scale across canister boundaries (even across multiple subnets).

ICU addresses common challenges in multi-canister architectures, including canister creation & upgrades, cross-canister state management, stable memory handling across upgrades, and establishing a clear canister hierarchy (with a **root** canister orchestrating child canisters). By using ICU, developers can focus on application logic rather than reinventing boilerplate for managing canister lifecycles and interactions.

## Features

- 🧩 Macros: `icu_start!` and `icu_start_root!` wire init/upgrade and expose a rich set of endpoints.
- 🔐 Auth helpers: composable rules (`auth_require_any!`, `auth_require_all!`) for controllers, parents, children, etc.
- 🧠 State: in-memory registries for delegation, ICRC standards, and WASM modules.
- 🧩 Sharding: generic canister shard registry to assign items (Principals) to child canisters with capacity limits.
- 📦 WASM registry: ship and look up child canister WASMs by `CanisterType`.
- ♻️ Upgrades: consistent state bundle cascade helpers between parent/children.
- 🧪 Testing: unit tests across memory/state modules; CI enforces fmt/clippy.
- 📈 Perf logs: `perf!` macros using `performance_counter(1)` for instruction deltas.

## Quickstart

Add ICU to your workspace and wire a canister:

1) In your canister crate `build.rs`:

For a root canister:

```rust
fn main() { icu::icu_build_root!("../icu.toml"); }
```

For a non-root canister (e.g., example, game, instance, player_hub):

```rust
fn main() { icu::icu_build!("../icu.toml"); }
```

2) In your canister `lib.rs`:

```rust
use icu::prelude::*;
use icu::canister::EXAMPLE;
icu_start_root!(); // or icu_start!(EXAMPLE)

const fn icu_setup() {}
async fn icu_install() {}
async fn icu_upgrade() {}
```

See `crates/canisters/root` and `crates/canisters/example` for full patterns.

MSRV: Rust 1.89.0 (pinned via `rust-toolchain.toml`).

## Delegation Sessions 🔑

Short‑lived “delegation sessions” map a temporary session principal to a wallet principal. Useful for frontends delegating limited permissions to canisters.

- Types: `DelegationSession { wallet_pid, expires_at, requesting_canisters }` and `DelegationSessionView` (read model).
- Expiry: Sessions are considered expired at the boundary (`expires_at <= now`).
- Typical flow: 🧪 create session → 📡 track usage → 🔍 resolve wallet → ⏳ expire or ❌ revoke.

Endpoints (provided by `icu_endpoints!` and enabled when delegation is turned on in config):

- 📥 `icu_delegation_register(args)` (update): register a session for the caller wallet.
- 👣 `icu_delegation_track(session_pid)` (update): record the calling canister as a requester.
- 🔎 `icu_delegation_get(session_pid)` (query): fetch session view (includes `is_expired`).
- 🧹 Cleanup: expired sessions are pruned automatically during registrations (every 1000 calls). There is no public cleanup endpoint.
- 📜 `icu_delegation_list_all()` (query): list all sessions. Auth: controller only.
- 🧭 `icu_delegation_list_by_wallet(wallet_pid)` (query): list sessions for a wallet. Auth: controller only.

Notes:
- Minimum duration: 60s ⏱️, Maximum: 24h 🕛 (configurable in code today).
- Registry also exposes pure functions (e.g., `list_all_sessions`) used by these endpoints.
- Delegation endpoints are cfg-gated. Enable per-canister by setting `delegation = true` in `icu.toml` under the relevant `[canisters.<name>]` entry so that `icu_build!` emits the feature.

## WASM Registry 📦

Root canisters can import a static set of gzipped child canister WASMs and expose them by `CanisterType`.

- Import: `WasmRegistry::import(WASMS)` runs during `icu_start_root!()` setup.
- Lookup: `WasmRegistry::try_get(&CanisterType)` returns a `WasmModule` with bytes and module hash.
- Usage: `ops::canister` fetches the module to `install_code` and stores the module hash in the registry.

Tip: add your WASMs to the `WASMS` slice in the root canister crate. Example is in `crates/canisters/root/src/lib.rs`.

## Sharding 📦

- Registry: assign items (`Principal`) to shard canisters with capacities.
- Use `CanisterShardRegistry::register(pid, capacity)` to add/resize shards.
- Assign items automatically with `icu::ops::shard::ensure_item_assignment(&CanisterType::new("game_instance_shard"), item, policy, parents, None)`.

Policy example:

```rust
use icu::prelude::*;
let policy = icu::ops::shard::ShardPolicy { initial_capacity: 100, max_shards: 64, growth_threshold_pct: 80 };
let shard = icu::ops::shard::ensure_item_assignment(&CanisterType::new("game_instance_shard"), item_principal, policy, &parents, None).await?;
```

## ICRC Support 📚

- ICRC‑10: `icrc10_supported_standards()` returns the `(name, url)` pairs enabled by config.
- ICRC‑21: Register consent message handlers via `Icrc21Registry::register` or `register_static_with`, then call `icrc21_canister_call_consent_message`.

## Dev UX 🛠️

- Run all checks: `make all`
- Lint: `make clippy` (warnings denied) • Format: `make fmt` / `make fmt-check`
- Tests: `make test` (includes optional dfx flow if available)
- Examples: `make examples` or `cargo build -p icu --examples`

Some targets (e.g., `make build`, `make all`, and version bumps) enforce a clean git working tree.

## Contributing

This is an internal project. External contributions are not accepted. For internal changes, follow the Repository Guidelines in `AGENTS.md` and use `VERSIONING.md` / `RELEASE_GUIDE.md` for tagging and release flow.

### Setup

Install required toolchain components once:

```bash
make install-canister-deps
```

## Examples

Example files:

- [crates/icu/examples/auth_rules.rs](crates/icu/examples/auth_rules.rs) — basic auth rule composition
- [crates/icu/examples/minimal_root.rs](crates/icu/examples/minimal_root.rs) — minimal root canister scaffold
- [crates/icu/examples/ops_create_canister.rs](crates/icu/examples/ops_create_canister.rs) — create-canister request flow

Build all examples:

```bash
make examples
# or
cargo build -p icu --examples
```

Run a specific example: `cargo run -p icu --example auth_rules`

Note: The `ic` cfg is used internally for tests/build tooling and is not a user-settable feature flag.

## Licensing

Proprietary and Confidential. All rights reserved. See `LICENSE`.

## Module Guides

### Directory & Registry

- Source of truth: `CanisterRegistry` (root-only) tracks type, parent, lifecycle, and module hash.
- Directory is a read model generated from the registry on root and cascaded to children.
- Writes:
  - Root does not mutate the directory directly; it generates and cascades a full view.
  - Children accept full re-imports; no partial insert/remove APIs exist.
- Invariants:
  - Root directory view equals generation from registry; children align after cascade.

### Spec

- Purpose: Protocol and spec types for IC/ICRC/SNS.
- Scope: Candid-friendly data structures only; no business logic.
- Stability: Aim to keep types stable; document breaking changes in the changelog.
