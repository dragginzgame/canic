# Canic 0.102 Component Policy Diagnostic Leaves

Date: 2026-08-12

## Status

This is the provisional semantic leaf ledger for the two pure Component
allocation policy errors at immutable baseline `v0.101.53`. Labels are review
names, not Rust constants or allocated protocol identities. Numeric assignment
remains forbidden until the complete B1 ledger is reviewed.

`public` in the observability column means the exact numeric leaf is returned
to the caller. `recent failure` means B4 must write the masked internal numeric
leaf to the existing bounded heap-only runtime recent-failure owner while
returning the listed safe public projection. No diagnostic prose is a recovery
authority.

## Top-Level Component Allocation

All 17 `ComponentAllocationPolicyError` variants have current producers.

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry | Observability |
| --- | --- | --- | --- | --- | --- |
| `COMPONENT_ALLOCATION_OPERATION_ID_EMPTY` | `EmptyOperationId` | `InvalidInput` / Component allocation | self | Supply a nonzero operation ID; retry corrected request | public |
| `COMPONENT_ALLOCATION_SEQUENCE_EXHAUSTED` | `AllocationSequenceExhausted` | `ResourceExhausted` / Component allocation | self | Retire or replace the root; never retry unchanged on this root | public |
| `COMPONENT_ALLOCATION_ROOT_TOPOLOGY_PROJECTION_INVALID` | `InvalidRootTopologyProjection` | `Invariant` / Component allocation | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Reinstall from one canonical plan; do not retry | recent failure |
| `COMPONENT_ALLOCATION_ROOT_TOPOLOGY_DIGEST_MISMATCH` | `RootTopologyDigestMismatch` | `Invariant` / Component allocation | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Reinstall the inconsistent root; do not retry | recent failure |
| `COMPONENT_SPEC_NOT_ADMITTED_ON_ROOT` | `ComponentSpecNotAdmitted` | `InvalidInput` / Component allocation | self | Select a Spec admitted by this root; retry corrected request | public |
| `COMPONENT_ALLOCATION_ADMITTED_SPEC_MISSING` | `ComponentSpecUnknown` | `Invariant` / Component allocation | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Repair/reinstall protected topology; do not retry | recent failure |
| `COMPONENT_ALLOCATION_SPEC_HASH_MISMATCH` | `ComponentSpecHashMismatch` | `Invariant` / Component allocation | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Repair/reinstall protected topology; do not retry | recent failure |
| `COMPONENT_ALLOCATION_COUNT_OVERFLOW` | `ComponentCountOverflow` | `Invariant` / Component allocation capacity | `COMPONENT_ALLOCATION_CAPACITY_EXHAUSTED` | Inspect accounting and stop allocation; do not blindly retry | recent failure |
| `COMPONENT_ALLOCATION_CAPACITY_EXHAUSTED` | `ComponentCapacityExhausted` | `ResourceExhausted` / Component allocation capacity | self | Free root-local Component capacity or select another root; exact retry after state change | public |
| `COMPONENT_SPEC_ALLOCATION_COUNT_OVERFLOW` | `ComponentSpecCountOverflow` | `Invariant` / Component Spec capacity | `COMPONENT_SPEC_ALLOCATION_CAPACITY_EXHAUSTED` | Inspect Spec accounting and stop allocation; do not blindly retry | recent failure |
| `COMPONENT_SPEC_ALLOCATION_CAPACITY_EXHAUSTED` | `ComponentSpecCapacityExhausted` | `ResourceExhausted` / Component Spec capacity | self | Free this Spec's root-local capacity or select another root; exact retry after state change | public |
| `PEER_COMPONENT_REQUESTER_BINDING_INVALID` | `InvalidPeerRequesterBinding` | `Forbidden` / peer Component provisioning | `PEER_COMPONENT_REQUESTER_UNAUTHORIZED` | Reauthenticate against exact Fleet/root Registry authority; do not retry unchanged | recent failure |
| `PEER_COMPONENT_TARGET_ROOT_INACTIVE` | `PeerRootRuntimeInactive` | `Unavailable` / peer Component provisioning | self | Wait for the target root to become Active; bounded exact retry after state change | public |
| `PEER_COMPONENT_REQUESTER_INACTIVE` | `PeerRequesterRegistryMemberInactive` | `Forbidden` / peer Component provisioning | self | Restore Active requester membership; retry only after state change | public |
| `PEER_COMPONENT_GRANT_MISSING` | `PeerProvisioningGrantMissing` | `Forbidden` / peer Component provisioning | self | Select a granted target Spec or change checked-in topology and reinstall | public |
| `PEER_COMPONENT_ALLOCATION_COUNT_OVERFLOW` | `PeerProvisioningCountOverflow` | `Invariant` / peer Component capacity | `PEER_COMPONENT_CAPACITY_EXHAUSTED` | Inspect peer accounting and stop allocation; do not blindly retry | recent failure |
| `PEER_COMPONENT_CAPACITY_EXHAUSTED` | `PeerProvisioningCapacityExhausted` | `ResourceExhausted` / peer Component capacity | self | Free requester/root peer capacity; exact retry after state change | public |

