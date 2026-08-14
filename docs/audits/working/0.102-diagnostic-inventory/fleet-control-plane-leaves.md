# Canic 0.102 Fleet Control-Plane Diagnostic Leaves

Date: 2026-08-13

## Status

This provisional B1 ledger covers the canonical Fleet Registry compiler,
Component provisioning-plan compiler, Fleet-service binding compiler and
shared provisioning-receipt hashing boundary at immutable baseline
`v0.101.53`. It allocates no numbers. All direct typed variants below have a
production producer; wrappers and formatted typed causes receive no duplicate
code.

## Fleet Registry

All 44 direct `FleetRegistryOpsError` variants have current production
producers. Their exact candidate labels are grouped below by operator action;
each label remains independently identifiable and is safe as its own public
projection because no dynamic Fleet, principal, Spec, service, hash, count or
limit is included.

### Authority And Genesis

Correct the immutable authority/genesis evidence and retry only with the exact
expected Fleet identity:

```text
FLEET_REGISTRY_COORDINATOR_ANONYMOUS
FLEET_REGISTRY_COORDINATOR_SUBNET_ANONYMOUS
FLEET_REGISTRY_ROOT_ANONYMOUS
FLEET_REGISTRY_AUTHORITY_MISMATCH
FLEET_REGISTRY_GENESIS_APP_MISMATCH
FLEET_REGISTRY_GENESIS_AUTHORITY_EPOCH_INVALID
FLEET_REGISTRY_AUTHORITY_EPOCH_NONPOSITIVE
FLEET_REGISTRY_REVISION_NONPOSITIVE
FLEET_REGISTRY_ROOT_CONFLICTS_WITH_COORDINATOR
FLEET_REGISTRY_ROOT_RELEASE_BUILD_MISMATCH
```

### Canonical Snapshot And Capacity

Reject the snapshot and reconstruct it canonically from the Coordinator's
protected authority; arithmetic exhaustion never wraps:

```text
FLEET_REGISTRY_CANONICAL_BYTES_EXCEEDED
FLEET_REGISTRY_ROOT_DUPLICATED
FLEET_REGISTRY_ADMISSIONS_EXCEED_FLEET_MAXIMUM
FLEET_REGISTRY_ADMISSION_COUNT_OVERFLOW
FLEET_REGISTRY_COMPONENT_SPEC_MISMATCH
FLEET_REGISTRY_COMPONENT_SPEC_SET_MISMATCH
FLEET_REGISTRY_ROOT_ORDER_NONCANONICAL
FLEET_REGISTRY_REVISION_EXHAUSTED
```

### Root And Directory State Machines

Inspect the exact current Registry revision and issue only its admitted next
transition. Missing targets are not treated as already transitioned:

```text
FLEET_REGISTRY_ROOT_JOIN_IDENTITY_CONFLICT
FLEET_REGISTRY_ROOT_JOIN_REQUIRES_JOINING
FLEET_REGISTRY_ACTIVATION_REQUIRES_ALL_JOINING
FLEET_REGISTRY_ROOT_DRAIN_REQUIRES_ACTIVE
FLEET_REGISTRY_ROOT_DRAIN_TARGET_MISSING
FLEET_REGISTRY_ROOT_REMOVE_REQUIRES_DRAINING
FLEET_REGISTRY_ROOT_REMOVE_TARGET_MISSING
FLEET_DIRECTORY_REQUIRES_PUBLISHED_ROOTS
FLEET_DIRECTORY_SOURCE_MISSING
```

### Service Publication And Validation

Initial publication must supply the complete configured set; later
publication may append only admitted Replica or PoolMember rows without
changing existing authority or membership:

