# Canic 0.102 Fleet Activation And Scaling Constructor Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger classifies six production `InternalError`
constructor references at the root Fleet-activation adapter and legacy
placement-scaling workflow. It assigns no number and changes no runtime
behavior.

| Production owner | Sites |
| --- | ---: |
| `canic-control-plane/workflow/runtime/fleet_activation/mod.rs` | 3 |
| `canic-core/workflow/placement/scaling/mod.rs` | 3 |
| **Total** | **6** |

## Root Fleet Activation Adapter

The three static branches add three exact meanings:

| Exact candidate | Sites | Producer function/branch | Action and retry |
| --- | ---: | --- | --- |
| `FLEET_ACTIVATION_ROOT_BOOTSTRAP_PREPARATION_INCOMPLETE` | 1 | `workflow::runtime::fleet_activation::prepare_root`; root bootstrap has not prepared the complete managed inventory before activation preparation | Inspect bootstrap status and resume the exact preparation journey |
| `FLEET_ACTIVATION_RESUME_ROOT_INACTIVE` | 1 | `workflow::runtime::fleet_activation::resume_root`; core activation resume returned without placing the root runtime in `Active` | Preserve activation state and recover/retry the exact operation |
| `FLEET_ACTIVATION_ROOT_BOOTSTRAP_NOT_READY` | 1 | `workflow::runtime::fleet_activation::resume_root`; activation reached `Active` but the independently restored bootstrap readiness fence remains false | Inspect bootstrap status and retry the same activation resume |

These do not reuse `FLEET_SUBNET_ROOT_RUNTIME_INACTIVE`. That existing leaf
denies an ordinary root-local lifecycle operation before root activation; the
three branches here identify distinct activation prerequisites or contradictory
postconditions and have different recovery owners.

## Placement Scaling Policy

One source constructor currently publishes a preformatted policy reason. Its
typed `ScalingPlanReason` must select one of two exact meanings:

| Exact candidate | Producer function/typed decision | Action and retry |
| --- | --- | --- |
| `SCALING_MAX_WORKERS_REACHED` | `ScalingWorkflow::create_worker`: `AtMaxWorkers` | Free/recycle a worker or increase the configured maximum before retry |
| `SCALING_WITHIN_POLICY_BOUNDS` | `ScalingWorkflow::create_worker`: `WithinBounds` | Do not create a worker until observed demand/policy admits scale-out |

`BelowMinWorkers` admits creation and therefore cannot reach this error
constructor. B4 must stop forwarding `ScalingPlan.reason: String`; the typed
plan reason selects the registered identity directly.

The other two scaling constructors are current type-shape sediment:

| Disposition | Sites | Proof and hard cut |
| --- | ---: | --- |
| admitted plan lacks `worker_entry` | 1 | Current pure policy sets `Some(entry)` on its only `should_spawn = true` branch; replace boolean-plus-option with an admitted-plan enum carrying the entry |
| admitted plan's pool is absent from the source config | 1 | Policy input is derived synchronously from the same immutable config value later queried; carry the admitted pool config in the typed plan instead of re-looking it up |

Neither impossible state receives a permanent diagnostic number. B4 must make
the invalid combinations unrepresentable rather than preserving runtime
branches whose only evidence is an internally contradictory local value.

## Dynamic Public Context

Rows `DPC-331` through `DPC-337` in
[dynamic-public-context.md](dynamic-public-context.md) classify the current
formatted policy values. Pool is caller-derivable; current worker count and
configured limits already have typed Registry/configuration owners. Compact
errors retain none of the formatter text.

Fleet activation uses only static messages and adds no dynamic row.

## Reconciliation

All six sites have one disposition. They add five exact meanings and remove two
impossible scaling branches from allocation. The effective constructor frontier
moves from 2,452 to 2,458 classified sites and from 47 to 41 open sites. The
qualified semantic set reaches 2,666 exact candidates plus 31 safe projections:
2,697 current symbolic identities.

## Required Tests

- reject activation preparation before complete bootstrap inventory;
- reject a resume result that does not become exactly `Active`;
- independently require post-activation bootstrap readiness;
- select max-workers and within-bounds identities from typed plan reasons;
- prove `BelowMinWorkers` cannot reach the denial conversion;
- make admitted scaling plans carry their worker entry and pool configuration;
- remove the impossible missing-entry/missing-pool branches; and
- prove compact scaling errors contain no pool, count or limit prose.

## Next Slice

Continue with the remaining two-site core ops/RPC/runtime adapters.
