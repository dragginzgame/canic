# Canic 0.102 Configuration Diagnostic Leaves

Date: 2026-08-15

## Status

This is the provisional B1 allocation input for the `ConfigError` conversion
path at immutable baseline `v0.101.53`. It distinguishes native configuration
authoring failures from runtime diagnostics and records every runtime-reachable
compiled-topology variant in this path. It assigns no numbers.

Candidate labels below are formed mechanically from the stated prefix and exact
Rust variant name in upper snake case. For example,
`COMPONENT_TOPOLOGY::FleetAdmissionOverflow` denotes candidate label
`CONFIG_COMPONENT_TOPOLOGY_FLEET_ADMISSION_OVERFLOW`. This notation is an
explicit label mapping, not a reserved numeric band.

## Native Authoring Errors Are Not Runtime Codes

The complete TOML/schema validation producer path is compiled only for native
or test targets:

- `Config::parse_toml` is `cfg(any(not(target_arch = "wasm32"), test))`;
- the `config::validation` module is under the same target gate; and
- every current `ConfigSchemaError::ValidationError(String)` producer lives in
  that native/test validation path.

These authoring failures remain typed native errors with rich host context;
they do not receive `DiagnosticCode` values merely because `ConfigError`
currently wraps their types. The native set is:

- `ConfigTomlIssue::{InvalidDocument, InvalidValue, UnknownField}`;
- `ConfigSchemaError::InvalidCanisterRoleName` with
  `CanisterRoleNameIssue::{Empty, InvalidSnakeCase, TooLong}`; and
- the current finite validation decisions hidden behind
  `ConfigSchemaError::ValidationError(String)` in log, App, whitelist,
  delegated-auth, role-attestation, Component Spec, spawn/provisioning grant,
  cycles-funding, top-up, scaling, sharding, Index, role declaration and
  complete-topology validation.

B4 must split or relocate the native error ownership so these prose-rich
variants and their formatting do not remain in release Wasm. It must not replace
them with compact runtime codes, parse their text or move their host catalogue
into `canic-core`.

This corrects the raw transitive count: all 150 structural variants remain in
the review perimeter, but native-only authoring errors are not public or
operator-emitted runtime diagnostics.

## Path-Qualified Ownership

Several compiled-topology types are also reached through Fleet Registry,
activation and protected-deployment validation. A source variant does not own
one universal code independent of context. This document covers only the
`ConfigError` path:

```text
compiled ConfigModel
  -> ComponentDeploymentConfigurationDigestError
  -> ConfigError
  -> runtime initialization/config access
```

The later Fleet Registry and activation tables must allocate their own
path-qualified leaves when the owning subsystem, action or recovery differs.
They must not reuse a `CONFIG_*` leaf solely because the innermost Rust error
type is shared.

## Common Runtime Projection

Except for the direct lifecycle cases below, every exact compiled-topology leaf
has the same boundary policy in this path:

| Property | Provisional decision |
| --- | --- |
| Class/origin | `Invariant` / compiled configuration |
| Safe public projection | `RUNTIME_CONFIGURATION_INVALID` |
| Action | Correct checked-in App configuration or compiler drift, rebuild and reinstall |
| Retry | Never retry unchanged runtime bytes |
| Exact observability during initialization | Compact numeric lifecycle trap/log before ordinary runtime status exists |
| Exact observability after initialization | Existing bounded recent-failure owner |

The safe projection reveals no private topology, principal, digest, label or
capacity value. `RUNTIME_CONFIGURATION_INVALID` is one additional public leaf;
the exact `CONFIG_*` leaves remain internal.

## Direct Runtime Configuration Leaves

