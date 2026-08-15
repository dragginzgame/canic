# Canic 0.102 Component Directory And Fleet-Service Peer Constructor Leaves

Date: 2026-08-14

## Status

This B1 evidence ledger classifies the 26 production `InternalError`
constructor sites owned by root-level Component Directory synchronization and
cross-root Fleet-service requester resolution. It assigns no number and
changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/component_directory_synchronization/mod.rs` | 13 |
| `ops/fleet_service_peer/mod.rs` | 13 |
| **Total** | **26** |

The Fleet-service peer inline test tail is excluded. Component Registry,
Component provisioning, Fleet Registry Mirror and compiled-topology typed
causes remain owned by their qualified ledgers and are not duplicated by these
adapters.

## Root Component Directory Synchronization

| Exact candidate or disposition | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_SUBNET_ROOT_RUNTIME_INACTIVE` | 1 | `workflow::component_directory_synchronization::synchronize`; scale-out Directory synchronization is requested before exact root runtime activation | self; existing exact identity | Wait for exact root activation before retry | Fleet activation status |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 1 | `workflow::component_directory_synchronization::synchronize`; root Component Registry meta authority is absent | self; existing exact identity | Complete exact Registry preparation before retry | root Component Registry status |
| `FLEET_DIRECTORY_SYNC_MIRROR_ROOT_MISMATCH` / `FLEET_DIRECTORY_SYNC_MIRROR_REGISTRY_MISMATCH` | 1 | `workflow::component_directory_synchronization::synchronize`; advanced Mirror response names another root or published Registry authority | self for either exact leaf | Preserve request and Mirror response; reconcile the independently named authority | synchronization request and Mirror status |
| `FLEET_DIRECTORY_SYNC_AUTHORITY_HASH_MISMATCH` | 1 | `workflow::component_directory_synchronization::synchronize`; derived Fleet Directory hash differs from the durable synchronization authority | self | Preserve the accepted operation and reconcile its exact Directory | synchronization status and active Mirror |
| `FLEET_DIRECTORY_SYNC_CURSOR_UNREPRESENTABLE` | 1 | `next_intent`; durable Component cursor cannot address the platform target collection | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing projection | Stop and inspect bounded synchronization accounting | guarded synchronization status |
| `FLEET_DIRECTORY_SYNC_ALLOCATION_MISSING` | 2 | `next_intent` or `synchronize_target`; selected affected Component allocation disappears before intent construction or replay | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the operation and fail closed | recent failure plus synchronization status |
| `COMPONENT_DIRECTORY_REFRESH_INTENT_CONFLICT` | 1 | `synchronize_target`; committed Directory refresh differs from the exact durable plan | self; existing exact identity | Reconstruct and replay only the retained refresh intent | Component Registry refresh status |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 1 | `validate_synchronized_target_coverage`; independently observed synchronized Component partition is absent | self; existing exact identity | Recover the exact committed partition before convergence | Component Registry status |
| `COMPONENT_RUNTIME_CURRENT_DIRECTORY_MISSING` | 1 | `validate_synchronized_target_coverage`; synchronized runtime reports no current Directory authority | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity and projection | Repair or reinstall the exact protected runtime state | recent failure plus runtime status |
| `FLEET_DIRECTORY_SYNC_RUNTIME_FLEET_MISMATCH` / `FLEET_DIRECTORY_SYNC_COMPONENT_HEAD_MISMATCH` / `FLEET_DIRECTORY_SYNC_REGISTRY_REVISION_NOT_COVERED` / `FLEET_DIRECTORY_SYNC_REGISTRY_CONTENT_HASH_MISMATCH` / `FLEET_DIRECTORY_SYNC_RUNTIME_AUTHORITY_HASH_MISMATCH` / `FLEET_DIRECTORY_SYNC_DIRECT_CHILDREN_HASH_MISMATCH` | 1 | `validate_synchronized_target_coverage`; one post-call predicate merges Fleet Directory, Component head, current-or-later Registry coverage, exact-revision content, runtime authority hash and direct-child hash | self for every exact leaf | Preserve the intent and independently identify the unconverged authority | synchronization intent, Component Registry and runtime status |
| `FLEET_DIRECTORY_SYNC_DEPLOYMENT_NOT_GROUP_MEMBER` | 1 | `group_member_runtime_limits`; selected Fleet-service allocation is not protected by a Component Group member deployment | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve allocation and fail closed | recent failure plus provisioning result |
| `FLEET_DIRECTORY_SYNC_COORDINATOR_REQUIRED` | 1 | `require_coordinator`; caller is not the protected Fleet Coordinator | self | Invoke only from the bound Coordinator | transport caller and root authority |

All 13 sites have exact dispositions. They add 13 new exact meanings and reuse
five existing exact identities. The allocation-missing identity is shared by
its pre-intent and replay checks. The six-predicate coverage branch must become
named predicates or a typed coverage result in B4; one broad `unconverged`
code would conceal the exact retry/reconciliation boundary.

## Cross-Root Fleet-Service Requester Resolution