```text
FLEET_REGISTRY_SERVICE_INITIAL_REQUIRES_EMPTY_CURRENT
FLEET_REGISTRY_SERVICE_INITIAL_REQUIRES_NONEMPTY_COMPLETE_SET
FLEET_REGISTRY_SERVICE_APPEND_REQUIRES_ADDITIONS
FLEET_REGISTRY_SERVICE_APPEND_AUTHORITY_MISMATCH
FLEET_REGISTRY_SERVICE_APPEND_REMOVES_MEMBER
FLEET_REGISTRY_SERVICE_APPEND_ADDS_AUTHORITY
FLEET_REGISTRY_SERVICE_ORDER_NONCANONICAL
FLEET_REGISTRY_SERVICE_MEMBER_ORDER_NONCANONICAL
FLEET_REGISTRY_SERVICE_EMPTY
FLEET_REGISTRY_SERVICE_SPEC_MISMATCH
FLEET_REGISTRY_SERVICE_MODE_MISMATCH
FLEET_REGISTRY_SERVICE_PLACEMENT_MISMATCH
FLEET_REGISTRY_SERVICE_ROOT_MISMATCH
FLEET_REGISTRY_SERVICE_COMPONENT_ID_EMPTY
FLEET_REGISTRY_SERVICE_COMPONENT_ANONYMOUS
FLEET_REGISTRY_SERVICE_COMPONENT_DUPLICATED
FLEET_REGISTRY_SERVICE_CANISTER_DUPLICATED
```

`FleetRegistryOpsError::Topology` is transparent but path-qualified because it
validates root rows in canonical Coordinator Registry state, not native App
configuration. Ten `ComponentTopologyError` decisions are reachable:

```text
FLEET_REGISTRY_TOPOLOGY_PLACEMENT_SUBNET_ANONYMOUS
FLEET_REGISTRY_TOPOLOGY_ROOT_LIMIT_NONPOSITIVE
FLEET_REGISTRY_TOPOLOGY_CANISTER_POOL_RANGE_INVALID
FLEET_REGISTRY_TOPOLOGY_ADMISSIONS_EMPTY
FLEET_REGISTRY_TOPOLOGY_ADMISSION_ORDER_NONCANONICAL
FLEET_REGISTRY_TOPOLOGY_ADMISSION_ZERO
FLEET_REGISTRY_TOPOLOGY_ADMISSION_SPEC_UNKNOWN
FLEET_REGISTRY_TOPOLOGY_ADMISSION_SPEC_HASH_MISMATCH
FLEET_REGISTRY_TOPOLOGY_ADMISSION_EXCEEDS_FLEET_MAXIMUM
FLEET_REGISTRY_TOPOLOGY_DIGEST_MISMATCH
```

Canonical topology byte overflow is unreachable because a Registry root holds
a subset of the already bounded compiled topology. Fleet-wide duplicate and
admission-sum decisions are direct Registry leaves above, not nested topology
codes.

The Fleet Registry therefore contributes **54 exact candidates** and no
additional broad projection.

## Component Provisioning Plan

`ComponentProvisioningPlanOpsError::Configuration(String)` and
`FleetRegistry(String)` currently stringify typed configuration and Registry
causes. B4 replaces both with typed cause edges and reuses the already mapped
source diagnostic. The one direct prose construction hidden in
`Configuration(String)`—a root batch referencing an undeclared Fleet
service—becomes `COMPONENT_PROVISIONING_FLEET_SERVICE_UNKNOWN`.

The 45 direct enum decisions plus that hidden decision contribute 46 exact
candidates. Every label below is safe as its own public projection; submitted
IDs, roots, counts, hashes and limits are omitted.

### Bounds And Arithmetic

Reduce the exact plan to its documented bound, or stop on checked arithmetic
exhaustion:

```text
COMPONENT_PROVISIONING_CANONICAL_BYTES_EXCEEDED
COMPONENT_PROVISIONING_BATCH_COUNT_EXCEEDED
COMPONENT_PROVISIONING_CONFIRMATION_ROOT_COUNT_EXCEEDED
COMPONENT_PROVISIONING_PLACEMENT_COUNT_EXCEEDED
COMPONENT_PROVISIONING_COMPONENT_COUNT_EXCEEDED
COMPONENT_PROVISIONING_COUNT_OVERFLOW
```

### Fleet, Root And Canonical Authority

Rebuild the plan from the exact current Registry version and compiled App
authority. A stale plan is rejected rather than silently normalized:

