# Canic 0.102 Small Workflow Constructor Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger classifies ten production `InternalError`
constructor references in five two-site core workflow owners. It assigns no
number and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/cascade/snapshot/mod.rs` | 2 |
| `workflow/cost_guard/mod.rs` | 2 |
| `workflow/rpc/capability/mod.rs` | 2 |
| `workflow/runtime/cycles/mod.rs` | 2 |
| `workflow/runtime/log.rs` | 2 |
| **Total** | **10** |

## Fleet Activation Topology Snapshot

The direct root-to-Store snapshot builder reuses two exact Fleet-activation
topology identities:

| Existing exact identity | Sites | Producer function/branch |
| --- | ---: | --- |
| `FLEET_ACTIVATION_TOPOLOGY_ANONYMOUS_BINDING_PRINCIPAL` | 1 | `TopologySnapshotBuilder::for_direct_leaf`; the Store child principal is anonymous |
| `FLEET_ACTIVATION_TOPOLOGY_ROOT_PRINCIPAL_CONFLICT` | 1 | `TopologySnapshotBuilder::for_direct_leaf`; the Store child principal equals its Fleet Subnet Root |

These are the same protected topology predicates and reinstall action already
qualified for activation admission. The snapshot helper receives no wrapper
code.

## Cost-Guard Public Mapping

Both public constructors are transparent dispatch sites over the seven exact
`CostGuardReserveError` meanings qualified in
[cost-guard-leaves.md](cost-guard-leaves.md):

| Current branch | Sites | Required hard cut |
| --- | ---: | --- |
| broad `InvalidInput` kind | 1 | Select the exact protected configuration/accounting identity and approved `COST_GUARD_CONFIGURATION_INVALID` projection |
| broad `ResourceExhausted` kind | 1 | Select exact quota pressure or payer-cycle reserve rejection |

`CostGuardReservePublicKind` and `err.to_string()` disappear in B4. The two
branches allocate no broad code; typed store causes remain transparent and
reservation rollback keeps its separately observed secondary failure.

## Capability Endpoint Projection

The root and non-root endpoint constructors each wrap an already-typed public
`Error` returned by capability validation/execution. Both are transparent:

| Endpoint | Sites | Disposition |
| --- | ---: | --- |
| non-root cycles capability | 1 | Preserve the exact public capability/replay diagnostic |
| root capability | 1 | Preserve the exact public capability/replay diagnostic |

No local wrapper identity is allocated. The endpoint boundary authenticates
and delegates; it does not reinterpret the typed result.

## Automatic Cycle-Top-Up Deadlines

The two checked-arithmetic branches add exact meanings:

| Exact candidate | Sites | Producer function/branch | Action and retry |
| --- | ---: | --- | --- |
| `CYCLE_TOPUP_DEADLINE_DURATION_OVERFLOW` | 1 | `CycleWorkflow::deadline_after_secs`; configured delay cannot be represented in nanoseconds | Correct bounded top-up timing configuration; no unchanged retry |
| `CYCLE_TOPUP_DEADLINE_TIMESTAMP_OVERFLOW` | 1 | `CycleWorkflow::deadline_after_secs`; current time plus valid delay exceeds the timer timestamp range | Stop scheduling and wait for corrected time/state; never wrap the deadline |

Duration conversion and timestamp addition remain separate checked boundaries.

## Runtime-Log Retention Deadlines

The two checked-arithmetic branches add exact meanings:

| Exact candidate | Sites | Producer function/branch | Action and retry |
| --- | ---: | --- | --- |
| `RUNTIME_LOG_RETENTION_DEADLINE_SECONDS_OVERFLOW` | 1 | `LogRetentionWorkflow::next_deadline_ns`; oldest log time plus retention age cannot fit seconds | Correct retention/state; never drop entries against a wrapped cutoff |
| `RUNTIME_LOG_RETENTION_DEADLINE_NANOSECONDS_OVERFLOW` | 1 | `seconds_to_nanos`; valid seconds deadline cannot fit the IC timer nanosecond range | Stop scheduling and preserve retained logs |

The seconds and nanoseconds failures have different arithmetic owners and must
remain independently testable.

## Dynamic Public Context

Rows `DPC-345` and `DPC-346` in
[dynamic-public-context.md](dynamic-public-context.md) classify the two typed
cost-guard values currently flattened by the broad public mapper. The topology,
capability and deadline branches add no dynamic public value.

## Reconciliation

All ten sites have one disposition. They add four exact meanings, reuse two
existing activation identities and retain four transparent cost/capability
edges. Together with the preceding two small-adapter ledgers, the effective
constructor frontier moves from 2,452 to 2,478 classified sites and from 47 to
21 open sites. The qualified semantic set reaches 2,675 exact candidates plus
31 safe projections: 2,706 current symbolic identities.

## Required Tests

- preserve exact anonymous-Store and root/Store collision identities;
- exhaustively map every cost-guard variant without its broad-kind enum;
- preserve root and non-root capability public errors unchanged;
- independently overflow top-up duration multiplication and timestamp addition;
- independently overflow log-retention seconds and nanoseconds; and
- prove none of the four deadline paths wraps, schedules or deletes state.

## Next Slice

Finish the twenty-one one-site and remaining small constructor adapters.
