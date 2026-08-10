# Canister Runtime

Canic's public `canic` crate is the normal integration point for Rust canister
packages. It keeps lifecycle and generated configuration wiring small while
leaving application business logic in the consuming crate.

## What It Provides

- `canic::build!(...)` for compile-time App and role configuration
- `canic::start!()` for Canic lifecycle restoration and endpoint wiring
- stable-memory helpers under `canic::memory`
- non-overlapping application timers with explicit cancellation handles
- typed inter-canister calls with Canic metrics
- optional endpoint bundles selected with Cargo features

Use the same `canic` version in normal and build dependencies. Each canister
package declares its App and role through `[package.metadata.canic]`; the App
name must match the selected `canic.toml`.

## Boundary

The facade owns framework lifecycle invariants, not application state
machines. Lifecycle adapters restore synchronously and schedule user hooks;
application hooks run later and should be idempotent. Shared domain libraries
should remain framework-independent rather than depending on the facade.

## Start Here

- [Installing Canic](../../../INSTALLING.md#add-canic-to-canister-crates)
- [Facade crate guide](../../../crates/canic/README.md)
- [Minimal managed Fleet](../../getting-started/minimal-managed-fleet.md)
- [Configuration reference](../../../CONFIG.md)
- [Runtime architecture contract](../../contracts/ARCHITECTURE.md)
