# canic-core

Core orchestration logic for Canic canisters: config handling, ops layer, registries, and IC interface helpers.

Most canister projects should depend on `canic` (the facade crate) and use:
- `canic::build!` / `canic::build_root!` from `build.rs` to validate/embed `canic.toml`
- `canic::start!` / `canic::start_root!` from `lib.rs` to wire init/upgrade and export endpoints

`canic-core` is still published because it holds the underlying building blocks:
typed config, auth/policy helpers, stable-memory backed registries, and the `ops/` workflows.

See `../../README.md` for the workspace overview and `../../CONFIG.md` for the `canic.toml` schema.

## Architecture

Canic is intentionally layered to keep your boundary surface small and your policies centralized:

- `access/` – authorization and guard helpers used by endpoint macros.
- `config/` – parse + validate `canic.toml` into a typed schema.
- `model/` – stable storage (`model::memory`) and in-process registries/caches (non-memory).
- `ops/` – business logic over model + IC management calls (provisioning, pools, topology sync).
- `macros/` – macro entrypoints and generated endpoint bundles.

The default flow is: endpoints → ops → model.

## Module Map

- `canic_core::access::{auth, guard, policy}` – common auth and routing checks.
- `canic_core::config` – config loader/schema and validation errors.
- `canic_core::dto` – candid-friendly DTOs for paging and exports.
- `canic_core::env` – curated canister ID constants and helpers.
- `canic_core::ids` – typed identifiers (`CanisterRole`, `SubnetRole`, etc.).
- `canic_core::log` / `canic_core::perf` – logging + perf instrumentation helpers.
- `canic_core::ops` – orchestration workflows (IC calls, sharding/scaling/reserve, WASM registry).
- `canic_core::spec` – representations of external IC standards/specs (ICRC, NNS/SNS, etc.).

## Quick Start (Typical Canister)

In `build.rs`:

```rust
fn main() {
    canic::build!("../canic.toml");
}
```

In `src/lib.rs`:

```rust
use canic::prelude::*;
use canic::core::ids::CanisterRole;

const APP: CanisterRole = CanisterRole::new("app");

canic::start!(APP);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}
```
