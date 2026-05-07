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

* **Lifecycle and build macros**: `canic::start!`, `canic::start_root!`, `canic::build!`, `canic::build_root!`, and `canic::build_standalone!` wire IC hooks, endpoint bundles, and compile-time config validation.
* **Topology-aware config**: `canic.toml` describes subnets, roles, singleton/replica/shard/instance placement, warm pools, scaling pools, sharding pools, and directory pools.
* **Layered runtime APIs**: endpoint guards delegate into `workflow`, `policy`, `ops`, and storage-owned model state instead of mixing orchestration into canister methods.
* **Self-validating delegated auth**: root signs shard certificates, shards mint user tokens, and verifiers validate token + embedded proof with local root/shard key material. Verifiers do not require proof fanout or proof caches.
* **Stable memory helpers**: `ic_memory!`, `ic_memory_range!`, and `eager_static!` wrap stable structures and upgrade-safe runtime state.
* **Thin-root release flow**: the `canic` CLI builds child WASMs, stages release sets through the implicit `wasm_store`, and keeps ordinary child artifacts out of the root Wasm.
* **Operator CLI**: the `canic` binary builds Canic artifacts, lists registered fleets, captures topology-aware canister snapshots, validates backup manifests, and drives guarded restore planning/journals.
* **CI-oriented tooling**: Rust 2024, repo toolchain pinned to Rust `1.95.0`, published MSRV `1.91.0`, and standard `make` targets for format, lint, check, test, and build.

## 📁 Repository Layout

All Rust workspace crates live under `crates/`, but they fall into separate
roles:

**Canister author/runtime crates**

* `crates/canic/` – public facade crate, macros, endpoint bundles, and protocol constants.
* `crates/canic-core/` – shared canister runtime foundation: config, lifecycle, ingress limits, auth, storage, workflow, DTOs, and IDs.
* `crates/canic-cdk/` – curated IC CDK facade used by Canic runtime crates.
* `crates/canic-memory/` – standalone stable-memory helpers.
* `crates/canic-macros/` – proc macros behind the public `canic` facade.

**Control-plane canister crates**

* `crates/canic-control-plane/` – root/control-plane runtime support built on `canic-core`.
* `crates/canic-wasm-store/` – canonical implicit bootstrap `wasm_store` canister crate.

**Host/operator crates**

* `crates/canic-cli/` – published `canic` operator binary for build, install, fleet listing, snapshot, backup, manifest, and restore workflows.
* `crates/canic-host/` – host-side build/install/fleet/release-set library used by `canic` and scripts.
* `crates/canic-backup/` – backup/restore domain library for manifests, journals, topology snapshots, layout verification, and restore planning.

**Testing crates**

* `crates/canic-testkit/` – public PocketIC helpers for downstream tests.
* `crates/canic-testing-internal/` and `crates/canic-tests/` – repo-only PocketIC harnesses and integration tests.

The crate directory is intentionally still flat. Cargo, publishing, and
`[patch.crates-io]` paths stay simpler this way, while crate names and this
taxonomy carry the role boundary. If the workspace grows enough that scanning
`crates/` becomes painful, the next step would be a deliberate directory split
such as `crates/runtime/`, `crates/host/`, and `crates/testing/`; that should be
treated as a repo-structure migration rather than a naming cleanup.

* `canisters/demo/` – local reference topology for root, app, user shard/hub, scaling, and minimal baselines.
* `canisters/test/` and `canisters/audit/` – repo-only correctness canisters and audit probes.
* `canisters/sandbox/minimal/` – manual local sandbox canister for temporary endpoint experiments.
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

Tree canisters should declare a config file, usually named `canic.toml`. Use one of the provided build macros:

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

For a temporary sandbox, probe, or one-off local canister that is not the root
of a configured tree, use a generated standalone config instead:

```rust
// Standalone non-root build.rs
fn main() {
    canic::build_standalone!("sandbox_minimal");
}
```

`build_standalone!` generates a minimal topology containing `root` and the
requested non-root role. If a local `canic.toml` exists, it is used instead.
If `CANIC_CONFIG_PATH` is set, the build remains strict and the explicit
config path must exist.

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

See `canisters/demo/root` and the reference canisters under `canisters/demo/*` for end‑to‑end patterns, including managed `wasm_store` publication and endpoint exports.

### 4. Define your topology

Populate `canic.toml` with subnet definitions, role policies, index exposure, and pool settings. Each `[subnets.<name>]` block lists bootstrap roles and subnet index roles, then nests `[subnets.<name>.canisters.<role>]` tables for cycles, randomness, sharding, scaling, directory pools, and delegated-auth role behavior. The full schema lives in `CONFIG.md`.

### 5. Local Build and Install

To get the `canic` operator binary from a checkout:

```bash
make install
canic help
```

Without `make`, the equivalent command is:

```bash
cargo install --locked --path crates/canic-cli
```

After a release is published, install the same binary from crates.io with:

```bash
cargo install --locked canic-cli --version <same-version-as-canic>
```

Use `canic help` or `canic <command> help` for command-specific options, and
`canic --version` to print the installed CLI version. The first operational
commands are covered in the snapshot/restore flow below.

For local DFX workflows, prefer the shared setup script:

```bash
curl -fsSL https://raw.githubusercontent.com/dragginzgame/canic/v0.32.0/scripts/dev/install_dev.sh | bash
```