| Candidate label | Current producer | Class/origin | Public projection | Action/retry | Observability |
| --- | --- | --- | --- | --- | --- |
| `CONFIG_ALREADY_INITIALIZED` | `ConfigError::AlreadyInitialized` | `Conflict` / runtime configuration | `RUNTIME_CONFIGURATION_CONFLICT` | Reject contradictory initialization; never retry as a fresh init | lifecycle numeric log or recent failure |
| `CONFIG_NOT_INITIALIZED` | `ConfigError::NotInitialized` | `Unavailable` / runtime configuration | `RUNTIME_CONFIGURATION_UNAVAILABLE` | Complete initialization before request; retry only after readiness changes | recent failure |
| `CONFIG_DEPLOYMENT_CANONICAL_BYTES_EXCEEDED` | `ConfigError::ComponentDeploymentConfigurationCanonicalBytesBoundExceeded` | `Invariant` / compiled configuration | `RUNTIME_CONFIGURATION_INVALID` | Reduce checked-in configuration, rebuild and reinstall; do not retry unchanged | lifecycle numeric log |

`RUNTIME_CONFIGURATION_CONFLICT` and `RUNTIME_CONFIGURATION_UNAVAILABLE` are
safe public projection leaves. They deliberately do not disclose the stored
configuration or initialization owner.

## Runtime IC Root-Key Hard Cut

`ConfigError::RuntimeRootKey(String)` currently combines three exact runtime
decisions produced by `bootstrap::inject_runtime_ic_root_public_key_from` and
`domain::auth::ic_root_public_key_raw_from_der_or_raw`:

| Candidate exact leaf | Required typed producer | Public projection | Action/retry |
| --- | --- | --- | --- |
| `CONFIG_RUNTIME_IC_ROOT_KEY_LENGTH_INVALID` | raw/DER key length differs from both accepted forms | `RUNTIME_CONFIGURATION_INVALID` | Correct local replica root-key evidence; reinstall/restart, no unchanged retry |
| `CONFIG_RUNTIME_IC_ROOT_KEY_DER_PREFIX_INVALID` | DER prefix is not the canonical IC BLS prefix | `RUNTIME_CONFIGURATION_INVALID` | Correct local replica root-key evidence; reinstall/restart, no unchanged retry |
| `CONFIG_RUNTIME_IC_ROOT_KEY_NETWORK_MISMATCH` | local build observes the mainnet IC root key | `RUNTIME_CONFIGURATION_INVALID` | Correct build-network/replica selection; reinstall/restart, no unchanged retry |

B4 must introduce the three typed reasons and delete the string bucket. All are
masked internal leaves with compact lifecycle-log observability during init.

## Component Topology Reachability

Prefix: `CONFIG_COMPONENT_TOPOLOGY_`.

The structural perimeter contains all 43 `ComponentTopologyError` variants, but
only 13 have a producer on this exact path. `ComponentTopology::compile` builds
Specs and grants from canonical `BTreeMap` inputs, then calls
`canonical_bytes`. It does not call root-admission, binding or Fleet-placement
validation.

The 13 current path-reachable variants are retained as candidate internal
leaves:

```text
CanonicalBytesExceeded
ProvisioningGrantBoundExceeded
SelfProvisioningGrant
CyclicProvisioningGrant
UnknownProvisioningGrantTarget
ZeroProvisioningGrantLimit
MissingRoleDeclaration
ZeroSpawnGrantLimit
InvalidSingletonSpawnGrantLimit
UnknownSpawnGrantParent
UnknownSpawnGrantChild
ChildRoleWithoutSpawnGrant
SpawnGrantBoundExceeded
```

Thirty variants are excluded from the `ConfigError` allocation path:

