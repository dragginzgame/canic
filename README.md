<p align="center">
  <img src="assets/canic_logo.svg" alt="Canic logo" width="360" />
</p>

# 🧑‍🔧 Canic 🧑‍🔧 – Internet Computer Orchestration

[![Crates.io](https://img.shields.io/crates/v/canic.svg)](https://crates.io/crates/canic)
[![Docs.rs](https://docs.rs/canic/badge.svg)](https://docs.rs/canic)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Canic is a Rust toolkit and operator CLI for Internet Computer canister fleets.
It gives canister crates lifecycle macros, validated topology config,
stable-memory helpers, endpoint guards, thin-root artifact builds, local fleet
install, snapshot, backup, and restore workflows.

Install the operator binary with Cargo:

```bash
cargo install --locked canic-cli --version <same-version-as-canic>
canic --version
```

When working from this checkout:

```bash
make install
```

## Highlights

* **Lifecycle and build macros**: `canic::start!`, `canic::start_root!`, `canic::build!`, `canic::build_root!`, and `canic::build_standalone!` wire IC hooks, endpoint bundles, and compile-time config validation.
* **Topology-aware config**: `canic.toml` describes subnets, roles, singleton/replica/shard/instance placement, warm pools, scaling pools, sharding pools, and directory pools.
* **Self-validating delegated auth**: root signs shard certificates, shards mint user tokens, and verifiers validate token + embedded proof with local root/shard key material. Verifiers do not require proof fanout or proof caches.
* **Stable memory helpers**: `ic_memory!`, `ic_memory_range!`, and `eager_static!` wrap stable structures and upgrade-safe runtime state.
* **Thin-root install flow**: the `canic` CLI builds child WASMs, stages ordinary fleet artifacts through the implicit `wasm_store`, and keeps child artifacts out of the root Wasm.
* **Operator CLI**: the `canic` binary builds artifacts, manages local fleet configs and replica status, installs fleets, captures topology-aware snapshots, validates backup manifests, and drives guarded restore planning/journals.

## 📁 Repository Layout

All Rust workspace crates live under `crates/`:

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

* `crates/canic-cli/` – published `canic` operator binary for install, fleet, replica/status, snapshot, backup, manifest, and restore workflows.
* `crates/canic-host/` – host-side build, install, fleet, and thin-root staging library used by `canic` and scripts.
* `crates/canic-backup/` – backup/restore domain library for manifests, journals, topology snapshots, layout verification, and restore planning.
* `crates/canic-testkit/` – public PocketIC helpers for downstream tests.
* `crates/canic-testing-internal/` and `crates/canic-tests/` – repo-only PocketIC harnesses and integration tests.

* `fleets/test/` – config-defined reference topology used by local ICP CLI, CI wasm builds, and repo tests.
* `fleets/demo/` – minimal root-plus-app fleet for quick experiments.
* `canisters/audit/`, `canisters/sandbox/`, and `canisters/test/` – runnable canisters that are not Canic fleets. See `TESTING.md` for placement rules.
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

The macro validates the TOML during compilation and embeds the canonical config
path for runtime diagnostics.

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

See `fleets/test/root` and the reference canisters under `fleets/test/*` for end-to-end patterns, including managed `wasm_store` publication and endpoint exports.

### 4. Define your topology

Populate `canic.toml` with subnet definitions, role policies, index exposure, and pool settings. Each `[subnets.<name>]` block lists bootstrap roles and subnet index roles, then nests `[subnets.<name>.canisters.<role>]` tables for cycles, randomness, sharding, scaling, directory pools, and delegated-auth role behavior. The full schema lives in `CONFIG.md`.

### 5. Install the Operator CLI

Install the `canic` binary from crates.io with the same version as your `canic`
crate dependency:

```bash
cargo install --locked canic-cli --version <same-version-as-canic>
```

From this repository checkout, use:

```bash
make install
```

For the full repo development toolchain, including pinned ICP CLI tooling and
wasm/Candid helpers, run:

```bash
make install-dev
```

The setup commands install tools only; they do not start a local replica. Use
explicit replica commands when needed:

```bash
canic replica status
canic replica start --background
canic replica start --port 8001 --background
```

The common local loop is:

```bash
canic status
canic install --profile fast test
canic list test
```

Build one canister artifact without a full install:

```bash
canic build --profile fast app
```

For split repos, pass paths as flags instead of exporting build environment
variables:

```bash
canic --network local build \
  --profile fast \
  --workspace backend \
  --icp-root . \
  --config backend/src/canisters/canic.toml \
  root
```

`canic install` owns the local thin-root flow: create local canisters, build
`root` plus ordinary roles from the subnet that owns `root`, emit the root
staging manifest, reinstall `root`, stage the ordinary fleet artifacts, resume
bootstrap, and wait for `canic_ready`.

Fleet selection is explicit. `canic install <fleet>` uses
`fleets/<fleet>/canic.toml`, the conventional `root` ICP canister name, and
Canic's built-in readiness timeout:

```bash
canic install test
```

After a successful install, Canic writes project-local fleet state under
`.canic/<network>/fleets/<fleet>.json`. That state records the selected root
target, resolved root principal, build target, config path, and staging
manifest path so later commands can inspect the explicitly named fleet.

The local ICP CLI replica does not persist canister state across stop/start.
If `canic status` shows a local fleet as `lost`, the recorded root canister is
gone from the restarted local replica; run `canic install <fleet>` to recreate
the local deployment.

Install configs must declare the fleet identity that will be written to
project-local state:

```toml
[fleet]
name = "test"
```

Use `canic fleet list` to list config-defined fleets. Use `canic config <fleet>`
for declared config, and pass `<fleet>` as the first argument to deployed-fleet
commands. Use `canic fleet delete <fleet>` to remove a config-defined fleet
directory after confirming the exact fleet name:

```bash
canic config test
canic list test
canic status
canic --network local fleet list
canic fleet create demo --yes
canic fleet delete demo
```

Use `canic medic` when the local project state, replica, or named fleet does
not look right:

```bash
canic medic test
```

Named-fleet commands default to the local ICP CLI environment. Pass top-level
`--network <name>` for one command against another configured ICP CLI
environment. Nonlocal targets must be managed externally.

`root` embeds only the bootstrap `wasm_store.wasm.gz`; ordinary child releases
stay outside `root` and are staged after install. Visible canister Candid files
are generated under `.icp/local/canisters/<role>/<role>.did`. The checked-in
exception is `crates/canic-wasm-store/wasm_store.did`, the canonical interface
for the implicit bootstrap `wasm_store` crate.

For build profiles, split workspace/ICP roots, custom canister roots, role
metadata, and lower-level build/install commands, see
`crates/canic-host/README.md`.

### 6. Operator Snapshot and Restore Flow

The `canic` binary is the operator entry point for fleet backup/restore work.
It uses ICP CLI for live IC snapshot operations, while Canic owns the higher-level
topology selection, manifests, journals, backup verification, and restore
planning.

Show local test-fleet canisters:

```bash
canic --network local list test
```

If this only prints the `root` row, ICP CLI has reserved the root id but the Canic
tree is not installed yet. Run `canic install test`, then query the installed
registry with `canic --network local list test`.

Use `--subtree` to render one live subtree with the selected node as the
displayed root:

```bash
canic --network local list test --subtree app
```

The CLI calls `canic_ready` on each listed canister and includes a `READY`
column without failing the whole list for one unavailable canister.

Plan or capture a topology-aware fleet backup:

```bash
canic backup create test --dry-run
canic backup create test --subtree app --out backups/<run-id>
```

Non-dry-run backup creation recomputes the selected topology immediately before
snapshot creation and fails if the topology hash changed since discovery.
Because ICP CLI creates snapshots only for stopped canisters, Canic quiesces the
selected members, captures snapshots, restarts them, downloads artifacts,
verifies checksums, and writes the backup manifest plus execution journal.

Validate a captured backup before restore planning:

```bash
canic backup verify backups/<run-id>
```

Restore work is manifest/journal driven. `restore plan`, `restore apply
--dry-run`, and `restore run --dry-run` are no-mutation paths for checking
mappings, ordering, checksums, verification coverage, and runner commands
before execution. `restore run --execute` advances the durable journal through
upload, stop, snapshot load, start, and verification operations.

See `crates/canic-cli/README.md` for the operator guide and
`docs/operations/0.31-backup-restore-checklist.md` for the current
backup/restore checklist.

If you are writing host-side PocketIC tests against Canic, prefer
`crates/canic-testkit/` for the public wrapper surface. The unpublished
`crates/canic-testing-internal/` crate owns Canic's heavier root/auth harnesses
and other repo-only fixtures.

## Architecture And Contracts

Canic follows the layering rules in `AGENTS.md`: endpoints authenticate and
delegate, workflow orchestrates, policy decides, ops performs approved state or
platform actions, and model/storage own invariants.

Reference docs:

* Config schema: `CONFIG.md`
* Build artifacts: `docs/architecture/build-artifacts.md`
* Delegated auth: `docs/contracts/AUTH_DELEGATED_SIGNATURES.md`
* Access architecture: `docs/contracts/ACCESS_ARCHITECTURE.md`
* Authentication overview: `docs/architecture/authentication.md`

## Tooling & DX

* Format: `cargo fmt --all` (or `make fmt`)
* Fmt check: `make fmt-check`
* Check (type‑check only): `make check`
* Lint: `make clippy`
* Test: `make test`
* Build workspace release artifacts: `make build`
* Build local canister artifacts: `canic build --profile fast <role>`
* Build example targets: `cargo build -p canic --examples`
* Role-attestation PocketIC flow: `cargo test -p canic-tests --test pic_role_attestation capability_endpoint_policy_and_structural_paths -- --nocapture --test-threads=1`
* Root replay dispatcher coverage: `cargo test -p canic-tests --test root_suite upgrade_routes_through_dispatcher_non_skip_path -- --nocapture --test-threads=1`

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
