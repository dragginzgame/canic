# Canic 0.102 Component Runtime Constructor Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger classifies all thirty-four production
`InternalError` constructor references in Component runtime Directory
preparation, synchronization, activation, validation and canonical hashing. It
assigns no number and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/component_runtime.rs` | 32 |
| `ops/component_runtime.rs` | 2 |
| **Total** | **34** |

## Storage Edges And Existing Runtime Identities

Three workflow sites are transparent typed storage transitions. Ten existing
identities cover the repeated runtime-state decisions:

| Exact candidate or disposition | Sites | Producer function/branch | Required hard cut |
| --- | ---: | --- | --- |
| transparent typed Component-runtime storage cause | 3 | `prepare_directory`, `synchronize_directory` and `activate` storage adapters | Preserve exact prepare, synchronize or activation storage transition |
| reuse `COMPONENT_RUNTIME_DIRECTORY_OPERATION_MISMATCH` | 2 | `validate_request` and `synchronize_directory`; installation operation differs | Bind preparation/synchronization to the protected installation operation |
| reuse `COMPONENT_RUNTIME_PHASE_NOT_ACTIVE` | 1 | `synchronize_directory`; retained runtime is not exactly Active | Synchronize only an exactly Active runtime |
| reuse `COMPONENT_RUNTIME_CURRENT_DIRECTORY_MISSING` | 2 | `synchronize_directory` and `status`; required current authority is absent or incomplete | Fail closed when Active or partially retained state lacks current authority |
| reuse `COMPONENT_RUNTIME_CURRENT_DIRECTORY_HASH_MISSING` | 1 | `component_runtime::status`; retained authority and hash presence disagree | Fail closed when authority exists without its retained hash |
| reuse `COMPONENT_RUNTIME_CURRENT_DIRECTORY_HASH_INVALID` | 1 | `component_runtime::status`; recomputed authority hash differs | Recompute and compare exact retained authority bytes |
| reuse `COMPONENT_RUNTIME_ACTIVATION_DIRECTORY_HASH_MISSING` | 1 | `component_runtime::activate`; activation supplies the zero authority hash | Reject zero activation authority before transition |
| reuse `COMPONENT_RUNTIME_PROTECTED_BINDING_MISMATCH` | 1 | `validate_binding`; protected binding names another Canister | Require the protected binding to name the receiver Canister |
| reuse `COMPONENT_DIRECTORY_COMPONENT_AUTHORITY_MISMATCH` | 1 | `validate_authority`; Component provenance differs from the protected binding | Reject a Component Directory owned by another Component |
| reuse `COMPONENT_DIRECTORY_FORWARD_PROGRESSION_INVALID` | 1 | `validate_directory_progression`; next authority is neither exact replay nor monotonic advance | Accept exact replay or one monotonic later authority only |
| reuse `FLEET_DIRECTORY_SOURCE_MISSING` | 1 | `validate_fleet_directory`; source root is absent | Require the local Component's source root in the Fleet Directory |

The Directory-operation and missing-current-authority identities each occur at
two source sites but remain one semantic candidate. Storage conversion through
`StorageOpsError::to_string()` is a temporary adapter, not three additional
runtime codes.

## Direct Children, Service Authority And Activation

Five workflow branches add six exact meanings:

| Exact candidate | Sites | Producer function/branch | Action and retry |
| --- | ---: | --- | --- |
| `COMPONENT_RUNTIME_DIRECT_CHILD_ORDER_INVALID` / `COMPONENT_RUNTIME_DIRECT_CHILD_DUPLICATE` | 1 | `validate_direct_children`; direct-child projection is not canonical or repeats a row | Rebuild from exact registered direct children |
| `COMPONENT_RUNTIME_DIRECT_CHILD_SELF` | 1 | `validate_direct_children`; runtime appears in its own direct-child projection | Repair parentage; never import the self-edge |
| `COMPONENT_RUNTIME_SERVICE_AUTHORITY_REQUIRED` | 1 | `require_service_authority`; runtime lacks the exact Active Fleet-service Authority purpose | Invoke only the configured active Authority member |
| `COMPONENT_RUNTIME_ACTIVATION_OPERATION_MISMATCH` / `COMPONENT_RUNTIME_ACTIVATION_DIRECTORY_HASH_MISMATCH` | 1 | `component_runtime::activate`; activation names another installation or prepared Directory hash | Replay the exact installation-bound activation request |

The activation branch also reuses the existing missing-hash identity. B4 must
split operation, zero hash and changed hash before allocation; no compound
activation code may authorize a transition.

## Component Directory Authority

Initial authority validation adds four exact field identities alongside the
existing Component-authority check:

| Exact candidate | Sites | Producer function/branch | Required hard cut |
| --- | ---: | --- | --- |
| `COMPONENT_RUNTIME_COMPONENT_DIRECTORY_SOURCE_ROOT_MISMATCH` | 1 | `validate_authority`; source-root identity predicate | Bind provenance to the Component's exact Fleet Subnet Root |
| `COMPONENT_RUNTIME_COMPONENT_DIRECTORY_REVISION_INVALID` | 1 | `validate_authority`; positive-revision predicate | Require a positive Component Registry revision |
| `COMPONENT_RUNTIME_COMPONENT_DIRECTORY_HASH_INVALID` | 1 | `validate_authority`; nonzero-content-hash predicate | Require a nonzero Component Registry content hash |
| `COMPONENT_RUNTIME_COMPONENT_DIRECTORY_TIME_INVALID` | 1 | `validate_authority`; positive-synchronization-time predicate | Require a positive synchronization observation time |

The later-authority progression branch reuses
`COMPONENT_DIRECTORY_FORWARD_PROGRESSION_INVALID`. Its implementation still
needs named predicates for stable Component/source identity, advancing
revision/hash/time and monotonic Fleet authority, but those predicates share
the existing authenticated caller action and exact progression decision.

## Fleet Directory Authority

Nine constructor branches expand into thirty exact runtime-validation
meanings:

| Exact candidates | Sites | Producer function/branch |
| --- | ---: | --- |
| `COMPONENT_RUNTIME_FLEET_DIRECTORY_AUTHORITY_MISMATCH` / `COMPONENT_RUNTIME_FLEET_DIRECTORY_SOURCE_ROOT_MISMATCH` / `COMPONENT_RUNTIME_FLEET_DIRECTORY_REVISION_INVALID` / `COMPONENT_RUNTIME_FLEET_DIRECTORY_HASH_INVALID` / `COMPONENT_RUNTIME_FLEET_DIRECTORY_ROOT_SET_EMPTY` | 1 | `validate_fleet_directory`; Registry authority, source root, version and nonempty-root predicates |
| `COMPONENT_RUNTIME_FLEET_ROOT_STATUS_UNPUBLISHED` / `COMPONENT_RUNTIME_FLEET_ROOT_ORDER_INVALID` / `COMPONENT_RUNTIME_FLEET_ROOT_DUPLICATE` | 1 | `validate_fleet_directory`; published canonical root-row predicates |
| `COMPONENT_RUNTIME_FLEET_SOURCE_ROOT_DUPLICATE` / `COMPONENT_RUNTIME_FLEET_SOURCE_SUBNET_MISMATCH` / `COMPONENT_RUNTIME_FLEET_SOURCE_STATUS_INVALID` | 1 | `validate_fleet_directory`; unique exact local-source-root predicates |
| `COMPONENT_RUNTIME_FLEET_SERVICE_ORDER_INVALID` / `COMPONENT_RUNTIME_FLEET_SERVICE_DUPLICATE` / `COMPONENT_RUNTIME_FLEET_SERVICE_MEMBERS_EMPTY` / `COMPONENT_RUNTIME_FLEET_SERVICE_MAXIMUM_PER_ROOT_INVALID` / `COMPONENT_RUNTIME_FLEET_SERVICE_MINIMUM_ROOTS_INVALID` | 1 | `validate_fleet_services`; canonical bounded service-header predicates |
| `COMPONENT_RUNTIME_FLEET_SERVICE_MEMBER_ORDER_INVALID` / `COMPONENT_RUNTIME_FLEET_SERVICE_MEMBER_DUPLICATE` | 1 | `validate_fleet_services`; canonical unique service-member predicate |
| `COMPONENT_RUNTIME_FLEET_SERVICE_AUTHORITY_COUNT_OVERFLOW` | 1 | `validate_fleet_services`; checked Authority-member count |
| `COMPONENT_RUNTIME_FLEET_COMPONENT_MEMBERSHIP_DUPLICATE` / `COMPONENT_RUNTIME_FLEET_COMPONENT_ROOT_MISMATCH` / `COMPONENT_RUNTIME_FLEET_COMPONENT_CANISTER_MISMATCH` / `COMPONENT_RUNTIME_FLEET_COMPONENT_SPEC_MISMATCH` | 1 | `validate_fleet_services`; unique protected Component-membership predicates |
| `COMPONENT_RUNTIME_FLEET_SERVICE_MODE_INVALID` | 1 | `validate_fleet_services`; `fleet_service_mode_is_valid` rejects member-purpose disagreement |
| `COMPONENT_RUNTIME_SERVICE_MEMBERSHIP_MISSING` / `COMPONENT_RUNTIME_SERVICE_MEMBERSHIP_UNEXPECTED` / `COMPONENT_RUNTIME_SERVICE_ID_MISMATCH` / `COMPONENT_RUNTIME_SERVICE_PURPOSE_MISMATCH` / `COMPONENT_RUNTIME_SERVICE_PLACEMENT_MISMATCH` / `COMPONENT_RUNTIME_SERVICE_MEMBER_PATH_MISMATCH` | 1 | `validate_component_service_membership`; Directory membership differs from protected deployment purpose |

The source-root absence branch separately reuses
`FLEET_DIRECTORY_SOURCE_MISSING`. Ordering and duplication must be tested
independently even though the current `previous < key` predicates merge them.
Likewise, a missing protected service membership differs from an unexpected
membership and from each changed service field.

## Component Group Directory Authority

Six constructor branches expand into twenty-two exact meanings:

| Exact candidates | Sites | Producer function/branch |
| --- | ---: | --- |
| `COMPONENT_RUNTIME_GROUP_DIRECTORY_UNEXPECTED` / `COMPONENT_RUNTIME_GROUP_DIRECTORY_MISSING` | 2 | `validate_component_group_directory`; ordinary/grouped Directory-presence branches |
| `COMPONENT_RUNTIME_GROUP_PROVENANCE_AUTHORITY_MISMATCH` / `COMPONENT_RUNTIME_GROUP_PROVENANCE_ROOT_MISMATCH` / `COMPONENT_RUNTIME_GROUP_PROVENANCE_PLACEMENT_MISMATCH` / `COMPONENT_RUNTIME_GROUP_PROVENANCE_GROUP_MISMATCH` / `COMPONENT_RUNTIME_GROUP_PROVENANCE_OPERATION_INVALID` / `COMPONENT_RUNTIME_GROUP_PROVENANCE_PLAN_HASH_INVALID` / `COMPONENT_RUNTIME_GROUP_PROVENANCE_RECEIPT_HASH_INVALID` / `COMPONENT_RUNTIME_GROUP_MEMBERS_EMPTY` | 1 | `validate_component_group_directory`; exact provenance and nonempty-membership predicates |
| `COMPONENT_RUNTIME_GROUP_MEMBER_PATH_ORDER_INVALID` / `COMPONENT_RUNTIME_GROUP_MEMBER_PATH_DUPLICATE` / `COMPONENT_RUNTIME_GROUP_MEMBER_CANISTER_DUPLICATE` / `COMPONENT_RUNTIME_GROUP_MEMBER_COMPONENT_DUPLICATE` / `COMPONENT_RUNTIME_GROUP_MEMBER_AUTHORITY_MISMATCH` / `COMPONENT_RUNTIME_GROUP_MEMBER_ROOT_MISMATCH` | 1 | `validate_component_group_directory`; canonical unique root-local member predicates |
| `COMPONENT_RUNTIME_GROUP_OWN_MEMBER_DUPLICATE` / `COMPONENT_RUNTIME_GROUP_OWN_MEMBER_SPEC_MISMATCH` / `COMPONENT_RUNTIME_GROUP_OWN_MEMBER_PURPOSE_MISMATCH` / `COMPONENT_RUNTIME_GROUP_OWN_MEMBER_LABELS_MISMATCH` / `COMPONENT_RUNTIME_GROUP_OWN_MEMBER_BINDING_MISMATCH` | 1 | `validate_component_group_directory`; exact own-member predicates |
| `COMPONENT_RUNTIME_GROUP_OWN_MEMBER_MISSING` | 1 | `validate_component_group_directory`; protected own-member row was not found |

None of these codes grants group membership. The protected deployment and
Directory remain the authorities; exact diagnostics identify only the failed
validation and repair path.

## Canonical Hashing

The two ops constructors add exact codec meanings:

| Exact candidate | Sites | Producer function/branch | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `COMPONENT_RUNTIME_DIRECTORY_AUTHORITY_ENCODE_FAILED` | 1 | `ComponentRuntimeOps::directory_authority_hash`; Candid encoding fails | `COMPONENT_REGISTRY_STATE_INVALID` | Stop transition and repair the canonical authority DTO/encoder |
| `COMPONENT_RUNTIME_DIRECT_CHILDREN_ENCODE_FAILED` | 1 | `ComponentRuntimeOps::direct_children_hash`; Candid encoding fails | `COMPONENT_REGISTRY_STATE_INVALID` | Stop transition and repair the canonical direct-child DTO/encoder |

Candid formatter text is implementation detail. B4 preserves a finite typed
codec cause and never hashes partially encoded or fallback bytes.

## Dynamic Public Context

Two values are classified as `DPC-329` and `DPC-330` in
[dynamic-public-context.md](dynamic-public-context.md). They are the typed
Candid encoding causes for the two canonical hashes.

Activation-service startup failure currently traps after the durable
transition. It remains lifecycle trap/log evidence rather than public
`Error.message`, and receives no dynamic-public row or wrapper code.

## Reconciliation

All thirty-four direct sites now have one disposition. They add sixty-four
exact meanings, reuse ten existing semantic identities and retain three typed
storage edges. The effective constructor frontier moves from 2,395 to 2,429
classified sites and from 104 to 70 open sites. The qualified semantic set
reaches 2,606 exact candidates plus 31 safe projections: 2,637 current
symbolic identities.

## Required Tests

- exhaustive prepare/synchronize/activate storage propagation;
- independent direct-child order, duplicate and self-edge rejection;
- independent activation operation, missing-hash and changed-hash rejection;
- complete Component Directory provenance field tests;
- exact replay and monotonic skipped-revision progression tests;
- independent Fleet Directory authority, version, source-root, ordering,
  membership, mode and protected deployment tests;
- independent Component Group provenance, canonical member and own-member
  tests;
- service Authority access only for the exact Active configured member;
- stable canonical hash vectors and typed encoding-failure mapping; and
- no runtime-service hook rescheduling or second transition after exact
  activation replay.

## Next Slice

Continue with Component Directory synchronization ops and the remaining small
runtime adapters.
