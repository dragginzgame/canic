# Canic 0.102 Final Small-Adapter Constructor Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger closes the final 21 production
`InternalError::*` references in the direct-constructor frontier. It assigns no
number and changes no runtime behavior.

Three one-reference helpers conceal more than one effective path:

- root-state cascade validation has five authority/reason combinations;
- Fleet-activation canonicalization has nine call-site meanings; and
- Sharding bootstrap exhaustion has four call sites selecting two existing
  policy identities.

The 21 mechanical references therefore expand to 36 semantic dispositions.
Counting only the wrapper references would lose authority, repair and
impossible-state distinctions.

## Control-Plane Configuration And Root Authority

| Exact candidate or disposition | Effective sites | Current meaning | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `WASM_STORE_RUNTIME_ROLE_INVALID` | 1 | The Store-only configuration facade is executing under another protected runtime role | `RUNTIME_CONFIGURATION_INVALID` | Reinstall the Wasm Store with its exact protected role; never reinterpret another role as a Store |
| reuse `ROOT_PROTECTED_AUTHORITY_MISMATCH` | 1 | Protected Fleet Subnet Root binding names another Canister | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the binding, fail closed and reinstall the exact root authority |

The current role string is not diagnostic authority. The protected environment
retains it; B4 selects the exact Store-role identity and emits no role prose.

## Root-State Cascade Inventory

The formatted `authority + reason` helper hides five independently repairable
protected-inventory failures:

| Exact candidate | Sites | Current meaning | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `ROOT_STATE_CASCADE_STORE_PRINCIPAL_ANONYMOUS` | 1 | Store inventory contains the anonymous principal | self | Repair/reinstall root-owned Store inventory before another cascade |
| `ROOT_STATE_CASCADE_COMPONENT_PRINCIPAL_ANONYMOUS` | 1 | Component Registry root membership contains the anonymous principal | self | Repair/reinstall Component Registry authority before another cascade |
| `ROOT_STATE_CASCADE_STORE_ROOT_PRINCIPAL_CONFLICT` | 1 | Store inventory names the Fleet Subnet Root as its own child | self | Repair the root/Store authority; never cascade to self as a child |
| `ROOT_STATE_CASCADE_COMPONENT_ROOT_PRINCIPAL_CONFLICT` | 1 | Component Registry membership names the Fleet Subnet Root as a Component | self | Repair Component membership; never cascade to self as a Component |
| `ROOT_STATE_CASCADE_INVENTORY_OVERLAP` | 1 | One principal appears in more than one root-owned inventory | self | Preserve both authorities and reconcile the duplicate before retry |

These exact identities reveal no principal. Source authority changes the repair
owner, so Store and Component anonymous/root-conflict paths must not collapse.
The overlap identity remains singular because both records are required to
repair it and the insertion order is not authority.

## Transparent Core Adapters

Five direct constructors are no-code typed edges:

| Adapter | Sites | Disposition |
| --- | ---: | --- |
| protected Component deployment lookup | 1 | Preserve the exact Fleet-activation storage diagnostic |
| Cycles Ledger operation facade | 1 | Preserve the exact qualified `IcInfraError` diagnostic and its approved projection |
| runtime-log storage facade | 1 | Preserve the exact one of four qualified runtime-log identities |
| Placement Index configuration lookup | 1 | Preserve `PLACEMENT_INDEX_DISABLED` or `PLACEMENT_INDEX_POOL_UNKNOWN` |
| Sharding pool configuration lookup | 1 | Preserve `SHARDING_POOL_NOT_FOUND`; the disabled-config branch is already typed before this adapter |

None receives a wrapper code or forwards source formatter text.

## Fleet-Activation Canonical Evidence