| Reason they are unreachable here | Exact variants |
| --- | --- |
| Root admission projection is not called | `AdmissionSpecHashMismatch`, `EmptyRootAdmissions`, `FleetAdmissionOverflow`, `FleetAdmissionsExceedMaximum`, `MissingFleetAdmission`, `NonCanonicalAdmissionOrder`, `RootAdmissionExceedsFleetMaximum`, `UnknownAdmissionSpec`, `ZeroRootAdmission` |
| Protected root/Component/child binding validation is not called | `AnonymousBindingPrincipal`, `BindingAuthorityMismatch`, `BindingComponentRoleMismatch`, `BindingPlacementSubnetMismatch`, `BindingRootMismatch`, `BindingSpecHashMismatch`, `ChildRoleNotAdmitted`, `ChildPrincipalConflictsWithOwner`, `ChildParentConflictsWithAuthority`, `ComponentPrincipalConflictsWithAuthority`, `DuplicateFleetSubnetRootPrincipal`, `DuplicateFleetSubnetRootSubnet`, `RootAuthorityMismatch`, `RootPrincipalConflictsWithCoordinator`, `RootTopologyDigestMismatch` |
| Root-local limit validation is not called | `NonPositiveRootLimit`, `InvalidRootCanisterPoolRange` |
| Fresh compilation constructs canonical order from `BTreeMap` iteration | `NonCanonicalComponentSpecOrder`, `NonCanonicalComponentChildOrder`, `NonCanonicalProvisioningGrantOrder`, `NonCanonicalSpawnGrantOrder` |

Those variants remain candidates in later admission, Registry or activation
paths only where a current producer is proved. They must not receive a
`CONFIG_COMPONENT_TOPOLOGY_*` code.

## Component Group Leaves

Prefix: `CONFIG_COMPONENT_GROUP_`.

Nineteen direct `ComponentGroupTopologyError` variants are exact internal
candidates before producer reachability:

```text
GroupBoundExceeded
EmptyGroup
MemberBoundExceeded
DeclaredMemberBoundExceeded
InclusionBoundExceeded
DuplicateMember
UnknownComponentSpec
UnknownIncludedGroup
UnknownGroup
InclusionCycle
FlattenedMemberBoundExceeded
InapplicableServicePurposeAssignment
LabelBoundExceeded
NonCanonicalLabelOrder
DuplicateEffectiveLabel
EffectiveLabelBoundExceeded
CanonicalBytesBoundExceeded
NonCanonicalGroupOrder
NonCanonicalMemberOrder
```

`ComponentGroupTopologyError::InvalidMemberPath` is an ownership wrapper. Its
three exact nested leaves use prefix `CONFIG_COMPONENT_GROUP_MEMBER_PATH_`:

```text
Empty
TooDeep
TooLong
```

The wrapper itself receives no code.

Fresh Group compilation and the deployment compiler that consumes it prove 18
of the 22 candidates reachable. Four are excluded from this `ConfigError` path:

- `InvalidMemberPath(Empty)`: flattening pushes one member before constructing
  a path;
- `NonCanonicalLabelOrder`: source labels originate in `BTreeMap` order;
- `NonCanonicalGroupOrder`: source groups originate in `BTreeMap` order; and
- `NonCanonicalMemberOrder`: `compile_group` explicitly sorts merged member
  declarations.

`UnknownGroup` remains reachable because a deployment may select a group absent
from the compiled graph. `InvalidMemberPath(TooDeep|TooLong)`, duplicate or
over-limit effective labels, inclusion cycles and flattened-member bounds are
all reachable while flattening source declarations.

## Component Group Deployment Leaves

Prefix: `CONFIG_COMPONENT_GROUP_DEPLOYMENT_`.

The transparent `ComponentGroupTopology`, `ComponentTopology` and `MemberLimit`
variants receive no duplicate codes in this path. The remaining 23 exact
candidates are:

```text
CanonicalBytesBoundExceeded
DeploymentBoundExceeded
DeploymentMemberBoundExceeded
ZeroMaximumPlacements
InitialPlacementsExceedMaximum
ZeroMaximumPerRoot
MaximumPerRootExceedsMaximumPlacements
ZeroMinimumDistinctRoots
MinimumDistinctRootsExceedMaximumPlacements
NonCanonicalDeploymentOrder
MemberProjectionMismatch
UnknownComponentSpec
ComponentSpecHashMismatch
InapplicableServicePurposeAssignment
MissingServicePurposeAssignment
MultipleServicePurposeAssignments
LabelBoundExceeded
NonCanonicalLabelOrder
DuplicateEffectiveLabel
EffectiveLabelBoundExceeded
MemberLabelProjectionMismatch
ComponentSpecDemandOverflow
ComponentSpecDemandExceedsMaximum
```