`ComponentSpecUnknown` is deliberately reclassified from the current broad
invalid-input projection. The policy checks root admission before topology
lookup, so an admitted Spec missing from the protected projection is an
internal authority inconsistency, not caller input.

## Direct-Child Allocation

Seventeen `ComponentChildAllocationPolicyError` variants have current
producers. `ComponentCountOverflow` does not and is excluded below.

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry | Observability |
| --- | --- | --- | --- | --- | --- |
| `COMPONENT_CHILD_OPERATION_ID_EMPTY` | `EmptyOperationId` | `InvalidInput` / Component child allocation | self | Supply a nonzero operation ID; retry corrected request | public |
| `COMPONENT_CHILD_COMPONENT_BINDING_INVALID` | `InvalidComponentBinding` | `Invariant` / Component child authority | `COMPONENT_CHILD_AUTHORITY_INVALID` | Re-resolve the registered Component authority; do not retry unchanged | recent failure |
| `COMPONENT_CHILD_PARENT_BINDING_INVALID` | `InvalidParentBinding` | `Invariant` / Component child authority | `COMPONENT_CHILD_AUTHORITY_INVALID` | Re-resolve exact parent membership; do not retry unchanged | recent failure |
| `COMPONENT_CHILD_PARENT_COMPONENT_MISMATCH` | `ParentComponentMismatch` | `Forbidden` / Component child authority | `COMPONENT_CHILD_PARENT_UNAUTHORIZED` | Use a parent in the same Component tree; do not retry unchanged | recent failure |
| `COMPONENT_CHILD_CALLER_NOT_PARENT` | `ParentCallerMismatch` | `Forbidden` / Component child authority | self | Submit from the exact registered immediate parent; do not retry unchanged | public |
| `COMPONENT_CHILD_FLEET_ROOT_INACTIVE` | `FleetRegistryRootNotActive` | `Unavailable` / Component child readiness | self | Wait for the Fleet Registry root to be Active; bounded exact retry | public |
| `FLEET_SUBNET_ROOT_RUNTIME_INACTIVE` | `RootRuntimeNotActive` | `Unavailable` / root-local lifecycle readiness | self | Wait for local root runtime activation; bounded exact retry | public |
| `COMPONENT_CHILD_REGISTRY_INACTIVE` | `ComponentRegistryNotActive` | `Unavailable` / Component child readiness | self | Wait for Component Registry activation; bounded exact retry | public |
| `COMPONENT_CHILD_PARENT_INACTIVE` | `ParentRegistryMemberNotActive` | `Forbidden` / Component child readiness | self | Restore Active immediate-parent membership; retry only after state change | public |
| `COMPONENT_CHILD_REGISTRY_AUTHORITY_STALE` | `ComponentRegistryAuthorityMismatch` | `Conflict` / Component Registry authority | self | Refresh the exact Registry head and retry with the same operation identity | public |
| `COMPONENT_CHILD_SPEC_MISSING` | `ComponentSpecUnknown` | `Invariant` / Component child authority | `COMPONENT_CHILD_AUTHORITY_INVALID` | Repair/reinstall protected topology; do not retry | recent failure |
| `COMPONENT_CHILD_ROLE_NOT_ADMITTED` | `ChildRoleNotAdmitted` | `InvalidInput` / Component child admission | self | Select a child role admitted by the Component Spec; retry corrected request | public |
| `COMPONENT_CHILD_SPAWN_GRANT_MISSING` | `SpawnGrantMissing` | `Forbidden` / Component child admission | self | Use an exact granted parent/child role pair or change topology and reinstall | public |
| `COMPONENT_CHILD_PARENT_ROLE_COUNT_OVERFLOW` | `ParentRoleCountOverflow` | `Invariant` / per-parent capacity | `COMPONENT_CHILD_PARENT_ROLE_CAPACITY_EXHAUSTED` | Inspect parent-role accounting and stop allocation; do not blindly retry | recent failure |
| `COMPONENT_CHILD_PARENT_ROLE_CAPACITY_EXHAUSTED` | `ParentRoleCapacityExhausted` | `ResourceExhausted` / per-parent capacity | self | Free the parent's role capacity; exact retry after state change | public |
| `COMPONENT_DESCENDANT_CAPACITY_EXHAUSTED` | `ComponentDescendantCapacityExhausted` | `ResourceExhausted` / Component capacity | self | Free capacity in this Component tree or change deployment limits on reinstall; exact retry after state change | public |
| `COMPONENT_CHILD_DEPLOYMENT_LIMITS_INVALID` | `InvalidDeploymentLimits` | `Invariant` / Component child authority | `COMPONENT_CHILD_AUTHORITY_INVALID` | Repair/reinstall protected deployment limits; do not retry | recent failure |