| Exact candidate or disposition | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_PEER_TARGET_ROOT_INACTIVE` | 1 | `FleetServicePeerOps::resolve`; target root is not exactly `Active` | self | Wait for exact target activation; Draining is not admitted | active Mirror status |
| `FLEET_PEER_REGISTRY_TARGET_AUTHORITY_MISMATCH` | 1 | `FleetServicePeerOps::resolve`; validated Mirror Registry differs from protected target-root authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both authorities and fail closed | recent failure plus Mirror status |
| `FLEET_PEER_DIRECTORY_REGISTRY_MISMATCH` | 1 | `FleetServicePeerOps::resolve`; active Fleet Directory provenance differs from current Registry authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve Mirror/Directory and fail closed | recent failure plus Mirror status |
| `FLEET_PEER_SERVICE_MISMATCH` | 1 | `FleetServicePeerOps::resolve`; registered caller belongs to another Fleet service than the requested service | self | Request only the exact registered service authority | authenticated caller and Registry service row |
| `FLEET_PEER_OWNER_ROOT_INACTIVE` / `FLEET_PEER_OWNER_ROOT_NOT_REMOTE` | 1 | `FleetServicePeerOps::resolve`; requester owner is not `Active` or is the same root as the target | self for either exact leaf | Use a distinct currently Active owner root | active Registry snapshot |
| `FLEET_PEER_OWNER_ADMISSION_MISSING` | 1 | `FleetServicePeerOps::resolve`; requester Spec is absent from its owner-root admission | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve Registry state and fail closed | recent failure plus active Registry snapshot |
| transparent: typed protected Component-binding cause | 1 | `FleetServicePeerOps::resolve`; derived requester binding fails `ComponentTopology::validate_component_binding` and the typed cause is currently formatted into prose | preserve the exact qualified topology cause and its approved projection | Remove the formatter and propagate the typed cause without a peer wrapper | protected topology/Registry authority |
| `FLEET_PEER_REQUESTER_SPEC_MISSING` | 1 | `FleetServicePeerOps::validate_origin`; stored requester Spec is absent from protected topology during origin replay | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the origin and fail closed | recent failure plus protected topology |
| `FLEET_PEER_ORIGIN_TARGET_AUTHORITY_MISMATCH` / `FLEET_PEER_ORIGIN_COMPONENT_AUTHORITY_MISMATCH` / `FLEET_PEER_ORIGIN_REGISTRY_REVISION_INVALID` / `FLEET_PEER_ORIGIN_REGISTRY_HASH_INVALID` / `FLEET_PEER_ORIGIN_ROOT_NOT_REMOTE` / `FLEET_PEER_ORIGIN_SPEC_HASH_MISMATCH` / `FLEET_PEER_ORIGIN_ROLE_MISMATCH` / `FLEET_PEER_ORIGIN_COMPONENT_ID_INVALID` / `FLEET_PEER_ORIGIN_CANISTER_ID_INVALID` / `FLEET_PEER_ORIGIN_TARGET_SPEC_MISMATCH` / `FLEET_PEER_ORIGIN_GRANT_MISSING` / `FLEET_PEER_ORIGIN_GRANT_MISMATCH` | 1 | `FleetServicePeerOps::validate_origin`; one durable-origin predicate merges four Registry fields, remote-root proof, four Component fields, target Spec and absent-versus-changed provisioning grant | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Preserve the origin and identify the exact authority contradiction | recent failure plus protected topology/Registry |
| `FLEET_PEER_CALLER_NOT_MEMBER` | 1 | `exact_registry_service_caller`; caller is absent from current Fleet-service membership | self | Invoke only as an exact active registered service member | authenticated caller and active Registry snapshot |
| `FLEET_PEER_CALLER_MEMBERSHIP_AMBIGUOUS` | 1 | `exact_registry_service_caller`; caller appears in more than one Fleet-service member row | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve Registry state and fail closed | recent failure plus active Registry snapshot |
| `FLEET_PEER_OWNER_ROOT_MISSING` | 1 | `exact_service_member_root`; requester member names no owning root in Fleet Registry | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve Registry state and fail closed | recent failure plus active Registry snapshot |
| `FLEET_PEER_OWNER_ROOT_AMBIGUOUS` | 1 | `exact_service_member_root`; requester member resolves to duplicate owning-root rows | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve Registry state and fail closed | recent failure plus active Registry snapshot |

All 13 sites have dispositions. One formatted typed-topology adapter is
transparent. The other sites add 24 exact meanings and no safe projection.
The twelve-predicate origin branch must split absent grant from changed grant;
otherwise retry policy cannot distinguish missing compiled admission from
substituted durable evidence.

## Dynamic Public Context

Every direct message is static except the formatted typed
`ComponentTopologyError` at the requester-binding adapter. Dynamic-context row
`DPC-155` classifies that value as already authoritatively typed and requires
transparent registered-code propagation. Fleet, root, Registry, Spec,
Component, grant and caller facts remain in protected authority or the exact
request and receive no diagnostic text owner.

## Reconciliation

All 26 direct sites have dispositions. One is transparent. The remaining
sites add 37 new exact meanings, reuse five existing exact identities and add
no safe projection. The effective whole-program constructor frontier therefore
moves from 2,107 to 2,133 classified sites and from 392 to 366 open sites.

The qualified semantic ledgers move from 2,399 to 2,436 provisional exact
candidates. Their 31 additional safe projections remain unchanged, producing
2,467 current symbolic identities before final whole-program reuse and
allocation review.

## Required Tests

- distinguish Mirror root from Registry mismatch and durable Fleet Directory
  hash mismatch before Component calls;
- prove the affected allocation remains exact both before intent creation and
  after response-loss replay;
- independently reject all six post-call Directory coverage predicates;
- preserve the five existing Registry/runtime/refresh identities without
  allocating synchronization wrappers;
- distinguish inactive target, inactive owner and same-root owner;
- distinguish absent caller, ambiguous caller membership, missing owner and
  ambiguous owner;
- split every stored-origin Registry, Component, target-Spec and grant field,
  including absent versus changed grant; and
- prove the protected binding adapter propagates its typed topology cause
  without formatted text or a peer wrapper code.

## Next Slice

Continue the effective frontier with the remaining core Component provisioning
plan and Fleet Registry adapters before runtime auth/RPC owners.