```text
COMPONENT_PROVISIONING_FLEET_MISMATCH
COMPONENT_PROVISIONING_FLEET_REGISTRY_VERSION_MISMATCH
COMPONENT_PROVISIONING_CONFIGURATION_DIGEST_MISMATCH
COMPONENT_PROVISIONING_CONFIRMATION_ROOT_ORDER_NONCANONICAL
COMPONENT_PROVISIONING_CONFIRMATION_ROOT_ANONYMOUS
COMPONENT_PROVISIONING_BATCH_ORDER_NONCANONICAL
COMPONENT_PROVISIONING_ROOT_BINDING_MISMATCH
COMPONENT_PROVISIONING_ROOT_RELEASE_SET_MISMATCH
COMPONENT_PROVISIONING_SELECTED_ROOT_NOT_CONFIRMED
COMPONENT_PROVISIONING_FRESH_CONFIRMATION_ROOT_SET_MISMATCH
COMPONENT_PROVISIONING_FRESH_BATCH_ROOT_SET_MISMATCH
COMPONENT_PROVISIONING_CONFIRMATION_ROOT_NOT_ACTIVE
COMPONENT_PROVISIONING_FRESH_ROOT_NOT_ACTIVE
COMPONENT_PROVISIONING_PLACEMENT_ORDER_NONCANONICAL
```

### Placement, Admission And Density

Correct the configured deployment selection, root admission or density/spread
assignment before retrying:

```text
COMPONENT_PROVISIONING_PLACEMENT_DUPLICATED
COMPONENT_PROVISIONING_DEPLOYMENT_UNKNOWN
COMPONENT_PROVISIONING_COMPONENT_GROUP_MISMATCH
COMPONENT_PROVISIONING_PLACEMENT_ENTRIES_MISMATCH
COMPONENT_PROVISIONING_ROOT_ADMISSION_MISSING
COMPONENT_PROVISIONING_ROOT_ADMISSION_EXCEEDED
COMPONENT_PROVISIONING_ROOT_COMPONENT_CAPACITY_EXCEEDED
COMPONENT_PROVISIONING_ROOT_GROUP_PLACEMENT_CAPACITY_EXCEEDED
COMPONENT_PROVISIONING_FRESH_PLACEMENT_SET_MISMATCH
COMPONENT_PROVISIONING_FRESH_PLACEMENT_POLICY_MISMATCH
COMPONENT_PROVISIONING_FRESH_SERVICE_PLACEMENT_POLICY_MISMATCH
COMPONENT_PROVISIONING_ROOT_DEPLOYMENT_DENSITY_EXCEEDED
COMPONENT_PROVISIONING_ROOT_SERVICE_DENSITY_EXCEEDED
COMPONENT_PROVISIONING_FLEET_SERVICE_UNKNOWN
```

### Scale-Out

Scale-out may only extend the durable placement ledger's next ordinal range
over eligible installed roots, without adding Authority occurrences:

```text
COMPONENT_PROVISIONING_SCALE_OUT_STATE_UNAVAILABLE
COMPONENT_PROVISIONING_SCALE_OUT_COMMITTED_PLACEMENTS_NONCANONICAL
COMPONENT_PROVISIONING_SCALE_OUT_ELIGIBLE_ROOTS_NONCANONICAL
COMPONENT_PROVISIONING_SCALE_OUT_ROOT_INELIGIBLE
COMPONENT_PROVISIONING_SCALE_OUT_COUNT_MISMATCH
COMPONENT_PROVISIONING_SCALE_OUT_PLACEMENT_SET_MISMATCH
COMPONENT_PROVISIONING_SCALE_OUT_DEPLOYMENT_MISMATCH
COMPONENT_PROVISIONING_SCALE_OUT_AUTHORITY_FORBIDDEN
COMPONENT_PROVISIONING_SCALE_OUT_PLACEMENT_POLICY_MISMATCH
COMPONENT_PROVISIONING_SCALE_OUT_SERVICE_PLACEMENT_POLICY_MISMATCH
COMPONENT_PROVISIONING_SCALE_OUT_CONFIRMATION_ROOT_SET_MISMATCH
COMPONENT_PROVISIONING_SCALE_OUT_BATCH_EMPTY
```

## Fleet-Service Binding

`FleetServiceBindingOpsError::Configuration(String)` must preserve the compiled
configuration cause. `Plan(String)` currently flattens provisioning-plan,
Fleet Registry and receipt-hashing causes; B4 replaces it with typed edges to
those owners. Neither wrapper receives a code.