One `canonical_error` constructor currently merges eight live canonicalization
meanings and one impossible length branch:

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `FLEET_ACTIVATION_CASCADE_MANIFEST_CAPACITY_EXCEEDED` | 1 | Cascade manifest leaves no bounded slot for the root | `FLEET_ACTIVATION_STATE_INVALID` | Correct bounded inventory construction before hashing |
| `FLEET_ACTIVATION_CREDENTIAL_MANIFEST_GENERATION_INVALID` | 1 | Credential manifest generation is zero | `FLEET_ACTIVATION_STATE_INVALID` | Rebuild from a positive protected generation |
| `FLEET_ACTIVATION_CREDENTIAL_MANIFEST_ENTRY_LIMIT_EXCEEDED` | 1 | Credential manifest exceeds its frozen entry bound | `FLEET_ACTIVATION_STATE_INVALID` | Reduce/fix bounded manifest construction before hashing |
| `FLEET_ACTIVATION_CREDENTIAL_GENERATION_INVALID` | 1 | Activation evidence names generation zero | `FLEET_ACTIVATION_STATE_INVALID` | Rebuild evidence from the exact positive generation |
| `FLEET_ACTIVATION_CASCADE_MANIFEST_ORDER_INVALID` | 1 | Cascade principals are not in strict canonical order | `FLEET_ACTIVATION_STATE_INVALID` | Reconstruct the manifest canonically; do not hash reordered contradictory input |
| `FLEET_ACTIVATION_CREDENTIAL_MANIFEST_SUBJECT_ORDER_INVALID` | 1 | Credential subjects are not in strict canonical order | `FLEET_ACTIVATION_STATE_INVALID` | Reconstruct the credential manifest canonically |
| `FLEET_ACTIVATION_TOPOLOGY_PARENT_ORDER_INVALID` | 1 | Topology children-map parents are not strictly ordered | `FLEET_ACTIVATION_STATE_INVALID` | Reconstruct the protected topology map canonically |
| `FLEET_ACTIVATION_TOPOLOGY_CHILD_ORDER_INVALID` | 1 | One topology child list is not strictly ordered | `FLEET_ACTIVATION_STATE_INVALID` | Reconstruct the exact child list canonically |
| remove unreachable encoded-length overflow | 1 | A resident `Vec<u8>` length cannot exceed `u64` on supported Wasm32 or 64-bit host targets | none | Remove the impossible branch; do not allocate permanent sediment |

B4 replaces the string label passed to `require_strict_bytes_order` with a
closed typed context selecting one of the four ordering identities. It also
makes the infallible resident-length conversion explicit rather than retaining
an unreachable diagnostic.

## IC Funding, Reservation Hashing And Module Resolution

| Exact candidate or reuse | Sites | Current meaning | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `CANISTER_CREATION_FUNDING_OVERFLOW` | 1 | Configured initial balance plus live Subnet creation cost exceeds `u128` | self | Correct the funding plan; never wrap or attach a truncated amount |
| `ROOT_DRAINING_RESERVATION_ENCODE_FAILED` | 1 | Canonical Coordinator-owned draining reservation cannot be Candid encoded | self | Preserve the reservation and stop before accepting its hash |
| reuse `ENV_MANAGED_BINDING_UNAVAILABLE` | 1 | Runtime has no protected managed binding | `ACCESS_DEPENDENCY_UNAVAILABLE` | Use a Registry-managed Component runtime after valid initialization |
| `WASM_STORE_MODULE_SOURCE_RESOLVER_UNREGISTERED` | 1 | A maintained root/control-plane install facade has no registered Store-backed resolver | `RUNTIME_CONFIGURATION_UNAVAILABLE` | Complete resolver registration during root startup before install work |

The reservation codec cause remains a finite typed implementation detail. The
module-source facade remains maintained even though the current repository has
no ordinary endpoint caller, so missing registration is not dead code.

