# canic-core

Core orchestration logic for Canic canisters: config handling, ops layer, registries, and IC interface helpers.

Most canister projects should depend on `canic` (the facade crate) and use:
- `canic::build!` / `canic::build_root!` from `build.rs` to validate/embed `canic.toml`
- `canic::start!` / `canic::start_root!` from `lib.rs` to wire init/upgrade and export endpoints

`canic-core` is still published because it holds the underlying building blocks:
typed config, auth/policy helpers, storage/view layers, and the workflow and ops internals that power the facade crate.

See `../../README.md` for the workspace overview and `../../CONFIG.md` for the `canic.toml` schema.

## Architecture

Canic is intentionally layered to keep the boundary surface small and ownership explicit:

- `access/` – authorization and guard helpers used by endpoint macros.
- `config/` – parse + validate `canic.toml` into a typed schema.
- `storage/` – persisted schemas and stable-memory-backed state.
- `view/` – internal read-only projections over storage state.
- `ops/` – deterministic services over storage plus approved single-step platform effects.
- `domain/` – pure domain and policy logic.
- `workflow/` – orchestration, retries, and multi-step behavior over time.

The default flow is: endpoints → workflow → domain/policy → ops → storage.

## Module Map

- `canic_core::access` – common auth and routing checks used by the facade macros.
- `canic_core::api` – runtime APIs surfaced through `canic::api::*`.
- `canic_core::dto` – candid-friendly DTOs for paging, auth, topology, metrics, and RPC.
- `canic_core::ids` – typed identifiers (`CanisterRole`, `SubnetRole`, etc.).
- `canic_core::log` / `canic_core::perf` – logging and perf instrumentation helpers.
- `canic_core::protocol` – protocol constants and runtime service identifiers.

## Quick Start (Typical Canister)

Make sure `canic` is available in both `[dependencies]` and `[build-dependencies]`, because the `build!` macros run inside `build.rs`.

In `build.rs`:

```rust
fn main() {
    canic::build!("../canic.toml");
}
```

In `src/lib.rs`:

```rust
use canic::prelude::*;
use canic::ids::CanisterRole;

const APP: CanisterRole = CanisterRole::new("app");

canic::start!(APP);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}
```
