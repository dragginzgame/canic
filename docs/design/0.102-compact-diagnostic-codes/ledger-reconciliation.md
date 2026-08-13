# Canic 0.102 Diagnostic Ledger Reconciliation

Date: 2026-08-13

## Status

This document reconciles the provisional symbolic identities discovered by the
B1 family audits. It allocates no numeric code and does not make a candidate a
runtime or protocol authority.

The family documents remain the evidence for producer ownership, action,
retry, exposure and current-path reachability. This document proves that their
current candidate names form one collision-free **qualified subset**. It does
not claim coverage of the direct-constructor frontier recorded in
[direct-constructor-frontier.md](direct-constructor-frontier.md).

## Reconciled Family Arithmetic

| Family | New exact meanings | New safe projections | Reused identities |
| --- | ---: | ---: | --- |
| Runtime configuration/topology | 86 | 3 | 0 |
| Component lifecycle policy | 34 | 4 | 0 |
| Authentication and policy | 132 | 6 | same-semantics auth leaves are already deducted inside the combined family |
| IC infrastructure | 24 | 4 | 0 |
| Bounded runtime owners | 37 | 3 | `RUNTIME_CONFIGURATION_UNAVAILABLE` |
| Cost guard | 7 | 1 | 0 |
| Access boundary | 20 | 2 | six exact auth meanings and two auth projections |
| Runtime/config/environment/RPC ops | 18 | 2 | `ENV_REQUIRED_FIELDS_MISSING`, `ACCESS_ROOT_REQUIRED` and existing dependency/configuration projections |
| Fleet activation | 30 | 1 | `FLEET_ACTIVATION_STATE_INVALID` is an exact leaf reused as a projection and is not counted twice |
| Bounded storage registries | 18 | 2 | 0 |
| Fleet control-plane compilers | 124 | 1 | 0 |
| Durable intent store | 51 | 1 | 0 |
| Pinned `ic-memory` adapter | 74 | 0 | 0 |
| **Reconciled symbolic total** | **655** | **30** | **685 distinct proposed identities** |

“New” is relative to the preceding rows in this table. A later producer that
uses an already declared meaning is evidence for that identity, not permission
to allocate another label or number.

## Mechanical Collision Check

Every currently proposed exact label and projection is materialized in 14
inventory documents across the 13 families. The deliberately broad
uppercase-token census finds 689
unique tokens. Exactly four are documentation notation rather than proposed
diagnostic identities:

| Token | Why it is not a candidate |
| --- | --- |
| `COMPONENT_TOPOLOGY` | Namespace shorthand used to explain configuration-label construction |
| `CONFIG_COMPONENT_TOPOLOGY_FLEET_ADMISSION_OVERFLOW` | Naming example for a configuration variant proved unreachable on the current path |
| `AUTH_VALIDATION_FAILED` | Explicitly forbidden generic authentication collapse |
| `ICP_ENVIRONMENT` | Existing build-time environment variable named in an operator action |

Removing those four leaves 685 unique proposed identities, exactly matching
the 655 exact meanings plus 30 distinct additional safe projections. Therefore:

- no two different proposed meanings currently collide on one symbolic label;
- no additional projection silently duplicates an exact identity;
- all documented cross-family reuse has already been deducted from the family
  arithmetic; and
- the current symbolic total fits the proposed nonzero `u16` space without a
  semantic numeric band or reserved gap.

This is a symbolic consistency result only. It does not prove that two
differently named meanings should remain distinct after the final action/retry
review, does not cover all direct constructors, and does not authorize
`1..=685`.

## Direct-Constructor Additions

