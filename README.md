<p align="center">
  <img src="assets/canic_logo.svg" alt="Canic logo" width="360" />
</p>

# 🧑‍🔧 Canic 🧑‍🔧 – Internet Computer Orchestration

[![Crates.io](https://img.shields.io/crates/v/canic.svg)](https://crates.io/crates/canic)
[![Docs.rs](https://docs.rs/canic/badge.svg)](https://docs.rs/canic)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Canic is a Rust toolkit for orchestrating Internet Computer (IC) canister fleets. It provides lifecycle macros, validated topology config, stable-memory helpers, endpoint guards, release-set tooling, and root/bootstrap workflows for multi-canister systems.

The crate was historically known as **ICU** (Internet Computer Utilities). All core APIs have been renamed to **Canic** for the crates.io release.

## Highlights

* **Lifecycle and build macros**: `canic::start!`, `canic::start_root!`, `canic::build!`, and `canic::build_root!` wire IC hooks, endpoint bundles, and compile-time config validation.
* **Topology-aware config**: `canic.toml` describes subnets, roles, singleton/replica/shard/instance placement, warm pools, scaling pools, sharding pools, and directory pools.
* **Layered runtime APIs**: endpoint guards delegate into `workflow`, `policy`, `ops`, and storage-owned model state instead of mixing orchestration into canister methods.
* **Self-validating delegated auth**: root signs shard certificates, shards mint user tokens, and verifiers validate token + embedded proof with local root/shard key material. Verifiers do not require proof fanout or proof caches.
* **Stable memory helpers**: `ic_memory!`, `ic_memory_range!`, and `eager_static!` wrap stable structures and upgrade-safe runtime state.
* **Thin-root release flow**: `canic-installer` builds child WASMs, stages release sets through the implicit `wasm_store`, and keeps ordinary child artifacts out of the root Wasm.
* **CI-oriented tooling**: Rust 2024, repo toolchain pinned to Rust `1.95.0`, published MSRV `1.91.0`, and standard `make` targets for format, lint, check, test, and build.

## 📁 Repository Layout

* `crates/canic/` – public facade crate, macros, endpoint bundles, and protocol constants.
* `crates/canic-core/` – runtime orchestration, config validation, auth, storage, workflow, and endpoint-adjacent APIs.
* `crates/canic-cdk/` – curated IC CDK facade used by Canic runtime crates.
* `crates/canic-memory/` – standalone stable-memory helpers.
* `crates/canic-installer/` – published build/install/release-set CLIs.
* `crates/canic-control-plane/` – root/control-plane runtime support.
* `crates/canic-wasm-store/` – canonical implicit bootstrap `wasm_store` canister crate.
* `crates/canic-testkit/` – public PocketIC helpers for downstream tests.
* `crates/canic-testing-internal/`, `crates/canic-tests/`, `crates/canic-core/test-canisters/`, and `crates/canic-core/audit-canisters/` – repo-only integration, correctness, and audit fixtures.
* `canisters/` – reference canisters for root, app, user shard/hub, scaling, minimal baselines, and shared support.
* `scripts/` – dev setup, CI, release, wasm, and audit helpers.
* `assets/`, `docs/`, `.github/workflows/` – documentation assets, design/audit notes, and CI.

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

Populate `canic.toml` with subnet definitions, role policies, index exposure, and pool settings. Each `[subnets.<name>]` block lists bootstrap roles and subnet index roles, then nests `[subnets.<name>.canisters.<role>]` tables for cycles, randomness, sharding, scaling, directory pools, and delegated-auth role behavior. The full schema lives in `CONFIG.md`.

### 5. Local Build and Install

For local DFX workflows, prefer the shared setup script:

```bash
curl -fsSL https://raw.githubusercontent.com/dragginzgame/canic/v0.30.6/scripts/dev/install_dev.sh | bash
```

The script installs Rust when needed, the repo-local Rust `1.95.0` toolchain, `wasm32-unknown-unknown`, `rustfmt`, `clippy`, Candid/wasm utilities, `actionlint`, common Cargo helper tools, and `dfx` when missing.

It also installs:

- `canic-installer` `0.30.6`

Published crates still declare MSRV `1.91.0` for downstream source builds.

The setup script installs tools only; it does not start a local `dfx` replica. When run from a repo checkout, it also configures `.githooks/` if present.

If you only want the thin-root helper without the broader setup path, you can still install it directly:

```bash
cargo install --locked canic-installer --version <same-version-as-canic>
```

Then, from your workspace root:

```bash
canic-install-root root
```

`canic-install-root` owns the local thin-root flow: create local canisters, build `root` plus ordinary roles from the subnet that owns `root`, emit `.dfx/local/canisters/root/root.release-set.json`, reinstall `root`, stage the ordinary release set, resume bootstrap, and wait for `canic_ready`.

For `DFX_NETWORK=local`, the installer attempts one clean local `dfx` recovery if `dfx ping local` fails. Nonlocal targets must be managed externally.

`root` embeds only the bootstrap `wasm_store.wasm.gz`; ordinary child releases stay outside `root` and are staged after install. Visible canister Candid files are generated under `.dfx/local/canisters/<role>/<role>.did`. The checked-in exception is `crates/canic-wasm-store/wasm_store.did`, the canonical interface for the implicit bootstrap `wasm_store` crate.

Canic treats wasm build selection as an explicit three-profile contract:

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

`CANIC_WORKSPACE_ROOT` controls Cargo, `canic.toml`, and canister manifests. `CANIC_DFX_ROOT` controls `dfx.json`, `.dfx`, emitted artifacts, and the generated bootstrap-store wrapper.

If your canister crates do not live under the default `canisters/` directory, Canic tries Cargo workspace metadata first. Usually no extra config is needed when package names follow the `canister_<role>` convention, even in nested paths such as `src/canisters/project/ledger`.

If you need to override discovery explicitly, set:

```bash
CANIC_CANISTERS_ROOT=src/canisters
```

relative to `CANIC_WORKSPACE_ROOT`, or point `CANIC_CONFIG_PATH` at the real `canic.toml` location and Canic will infer the canister-manifest root from that config path.

If a package name does not follow `canister_<role>`, declare the mapping in its
`Cargo.toml`:

```toml
[package.metadata.canic]
role = "project_ledger"
```

`canic-installer` also publishes lower-level build/install commands:

- `canic-build-canister-artifact`
- `canic-build-wasm-store-artifact`
- `canic-emit-root-release-set-manifest`
- `canic-list-install-targets`
- `canic-stage-root-release-set`

`canic-list-install-targets` prints `root` first, then ordinary roles from the single subnet that owns `root`. It excludes the implicit bootstrap `wasm_store`.

If you are writing host-side PocketIC tests against Canic, prefer
`crates/canic-testkit/` for the public wrapper surface. The unpublished
`crates/canic-testing-internal/` crate owns Canic's heavier root/auth harnesses
and other repo-only fixtures.

## Layered Architecture

Canic follows the layering rules in `AGENTS.md`. Dependencies flow downward, and endpoint/boundary code must not reach directly into authoritative state.

* `storage/` and runtime registries – authoritative persisted or in-memory state, including stable-memory layout and local invariants.
* `view/` – internal read-only projections consumed by `ops`, `workflow`, and pure decision helpers.
* `ops/` – deterministic application services plus approved single-step platform effects.
* `policy/` and pure helpers – deterministic decision/value logic with no mutation or IC calls.
* `workflow/` – orchestration and multi-step behavior over time.
* `access/` plus macro-generated endpoints – request guards and system-boundary wiring that delegate immediately to `workflow` or `ops`.

## Capabilities & Endpoints

### Delegated Auth 🔐

Delegated auth is self-validating. Root canisters issue signed shard delegation certificates, shard canisters mint user-bound `DelegatedToken` values, and verifier canisters validate the token plus embedded proof locally. Verification does not require verifier-local proof caches, proof fanout, or creation-time catch-up.

Authenticated endpoints enforce:

- caller-subject binding (`token.claims.subject == caller`)
- explicit audience membership (`self in token.claims.aud`)
- required scope binding (`required_scope in token.claims.scopes`)
- root signature, shard signature, key-binding, and token/cert expiry checks

Reference contracts:
- `docs/contracts/AUTH_DELEGATED_SIGNATURES.md`
- `docs/contracts/ACCESS_ARCHITECTURE.md`
- `docs/architecture/authentication.md`

### Sharding 📦

Sharding is configured via `canic.toml` and executed through the ops layer. Canisters only export sharding registry endpoints when their validated role config includes sharding support.

```rust
canic_sharding_registry()
    -> Result<canic::dto::placement::sharding::ShardingRegistryResponse, canic::Error>
```

### Scaling & Pool Capacity ⚖️

* `canic_scaling_registry()` is exported only for roles whose config enables scaling.
* `canic_pool_list()` and the controller‑only `canic_pool_admin(cmd)` are root-only endpoints for spare-capacity management.

### Index Listings 📇

* `canic_app_index(PageRequest)` returns the prime root index listing for operator dashboards.
* `canic_subnet_index(PageRequest)` exposes the per-subnet index so children can discover peers.

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

`rust-toolchain.toml` pins the internal toolchain so CI and local builds stay in sync.
Published crates declare MSRV `1.91.0` separately through `workspace.package.rust-version`.

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
