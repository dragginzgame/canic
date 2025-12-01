<p align="center">
  <img src="assets/2025_12_canic_logo.svg" alt="Canic logo" width="360" />
</p>

# 🧑‍🔧 Canic 🧑‍🔧 – Internet Computer Orchestration

[![Crates.io](https://img.shields.io/crates/v/canic.svg)](https://crates.io/crates/canic)
[![Docs.rs](https://docs.rs/canic/badge.svg)](https://docs.rs/canic)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Canic is a Rust toolkit for orchestrating Internet Computer (IC) canisters at scale. It packages the battle-tested patterns from large multi-canister deployments into a reusable crate: lifecycle macros, stable-memory helpers, orchestration ops, and endpoint bundles that keep your boundary layer thin while enforcing clean layering inside the canister graph.

The crate was historically known as **ICU** (Internet Computer Utilities). All core APIs have been renamed to **Canic** for the crates.io release.

## Highlights

- 🧩 **Bootstrap macros** – `canic::start!`, `canic::start_root!`, `canic_build!`, and `canic_build_root!` wire init/upgrade hooks, export endpoints, and validate config at compile time.
- 🧠 **State layers** – opinionated separation for stable memory, volatile state, ops/business logic, and public endpoints.
- 🗺️ **Topology-aware config** – typed subnet blocks, app directories, and reserve policies validated straight from `canic.toml`.
- 🔐 **Auth utilities** – composable guards (`auth_require_any!`, `auth_require_all!`) for controllers, parents, whitelist principals, and more.
- 🗃️ **Stable memory ergonomics** – `ic_memory!`, `ic_memory_range!`, and `eager_static!` manage IC stable structures safely across upgrades.
- 📦 **WASM registry** – consistently ship/lookup child canister WASMs with hash tracking.
- 🪵 **Configurable logging** – ring/age retention with second-level timestamps and paged log/query helpers.
- ♻️ **Lifecycle helpers** – shard policies, reserve pools, scaling helpers, and sync cascades keep fleets healthy.
- 🧪 **Ready for CI** – Rust 2024 edition, MSRV 1.90, with `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` wired via `make` targets.

## 📁 Repository Layout

- `assets/` – documentation media (logo and shared imagery).
- `crates/` – workspace crates.
  - `canic/` – core library crate with orchestration primitives and macros.
    - `src/auth.rs` & `src/guard.rs` – reusable authorization helpers.
    - `src/cdk/` – IC CDK shims and patched utilities used by the macros.
    - `src/config/` – configuration loaders, validators, and schema helpers.
    - `src/env/` – curated canister ID constants (ck, NNS, SNS) and helpers.
    - `src/interface/` – typed wrappers for IC management calls, ck-ledgers, and ICRC ledgers.
    - `src/log.rs` – logging macros.
    - `src/macros/` – public macro entrypoints (`canic::start!`, `canic_endpoints_*`, memory helpers).
    - `src/memory/` – stable storage abstractions and registries built on `ic-stable-structures`.
    - `src/ops/` – orchestration/business logic bridging memory and state layers.
    - `src/runtime.rs` – runtime glue shared by macros.
    - `src/serialize.rs` – deterministic codecs.
    - `src/spec/` – representations of external IC specs (ICRC, NNS, SNS, etc.).
    - `src/state/` – volatile runtime state caches and registries.
    - `src/types/` – shared domain types.
    - `src/utils/` – time helpers, wasm utilities, etc.
    - `examples/` – runnable demos for guards, shard lifecycle, and canister ops.
  - `canisters/` – reference canisters that exercise the library end to end:
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
- `.cargo/` – workspace Cargo config that pins the tmp dir to avoid cross-device link errors when sandboxed.

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
    canic::build!("../canic.toml");
}
```

The macro validates the TOML during compilation, emits the right `cfg` flags (such as `canic` and `canic_root`), and exposes the canonical config path via `CANIC_CONFIG_PATH`.

### 3. Bootstrap your canister

In `lib.rs`:

```rust
use canic::prelude::*;
use canic::canister::EXAMPLE;

canic::start!(EXAMPLE); // or canic::start_root!() for the orchestrator canister

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}
```

See `crates/canisters/root` and the hub/shard reference canisters under `crates/canisters/*` for end-to-end patterns, including WASM registries and endpoint exports.

### 4. Define your topology

Populate `canic.toml` with subnet definitions, directory membership, and per-canister policies. Each `[subnets.<name>]` block lists `auto_create` and `subnet_directory` canister types, then nests `[subnets.<name>.canisters.<type>]` tables for top-up settings plus optional sharding and scaling pools. Global tables such as `controllers`, `app_directory`, `reserve`, `log`, and `standards` shape the overall cluster. The `[log]` block controls ring/age retention in seconds. The full schema lives in `CONFIG.md`.

## Layered Architecture

Canic enforces clear separation between storage, transient state, orchestration logic, and public endpoints:

- `memory/` – stable data backed by `ic-stable-structures` (e.g. shard registries, reserve pools).
- `state/` – volatile caches and session stores that reset on upgrade.
- `ops/` – business logic tying state + memory together (sharding policies, scaling flows, reserve management).
- `endpoints/` – macro-generated IC entrypoints that delegate to `ops/` and keep boundary code minimal.
- Temporary exception (target revisit in ~2 weeks): when no ops façade exists yet, read-only queries may pull directly from `memory/` or `state/`; mutations should still flow through `ops/`.

## Capabilities & Endpoints

### Sharding 📦

`canic::ops::ext::sharding` assigns tenants to shard canisters according to a `ShardingPolicy` (initial capacity, max shards, growth thresholds). Admin work flows through a single controller-only endpoint:

```rust
canic_sharding_admin(cmd: canic::ops::ext::sharding::AdminCommand)
    -> Result<canic::ops::ext::sharding::AdminResult, canic::Error>
```

Command variants cover register, audit, drain, rebalance, and decommission flows. Your application is responsible for data migration around these moves.

### Scaling & Reserve Pools ⚖️

- `canic_scaling_registry()` provides controller insight into scaling pools via the shared endpoint bundle.
- Root canisters manage spare capacity through `canic::ops::root::reserve` and the `canic_reserve_*` endpoints.

### Directory Views 📇

- `canic_app_directory()` returns the prime root directory view for operator dashboards.
- `canic_subnet_directory()` exposes the per-subnet directory so children can discover peers.

### ICRC Support 📚

The base endpoint bundle includes:

- `icrc10_supported_standards()`
- `icrc21_canister_call_consent_message(request)`

Register consent messages via `state::icrc::Icrc21Registry` for rich UX flows.

## Tooling & DX

- Format: `cargo fmt --all` (or `make fmt`)
- Fmt check: `make fmt-check`
- Check (type-check only): `make check`
- Lint: `make clippy`
- Test: `make test`
- Build release WASMs: `make build`
- Run the example suite: `make examples` or `cargo build -p canic --examples`

Sandboxed builds can hit `Invalid cross-device link` during Cargo’s atomic renames. Pin the target/temp dirs to the workspace when sandboxed:

```bash
CARGO_TARGET_DIR=$PWD/target_tmp TMPDIR=$PWD/target_tmp cargo build -p canic --examples
```

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

MIT. See `LICENSE` for details.