The first seventy-three site-level passes in
[component-registry-constructor-leaves.md](component-registry-constructor-leaves.md)
and
[component-registry-workflow-constructor-leaves.md](component-registry-workflow-constructor-leaves.md)
and
[fleet-subnet-root-workflow-constructor-leaves.md](fleet-subnet-root-workflow-constructor-leaves.md)
and
[component-provisioning-constructor-leaves.md](component-provisioning-constructor-leaves.md)
and
[component-provisioning-workflow-constructor-leaves.md](component-provisioning-workflow-constructor-leaves.md)
and
[fleet-coordinator-constructor-leaves.md](fleet-coordinator-constructor-leaves.md)
and
[fleet-coordinator-receipt-invariant-frontier.md](fleet-coordinator-receipt-invariant-frontier.md)
and
[fleet-coordinator-root-deletion-constructor-leaves.md](fleet-coordinator-root-deletion-constructor-leaves.md)
and
[fleet-coordinator-deployment-ledger-constructor-leaves.md](fleet-coordinator-deployment-ledger-constructor-leaves.md)
and
[fleet-coordinator-workflow-constructor-leaves.md](fleet-coordinator-workflow-constructor-leaves.md)
and
[canister-pool-constructor-leaves.md](canister-pool-constructor-leaves.md)
and
[canister-pool-workflow-constructor-leaves.md](canister-pool-workflow-constructor-leaves.md)
and
[root-store-bootstrap-constructor-leaves.md](root-store-bootstrap-constructor-leaves.md)
and
[root-bootstrap-store-state-constructor-leaves.md](root-bootstrap-store-state-constructor-leaves.md)
classify 2,053 effective top-level and direct-child allocation/install/activation plus
top-level draining/quiescence/recycling and subtree-removal orchestration
and physical-effect references plus root draining, final-inventory and logical
removal persistence, Store reclamation/publication-binding finalization and
Store deletion/root-deletion preparation and final/initial root-inventory
persistence, root-activation/initial-inventory convergence, root
draining/final-inventory/logical-removal, Store reclamation/binding finalization
Store deletion/root-deletion readiness, sibling Store adoption, Component
Directory paging/protected-member, Directory convergence/runtime-status,
peer/protected-allocation, Registry preparation/allocation/create-install
workflow, direct Component Directory/protected-status persistence and top-level
Component draining/removal transition and protected-validation persistence.
They also cover subtree fence/advancement, leaf stop/deletion and membership,
Directory and leaf-finalization persistence plus protected subtree authority
and history validation plus top-level commitment, Directory/runtime and
membership activation plus Fleet-service refresh and remaining generic
Registry/accounting/hash persistence.
They also cover the complete Component Group provisioning ops file: acceptance,
four-cursor member progress, Directory publication, runtime activation,
allocation/draining exclusion, protected request and persisted-phase
validation, result/Directory integrity, member authority, cursor advancement,
canonical hashing and commit-error mapping. They also cover its complete
Coordinator-authenticated orchestration: Registry/Store/Directory observation,
claim/install/commit recovery, runtime activation, capacity and artifact gates.
They also cover all 154 direct Coordinator parent-file constructors plus the
all 235 hidden receipt-invariant calls: genesis/join/snapshot,
provisioning, permanent scale-out history, service publication, Directory and
runtime barriers, retained intents, atomic publication and root-lifecycle
authority. They also close the dedicated Coordinator root-deletion owner across
all public transitions, exact-retry lookups and protected durable-history
validation and the deployment ledger across reservation, terminal activation,
exact reconstruction and retired Scale Out replay. The Coordinator workflow is
closed across endpoint admission, Scale Out publication/cursor fences and
transparent root-error propagation. The first Canister pool range is closed
across Store/import initialization, reset transitions, recycling and claims
plus autonomous creation intent, paid-attempt settlement, adoption, commit and
explicit retry/cancel/rollover plus exclusive handoff, Store deletion,
configuration, capacity and recycling helpers.
The pool workflow is closed across maintenance, import, exclusive handoff and
recoverable Cycles Ledger refill.
Root Store bootstrap is closed across manifest/projection authority, staged
artifacts, protected capacity and exact live-catalog verification. Root Subnet
discovery and sibling Store adoption state are also closed.
Together they add:

- 1,678 exact Component Registry/provisioning/allocation/lifecycle meanings; and
- one safe projection, `COMPONENT_REGISTRY_STATE_INVALID`.

The current qualified set is therefore **2,333 exact candidates plus 31
additional safe projections: 2,364 collision-free identities**. This remains a
qualified subset, not the final allocation. The effective frontier is now
2,499 sites after replacing the Coordinator's one generic receipt adapter with
292 call-site meanings; 446 effective dispositions remain open.

The Component Registry subset is mechanically closed: a whole-file range-owner
manifest assigns all 800 ops constructors exactly once with zero uncovered or
overlapping sites, while the current workflow source count and disposition-row
sum independently agree at 354.
The Component provisioning subset is likewise closed by consecutive range
owners: 177 ops constructors and 56 workflow constructors, with each source
count independently equal to its disposition-table sum.
The Coordinator parent file is mechanically and semantically closed: all 154
direct constructors and all 235 parent-file receipt-invariant calls have
dispositions. Its 10 `root_deletion` and 47 `deployment_ledger` calls are also
closed, so all 292 funnel calls now have exact dispositions. The 12-site
Coordinator workflow is also closed.

