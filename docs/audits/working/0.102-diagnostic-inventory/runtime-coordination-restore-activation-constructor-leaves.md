# Canic 0.102 Runtime Coordination, Restore And Activation Constructor Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger classifies all seventeen production
`InternalError` constructor references in runtime coordination, authority-
restore orchestration and Fleet-activation workflow. It assigns no number and
changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/runtime/mod.rs` | 3 |
| `workflow/runtime/authority_restore.rs` | 3 |
| `workflow/runtime/fleet_activation.rs` | 11 |
| **Total** | **17** |

## Runtime Coordination

The three runtime-coordination sites add one exact meaning:

| Exact candidate or disposition | Sites | Producer function/branch | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| reuse `ACCESS_ROOT_REQUIRED` | 1 | `RuntimeWorkflow::start_all_root`; exact root-access denial | source identity | Start root-only services only on the protected Fleet Subnet Root |
| transparent typed memory-bootstrap cause | 1 | `init_post_upgrade_memory_registry`; memory initialization | source projection | Preserve the exact `ic-memory` adapter diagnostic |
| `ICP_REFILL_UPGRADE_RESUMABLE` | 1 | `validate_refill_upgrade_admission`; root upgrade observes resumable refill work | self | Complete or explicitly recover every resumable refill operation before reinstalling |

The root-context wrapper must not change an access denial into a lifecycle
invariant. Derived intent/refill index rebuilds also append static context to
typed errors outside these three constructors; those context calls remain in
the transitive context frontier and receive no runtime-coordination code.

## Authority Restore

The three restore sites add two exact meanings and reuse the existing endpoint
fence:

| Exact candidate or disposition | Sites | Producer function | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- | --- |
| reuse `AUTHORITY_RESTORE_UPDATE_FENCED` | 1 | `AuthorityRestoreWorkflow::require_endpoint_allowed` through typed endpoint policy | `Conflict` / sealed update policy | self | Resume the exact sealed operation before invoking ordinary updates |
| `AUTHORITY_RESTORE_TIMER_RUNNING` | 1 | `require_resumable_timer_state` | `Invariant` / snapshot interruption boundary | self | Wait for the running Canic timer to become resumable before sealing or resuming |
| `AUTHORITY_RESTORE_RUNTIME_REQUIRED` | 1 | `require_authority_runtime` | `Forbidden` / authority runtime | self | Invoke snapshot fencing only on the Fleet Coordinator or Fleet Subnet Root |

The timer result is a closed static state, not free-form authority. It must
become a typed decision before the prose cut. The runtime predicate remains
independent from controller status: only the exact Coordinator/root role may
own an authority fence.

## Fleet Activation

The eleven Fleet-activation sites add one exact meaning, reuse established
activation identities and retain typed storage causes:

| Exact candidate or disposition | Sites | Producer function/branch | Public projection | Required hard cut |
| --- | ---: | --- | --- | --- |
| reuse `FLEET_ACTIVATION_IDENTITY_MISMATCH` | 1 | `FleetActivationWorkflow::resume_root`; request differs from protected root operation/credential | self | Preserve exact operation identity |
| reuse `FLEET_ACTIVATION_STATE_INVALID` | 3 | `FleetActivationWorkflow::resume_root` and `FleetActivationWorkflow::complete_provisioned_nonroot_activation`; protected cascade manifest, root cascade or active credential is missing | self | Fail closed against the protected record |
| transparent typed activation-storage cause | 2 | `FleetActivationWorkflow::activate_nonroot` and `FleetActivationWorkflow::require_endpoint_allowed`; storage transitions | source projection | Preserve exact record/transition diagnostic |
| reuse `FLEET_ACTIVATION_ENDPOINT_FENCED` | 1 | `require_endpoint_for_phase`; Prepared endpoint is not admitted | self | Preserve exact phase-policy denial |
| split reuse of `FLEET_ACTIVATION_IDENTITY_MISMATCH`, `FLEET_ACTIVATION_EVIDENCE_MISMATCH`, `FLEET_ACTIVATION_STATE_INVALID` and `FLEET_ACTIVATION_TRANSITION_INVALID` | 1 | `validate_nonroot_activation_status`; combined child-status predicate | respective source identities | Split identity, cascade, forbidden root-only evidence, timestamp and required-phase predicates before allocation |
| `FLEET_ACTIVATION_CREDENTIAL_BUNDLE_REQUIRED` | 1 | `require_empty_prepared_credential_authority`; unsupported prepared credential authority | self | Use the credential-bundle activation slice before configuring issuer policy/templates |
| reuse `FLEET_ACTIVATION_WASM_STORE_PRINCIPAL_INVALID` | 2 | `require_root_activation_wasm_store`; Store principal is anonymous or equals the root | self | Supply one distinct non-anonymous sibling Store principal |

The combined child-status constructor is not one generic state code. B4 must
split its predicates so identity, evidence, protected-record and required-
phase contradictions retain their existing distinct meanings. A later valid
status observation may establish an uncertain call outcome only when every
required predicate passes.

The credential-bundle fence is a current hard capability boundary, not a
generic activation failure. It remains exact until the separately designed
credential-bundle activation slice removes the fence; its number then retires
without reuse if allocated before that slice.

## Dynamic Public Context

Ten values are classified as `DPC-264` through `DPC-273` in
[dynamic-public-context.md](dynamic-public-context.md). Six are derivable timer,
refill-count, operation and target values. Four are typed memory or uncertain-
call observation causes that must retain their registered identities.

Activation-service startup failures currently trap after durable transition;
they are lifecycle trap/log evidence rather than `Error.message` and therefore
remain outside this dynamic public-message ledger.

## Reconciliation

All seventeen direct sites now have one disposition. They add four exact
meanings, reuse nine existing exact-identity occurrences and retain three
transparent typed storage/memory edges. The effective constructor frontier
moves from 2,310 to 2,327 classified sites and from 189 to 172 open sites. The
qualified semantic set reaches 2,508 exact candidates plus 31 safe projections:
2,539 current symbolic identities.

## Required Tests

- root-service access preserves `ACCESS_ROOT_REQUIRED`;
- resumable refill work blocks upgrade until exact terminal recovery;
- restore timer-running and non-authority-runtime denials remain independent;
- sealed endpoint policy preserves its existing exact identity;
- missing protected activation evidence reuses the state-invalid identity;
- child-status identity, evidence, root-only fields, timestamp and phase
  predicates reject independently;
- uncertain call reconciliation accepts only a fully matching later status;
- anonymous and root-equal Store principals share the existing invalid-
  principal meaning; and
- the credential-bundle capability fence cannot be bypassed by partial issuer
  state.

## Next Slice

Continue with environment workflow, component-RPC lifecycle and remaining
Fleet-service/deployment adapters.