Fresh deployment compilation proves 17 reachable. Six are decoded-projection
or canonical-order checks that cannot fail on the value just constructed from
the same topologies:

```text
NonCanonicalDeploymentOrder
MemberProjectionMismatch
UnknownComponentSpec
ComponentSpecHashMismatch
NonCanonicalLabelOrder
MemberLabelProjectionMismatch
```

They remain eligible in a later decoded-projection boundary only if that exact
boundary has a public or operator-visible producer; they receive no
`CONFIG_COMPONENT_GROUP_DEPLOYMENT_*` code.

## Component Deployment Member-Limit Leaves

Prefix: `CONFIG_COMPONENT_MEMBER_LIMIT_`.

All 15 `ComponentDeploymentMemberLimitError` variants enter the structural
perimeter:

```text
MemberLimitBoundExceeded
SpawnGrantReductionBoundExceeded
UnknownMemberLimitPath
UnknownComponentSpec
DuplicateMemberLimitPath
NonCanonicalMemberLimitOrder
NonCanonicalMemberLimitProjection
ZeroAggregateLimit
AggregateLimitExceedsSpec
DuplicateSpawnGrantLimit
UnknownSpawnGrant
ZeroSpawnGrantLimit
InvalidSingletonSpawnGrantLimit
SpawnGrantLimitExceedsSpec
EffectiveLimitProjectionMismatch
```

Fresh member-limit compilation proves 11 reachable. The following four are
revalidation-only or contradict the just-compiled topology:

```text
NonCanonicalMemberLimitOrder
NonCanonicalMemberLimitProjection
UnknownComponentSpec
EffectiveLimitProjectionMismatch
```

The compiled limit vector is sorted canonically, the flattened member's Spec is
looked up in the same Component topology before limit construction, and the
effective projection is derived and checked by the same function. These four
therefore receive no configuration-path code.

## Fleet-Service Topology Leaves

Prefix: `CONFIG_FLEET_SERVICE_TOPOLOGY_`.

The transparent `ComponentGroupDeploymentTopology` and `ComponentTopology`
variants receive no duplicate codes in this path. The remaining 23 exact
candidates are:

```text
CanonicalBytesBoundExceeded
TargetBoundExceeded
NonCanonicalTargetOrder
TargetProjectionMismatch
OrphanServiceTarget
OrphanServiceOccurrence
UnknownTargetComponentSpec
TargetRoleMismatch
OccurrenceComponentSpecMismatch
AuthorityDeploymentPlacementCountInvalid
MissingServiceAuthority
DuplicateServiceAuthority
AuthoritySelectorMismatch
AuthorityReplicaContainsPoolMember
ActivePoolContainsNonPoolMember
ActivePoolHasNoInitialMember
ServiceMemberCountOverflow
ZeroMaximumMembersPerRoot
MaximumMembersPerRootExceedsMaximum
MaximumMembersPerRootBelowPlacementWidth
ZeroMinimumDistinctRoots
MinimumDistinctRootsExceedsMaximum
MinimumDistinctRootsExceedsMaximumPlacements
```

Fresh target compilation proves 21 reachable. `NonCanonicalTargetOrder` is
excluded because targets originate in `BTreeMap` order, and
`TargetProjectionMismatch` belongs only to decoded-projection `validate`; the
configuration compiler constructs the expected target itself. Neither receives
a `CONFIG_FLEET_SERVICE_TOPOLOGY_*` code.

## Candidate Count And Remaining Review

Before path-specific pruning, this configuration path exposed:

- three direct runtime configuration leaves;
- three replacement leaves for the root-key string bucket;
- 43 Component topology candidates;
- 22 Component Group leaves including member-path reasons;
- 23 deployment leaves;
- 15 member-limit leaves; and
- 23 Fleet-service topology leaves.