The sixty-eight latest slices' candidate-column extraction finds 1,900 exact-label
occurrences. Three hundred twenty-seven intentionally reuse existing partition, physical-absence,
Registry-readiness or root-retirement identities, two adopt DPC names already
reserved for the same exact denials and the other 1,571 do not
occur in any preceding
qualified ledger. That collision check independently matches the
`795 + 32 + 5 + 22 + 50 + 28 + 40 + 24 + 9 + 22 + 0 + 7 + 7 + 15 + 30 + 12 + 28 + 3 + 19 + 50 + 11 + 7 + 11 + 40 + 22 + 8 + 19 + 57 + 53 + 56 + 62 + 28 + 10 + 34 + 10 + 23 + 21 + 15 + 1 + 14 + 51 + 30 + 19 + 15 + 16 + 57 + 29 + 39 + 16 + 19 + 14 + 11 + 37 + 24 + 13 + 66 + 33 + 51 + 23 + 10 + 21 + 10 + 15 + 11 + 4 + 22 + 8 = 2,364`
arithmetic without
treating uppercase prose or static Rust enum names as candidate labels. The
final allocation still requires the mechanical producer manifest rather than
relying on documentation token scans.

## Intentional Reuse Ledger

The cross-family census identifies these current shared identities:

- `ACCESS_DEPENDENCY_UNAVAILABLE`;
- `ACCESS_ROOT_REQUIRED`;
- `AUTH_ATTESTATION_SUBJECT_MISMATCH`;
- `AUTH_CERT_EXPIRED`;
- `AUTH_DELEGATED_TOKENS_DISABLED`;
- `AUTH_ISSUER_PRINCIPAL_MISMATCH`;
- `AUTH_PROOF_INVALID`;
- `AUTH_ROOT_AUTHORITY_INVALID`;
- `AUTH_ROOT_ISSUER_CERT_TTL_ZERO`;
- `AUTH_SUBJECT_CALLER_MISMATCH`;
- `AUTH_TOKEN_EXPIRED`;
- `AUTH_TOKEN_NOT_YET_VALID`;
- `AUTH_TOKEN_TTL_EXCEEDED`;
- `ENV_REQUIRED_FIELDS_MISSING`;
- `RUNTIME_CONFIGURATION_INVALID`;
- `RUNTIME_CONFIGURATION_UNAVAILABLE`; and
- `RUNTIME_ENVIRONMENT_INVALID`.

Each reuse is recorded by the owning family document. No reuse is justified
only by similar wording: the owner, failure meaning, safe exposure, caller or
operator action, retry behavior and machine decision must all agree.

## Naming Review Before Allocation

The reconciliation replaces five unambiguous but awkward mechanically formed
names before numeric identity is frozen:

| Typed meaning | Reconciled provisional label |
| --- | --- |
| Component Group count bound | `CONFIG_COMPONENT_GROUP_COUNT_BOUND_EXCEEDED` |
| Component Group deployment count bound | `CONFIG_COMPONENT_GROUP_DEPLOYMENT_COUNT_BOUND_EXCEEDED` |
| Flattened deployment-member count bound | `CONFIG_COMPONENT_GROUP_DEPLOYMENT_MEMBER_COUNT_BOUND_EXCEEDED` |
| Deployment member-limit count bound | `CONFIG_COMPONENT_MEMBER_LIMIT_COUNT_BOUND_EXCEEDED` |
| Chain-key version is older than accepted | `AUTH_CHAIN_KEY_VERSION_STALE` |

This removes duplicated `GROUP_GROUP`, `DEPLOYMENT_DEPLOYMENT`,
`MEMBER_LIMIT_MEMBER_LIMIT` and `CHAIN_KEY_KEY` wording without merging any
meaning. The typed variant remains the producer anchor; the stable host label
is not required to repeat its Rust name mechanically.

## Remaining B1 Gate

The final table must contain every identity remaining after the 685-name
qualified subset is reconciled with all 2,208 baseline direct-constructor
references. For every exact leaf it must record:

1. its concrete producer owners and exhaustive typed variants;
2. one broad host class and one narrow origin;
3. its exact public projection;
4. its numeric observability owner when the public identity is masked;
5. one action and one retry disposition; and
6. its proposed dense nonzero number after final same-semantics review.

Projection-only rows require their safe exposure rationale and every exact leaf
that maps to them. B2 remains blocked until that complete table and the policy
choices in [allocation-proposal.md](allocation-proposal.md) are approved
together.
