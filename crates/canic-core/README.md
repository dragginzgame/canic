# canic-core

`canic-core` contains Canic's shared internal runtime and orchestration logic,
including compiled configuration, decisions, multi-step workflows, persistent
state, registries, and IC interface helpers.

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
- `config/` – validate and expose the typed configuration compiled by the
  host-side build path. TOML parsing does not enter deployed Wasm.
- `workflow/` – orchestration, retries, and multi-step behavior over time.
- `domain/policy/pure/` – pure decisions invoked by workflow.
- `ops/` – deterministic services over stored/runtime state plus approved
  single-step platform effects.
- `model/` – authoritative runtime state and storage invariants.
- `storage/` – passive persisted schemas and stable-memory representations;
  model retains invariant ownership and ops owns access/conversion.
- `view/` – internal read-only projections over stored/runtime state.

The dependency flow is:

```text
endpoints -> workflow -> policy
                     +-> ops -> model
```

Workflow may call policy and ops independently. Policy is pure and never calls
ops.

## Module Map

- `canic_core::access` – common auth and routing checks used by the facade macros.
- `canic_core::api` – runtime APIs surfaced through `canic::api::*`.
- `canic-contracts` owns passive DTOs, shared IDs, diagnostics and wire names.
  Core depends on these declarations for its runtime behavior.
- `canic_core::ids` owns runtime intent identities.
- `canic_core::log` / `canic_core::perf` – logging and perf instrumentation helpers.
- `canic_contracts::protocol` – canonical endpoint names and protocol identities.

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

## Continue From Here

- [Explore runtime features](../../docs/features/runtime/README.md)
- [Read the architecture overview](../../docs/architecture/README.md)
- [Browse all documentation](../../docs/README.md)
- [Back to the main README](../../README.md)
