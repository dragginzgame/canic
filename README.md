<p align="center">
  <img src="assets/canic_logo.svg" alt="Canic logo" width="360" />
</p>

# 🧑‍🔧 Canic 🧑‍🔧 – Internet Computer Orchestration

[![Crates.io](https://img.shields.io/crates/v/canic.svg)](https://crates.io/crates/canic)
[![Docs.rs](https://docs.rs/canic/badge.svg)](https://docs.rs/canic)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Canic is a Rust toolkit for orchestrating Internet Computer (IC) canisters at scale. It packages the battle-tested patterns from large multi-canister deployments into a reusable crate: lifecycle macros, stable-memory helpers, orchestration ops, and endpoint bundles that keep your boundary layer thin while enforcing clean layering inside the canister graph.

The crate was historically known as **ICU** (Internet Computer Utilities). All core APIs have been renamed to **Canic** for the crates.io release.

## Highlights

- 🧩 **Bootstrap macros** – `canic::start!`, `canic::start_root!`, `canic::build!`, and `canic::build_root!` wire init/upgrade hooks, export endpoints, and validate config at compile time.
- 🪶 **Core utilities** – `canic::core` exposes perf counters, bounded types, MiniCBOR serializers, and deterministic utilities without pulling in the full ops stack.
- 🧠 **State layers** – opinionated separation for stable memory, volatile state, ops/business logic, and public endpoints.
- 🗺️ **Topology-aware config** – typed subnet blocks, app directories, and reserve policies validated straight from `canic.toml`.
- 🌿 **Linear topology sync** – targeted cascades ship a trimmed parent chain plus per-node direct children, validate roots/cycles, and fail fast to avoid quadratic fan-out.
- 🔐 **Auth utilities** – composable guards (`auth_require_any!`, `auth_require_all!`) for controllers, parents, whitelist principals, and more.
- 🗃️ **Stable memory ergonomics** – `ic_memory!`, `ic_memory_range!`, and `eager_static!` manage IC stable structures safely across upgrades.
- 📦 **WASM registry** – consistently ship/lookup child canister WASMs with hash tracking.
- 🪵 **Configurable logging** – ring/age retention with second-level timestamps and paged log/query helpers; provisioning calls log caller/parent context on create_canister_request failures to simplify bootstrap debugging.
- ♻️ **Lifecycle helpers** – shard policies, reserve pools, scaling helpers, and sync cascades keep fleets healthy.
- 🧪 **Ready for CI** – Rust 2024 edition, toolchain pinned to Rust 1.92.0, with `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` wired via `make` targets.

For canister signatures, use the ops façade (`ops::signature::prepare`/`get`/`verify`) instead of feeding raw principals into `ic-signature-verification`; `verify` builds the proper DER canister-sig public key and domain-prefixed message to avoid slice panics on short (10-byte) canister IDs. Pass the signing domain and seed from the caller rather than hardcoding them.

## 📁 Repository Layout

- `assets/` – documentation media (logo and shared imagery).
- `crates/` – workspace crates.
- `crates/canic/` – thin façade re-exporting `canic-core`, `canic-memory`, `canic-utils`, and `canic-cdk` for consumers.
- `crates/canic-core/` – orchestration crate used inside canisters.
  - `src/access/` – reusable authorization, guard, and policy helpers.
  - `src/config/` – configuration loaders, validators, and schema helpers.
  - `src/dispatch.rs` – endpoint routing helpers used by the macros.
  - `src/dto/` – candid-friendly DTOs for paging and exports.
  - `src/env/` – curated canister ID constants (ck, NNS, SNS) and helpers.
  - `src/ids/` – strongly-typed role identifiers (`CanisterRole`, `SubnetRole`, etc.).
  - `src/log.rs` – logging macros.
  - `src/macros/` – public macro entrypoints (`canic::start!`, `canic_endpoints_*`, memory helpers).
  - `src/model/` – stable-memory registries plus volatile state caches that back the ops layer.
  - `src/ops/` – orchestration/business logic bridging model to endpoints (including instrumented IC/ledger helpers).
  - `src/spec/` – representations of external IC specs (ICRC, NNS, SNS, etc.).
  - `examples/` – runnable demos for guards, shard lifecycle, and canister ops.
- `crates/canic-memory/` – standalone stable-memory crate (manager, registry, eager TLS, memory macros) usable by Canic and external crates.
- `crates/canic-types/` – candid-friendly domain types (cycles, decimals, bounded strings, WASM helpers).
- `crates/canic-utils/` – small deterministic helpers (casing, formatting, xxHash3 hashing, simple RNG).
- `crates/canic-macros/` – proc macros for defining endpoints (`#[canic_query]`, `#[canic_update]`).
- `crates/canic-cdk/` – curated IC CDK façade used by `canic`, `canic-core`, and `canic-utils` (management, timers, stable-structures glue).
- `crates/canisters/` – reference canisters that exercise the library end to end:
  - `root/` orchestrator tying together shards, scaling, and reserve flows.
  - `app/` – sample application canister used in integration flows.
  - `auth/` – auxiliary canister covering authorization patterns.
  - `shard/`, `shard_hub/` – shard lifecycle pair for pool management.
  - `scale/`, `scale_hub/` – reserve scaling agents demonstrating capacity workflows.
  - `blank/` – minimal canister template.
- `scripts/` – build, release, and environment helpers.
  - `app/` – dfx bootstrap scripts for the demo topology.
  - `ci/` – version bumping and security checks used by CI.
  - `env/` – local environment utilities (e.g., shared env updates).
- `.github/workflows/` – CI pipelines (fmt, clippy, tests, release).
- `.githooks/` – optional git hooks; `pre-commit` formats and runs cargo sort before committing.

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
    canic::build_root!("../canic.toml");
}
```

```rust
// Non-root canister build.rs
fn main() {
    canic::build!("../canic.toml");
}
```

The macro validates the TOML during compilation and exposes the canonical config path via `CANIC_CONFIG_PATH`.

### 3. Bootstrap your canister

In `lib.rs`:

```rust
use canic::prelude::*;
use canic::core::ids::CanisterRole;

const APP: CanisterRole = CanisterRole::new("app");

canic::start!(APP); // or canic::start_root!() for the orchestrator canister

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}
```

See `crates/canisters/root` and the hub/shard reference canisters under `crates/canisters/*` for end-to-end patterns, including WASM registries and endpoint exports.

### 4. Define your topology

Populate `canic.toml` with subnet definitions, directory membership, and per-canister policies. Each `[subnets.<name>]` block lists `auto_create` and `subnet_directory` canister roles, then nests `[subnets.<name>.canisters.<role>]` tables for top-up settings plus optional sharding and scaling pools. Global tables such as `controllers`, `app_directory`, `reserve`, `log`, and `standards` shape the overall cluster. The `[log]` block controls ring/age retention in seconds. The full schema lives in `CONFIG.md`. The role identifiers resolve to the `CanisterRole`/`SubnetRole` wrappers in `crates/canic-core/src/ids/`.

## Layered Architecture

Canic enforces clear separation between storage, transient state, orchestration logic, and public endpoints:

- `model::memory` – stable data backed by `ic-stable-structures` (e.g. shard registries, reserve pools).
- `model::memory::state` – stable canister state/settings persisted across upgrades (e.g. `AppState`, `SubnetState`).
- `model::*` (non-memory) – volatile in-process registries/caches that reset on upgrade (e.g. WASM registry, metrics registries).
- `ops/` – business logic tying state + memory together (sharding policies, scaling flows, reserve management).
- `endpoints/` – macro-generated IC entrypoints that delegate to `ops/` and keep boundary code minimal.
- Temporary exception (target revisit in ~2 weeks): when no ops façade exists yet, read-only queries may pull directly from stable storage (`model::memory`) or runtime registries (`model::*`); mutations should still flow through `ops/`.

## Capabilities & Endpoints

### Sharding 📦

Sharding is configured via `canic.toml` and executed through the ops layer. The base endpoint bundle exposes a controller-only registry query for operator visibility:

```rust
canic_sharding_registry()
    -> Result<canic::core::ops::placement::sharding::ShardingRegistryDto, canic::Error>
```

### Scaling & Reserve Pools ⚖️

- `canic_scaling_registry()` provides controller insight into scaling pools via the shared endpoint bundle.
- Root canisters manage spare capacity through `canic_reserve_list()` and the controller-only `canic_reserve_admin(cmd)` endpoint.

### Directory Views 📇

- `canic_app_directory(PageRequest)` returns the prime root directory view for operator dashboards.
- `canic_subnet_directory(PageRequest)` exposes the per-subnet directory so children can discover peers.

Use `PageRequest::DEFAULT` or `PageRequest::bounded(limit, offset)` to avoid passing raw integers into queries.

### ICRC Support 📚

The base endpoint bundle includes:

- `icrc10_supported_standards()`
- `icrc21_canister_call_consent_message(request)`

Register consent messages via `model::icrc::Icrc21Registry` (or the `ops::ic::icrc` helpers) for rich UX flows.

The `Account` textual encoding matches the ICRC reference (CRC32 → base32, no padding) so checksums align with `icrc-ledger-types`; use `Display`/`FromStr` instead of hand-rolling account strings.

## Tooling & DX

- Format: `cargo fmt --all` (or `make fmt`)
- Fmt check: `make fmt-check`
- Check (type-check only): `make check`
- Lint: `make clippy`
- Test: `make test`
- Build release WASMs: `make build`
- Run the example suite: `make examples` or `cargo build -p canic --examples`

`rust-toolchain.toml` pins the toolchain so CI and local builds stay in sync.

## Examples

Explore the runnable examples under `crates/canic-core/examples/`:

- `auth_rules.rs` – compose guard policies.
- `minimal_root.rs` – bootstrap a bare-bones orchestrator.
- `ops_create_canister.rs` – walk through the create-canister flow.
- `shard_lifecycle.rs` – simulate register/assign/drain/rebalance operations.

```bash
cargo run -p canic-core --example auth_rules
```

## Project Status & Contributing

Canic is the successor to the internal ICU toolkit. The repository is in the process of being opened for wider use; issues and PRs are currently limited to the core team. Follow `AGENTS.md`, `CONTRIBUTING.md`, and the CI scripts under `scripts/ci/` for workflow expectations.

## License

MIT. See `LICENSE` for details.