The four safe projection leaves introduced by this table are:

| Candidate public leaf | Class/origin | Exact internal producers |
| --- | --- | --- |
| `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | `Invariant` / Component allocation | invalid projection, topology digest mismatch, admitted Spec missing and Spec hash mismatch |
| `PEER_COMPONENT_REQUESTER_UNAUTHORIZED` | `Forbidden` / peer Component provisioning | invalid protected requester/root binding |
| `COMPONENT_CHILD_AUTHORITY_INVALID` | `Invariant` / Component child authority | invalid Component/parent binding, protected Spec missing and invalid deployment limits |
| `COMPONENT_CHILD_PARENT_UNAUTHORIZED` | `Forbidden` / Component child authority | protected parent belongs to another Component tree |

The exact internal codes remain visible only through numeric recent-failure
observability. The public projection leaves reveal no principal, digest,
private topology or stable-state content.

## Unproduced Sediment

`ComponentChildAllocationPolicyError::ComponentCountOverflow` is mentioned
only by the broad conversion in `canic-core/src/error.rs`; no policy path can
construct it. It must be deleted and must not receive a diagnostic code.

The child-allocation input also retains three fields that the policy never
reads:

- `reserved_component_instances`;
- `committed_component_instances`; and
- `root_managed_descendants`.

The top-level allocation input similarly retains an unread
`managed_descendants` field. These are residue from the removed root-wide
physical-Canister ceiling. B4 should remove them from the pure input and all
callers instead of manufacturing capacity checks or diagnostic leaves that the
current design no longer owns.

## Required Tests

- one exhaustive mapping case for every 34 live policy variants;
- proof that the four projection leaves never reveal exact masked variants;
- exact retry tests for stale Registry and each recoverable capacity/readiness
  state;
- negative tests that foreign bindings and parents never enter a recovery or
  capacity path;
- numeric recent-failure evidence for every masked variant; and
- residue guards proving the unproduced variant and four unread input fields
  are absent after B4.
