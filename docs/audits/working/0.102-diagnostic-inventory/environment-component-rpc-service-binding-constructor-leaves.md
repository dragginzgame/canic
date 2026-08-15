# Canic 0.102 Environment, Component RPC And Service-Binding Constructor Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger classifies all fifteen production `InternalError`
constructor references in environment initialization, Component-child RPC
lifecycle execution and Fleet-service binding adapters. It assigns no number
and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/env/mod.rs` | 5 |
| `workflow/component_rpc/lifecycle.rs` | 5 |
| `ops/fleet_service_binding/mod.rs` | 5 |
| **Total** | **15** |

## Environment Initialization

The five environment sites reuse two existing identities, add two exact
protected-target meanings across two compound checks and retain one typed
topology edge:

| Exact candidate or disposition | Sites | Producer function/branch | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- | --- |
| reuse `ACCESS_BUILD_NETWORK_UNAVAILABLE` | 1 | `EnvWorkflow::init_env_from_args` missing `BuildNetworkOps::build_network` | `Invariant` / frozen build environment | self | Rebuild with an exact `ICP_ENVIRONMENT` identity |
| reuse `ENV_REQUIRED_FIELDS_MISSING` | 1 | `EnvWorkflow::init_env_from_args` handling `EnvPolicyError::MissingEnvFields` | `Invariant` / runtime environment | `RUNTIME_ENVIRONMENT_INVALID` | Reinstall with every required protected environment field |
| `MANAGED_CANISTER_INIT_ROLE_MISMATCH` / `MANAGED_CANISTER_INIT_CANISTER_MISMATCH` | 2 | `EnvWorkflow::init_component` and `EnvWorkflow::init_component_child` protected-target checks | `InvalidInput` / protected managed-runtime target | `COMPONENT_CHILD_AUTHORITY_INVALID` | Reinstall the Component or descendant with the exact compiled role and target Canister |
| transparent typed Component-topology cause | 1 | `map_binding_error` called by `EnvWorkflow::init_component` and `EnvWorkflow::init_component_child` | protected Component/child binding validation | source projection | Preserve the exact reachable `ComponentTopologyError`; do not format it into a managed-init wrapper |

The Component and Component-child constructors intentionally share the two
managed-target leaves. After exact topology validation, both enforce the same
compiled-role and receiver-Canister authority, expose the same protected
fields, and require reinstall rather than an unchanged retry. B4 must split
the current mixed role/Canister predicate before assigning a code so either
contradiction remains independently testable.

The topology adapter is transparent. Its reachable binding, root, Subnet,
Spec, role and principal decisions already belong to the qualified
`ComponentTopologyError` family; `managed init authority is invalid` adds no
meaning or authority.

## Component-Child RPC Lifecycle

The five workflow constructors expand to seven exact protected-lifecycle
meanings:

| Exact candidate | Sites | Producer function/branch | Current meaning | Public projection | Required hard cut |
| --- | ---: | --- | --- | --- | --- |
| `RPC_COMPONENT_CHILD_PROVISION_BINDING_VARIANT_INVALID` | 1 | `provision_component_child` terminal binding destructure | Child activation returned a top-level Component binding | `COMPONENT_CHILD_AUTHORITY_INVALID` | Preserve the returned binding kind and fail closed |
| `RPC_COMPONENT_CHILD_PROVISION_COMPONENT_MISMATCH` / `RPC_COMPONENT_CHILD_PROVISION_PARENT_MISMATCH` / `RPC_COMPONENT_CHILD_PROVISION_ROLE_MISMATCH` | 1 | `provision_component_child` comparison with `ProvisionedChildIdentity::from_binding` | Terminal binding differs from the requested Component, transport parent or admitted role | `COMPONENT_CHILD_AUTHORITY_INVALID` for every leaf | Compare each field independently and never return a substituted Canister |
| `RPC_COMPONENT_CHILD_RECYCLE_COMPONENT_MISMATCH` | 1 | `require_expected_recycle_identity` Component predicate | Removal progress belongs to another Component tree | `COMPONENT_CHILD_AUTHORITY_INVALID` | Preserve the exact removal operation and fail closed |
| `RPC_COMPONENT_CHILD_RECYCLE_TARGET_MISMATCH` | 1 | `require_expected_recycle_identity` target predicate | Removal progress names another target Canister | `COMPONENT_CHILD_AUTHORITY_INVALID` | Never redirect recycling to a different principal |
| `RPC_COMPONENT_CHILD_RECYCLE_PARENT_MISMATCH` | 1 | `require_expected_recycle_identity` caller-parent predicate | Target is no longer bound to the transport caller as immediate parent | `COMPONENT_CHILD_AUTHORITY_INVALID` | Reauthenticate the exact parent after every awaited removal step |

`ProvisionedChildIdentity` is a useful comparison shape, but its equality
result is not one diagnostic meaning. Component, parent and role are separate
authority predicates and need independent adversarial tests. The recycle
checks likewise validate the durable operation both before progress and after
every awaited phase; a later contradictory status must not be treated as
successful or merely in progress.

## Fleet-Service Binding Adapters

All five sites convert typed `FleetServiceBindingOpsError` values through the
generic `OpsError` string boundary. They allocate no wrapper identity:

| Adapter | Sites | Producer function | Disposition |
| --- | ---: | --- | --- |
| initial configuration compilation | 1 | `FleetServiceBindingOps::compile_initial` | Preserve the exact compiled configuration cause rather than `Configuration(String)` |
| complete initial-service compilation | 1 | `FleetServiceBindingOps::compile_initial_compiled` | Exhaustively preserve the already-qualified Fleet-service binding or dependency leaf |
| complete Scale Out compilation | 1 | `FleetServiceBindingOps::compile_scale_out_compiled` | Exhaustively preserve the already-qualified Fleet-service binding or dependency leaf |
| planned-root receipt lookup | 1 | `FleetServiceBindingOps::validate_provisioned_root_receipt_compiled` | Preserve `FLEET_SERVICE_BINDING_ROOT_RECEIPT_INDEX_INVALID` |
| terminal root-receipt validation | 1 | `FleetServiceBindingOps::validate_provisioned_root_receipt_compiled` calling `validate_root_receipt` | Preserve the exact qualified receipt identity, state, count, time, hash or result leaf |

The 22 Fleet-service binding meanings and their configuration, provisioning-
plan and receipt-hashing dependencies are already qualified in
[fleet-control-plane-leaves.md](fleet-control-plane-leaves.md). B4 replaces
`Configuration(String)`, `Plan(String)` and the generic `OpsError::to_string()`
conversion with typed edges. Route-specific compilation prose must not create
a second code family.

## Dynamic Public Context

Seven formatted values are classified as `DPC-274` through `DPC-280` in
[dynamic-public-context.md](dynamic-public-context.md). Environment validation
contributes two typed causes. The five Fleet-service adapters preserve typed
configuration, plan, receipt and binding values whose canonical owners are
the immutable configuration, plan and root receipts.

The Component-RPC lifecycle messages are static. Their Component, parent,
role and target values remain in the request, protected binding and durable
removal status rather than being interpolated into `Error.message`.

## Reconciliation

All fifteen direct sites now have one disposition. They add nine exact
meanings, reuse two existing identities and retain six transparent typed
edges. The effective constructor frontier moves from 2,327 to 2,342
classified sites and from 172 to 157 open sites. The qualified semantic set
reaches 2,517 exact candidates plus 31 safe projections: 2,548 current
symbolic identities.

## Required Tests

- missing build network and required environment fields preserve their
  existing identities;
- Component and descendant initialization reject role and receiver-Canister
  mismatch independently;
- every reachable topology binding failure preserves its exact typed cause;
- child activation rejects the wrong binding variant;
- provisioned Component, immediate parent and role substitutions reject
  independently before returning a principal;
- recycle status revalidates Component, target and transport parent before and
  after every awaited phase; and
- every Fleet-service adapter preserves exact configuration, plan, receipt and
  binding diagnostics without a route wrapper.

## Next Slice

Continue with cascade topology, ICP-refill workflow and the remaining storage
and placement adapters.
