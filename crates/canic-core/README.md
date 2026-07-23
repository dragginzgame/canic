# canic-core

Core orchestration logic for Canic canisters: config handling, ops layer, registries, and IC interface helpers.

Most canister projects should depend on `canic` (the facade crate) and use:
- `canic::build!` from `build.rs` to validate/embed `canic.toml`
- `canic::start!` from `lib.rs` to wire init/upgrade and export endpoints
- `[package.metadata.canic] app = "..."` and `role = "..."` in `Cargo.toml`
  to select the App-scoped canister role

`canic-core` is still published because it holds the underlying building blocks:
typed config, auth/decision helpers, storage/view layers, and the workflow and
ops internals that power the facade crate.

See `../../README.md` for the workspace overview and `../../CONFIG.md` for the `canic.toml` schema.

## Architecture

Canic is intentionally layered to keep the boundary surface small and ownership explicit:

- `access/` – authorization and guard helpers used by endpoint macros.
- `config/` – parse + validate `canic.toml` into a typed schema.
- `workflow/` – orchestration, retries, and multi-step behavior over time.
- `domain/policy/pure/` – pure decisions invoked by workflow.
- `ops/` – deterministic services over stored/runtime state plus approved
  single-step platform effects.
- `model/` – authoritative runtime state and storage invariants.
- `storage/` – passive persisted schemas and stable-memory representations;
  model retains invariant ownership and ops owns access/conversion.
- `view/` – internal read-only projections over stored/runtime state.

The dependency flow is: endpoints → workflow → policy → ops → model.

## Module Map

- `canic_core::access` – common auth and routing checks used by the facade macros.
- `canic_core::api` – runtime APIs surfaced through `canic::api::*`.
- `canic_core::dto` – candid-friendly DTOs for paging, auth, topology, metrics, and RPC.
- `canic_core::ids` – typed identifiers (`CanisterRole`, `SubnetSlotId`, etc.).
- `canic_core::log` / `canic_core::perf` – logging and perf instrumentation helpers.
- `canic_core::protocol` – protocol constants and runtime service identifiers.

## Quick Start (Typical Canister)

Make sure `canic` is available in both `[dependencies]` and `[build-dependencies]`, because the `build!` macros run inside `build.rs`.

In `Cargo.toml`:

```toml
[package.metadata.canic]
app = "demo"
role = "app"
```

In `build.rs`:

```rust
fn main() {
    canic::build!("../canic.toml");
}
```

In `src/lib.rs`:

```rust
use canic::prelude::*;

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

canic::finish!();
```
