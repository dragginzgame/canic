# Canic 0.102 Core Plan And Fleet Registry Adapter Constructor Leaves

Date: 2026-08-14

## Status

This B1 evidence ledger classifies all 26 production `InternalError`
constructor references in the core Component provisioning-plan and Fleet
Registry public adapters. It assigns no number and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `ops/component_provisioning_plan/mod.rs` | 13 |
| `ops/fleet_registry/mod.rs` | 13 |
| **Total** | **26** |

Both modules already perform their decisions in closed typed error families.
These constructor references are conversion sites, not 26 new failure
meanings.

## Component Provisioning-Plan Adapters

All 13 sites convert a typed configuration or
`ComponentProvisioningPlanOpsError` through `OpsError` into the current
string-first `InternalError`:

| Disposition | Sites | Current owner | Required hard cut |
| --- | ---: | --- | --- |
| transparent: configuration compilation cause | 5 | `ComponentDeploymentConfigurationError` and nested topology/service errors | Preserve the exact typed source diagnostic; remove `Configuration(String)` and the `OpsError` display funnel |
| transparent: complete plan validation/canonicalization cause | 3 | `ComponentProvisioningPlanOpsError` | Exhaustively map the already-qualified plan diagnostic without allocating an adapter code |
| transparent: scale-out validation cause | 2 | `ComponentProvisioningPlanOpsError` plus durable scale-out authority | Preserve the exact plan leaf and its retry disposition |
| transparent: root-batch validation/canonicalization cause | 3 | `ComponentProvisioningPlanOpsError` | Preserve the exact batch leaf; do not renumber it for the root adapter |

The 13 sites add no exact candidate and no projection. The 46 plan meanings,
including the hidden undeclared-Fleet-service decision, are already qualified
in [fleet-control-plane-leaves.md](fleet-control-plane-leaves.md). B4 must
replace `Configuration(String)` and `FleetRegistry(String)` with typed cause
edges before converting to registered diagnostics.

## Fleet Registry Adapters

All 13 sites convert `FleetRegistryOpsError` through `OpsError` into
`InternalError`:

| Disposition | Sites | Current owner | Required hard cut |
| --- | ---: | --- | --- |
| transparent: Registry compile transition | 7 | exact genesis, Joining, all-Active, service-publication, Draining and Removed `FleetRegistryOpsError` variants | Preserve the exact typed transition diagnostic at every wrapper |
| transparent: Registry validation/canonical evidence | 4 | exact validation, canonical bytes, manifest and root Directory variants | Preserve the exact typed Registry/topology diagnostic and its approved projection |
| transparent: affected-service derivation | 2 | exact authority and append-validation `FleetRegistryOpsError` variants | Preserve authority versus service-addition semantics; allocate no derivation wrapper |

The 13 sites add no exact candidate and no projection. The 54 reachable Fleet
Registry meanings, including ten path-qualified topology validation causes,
are already qualified in
[fleet-control-plane-leaves.md](fleet-control-plane-leaves.md).

## Dynamic Public Context

Although the adapters add no identities, their typed `Display` funnels still
place dynamic values in current public prose. Dynamic-context slices 22 and 23
classify 39 values:

- 15 from provisioning-plan bounds, selected identities and the two flattened
  nested-cause families; and
- 24 from Registry bounds, roots, admissions, App/epoch, services, release
  builds and the nested topology cause.

Thirty-six values are caller-derivable from the exact plan, Registry,
configuration or maintained contract. Three nested causes are already
authoritatively typed. None requires a new status DTO or diagnostic detail
field.

## Reconciliation

All 26 direct sites are transparent conversion adapters. They add no exact
meaning, no projection and no candidate reuse. The effective whole-program
constructor frontier therefore moves from 2,133 to 2,159 classified sites and
from 366 to 340 open sites.

The qualified semantic ledgers remain at 2,436 provisional exact candidates
plus 31 additional safe projections: 2,467 current symbolic identities before
final whole-program reuse and allocation review.

## Required Tests

- prove every public plan adapter returns the exact registered typed plan,
  configuration or Registry cause without wrapper prose;
- prove every Registry compile, validation, canonicalization and Directory
  adapter preserves its exact `FleetRegistryOpsError` identity;
- mechanically reject new `String` cause variants in both typed families;
- prove all 39 dynamic values disappear from diagnostic prose while remaining
  derivable from typed input/authority; and
- prove no adapter-level diagnostic is present in the approved allocation
  ledger.

## Next Slice

Continue the effective frontier with core authentication, runtime intent and
RPC execution owners, keeping typed auth causes and external-effect recovery
boundaries separate.
