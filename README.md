<p align="center">
  <img src="assets/canic_logo.png" alt="Canic logo" width="360" />
</p>

# 🧑‍🔧 Canic 🧑‍🔧 – Internet Computer Orchestration

[![Crates.io](https://img.shields.io/crates/v/canic.svg)](https://crates.io/crates/canic)
[![Docs.rs](https://docs.rs/canic/badge.svg)](https://docs.rs/canic)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Canic is a Rust toolkit for orchestrating Internet Computer (IC) canisters at scale. It packages the battle-tested patterns from large multi-canister deployments into a reusable crate: lifecycle macros, stable-memory helpers, orchestration ops, and endpoint bundles that keep your boundary layer thin while enforcing clean layering inside the canister graph.

The crate was historically known as **ICU** (Internet Computer Utilities). All core APIs have been renamed to **Canic** for the crates.io release.

## Highlights

- 🧩 **Bootstrap macros** – `canic_start!`, `canic_start_root!`, `canic_build!`, and `canic_build_root!` wire init/upgrade hooks, export endpoints, and validate config at compile time.
- 🧠 **State layers** – opinionated separation for stable memory, volatile state, ops/business logic, and public endpoints.
- 🔐 **Auth utilities** – composable guards (`auth_require_any!`, `auth_require_all!`) for controllers, parents, whitelist principals, and more.
- 🗃️ **Stable memory ergonomics** – `ic_memory!`, `ic_memory_range!`, and `eager_static!` manage IC stable structures safely across upgrades.
- 📦 **WASM registry** – consistently ship/lookup child canister WASMs with hash tracking.
- ♻️ **Lifecycle helpers** – shard policies, reserve pools, delegation sessions, and sync cascades keep fleets healthy.
- 🧪 **Ready for CI** – Rust 2024 edition, MSRV 1.90, with `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` wired via `make` targets.

## 📁 Repository Layout

- `crates/canic/` – core library crate with macros, memory/state layers, ops, and auth utilities.
  - `src/cdk` - the IC CDK with a few changes (docs coming soon)
  - 🧩 `src/macros/` – public macro entrypoints (`canic_start!`, `canic_endpoints_*`, memory helpers).
  - 🧠 `src/memory/` – stable storage abstractions and registries.
  - ⚡ `src/state/` – volatile runtime state caches.
  - 🔧 `src/ops/` – orchestration/business logic bridging memory and state.
  - 🛡️ `src/auth.rs` & `src/guard.rs` – reusable authorization guards.
  - 📦 `examples/`, `tests/`, `benches/` – runnable samples, integration tests, and benchmarking harnesses.
  - 🏗️ `build.rs` – ensures configs/macros wire up at compile time.
- `crates/canisters/` – reference canisters used for integration tests and examples:
  - `root/` orchestrator canister wiring the full stack.
  - `shard/`, `shard_hub/` shard lifecycle pair for pool management.
  - `scale/`, `scale_hub/` scaling agents demonstrating reserve orchestration.
  - `delegation/` auth delegation flows.
  - `blank/` minimal canister used in tests.
- `scripts/` – automation helpers (`app/` for versioning, `ci/` for workflows, `env/` bootstrap scripts).

## Getting Started

### 1. Install

Inside your workspace:

```bash
cargo add canic
```

Or reference the workspace path if you pulled the repository directly.

### 2. Configure `build.rs`

Every canister crate should declare a config file (default name: `canic.toml`). Use one of the provided build macros:

```rust
// Root canister build.rs
fn main() {
    canic::canic_build_root!("../canic.toml");
}
```

```rust
// Non-root canister build.rs
fn main() {
    canic::canic_build!("../canic.toml");
}
```

The macro validates the TOML during compilation, emits the right `cfg` flags (e.g. `canic_capability_delegation`), and exposes the canonical config path via `CANIC_CONFIG_PATH`.

### 3. Bootstrap your canister

In `lib.rs`:

```rust
use canic::prelude::*;
use canic::canister::EXAMPLE;

canic_start!(EXAMPLE); // or canic_start_root!() for the orchestrator canister

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}
```

See `crates/canisters/root` and the hub/shard reference canisters under `crates/canisters/*` for end-to-end patterns, including WASM registries and endpoint exports.

### 4. Define your topology

Populate `canic.toml` with canister metadata, capabilities, and auth lists. The schema is documented in `CONFIG.md`.

## Layered Architecture

Canic enforces clear separation between storage, transient state, orchestration logic, and public endpoints:

- `memory/` – stable data backed by `ic-stable-structures` (e.g. shard registries, reserve pools).
- `state/` – volatile caches and session stores that reset on upgrade.
- `ops/` – business logic tying state + memory together (sharding policies, delegation flows, reserve management).
- `endpoints/` – macro-generated IC entrypoints that delegate to `ops/` and keep boundary code minimal.

Endpoints must call into `ops/`; they should never touch `memory/` or `state/` directly.

## Capabilities & Endpoints

### Delegation Sessions 🔑

Enabled via `delegation = true` in `canic.toml`. When active, `canic_endpoints_delegation!()` (included automatically) exports:

- `canic_delegation_register(args)` – register a session for the caller wallet (update).
- `canic_delegation_track(session_pid)` – record a requesting canister (update).
- `canic_delegation_get(session_pid)` – fetch session metadata (query).
- `canic_delegation_list_all()` / `canic_delegation_list_by_wallet(pid)` – controller-only admin views.
- `canic_delegation_revoke(pid)` – parent-or-self revocation (update).

Sessions auto-clean during registrations; no manual cleanup endpoint is exposed.

### Sharding 📦

`canic::ops::shard` assigns tenants to shard canisters according to a `ShardPolicy` (initial capacity, max shards, growth thresholds). Admin work flows through a single controller-only endpoint:

```rust
canic_sharding_admin(cmd: canic::ops::sharding::AdminCommand)
    -> Result<canic::ops::sharding::AdminResult, canic::Error>
```

Command variants cover register, audit, drain, rebalance, and decommission flows. Your application is responsible for data migration around these moves.

### Scaling & Reserve Pools ⚖️

- `canic_endpoints_scaling!()` exposes `canic_scaling_registry()` for controller insight.
- Root canisters manage spare capacity through `canic::ops::reserve` and the `canic_reserve_*` endpoints.

### ICRC Support 📚

The base endpoint bundle includes:

- `icrc10_supported_standards()`
- `icrc21_canister_call_consent_message(request)`

Register consent messages via `state::icrc::Icrc21Registry` for rich UX flows.

## Tooling & DX

- Format: `cargo fmt --all`
- Lint: `make clippy`
- Test: `make test`
- Build release WASMs: `make build`
- Run the example suite: `make examples` or `cargo build -p canic --examples`

`rust-toolchain.toml` pins the toolchain so CI and local builds stay in sync.

## Examples

Explore the runnable examples under `crates/canic/examples/`:

- `auth_rules.rs` – compose guard policies.
- `minimal_root.rs` – bootstrap a bare-bones orchestrator.
- `ops_create_canister.rs` – walk through the create-canister flow.
- `shard_lifecycle.rs` – simulate register/assign/drain/rebalance operations.

```bash
cargo run -p canic --example auth_rules
```

## Project Status & Contributing

Canic is the successor to the internal ICU toolkit. The repository is in the process of being opened for wider use; issues and PRs are currently limited to the core team. Follow `AGENTS.md`, `VERSIONING.md`, and `RELEASE_GUIDE.md` for workflow expectations.

## License

Proprietary and confidential. See `LICENSE` for details.
