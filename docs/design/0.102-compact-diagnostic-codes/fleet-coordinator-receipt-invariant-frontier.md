# Canic 0.102 Fleet Coordinator Receipt-Invariant Frontier

Date: 2026-08-13

## Status

This B1 ledger expands the shared `receipt_invariant(&'static str)` funnel in
`crates/canic-control-plane/src/ops/fleet_coordinator/mod.rs`. It assigns no
number and changes no runtime behavior.

The parent constructor ledger correctly counts the funnel's one
`InternalError::invariant` definition, but that definition has **235 production
call sites in 102 functions**. Those call sites describe distinct protected
state, receipt, cursor, time and authority failures. The adapter itself must
receive no generic code; each call site must map to an exact typed leaf or an
explicit transparent/sediment disposition.

## Mechanical Frontier

The static call census excludes the function definition and partitions every
call by consecutive source range:

| Inclusive source lines | Calls |
| --- | ---: |
| 1–1739 | 16 |
| 1740–3435 | 85 |
| 3436–5060 | 26 |
| 5061–6805 | 70 |
| 6806–7995 | 38 |
| **Total** | **235 in 102 functions** |

The source uses a `&'static str`, so every current call is statically
enumerable. A future typed implementation must delete the string selector
rather than retain it beside numeric diagnostics.

## Public-Transition Persistence Calls

This first slice accounts for all 16 calls at lines 1–1739. These are state
contradictions discovered while public transitions recover or commit an
already-classified effect boundary; they project to the existing guarded state
diagnostic rather than exposing storage detail.

| Exact candidate | Calls | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_ROOT_PROVISION_RECONCILIATION_INTENT_MISSING` | 1 | Reconcile disposition has no retained root-provision intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve operation state and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_RESPONSE_INTENT_LOST` | 1 | Recording path loses the pre-call root-provision intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve before/after operation evidence and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_PREDECESSOR_MISSING` | 1 | Root-provision response has no durable previous progress | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve response/progress and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_ROOTS_ACCEPTED_TIME_MISSING` | 1 | Provisioning state loses immutable RootsAccepted time | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve state and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_COUNT_UNREPRESENTABLE` | 1 | Durable provisioned-root receipt count cannot fit `u32` | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve bounded state and fail closed | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_RECONCILIATION_INTENT_MISSING` | 1 | Reconcile disposition has no retained Directory intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve operation state and fail closed | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_CANONICAL_ROOT_MISMATCH` | 1 | Selected Directory predecessor root differs from canonical plan order | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan/progress and fail closed | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_RESPONSE_INTENT_LOST` | 1 | Fresh Directory response path loses its pre-call intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve before/after operation evidence and fail closed | recent failure |
| `FLEET_SCALE_OUT_SYNCHRONIZATION_RESPONSE_INTENT_LOST` | 1 | Scale-out synchronization response path loses its intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve before/after operation evidence and fail closed | recent failure |
| `FLEET_SCALE_OUT_PUBLICATION_RESPONSE_INTENT_LOST` | 1 | Scale-out publication response path loses its intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve before/after operation evidence and fail closed | recent failure |
| `FLEET_SCALE_OUT_PUBLICATION_SYNCHRONIZATION_MISSING` | 1 | Publication has no retained synchronization record | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve publication/progress and fail closed | recent failure |
| `FLEET_SCALE_OUT_PUBLICATION_SYNCHRONIZATION_INCOMPLETE` | 1 | Publication begins while retained synchronization is nonterminal | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve state; do not infer synchronization completion | recent failure |
| `FLEET_RUNTIME_ACTIVATION_RECONCILIATION_INTENT_MISSING` | 1 | Reconcile disposition has no retained runtime-activation intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve operation state and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_CANONICAL_ROOT_MISMATCH` | 1 | Selected activation progress root differs from canonical plan order | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan/progress and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_RESPONSE_INTENT_LOST` | 1 | Runtime-activation response path loses its pre-call intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve before/after operation evidence and fail closed | recent failure |
| `COORDINATOR_DEPLOYMENT_CONFIGURATION_INVALID` | 1 | Stored Component deployment configuration fails its canonical digest validation | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve configuration and fail closed | recent failure |

The 16 rows sum to all 16 selected calls and introduce 16 unique exact labels.
No safe projection is added: every row reuses the already qualified guarded
state projection.

## Required Tests

- remove each retained intent after its classifier selects reconcile;
- lose predecessor, RootsAccepted time or bounded count while preserving the
  rest of the operation record;
- substitute a noncanonical root in Directory/runtime progress;
- begin scale-out publication with missing and nonterminal synchronization
  independently; and
- corrupt stored deployment configuration while retaining Coordinator/Registry
  authority.

## Next Slice

Classify the 85 protected state/retired-receipt calls at lines 1740–3435, then
the progress, response and lifecycle-history ranges.