The initial perimeter was **132 exact internal candidates plus three safe public
projection leaves**. Exact-path analysis excludes 46 variants:

- 30 Component topology;
- four Component Group;
- six deployment;
- four member-limit; and
- two Fleet-service topology variants.

That leaves **86 producer-reachable exact internal candidates plus three safe
public projections** for semantic grouping. No candidate is retained merely
because its enum exposes a variant.

## Expanded Reachable Label Inventory

The shorthand above expands to the following complete 86-label set. This is a
mechanical review surface for collision checks; it assigns no numbers and does
not re-admit any excluded variant.

| Exact identity | Current typed owner |
| --- | --- |
| `CONFIG_ALREADY_INITIALIZED` | `ConfigError::AlreadyInitialized` |
| `CONFIG_NOT_INITIALIZED` | `ConfigError::NotInitialized` |
| `CONFIG_DEPLOYMENT_CANONICAL_BYTES_EXCEEDED` | `ConfigError::ComponentDeploymentConfigurationCanonicalBytesBoundExceeded` |
| `CONFIG_RUNTIME_IC_ROOT_KEY_LENGTH_INVALID` | `domain::auth::ic_root_public_key_raw_from_der_or_raw` |
| `CONFIG_RUNTIME_IC_ROOT_KEY_DER_PREFIX_INVALID` | `domain::auth::ic_root_public_key_raw_from_der_or_raw` |
| `CONFIG_RUNTIME_IC_ROOT_KEY_NETWORK_MISMATCH` | `bootstrap::inject_runtime_ic_root_public_key_from` |
| `CONFIG_COMPONENT_TOPOLOGY_CANONICAL_BYTES_EXCEEDED` | `ComponentTopologyError::CanonicalBytesExceeded` |
| `CONFIG_COMPONENT_TOPOLOGY_PROVISIONING_GRANT_BOUND_EXCEEDED` | `ComponentTopologyError::ProvisioningGrantBoundExceeded` |
| `CONFIG_COMPONENT_TOPOLOGY_SELF_PROVISIONING_GRANT` | `ComponentTopologyError::SelfProvisioningGrant` |
| `CONFIG_COMPONENT_TOPOLOGY_CYCLIC_PROVISIONING_GRANT` | `ComponentTopologyError::CyclicProvisioningGrant` |
| `CONFIG_COMPONENT_TOPOLOGY_UNKNOWN_PROVISIONING_GRANT_TARGET` | `ComponentTopologyError::UnknownProvisioningGrantTarget` |
| `CONFIG_COMPONENT_TOPOLOGY_ZERO_PROVISIONING_GRANT_LIMIT` | `ComponentTopologyError::ZeroProvisioningGrantLimit` |
| `CONFIG_COMPONENT_TOPOLOGY_MISSING_ROLE_DECLARATION` | `ComponentTopologyError::MissingRoleDeclaration` |
| `CONFIG_COMPONENT_TOPOLOGY_ZERO_SPAWN_GRANT_LIMIT` | `ComponentTopologyError::ZeroSpawnGrantLimit` |
| `CONFIG_COMPONENT_TOPOLOGY_INVALID_SINGLETON_SPAWN_GRANT_LIMIT` | `ComponentTopologyError::InvalidSingletonSpawnGrantLimit` |
| `CONFIG_COMPONENT_TOPOLOGY_UNKNOWN_SPAWN_GRANT_PARENT` | `ComponentTopologyError::UnknownSpawnGrantParent` |
| `CONFIG_COMPONENT_TOPOLOGY_UNKNOWN_SPAWN_GRANT_CHILD` | `ComponentTopologyError::UnknownSpawnGrantChild` |
| `CONFIG_COMPONENT_TOPOLOGY_CHILD_ROLE_WITHOUT_SPAWN_GRANT` | `ComponentTopologyError::ChildRoleWithoutSpawnGrant` |
| `CONFIG_COMPONENT_TOPOLOGY_SPAWN_GRANT_BOUND_EXCEEDED` | `ComponentTopologyError::SpawnGrantBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_COUNT_BOUND_EXCEEDED` | `ComponentGroupTopologyError::GroupBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_EMPTY_GROUP` | `ComponentGroupTopologyError::EmptyGroup` |
| `CONFIG_COMPONENT_GROUP_MEMBER_BOUND_EXCEEDED` | `ComponentGroupTopologyError::MemberBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_DECLARED_MEMBER_BOUND_EXCEEDED` | `ComponentGroupTopologyError::DeclaredMemberBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_INCLUSION_BOUND_EXCEEDED` | `ComponentGroupTopologyError::InclusionBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_DUPLICATE_MEMBER` | `ComponentGroupTopologyError::DuplicateMember` |
| `CONFIG_COMPONENT_GROUP_UNKNOWN_COMPONENT_SPEC` | `ComponentGroupTopologyError::UnknownComponentSpec` |
| `CONFIG_COMPONENT_GROUP_UNKNOWN_INCLUDED_GROUP` | `ComponentGroupTopologyError::UnknownIncludedGroup` |
| `CONFIG_COMPONENT_GROUP_UNKNOWN_GROUP` | `ComponentGroupTopologyError::UnknownGroup` |
| `CONFIG_COMPONENT_GROUP_INCLUSION_CYCLE` | `ComponentGroupTopologyError::InclusionCycle` |
| `CONFIG_COMPONENT_GROUP_FLATTENED_MEMBER_BOUND_EXCEEDED` | `ComponentGroupTopologyError::FlattenedMemberBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_INAPPLICABLE_SERVICE_PURPOSE_ASSIGNMENT` | `ComponentGroupTopologyError::InapplicableServicePurposeAssignment` |
| `CONFIG_COMPONENT_GROUP_LABEL_BOUND_EXCEEDED` | `ComponentGroupTopologyError::LabelBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_DUPLICATE_EFFECTIVE_LABEL` | `ComponentGroupTopologyError::DuplicateEffectiveLabel` |
| `CONFIG_COMPONENT_GROUP_EFFECTIVE_LABEL_BOUND_EXCEEDED` | `ComponentGroupTopologyError::EffectiveLabelBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_CANONICAL_BYTES_BOUND_EXCEEDED` | `ComponentGroupTopologyError::CanonicalBytesBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_MEMBER_PATH_TOO_DEEP` | `ComponentGroupTopologyError::InvalidMemberPath(ComponentGroupMemberPathError::TooDeep)` |
| `CONFIG_COMPONENT_GROUP_MEMBER_PATH_TOO_LONG` | `ComponentGroupTopologyError::InvalidMemberPath(ComponentGroupMemberPathError::TooLong)` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_CANONICAL_BYTES_BOUND_EXCEEDED` | `ComponentGroupDeploymentTopologyError::CanonicalBytesBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_COUNT_BOUND_EXCEEDED` | `ComponentGroupDeploymentTopologyError::DeploymentBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_MEMBER_COUNT_BOUND_EXCEEDED` | `ComponentGroupDeploymentTopologyError::DeploymentMemberBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_ZERO_MAXIMUM_PLACEMENTS` | `ComponentGroupDeploymentTopologyError::ZeroMaximumPlacements` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_INITIAL_PLACEMENTS_EXCEED_MAXIMUM` | `ComponentGroupDeploymentTopologyError::InitialPlacementsExceedMaximum` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_ZERO_MAXIMUM_PER_ROOT` | `ComponentGroupDeploymentTopologyError::ZeroMaximumPerRoot` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_MAXIMUM_PER_ROOT_EXCEEDS_MAXIMUM_PLACEMENTS` | `ComponentGroupDeploymentTopologyError::MaximumPerRootExceedsMaximumPlacements` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_ZERO_MINIMUM_DISTINCT_ROOTS` | `ComponentGroupDeploymentTopologyError::ZeroMinimumDistinctRoots` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_MINIMUM_DISTINCT_ROOTS_EXCEED_MAXIMUM_PLACEMENTS` | `ComponentGroupDeploymentTopologyError::MinimumDistinctRootsExceedMaximumPlacements` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_INAPPLICABLE_SERVICE_PURPOSE_ASSIGNMENT` | `ComponentGroupDeploymentTopologyError::InapplicableServicePurposeAssignment` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_MISSING_SERVICE_PURPOSE_ASSIGNMENT` | `ComponentGroupDeploymentTopologyError::MissingServicePurposeAssignment` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_MULTIPLE_SERVICE_PURPOSE_ASSIGNMENTS` | `ComponentGroupDeploymentTopologyError::MultipleServicePurposeAssignments` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_LABEL_BOUND_EXCEEDED` | `ComponentGroupDeploymentTopologyError::LabelBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_DUPLICATE_EFFECTIVE_LABEL` | `ComponentGroupDeploymentTopologyError::DuplicateEffectiveLabel` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_EFFECTIVE_LABEL_BOUND_EXCEEDED` | `ComponentGroupDeploymentTopologyError::EffectiveLabelBoundExceeded` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_COMPONENT_SPEC_DEMAND_OVERFLOW` | `ComponentGroupDeploymentTopologyError::ComponentSpecDemandOverflow` |
| `CONFIG_COMPONENT_GROUP_DEPLOYMENT_COMPONENT_SPEC_DEMAND_EXCEEDS_MAXIMUM` | `ComponentGroupDeploymentTopologyError::ComponentSpecDemandExceedsMaximum` |
| `CONFIG_COMPONENT_MEMBER_LIMIT_COUNT_BOUND_EXCEEDED` | `ComponentDeploymentMemberLimitError::MemberLimitBoundExceeded` |
| `CONFIG_COMPONENT_MEMBER_LIMIT_SPAWN_GRANT_REDUCTION_BOUND_EXCEEDED` | `ComponentDeploymentMemberLimitError::SpawnGrantReductionBoundExceeded` |
| `CONFIG_COMPONENT_MEMBER_LIMIT_UNKNOWN_MEMBER_LIMIT_PATH` | `ComponentDeploymentMemberLimitError::UnknownMemberLimitPath` |
| `CONFIG_COMPONENT_MEMBER_LIMIT_DUPLICATE_MEMBER_LIMIT_PATH` | `ComponentDeploymentMemberLimitError::DuplicateMemberLimitPath` |
| `CONFIG_COMPONENT_MEMBER_LIMIT_ZERO_AGGREGATE_LIMIT` | `ComponentDeploymentMemberLimitError::ZeroAggregateLimit` |
| `CONFIG_COMPONENT_MEMBER_LIMIT_AGGREGATE_LIMIT_EXCEEDS_SPEC` | `ComponentDeploymentMemberLimitError::AggregateLimitExceedsSpec` |
| `CONFIG_COMPONENT_MEMBER_LIMIT_DUPLICATE_SPAWN_GRANT_LIMIT` | `ComponentDeploymentMemberLimitError::DuplicateSpawnGrantLimit` |
| `CONFIG_COMPONENT_MEMBER_LIMIT_UNKNOWN_SPAWN_GRANT` | `ComponentDeploymentMemberLimitError::UnknownSpawnGrant` |
| `CONFIG_COMPONENT_MEMBER_LIMIT_ZERO_SPAWN_GRANT_LIMIT` | `ComponentDeploymentMemberLimitError::ZeroSpawnGrantLimit` |
| `CONFIG_COMPONENT_MEMBER_LIMIT_INVALID_SINGLETON_SPAWN_GRANT_LIMIT` | `ComponentDeploymentMemberLimitError::InvalidSingletonSpawnGrantLimit` |
| `CONFIG_COMPONENT_MEMBER_LIMIT_SPAWN_GRANT_LIMIT_EXCEEDS_SPEC` | `ComponentDeploymentMemberLimitError::SpawnGrantLimitExceedsSpec` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_CANONICAL_BYTES_BOUND_EXCEEDED` | `FleetServiceTopologyError::CanonicalBytesBoundExceeded` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_TARGET_BOUND_EXCEEDED` | `FleetServiceTopologyError::TargetBoundExceeded` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_ORPHAN_SERVICE_TARGET` | `FleetServiceTopologyError::OrphanServiceTarget` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_ORPHAN_SERVICE_OCCURRENCE` | `FleetServiceTopologyError::OrphanServiceOccurrence` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_UNKNOWN_TARGET_COMPONENT_SPEC` | `FleetServiceTopologyError::UnknownTargetComponentSpec` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_TARGET_ROLE_MISMATCH` | `FleetServiceTopologyError::TargetRoleMismatch` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_OCCURRENCE_COMPONENT_SPEC_MISMATCH` | `FleetServiceTopologyError::OccurrenceComponentSpecMismatch` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_AUTHORITY_DEPLOYMENT_PLACEMENT_COUNT_INVALID` | `FleetServiceTopologyError::AuthorityDeploymentPlacementCountInvalid` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_MISSING_SERVICE_AUTHORITY` | `FleetServiceTopologyError::MissingServiceAuthority` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_DUPLICATE_SERVICE_AUTHORITY` | `FleetServiceTopologyError::DuplicateServiceAuthority` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_AUTHORITY_SELECTOR_MISMATCH` | `FleetServiceTopologyError::AuthoritySelectorMismatch` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_AUTHORITY_REPLICA_CONTAINS_POOL_MEMBER` | `FleetServiceTopologyError::AuthorityReplicaContainsPoolMember` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_ACTIVE_POOL_CONTAINS_NON_POOL_MEMBER` | `FleetServiceTopologyError::ActivePoolContainsNonPoolMember` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_ACTIVE_POOL_HAS_NO_INITIAL_MEMBER` | `FleetServiceTopologyError::ActivePoolHasNoInitialMember` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_SERVICE_MEMBER_COUNT_OVERFLOW` | `FleetServiceTopologyError::ServiceMemberCountOverflow` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_ZERO_MAXIMUM_MEMBERS_PER_ROOT` | `FleetServiceTopologyError::ZeroMaximumMembersPerRoot` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_MAXIMUM_MEMBERS_PER_ROOT_EXCEEDS_MAXIMUM` | `FleetServiceTopologyError::MaximumMembersPerRootExceedsMaximum` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_MAXIMUM_MEMBERS_PER_ROOT_BELOW_PLACEMENT_WIDTH` | `FleetServiceTopologyError::MaximumMembersPerRootBelowPlacementWidth` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_ZERO_MINIMUM_DISTINCT_ROOTS` | `FleetServiceTopologyError::ZeroMinimumDistinctRoots` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_MINIMUM_DISTINCT_ROOTS_EXCEEDS_MAXIMUM` | `FleetServiceTopologyError::MinimumDistinctRootsExceedsMaximum` |
| `CONFIG_FLEET_SERVICE_TOPOLOGY_MINIMUM_DISTINCT_ROOTS_EXCEEDS_MAXIMUM_PLACEMENTS` | `FleetServiceTopologyError::MinimumDistinctRootsExceedsMaximumPlacements` |

Before numeric assignment, B1 must still decide whether any of the 86 reachable
candidates have identical origin, action, retry, exposure and remediation and
may therefore share a code. It must also verify that init-time compact numeric
logging is an approved observability owner before masking the exact code.

## Required Tests

- native compile guards proving TOML/schema prose and complete validation
  messages are absent from representative release Wasm;
- exact typed tests for the three root-key reasons;
- exhaustive path-specific mapping tests for every reachable `ConfigError`
  compiled-topology variant;
- projection tests proving exact topology and root-key identities map only to
  the three safe public leaves;
- initialization tests proving exact numeric lifecycle observability before
  recent-failure state is available; and
- residue guards proving no `RuntimeRootKey(String)`, text classification or
  duplicate wrapper allocation survives B4.