## Placement Index And Sharding Workflows

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `PLACEMENT_INDEX_STALE_REPAIR_CLAIM_LOST` | 1 | Stale repair loses its exact claim without crossing an await boundary | self | Preserve claim state and retry only after inspecting the conflicting local mutation |
| `PLACEMENT_INDEX_FINAL_BIND_CLAIM_LOST` | 1 | A provisional child is attached but its exact claim disappears before final bind | self | Reconcile/recycle the provisional child; do not substitute another claim |
| remove non-bound repair result branch | 1 | `repair_stale_entry` constructs only `Bound`, yet returns the broad status enum | none | Return a bound-only type and make the impossible match arm unrepresentable |
| reuse `SHARDING_POOL_AT_CAPACITY` | 2 | Bootstrap/assignment observes the configured maximum shard count | self | Free capacity or increase the admitted maximum before retry |
| reuse `SHARDING_NO_FREE_SLOTS` | 2 | Selection/bootstrap has no free configured slot | self | Increase/rebalance configured slots; do not retry unchanged |

The Sharding helper must accept a typed exhaustion reason. Pool, partition key
and the internal bootstrap sentinel are not part of either compact identity.

## Root RPC And Topology Guards

| Exact candidate | Sites | Current meaning | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `RPC_COMPONENT_MEMBER_REGISTRY_COMPONENT_MISMATCH` | 1 | Active member binding and supplied Registry head name different Component trees | `COMPONENT_CHILD_AUTHORITY_INVALID` | Re-resolve both values from the same protected membership lookup |
| `RPC_REQUEST_CYCLES_REPLAY_RESPONSE_KIND_MISMATCH` | 1 | A cycles operation replays a terminal response from another capability kind | `RPC_RESPONSE_INVALID` | Preserve the receipt and fail closed; never reinterpret its response |
| `RPC_CHILD_CREATION_OPERATION_ID_INVALID` | 1 | Generic child creation receives the all-zero operation identity | self | Generate one nonzero operation ID before request construction |
| `TOPOLOGY_MUTATION_IN_PROGRESS` | 1 | Another topology mutation holds the workflow-local guard | self | Retry after the current bounded mutation completes |

The active-member mismatch remains distinct from missing Registry authority:
both values exist, but their protected Component identity disagrees.

## Dynamic Public Context

Rows `DPC-347` through `DPC-358` in
[dynamic-public-context.md](dynamic-public-context.md) classify the protected
Store role, root-cascade labels, canonical-order label, reservation codec
cause, Sharding pool/partition context and four request-cycles secondary
failures. Exact codes replace every closed static discriminator; typed values
remain in their existing authority or operation-correlated status.

## Reconciliation

The 21 direct references expand to 36 effective dispositions:

- 23 new exact meanings;
- six call-site reuses of four existing exact identities;
- five transparent typed adapters; and
- two impossible-state branches removed rather than numbered.

The mechanical frontier is therefore fully classified at 2,208 of 2,208. The
effective semantic frontier grows by fifteen, from 2,499 to 2,514, and is fully
classified at 2,514 of 2,514. The later Component Registry capacity review
corrects five post-precharge meanings, so that pass reaches 2,703 exact
candidates plus 31 safe projections. The later complete current blob-family
pass adds 23 exact identities, bringing that checkpoint to 2,757
collision-free identities. The subsequent transitive RPC workflow-error pass
adds nineteen exact identities outside this direct-constructor frontier.

This pass closes only the direct-constructor frontier. Authentication dynamic
formatter work subsequently closes in slices 40-46 of
[dynamic-public-context.md](dynamic-public-context.md); transitive Component
Registry messages and the Canister stable failure-string census subsequently
close as well. Remaining dynamic context, allocation, host catalogue and
public-projection approval remain separate gates.

## Required Tests

- exhaustive five-way root-state cascade mapping with no principal text;
- exact eight-way Fleet canonicalization mapping and removal of the impossible
  resident-length branch;
- exact overflow, reservation-codec and missing-resolver mapping;
- Placement Index stale-repair and final-bind claim-loss tests plus a
  bound-only repair result type;
- Sharding capacity versus no-free-slot mapping without pool/key prose;
- exact active-member/Registry mismatch and cycles replay-kind rejection;
- zero child-operation and concurrent-topology-mutation rejection; and
- transparent-adapter tables proving no wrapper code or formatted source cause
  survives.