The script installs Rust when needed, the repo-local Rust `1.95.0` toolchain, `wasm32-unknown-unknown`, `rustfmt`, `clippy`, Candid/wasm utilities, `actionlint`, common Cargo helper tools, and `dfx` when missing.

It also installs `canic-cli` as the `canic` command.

Published crates still declare MSRV `1.91.0` for downstream source builds.

The setup script installs tools only; it does not start a local `dfx` replica. When run from a repo checkout, it also configures `.githooks/` if present.

The normal interface is the `canic` binary:

```bash
canic build root
canic install
```

`canic install` owns the local thin-root flow: create local canisters, build `root` plus ordinary roles from the subnet that owns `root`, emit `.dfx/local/canisters/root/root.release-set.json`, reinstall `root`, stage the ordinary release set, resume bootstrap, and wait for `canic_ready`.
After a successful install, Canic writes project-local fleet state under
`.canic/<network>/fleets/<fleet>.json` and marks that fleet current for the
network. That state records the selected root target, resolved root principal,
build target, config path, and release-set manifest path so later commands know
which installed Canic fleet this project is using.

The root target defaults to the `root` dfx canister name. To follow normal IC
operator style, you may pass either a canister name or a principal:

```bash
canic install root
canic install uxrrr-q7777-77774-qaaaq-cai
canic install --root uxrrr-q7777-77774-qaaaq-cai
canic install --config canisters/demo/canic.toml
```

Config selection is explicit when more than one topology could apply.
`canic install` uses `canisters/canic.toml` when that project default exists.
Otherwise it prints the discovered config choices and asks you to pass
`--config <path>`:

```bash
canic install --config canisters/demo/canic.toml
```

Install configs must declare the fleet identity that will be written to
project-local state:

```toml
[fleet]
name = "demo"
```

Use `canic fleets` to list installed fleets for the current network, and
`canic use <fleet>` to switch the default fleet used by commands such as
`canic list`:

```bash
canic fleets --network local
canic use demo --network local
```

For `DFX_NETWORK=local`, the install flow attempts one clean local `dfx`
recovery if `dfx ping local` fails. Nonlocal targets must be managed
externally.

`root` embeds only the bootstrap `wasm_store.wasm.gz`; ordinary child releases stay outside `root` and are staged after install. Visible canister Candid files are generated under `.dfx/local/canisters/<role>/<role>.did`. The checked-in exception is `crates/canic-wasm-store/wasm_store.did`, the canonical interface for the implicit bootstrap `wasm_store` crate.

For build profiles, split workspace/DFX roots, custom canister roots, role
metadata, and lower-level build/install commands, see
`crates/canic-host/README.md`.

### 6. Operator Snapshot and Restore Flow

The `canic` binary is the operator entry point for fleet backup/restore work.
It still uses `dfx` for live IC snapshot operations, but it owns the higher-level
topology selection, manifests, journals, backup verification, and restore
planning.

Show local demo canisters that already have ids:

```bash
canic list --network local
```

If this only prints the `root` row, `dfx` has reserved the root id but the Canic
tree is not installed yet. Run `canic install`, then query the installed
registry with `canic list --network local`. List output uses the canister
principal as the first column and renders parent/child relationships with
box-drawing tree branches.

Use `--root` to query a specific installed Canic root, and `--from` to render a
subtree with the selected node as the displayed root:

```bash
canic list --root root --from app --network local
canic list --fleet demo --from app --network local
```

The CLI calls `canic_ready` on each listed canister and includes a `READY`
column without failing the whole list for one unavailable canister.

Plan or capture a canister plus its registered children:

```bash
canic snapshot download \
  --canister <canister-id> \
  --root <root-canister-id> \
  --include-children \
  --out backups/<run-id> \
  --dry-run
```

Use `--recursive` for all descendants. Non-dry-run captures recompute the
selected topology immediately before snapshot creation and fail if the topology
hash changed since discovery. Because `dfx` creates snapshots only for stopped
canisters, Canic stops each canister before snapshot creation; pass
`--resume-after-snapshot` when the CLI should start each canister again after
capture.

Validate a captured backup before restore planning:

```bash
canic backup verify \
  --dir backups/<run-id>
```

Restore work is manifest/journal driven. `restore plan`, `restore apply
--dry-run`, and `restore run --dry-run` are no-mutation paths for checking
mappings, ordering, checksums, verification coverage, and runner commands
before execution.

See `crates/canic-cli/README.md` for the operator guide and
`docs/operations/0.31-backup-restore-checklist.md` for the current
backup/restore checklist.

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

### Update Payload Limits 🧱

Every `#[canic_update]` endpoint is registered with a generated ingress payload
limit. The default limit is `16 KiB`, and `canic::start!`,
`canic::start_root!`, and `canic::start_wasm_store!` wire the IC
`inspect_message` hook that rejects oversized ingress before consensus.

Use `payload(max_bytes = ...)` when an endpoint intentionally accepts a larger
request body:

```rust
use canic::{Error, prelude::*};

#[canic_update(payload(max_bytes = 32 * 1024))]
fn import_blob(bytes: Vec<u8>) -> Result<usize, Error> {
    Ok(bytes.len())
}
```

The payload check applies to ingress update calls. It is a pre-consensus
admission guard, not an in-canister audit log; rejected oversized ingress does
not enter replicated execution.

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
