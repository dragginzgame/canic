# Canister Runtime

Canic's public `canic` crate is the normal integration point for Rust canister
packages. It keeps lifecycle and generated configuration wiring small while
leaving application business logic in the consuming crate.

## What It Provides

- `canic::build!(...)` for compile-time App and role configuration
- `canic::start!()` for Canic lifecycle restoration and endpoint wiring
- stable-memory helpers under `canic::memory`
- one shared cross-framework inventory through `ic-timers 0.6.1`, with
  application timer registrations owned directly by their consuming crate
- typed inter-canister calls with Canic metrics
- optional endpoint bundles selected with Cargo features

Use the same `canic` version in normal and build dependencies. Each canister
package declares its App and role through `[package.metadata.canic]`; the App
name must match the selected `canic.toml`.

## Boundary

The facade owns framework lifecycle invariants, not application state
machines. Lifecycle adapters restore synchronously. An optional paired
lifecycle participant reconstructs application-owned volatile runtime state
before Canic schedules deferred user hooks; those async hooks run later and
should be idempotent. Shared domain libraries should remain
framework-independent rather than depending on the facade.

Canic provides no application timer facade. Timer-owning crates use native
`ic-timers` registrations directly and retain a registration whenever later
cancellation or reconciliation is required. Dropping a registration detaches
control without cancelling its timer.
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
not inferred from shared observation. The paired synchronous lifecycle
participant lets each application reconstruct its own volatile claims after
Canic restoration; combined Canic/IcyDB qualification remains a separate
0.104 proof. Auth renewal, automatic cycle top-up and placement acknowledgement
contribute their exact domain-owned native claims directly to Canic's current
snapshot fence rather than through a central timer selector.

## Timer Reliability Classes

The shared registry makes every owner observable; it does not give every timer
the same recovery guarantee. Current Canic jobs are classified as follows:

| Owner | Policy | Reliability contract |
| --- | --- | --- |
| Application one-shots and intervals | `Once` / `AfterCompletion` | Application-owned and fail-stop after a trap or instruction exhaustion |
| Deferred lifecycle hooks | `Once` | Activation/readiness must expose an explicit retry path; automatic trap recovery is not provided |
| Runtime-log retention | retained `Once` | Advisory cleanup; lifecycle restoration or a later append reconstructs its exact deadline |
| Intent cleanup | retained `Once` | Durable expiry indexes reconstruct demand, but trap/exhaustion liveness is not yet qualified |
| Root issuer renewal | domain-owned retained `Once` | Declared lazily from enabled issuer configuration and current proof demand; its durable attempt lease admits watchdog takeover after five minutes while the auth domain's delegation batch and proof state preserve exact work |
| Automatic cycle top-up | domain-owned retained `Once` | Declared only for an `AutomaticTopup` capability and current configuration; its durable attempt lease and exact retry operation ID are carried into the replay-protected parent funding request |
| Placement-receipt acknowledgement | domain-owned retained `Once` | Declared only while the durable terminal-receipt index contains work; its attempt lease surrounds acknowledgement work whose receipt records retain each exact operation ID |
| Root Canister-pool maintenance | retained `AfterCompletion` | Durable attempt lease around journaled maintenance; the native watchdog requests takeover after expiry while pool journals preserve exact external effects |

One retained `canic/async_job_recovery/watchdog` is pre-armed before it
dispatches fallible work and advances every 30 seconds independently of the
worker continuation. The four recovery-critical owners share a private bounded
stable record at core memory ID 60. Every owner retains a checked attempt
generation and optional active lease; only automatic cycle top-up retains a
generated operation generation and pending exact retry. The record contains no
timer command, provider deadline, schedule-owner flag, provider retry streak or
terminal provider condition. A live lease coalesces competing dispatch, an
expired lease admits one takeover, and stale completion cannot clear its
successor. Cycle takeover reuses its exact funding identity; the other domains
use the replay identities in their own durable records.

This protocol does not make an untracked spawned future safe by itself. The
pre-armed successor supplies liveness, while durable operation identities and
the owners' replay or journal state supply idempotency if the old continuation
later resumes. Application timers, lifecycle deferrals and advisory cleanup
remain in their separately documented reliability classes.

## Start Here

- [Installing Canic](../../../INSTALLING.md#add-canic-to-canister-crates)
- [Facade crate guide](../../../crates/canic/README.md)
- [Native timer adoption and lifecycle composition](native-timers.md)
- [Minimal managed Fleet](../../getting-started/minimal-managed-fleet.md)
- [Configuration reference](../../../CONFIG.md)
- [Runtime architecture contract](../../contracts/ARCHITECTURE.md)
- [Scheduled 0.104 timer/lifecycle hard cut](../../design/0.104-ic-timers-consumer-hard-cut/0.104-design.md)
