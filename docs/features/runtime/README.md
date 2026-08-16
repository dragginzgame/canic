# Canister Runtime

Canic's public `canic` crate is the normal integration point for Rust canister
packages. It keeps lifecycle and generated configuration wiring small while
leaving application business logic in the consuming crate.

## What It Provides

- `canic::build!(...)` for compile-time App and role configuration
- `canic::start!()` for Canic lifecycle restoration and endpoint wiring
- stable-memory helpers under `canic::memory`
- fallible, non-overlapping application timers with explicit cancellation
  handles and one shared cross-framework inventory through `ic-timers 0.6.1`
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

Dropping a timer handle detaches caller control without cancelling its timer;
call `handle.detach()` to make deliberate detachment explicit, or consume it
through `canic::api::timer::cancel` when cancellation is required.
Every timer-owning crate linked into the final canister must resolve the same
exact `ic-timers` Cargo package ID. Two resolved versions contain two separate
library statics and therefore two inventories; dependency-tree qualification
is part of combined-framework integration.
Runtime status schema 3 projects policy-specific scheduler/work instruction
aggregates plus bounded latest and maximum-growth Wasm/stable-memory page
observations. Page extents are runtime-epoch high-water observations, not live
bytes or exclusive allocation attribution for asynchronous work.
Each observation brackets the complete accepted shared-runtime callback path,
including registry acceptance, consumer work, completion accounting, and
successor binding; it is not an isolated application-function benchmark. A
transient `RemoveWhenStopped` declaration disappears from the shared inventory
when it reaches terminal state, along with its final status and performance
sample. Retained declarations preserve their normally completed observations.

Authority snapshots currently fail closed when the shared registry contains a
timer claim outside Canic custody. Combined-framework snapshot composition is
not inferred from shared observation; it requires the separately qualified
synchronous lifecycle seam before Canic can reconstruct another owner's
volatile claims after upgrade.

## Timer Reliability Classes

The shared registry makes every owner observable; it does not give every timer
the same recovery guarantee. Current Canic jobs are classified as follows:

| Owner | Policy | Reliability contract |
| --- | --- | --- |
| Application one-shots and intervals | `Once` / `AfterCompletion` | Application-owned and fail-stop after a trap or instruction exhaustion |
| Deferred lifecycle hooks | `Once` | Activation/readiness must expose an explicit retry path; automatic trap recovery is not provided |
| Runtime-log retention | retained `Once` | Advisory cleanup; lifecycle restoration or a later append reconstructs its exact deadline |
| Intent cleanup | retained `Once` | Durable expiry indexes reconstruct demand, but trap/exhaustion liveness is not yet qualified |
| Root issuer renewal | retained `Once` | Durable attempt lease and operation generation; the pre-armed watchdog takes over an orphan after five minutes while the renewal batch preserves its exact work |
| Automatic cycle top-up | retained `Once` | Durable attempt lease and exact retry operation ID carried into the replay-protected parent funding request |
| Placement-receipt acknowledgement | retained `Once` | Durable attempt lease around acknowledgement work whose receipt records retain each exact operation ID |
| Root Canister-pool maintenance | retained `AfterCompletion` | Durable attempt lease around journaled maintenance, with watchdog-owned 30-second recurrence after takeover |

One retained `canic/async_recovery/watchdog` is pre-armed before it dispatches
fallible work and advances every 30 seconds independently of the worker
continuation. The four recovery-critical owners share a private bounded stable
record at core memory ID 60. It retains exact attempt and operation
generations, a five-minute lease, retry state, watchdog-owned deadlines and
request-ordered scheduling that arrives during an active attempt. A live lease
coalesces competing dispatch; an expired lease admits one exact-operation
takeover; and stale completion cannot clear or reschedule its successor.

This protocol does not make an untracked spawned future safe by itself. The
pre-armed successor supplies liveness, while durable operation identities and
the owners' replay or journal state supply idempotency if the old continuation
later resumes. Application timers, lifecycle deferrals and advisory cleanup
remain in their separately documented reliability classes.

## Start Here

- [Installing Canic](../../../INSTALLING.md#add-canic-to-canister-crates)
- [Facade crate guide](../../../crates/canic/README.md)
- [Minimal managed Fleet](../../getting-started/minimal-managed-fleet.md)
- [Configuration reference](../../../CONFIG.md)
- [Runtime architecture contract](../../contracts/ARCHITECTURE.md)
