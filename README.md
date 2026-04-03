<p align="center">
  <img src="assets/canic_logo.svg" alt="Canic logo" width="360" />
</p>

# 🧑‍🔧 Canic 🧑‍🔧 – Internet Computer Orchestration

[![Crates.io](https://img.shields.io/crates/v/canic.svg)](https://crates.io/crates/canic)
[![Docs.rs](https://docs.rs/canic/badge.svg)](https://docs.rs/canic)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Canic is a Rust toolkit for orchestrating Internet Computer (IC) canisters at scale. It packages battle‑tested patterns from large multi‑canister deployments into a reusable crate: lifecycle macros, stable‑memory helpers, orchestration ops, and endpoint bundles that keep your boundary layer thin while **encouraging clean layering** inside the canister graph.

The crate was historically known as **ICU** (Internet Computer Utilities). All core APIs have been renamed to **Canic** for the crates.io release.

## Highlights

* 🧩 **Bootstrap macros** – `canic::start!`, `canic::start_root!`, `canic::build!`, and `canic::build_root!` wire init/upgrade hooks, export endpoints, and validate config at compile time.
* 🪶 **Runtime utilities** – use `canic::api::ops::{log, perf}` for observability and `canic::cdk::types` for bounded types.
* 🧠 **State layers** – opinionated separation for stable memory, volatile state, orchestration logic, and public endpoints.
* 🗺️ **Topology‑aware config** – typed subnet blocks, app directories, and pool policies validated straight from `canic.toml`.
* 🌿 **Linear topology sync** – targeted cascades ship a trimmed parent chain plus per‑node direct children, validate roots/cycles, and fail fast to avoid quadratic fan‑out.
* 🔐 **Auth utilities** – composable `requires(...)` expressions with `all(...)`, `any(...)`, and `not(...)` for controllers, parents, whitelist principals, and more.
* 🔏 **Delegated auth model** – root-anchored delegated token flow (`root -> user_shard -> user token`) with direct caller binding (`sub == caller`), explicit audience/scope checks, and local verification.
* 🗃️ **Stable memory ergonomics** – `ic_memory!`, `ic_memory_range!`, and `eager_static!` manage IC stable structures safely across upgrades.
* 📦 **Managed `wasm_store` publication** – stage and publish child canister WASMs with hash tracking while keeping `root` thin.
* 🪵 **Configurable logging** – ring/age retention with second‑level timestamps and paged log/query helpers; provisioning calls log caller/parent context on `create_canister_request` failures to simplify bootstrap debugging.
* ♻️ **Lifecycle helpers** – shard policies, pool capacity, scaling helpers, and sync cascades keep fleets healthy.
* 🧪 **Ready for CI** – Rust 2024 edition, toolchain pinned to Rust 1.94.1, with `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` wired via `make` targets.

## 📁 Repository Layout

* `assets/` – documentation media (logo and shared imagery).
* `crates/` – workspace crates.
* `crates/canic/` – thin façade re‑exporting the public API plus `canic-dsl-macros`, `canic-cdk`, `canic-memory`, and the optional control-plane / sharding lanes for consumers.

  * `src/macros/` – public macro entrypoints (`canic::start!`, `canic::start_root!`, `canic::build!`, endpoint bundles, timer helpers).
  * `src/protocol.rs` – shared protocol method names and exported endpoint IDs.
* `crates/canic-core/` – orchestration crate used inside canisters.

  * `src/access/` – boundary helpers (authorization, guards, endpoint‑adjacent policy). Must not depend on concrete model types.
  * `src/api/` – public runtime APIs re-exported through the `canic` facade.
  * `src/bootstrap/` – config bootstrap and embedded-config helpers.
  * `src/config/` – configuration loaders, validators, and schema helpers.
  * `src/dispatch/` – endpoint routing helpers used by the macros.
  * `src/domain/` – pure domain and policy logic.
  * `src/dto/` – candid‑friendly DTOs for paging and exports.
  * `src/ids/` – strongly‑typed role identifiers (`CanisterRole`, `SubnetRole`, etc.).
  * `src/infra/` – low‑level IC capability bindings (no domain logic).
  * `src/log.rs` – logging macros.
  * `src/lifecycle/` – synchronous lifecycle adapters that restore env and schedule async bootstrap.
  * `src/ops/` – application services bridging model to endpoints (includes single‑step IC/timer façades).
  * `src/storage/` – persisted schemas and storage helpers backing stable memory.
  * `src/view/` – internal read‑only projections used by workflow/policy/ops.
  * `src/workflow/` – orchestration, retries, cascades, and multi‑step behaviors.
* `crates/canic-installer/` – published installer and release-set tooling for downstream workspaces.
* `crates/canic-control-plane/` – root/store control-plane runtime used by the orchestrator lane.
* `crates/canic-memory/` – standalone stable‑memory crate (manager, registry, eager TLS, memory macros) usable by Canic and external crates.
* `crates/canic-sharding-runtime/` – optional sharding runtime lane used by sharded deployments.
* `crates/canic-testkit/` – host‑side test utilities and fixtures for Canic canisters.
* `crates/canic-tests/` – workspace-only integration test host package for the PocketIC and root-suite coverage.
* `crates/canic-dsl-macros/` – proc macros for defining endpoints (`#[canic_query]`, `#[canic_update]`).
* `crates/canic-cdk/` – curated IC CDK façade used by the public/runtime crates (management, timers, stable‑structures glue).
* `crates/canic-wasm-store/` – canonical publishable `wasm_store` canister crate used for the implicit bootstrap store artifact; downstream build helpers can also synthesize the same hidden wrapper directly from `canic` when they only depend on the facade crate.
* `canisters/` – reference canisters and workspace-only support crates that exercise the library end to end:

  * `root/` orchestrator tying together shards, scaling, pool flows, and the implicit bootstrap `wasm_store`.
  * `app/` – sample application canister used in integration flows.
  * `user_hub/`, `user_shard/` – sharding placement and delegated signing pool.
  * `scale/`, `scale_hub/` – pool scaling agents demonstrating capacity workflows.
  * `minimal/` – minimal runtime baseline canister.
  * `test/` – workspace‑only test canister used by host‑side fixtures.
  * `reference-support/` – workspace-only shared support crate published internally as `canic-internal`.
* `scripts/` – build, release, audit, and environment helpers.

  * `app/` – bootstrap scripts for the demo topology.
  * `bench/` – local benchmarking helpers.
  * `ci/` – version bumping and recurring audit helpers used by CI and local maintenance flows.
  * `env/` – local environment utilities (e.g., shared env updates).
  * `env.sh` – shared environment bootstrap for scripts and tooling.
* `.github/workflows/` – CI checks and tag-driven build workflows.

## Getting Started

### 1. Install

Inside your workspace:

```bash
cargo add canic
cargo add canic --build
```

The `build.rs` macros (`canic::build!` / `canic::build_root!`) run in the build script, so `canic` must be present in both `[dependencies]` and `[build-dependencies]`.

Or reference the workspace path if you pulled the repository directly:

```toml
[dependencies]
canic = { path = "/path/to/canic/crates/canic" }

[build-dependencies]
canic = { path = "/path/to/canic/crates/canic" }
```

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
use canic::ids::CanisterRole;

const APP: CanisterRole = CanisterRole::new("app");

canic::start!(APP); // or canic::start_root!() for the orchestrator canister

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}
```

See `canisters/root` and the reference canisters under `canisters/*` for end‑to‑end patterns, including managed `wasm_store` publication and endpoint exports.

### 4. Define your topology

Populate `canic.toml` with subnet definitions, directory membership, and per‑canister policies. Each `[subnets.<name>]` block lists `auto_create` and `subnet_directory` canister roles, then nests `[subnets.<name>.canisters.<role>]` tables for top‑up settings plus optional sharding and scaling pools. Global tables such as `controllers`, `app_directory`, `app`, `auth`, `log`, and `standards` shape the overall cluster, while per-subnet warm-pool policy lives under `[subnets.<name>.pool]` (older configs may still refer to this as `reserve`). The `[log]` block controls ring/age retention and per‑entry size caps. The full schema lives in `CONFIG.md`. The role identifiers resolve to the `CanisterRole`/`SubnetRole` wrappers in `crates/canic-core/src/ids/`.

### 5. Local Build and Install

For local DFX workflows, install the published helper that owns Canic's thin-root build and install boundary:

```bash
cargo install --locked canic-installer --version <same-version-as-canic>
```

Then, with the target `dfx` replica already running, from your workspace root:

```bash
canic-install-root root
```

`canic-install-root` now owns the local thin-root flow end to end. It creates local canisters, runs `dfx build --all`, emits `.dfx/local/canisters/root/root.release-set.json`, reinstalls `root`, stages the ordinary release set, resumes bootstrap, and waits for `canic_ready`.

`root` stays thin in this flow. It embeds only the bootstrap `wasm_store.wasm.gz`; ordinary child releases stay outside `root` and are staged after install from `.dfx/local/canisters/root/root.release-set.json`.

Visible canister Candid files are generated build artifacts under `.dfx/local/canisters/<role>/<role>.did`. They are not committed source files. The checked-in exception is `crates/canic-wasm-store/wasm_store.did`, which remains the canonical published interface for the hidden bootstrap store crate.

Canic now treats wasm build selection as an explicit three-profile contract:

- `CANIC_WASM_PROFILE=debug` for raw large debug wasm
- `CANIC_WASM_PROFILE=fast` for the middle local/test/demo lane
- `CANIC_WASM_PROFILE=release` for shipping/install artifacts

If unset, the published installer/build tools default to `release`.

Typical local fast flow:

```bash
CANIC_WASM_PROFILE=fast canic-install-root root
```

If your repo splits the Rust workspace and the DFX app root (for example `backend/` + `frontend/`), point Canic at both roots explicitly:

```bash
CANIC_WORKSPACE_ROOT=/path/to/repo/backend \
CANIC_DFX_ROOT=/path/to/repo \
CANIC_WASM_PROFILE=fast \
canic-build-canister-artifact root
```

`CANIC_WORKSPACE_ROOT` controls Cargo, `canic.toml`, and canister manifests. `CANIC_DFX_ROOT` controls `dfx.json`, `.dfx`, emitted artifacts, and the hidden generated bootstrap-store wrapper.

If you need the lower-level build/install boundaries directly, `canic-installer` also publishes:

- `canic-build-canister-artifact`
- `canic-build-wasm-store-artifact`
- `canic-emit-root-release-set-manifest`
- `canic-stage-root-release-set`

## Layered Architecture

Canic follows a strict layered design to keep boundaries stable and refactors cheap. Dependencies must flow inward; boundary code must not depend on concrete storage representations.

* `storage/` – authoritative persisted state and storage helpers for stable memory.
* `view/` – internal read-only projections used by workflow, ops, and policy.
* `ops/` – deterministic application services over storage plus approved single-step platform effects.
* `domain/policy` – pure decision logic (no mutation, no IC calls).
* `workflow/` – orchestration and multi-step behavior over time.
* `access/` plus macro-generated endpoints – request guards and system-boundary wiring that delegate immediately to `workflow` or `ops`.

## Capabilities & Endpoints

### Delegated Auth 🔐

- Root canisters issue shard delegation certificates.
- User shard canisters mint user-bound delegated tokens.
- Verifier canisters validate tokens locally (no relay envelope mode).
- Authenticated endpoints require:
  - caller-subject binding (`token.claims.sub == caller`)
  - explicit audience membership (`self in token.claims.aud`)
  - required scope binding (`required_scope in token.claims.scopes`)
  - token/cert expiry checks

Reference contracts:
- `docs/contracts/AUTH_DELEGATED_SIGNATURES.md`
- `docs/contracts/ACCESS_ARCHITECTURE.md`

### Sharding 📦

Sharding is configured via `canic.toml` and executed through the ops layer. Canisters only export sharding registry endpoints when their validated role config includes sharding support.

```rust
canic_sharding_registry()
    -> Result<canic::dto::placement::sharding::ShardingRegistryResponse, canic::Error>
```

### Scaling & Pool Capacity ⚖️

* `canic_scaling_registry()` is exported only for roles whose config enables scaling.
* `canic_pool_list()` and the controller‑only `canic_pool_admin(cmd)` are root-only endpoints for spare-capacity management.

### Directory Listings 📇

* `canic_app_directory(PageRequest)` returns the prime root directory listing for operator dashboards.
* `canic_subnet_directory(PageRequest)` exposes the per‑subnet directory so children can discover peers.

Use `PageRequest { limit, offset }` to avoid passing raw integers into queries.

## Tooling & DX

* Format: `cargo fmt --all` (or `make fmt`)
* Fmt check: `make fmt-check`
* Check (type‑check only): `make check`
* Lint: `make clippy`
* Test: `make test`
* Build workspace release artifacts: `make build`
* Build local canister WASMs through `dfx`: `dfx build --all`
* Build example targets: `cargo build -p canic --examples`
* Role-attestation PocketIC flow: `cargo test -p canic-core --test pic_role_attestation capability_endpoint_policy_and_structural_paths -- --nocapture`
* Root replay dispatcher coverage: `cargo test -p canic-tests --test root_suite --locked upgrade_routes_through_dispatcher_non_skip_path -- --nocapture --test-threads=1`

`rust-toolchain.toml` pins the toolchain so CI and local builds stay in sync.

## Examples

Explore the runnable example under `crates/canic/examples/`:

* `minimal_root.rs` – bootstrap a bare‑bones orchestrator.

```bash
cargo run -p canic --example minimal_root --features control-plane
```

## Project Status & Contributing

Canic is the successor to the internal ICU toolkit. The repository is in the process of being opened for wider use; issues and PRs are currently limited to the core team. Follow `AGENTS.md`, `CONFIG.md`, and the CI scripts under `scripts/ci/` for workflow expectations.

## License

MIT. See `LICENSE` for details.