The 22 direct decisions are:

```text
FLEET_SERVICE_BINDING_COMPONENT_ID_DUPLICATED
FLEET_SERVICE_BINDING_CANISTER_DUPLICATED
FLEET_SERVICE_BINDING_AUTHORITY_DUPLICATED
FLEET_SERVICE_BINDING_OPERATION_ID_EMPTY
FLEET_SERVICE_BINDING_SERVICE_EMPTY
FLEET_SERVICE_BINDING_AUTHORITY_INVALID
FLEET_SERVICE_BINDING_MEMBER_PURPOSE_INVALID
FLEET_SERVICE_BINDING_SCALE_OUT_MEMBER_PURPOSE_INVALID
FLEET_SERVICE_BINDING_SCALE_OUT_OPERATION_INVALID
FLEET_SERVICE_BINDING_PLACEMENT_INVALID
FLEET_SERVICE_BINDING_ROOT_RECEIPT_COUNT_MISMATCH
FLEET_SERVICE_BINDING_ROOT_RECEIPT_INDEX_INVALID
FLEET_SERVICE_BINDING_ROOT_RECEIPT_COUNTS_MISMATCH
FLEET_SERVICE_BINDING_ROOT_RECEIPT_IDENTITY_MISMATCH
FLEET_SERVICE_BINDING_ROOT_RECEIPT_HASH_MISMATCH
FLEET_SERVICE_BINDING_ROOT_RECEIPT_RESULT_MISMATCH
FLEET_SERVICE_BINDING_ROOT_RECEIPT_STATE_MISMATCH
FLEET_SERVICE_BINDING_ROOT_RECEIPT_TIME_MISMATCH
FLEET_SERVICE_BINDING_COUNT_OVERFLOW
FLEET_SERVICE_BINDING_SERVICE_UNEXPECTED
FLEET_SERVICE_BINDING_PUBLISHED_SERVICE_MISMATCH
FLEET_SERVICE_BINDING_PUBLISHED_SERVICE_SET_MISMATCH
```

Identity/configuration and placement failures require rebuilding from the
compiled service target. Receipt failures reject the entire publication and
retry only after fetching the exact terminal root receipt. Count overflow stops
the compiler. All labels are safe public projections as written; receipt
contents and service identities remain out of the compact error.

## Shared Receipt Hashing

`RootComponentProvisioningReceiptOps` currently constructs two direct
prose-rich `InternalError` values for every receipt/directory hash operation:

| Candidate label | Current producer | Public projection | Action and retry |
| --- | --- | --- | --- |
| `COMPONENT_PROVISIONING_RECEIPT_ENCODE_FAILED` | Candid encode error | `COMPONENT_PROVISIONING_RECEIPT_INVALID` | Stop publication; fix the canonical DTO/encoder |
| `COMPONENT_PROVISIONING_RECEIPT_BYTE_COUNT_EXCEEDED` | `usize` to `u64` failure | `COMPONENT_PROVISIONING_RECEIPT_INVALID` | Stop publication; enforce the canonical receipt bound |

B4 must introduce a typed receipt-hash error and preserve it through
Fleet-service `Plan` and all other callers. Candid formatter text and the
dynamic receipt label do not become diagnostic identity.

## Current Count

This pass contributes **124 exact semantic candidates**:

- 54 Fleet Registry leaves, including ten reachable topology causes;
- 46 Component provisioning-plan leaves;
- 22 Fleet-service binding leaves; and
- two shared receipt-hashing leaves.

It introduces one additional safe projection,
`COMPONENT_PROVISIONING_RECEIPT_INVALID`. The Registry, plan and service
compiler leaves are safe exact public identities and need no broad additional
projection.

## Required Tests

- exhaustive direct-variant mappings for all three compiler enums;
- source-reachability guard for the ten Registry topology causes;
- typed configuration, Registry, plan and receipt cause preservation with no
  formatted wrapper;
- exact canonical-order, bound, authority and state-transition negatives;
- conflicting/stale Registry and plan evidence never normalized or retried as
  fresh;
- receipt mismatch cases remain independently actionable while receipt values
  are absent publicly; and
- receipt encoding/provider text is masked but numerically observable.
