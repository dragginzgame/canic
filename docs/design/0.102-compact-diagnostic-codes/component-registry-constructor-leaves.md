# Canic 0.102 Component Registry Direct-Constructor Leaves

Date: 2026-08-13

## Status

This B1 ledger begins the site-level reconciliation required by
[direct-constructor-frontier.md](direct-constructor-frontier.md). It allocates
no number and changes no runtime behavior.

The two Component Registry modules contain 1,154 production
`InternalError::*` references:

| Owner | References | Functions containing references |
| --- | ---: | ---: |
| `ops/component_registry/mod.rs` | 800 | 207 |
| `workflow/component_registry/mod.rs` | 354 | 167 |

This pass closes **all 800 ops references** in seventeen bounded slices: 55 for top-level
Component reservation, creation intent/result, install intent/result and
atomic storage commit mapping, then 83 for direct-child reservation,
creation/install intent/result and their capacity helpers, then 73 for child
commitment, Directory/runtime receipts and active membership, then 73 for the
root draining fence, final inventory and logical-removal publication, then 45
for Store reclamation and publication-binding finalization, then 61 for Store
deletion/cycle reclamation and root-deletion preparation, then 35 for final
root-inventory commit and initial-inventory sealing/activation persistence,
then 15 for Component Directory paging, committed-authority reads,
registered-parent lookup and subtree-removal status, then 50 for top-level
Component draining, quiescence, final inventory, deletion and membership
removal, then 58 for the matching byte-state, protected-record, terminal-
history and hash validators, then 33 for subtree fence creation, advancement
and their byte-state derivation, then 31 for leaf stop and deletion intent/
result persistence, then 31 for membership/Directory/leaf finalization, then
33 for protected subtree authority/history validation and then 61 for top-
level commitment, Directory/runtime activation and immutable partition
reconstruction, then 21 for Fleet-service Directory refresh persistence and
then the final 42 Registry, accounting and hash adapter sites. The companion
[workflow ledger](component-registry-workflow-constructor-leaves.md) closes all
354 workflow sites. No direct constructor remains open across the two Component
Registry modules. A function count is an audit navigation aid, not a code count.

A fresh range-owner manifest reports `total=800`, `covered=800`,
`uncovered=0` and `overlap=0` for ops. An independent current-source count and
table-row sum both report 354 workflow sites. These mechanical checks establish
the Component Registry frontier closure; the candidate allocation remains
provisional until the whole-program B1 inventory is complete.

## Top-Level Allocation Persistence Slice

The exact baseline sites are lines 3029–3371 and 13247–13434 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. The table
accounts for every `InternalError::*` reference in the selected functions and
helpers, including all nine variants of
`RootComponentAllocationCommitError`.

`recent failure` below means the guarded runtime observation after its code is
hard-cut from `String` to `DiagnosticCode`. These storage contradictions must
not write diagnostic evidence into the Registry record they have just proved
invalid.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 10 | Root Component Registry meta authority is absent | self | Complete root Registry preparation; retry only after readiness changes | public |
| `COMPONENT_ALLOCATION_OPERATION_UNRESERVED` | 8 | Exact allocation operation record is absent | self | Reserve/query the exact operation first; absence is not commitment | public |
| `COMPONENT_ALLOCATION_OPERATION_CONFLICT` | 2 | Operation ID is already bound to different immutable intent | self | Replay only the exact original request | public |
| `COMPONENT_ALLOCATION_INITIAL_INVENTORY_SEALED` | 1 | Dynamic allocation was attempted while the sealed root is still `Prepared` | self | Complete root activation before dynamic allocation | public |
| `COMPONENT_REGISTRY_INITIAL_INVENTORY_RECEIPT_MISSING` | 1 | Runtime claims `Active` without terminal initial-inventory receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect/reinstall protected root state; no unchanged retry | recent failure |
| `COMPONENT_REGISTRY_INITIAL_INVENTORY_UNSEALED` | 1 | Runtime claims `Active` without a sealed initial inventory | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect/reinstall protected root state; no unchanged retry | recent failure |
| `COMPONENT_ALLOCATION_SEQUENCE_STALE` | 1 | Durable next sequence changed before reservation commit | self | Reload Registry authority and exact retry | public |
| `COMPONENT_REGISTRY_BYTE_COUNT_OVERFLOW` | 3 | Checked Registry byte arithmetic overflowed | `COMPONENT_REGISTRY_STATE_INVALID` | Stop mutation and inspect byte accounting; no blind retry | recent failure |
| `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | 3 | Exact reservation/evidence would exceed the protected root Registry byte ceiling | self | Free Registry capacity or select/reinstall with an admitted larger limit | public |
| `COMPONENT_ALLOCATION_SEQUENCE_EXHAUSTED` | 1 | Durable Component identity sequence cannot advance | self; existing policy identity | Retire/replace the root; never reuse an identity | public |
| `COMPONENT_ALLOCATION_COUNT_OVERFLOW` | 1 | Reserved top-level Component count overflowed | `COMPONENT_ALLOCATION_CAPACITY_EXHAUSTED`; existing exact identity | Stop allocation and inspect accounting; do not blindly retry | recent failure |
| `COMPONENT_ALLOCATION_CREATION_TRANSITION_INVALID` | 3 | Creation intent is already crossed or missing for the requested edge | self | Inspect allocation status and request only its admitted next edge | public |
| `COMPONENT_ALLOCATION_CANISTER_CONFLICT` | 2 | Allocation/result principal conflicts with its retained or indexed Canister | self | Re-observe/replay the exact created Canister; never substitute another principal | public |
| `COMPONENT_REGISTRY_KNOWN_CREATED_COUNT_OVERFLOW` | 1 | Known-created physical-Canister counter overflowed | `COMPONENT_REGISTRY_STATE_INVALID` | Stop mutation and inspect physical inventory accounting | recent failure |
| `COMPONENT_REGISTRY_ALLOCATED_COUNT_OVERFLOW` | 1 | Reserved, committed and descendant allocation totals cannot be added | `COMPONENT_REGISTRY_STATE_INVALID` | Stop mutation and inspect allocation accounting | recent failure |
| `COMPONENT_REGISTRY_KNOWN_CREATED_EXCEEDS_ALLOCATED` | 1 | Known-created Canisters exceed all allocated Component-tree capacity | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; reconcile exact physical and logical inventory | recent failure |
| `COMPONENT_ALLOCATION_INSTALL_TRANSITION_INVALID` | 5 | Install intent/result/verification edge is not admitted from the current allocation phase | self | Inspect allocation status and request only its admitted next edge | public |
| `COMPONENT_REGISTRY_BYTE_ACCOUNTING_UNDERFLOW` | 2 | Current Registry bytes are below the retained creation/install charge | `COMPONENT_REGISTRY_STATE_INVALID` | Stop mutation and inspect byte ledger; no blind repair | recent failure |
| `COMPONENT_INSTALL_INTENT_MISMATCH` | 1 | Durable module/binding intent differs from verified install plan | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve first intent and reject contradictory evidence | recent failure |
| `COMPONENT_CREATION_RECORD_CAPACITY_EXCEEDED` | 1 | Creation effect evidence exceeds its bounded stable record | self | Reduce bounded evidence before persisting intent; no effect has run | public |
| `COMPONENT_ALLOCATION_BYTE_CHARGE_EXCEEDED` | 1 | Encoded allocation record exceeds its pre-effect byte reservation | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; pre-effect capacity proof was insufficient | recent failure |
| `COMPONENT_ALLOCATION_IDENTITY_CONFLICT` | 1 | Derived Component identity is reserved by another operation | self | Reconcile the durable sequence/operation; never derive a substitute identity | public |
| `COMPONENT_CHILD_RESERVATION_CONFLICT` | 1 | Child reservation differs from its partition or count index | `COMPONENT_REGISTRY_STATE_INVALID` | Stop mutation and reconcile the exact reservation/index set | recent failure |
| `COMPONENT_REGISTRY_PARTITION_CONFLICT` | 1 | Partition is already committed under different authority | self | Reload exact Registry authority; never overwrite the partition | public |
| `COMPONENT_REGISTRY_AUTHORITY_STALE` | 1 | Meta authority changed before the atomic storage mutation | self | Reload current head and exact retry through the owning operation | public |
| `COMPONENT_CHILD_PARENT_BINDING_INVALID` | 1 | Commit adapter found a parent absent from its Component principal index | `COMPONENT_CHILD_AUTHORITY_INVALID`; existing exact identity | Re-resolve exact registered parent authority; no unchanged retry | recent failure |

The 55 sites therefore contribute **23 new exact candidates**, reuse three
existing exact candidates and introduce one safe projection:
`COMPONENT_REGISTRY_STATE_INVALID`.

## Direct-Child Reservation and Install Persistence Slice

This slice accounts for all 83 direct constructor references in these selected
functions and their direct capacity/authority helpers:

- `parent_role_instances`, `reserve_child_allocation`,
  `validate_child_creation_capacity`, `begin_child_creation`,
  `mark_child_created`, `validate_child_install_capacity`,
  `begin_child_install`, `renew_child_install_intent`,
  `mark_child_installed`/`mark_child_verified` through
  `advance_child_install_phase`;
- `child_reservation_partition`, `validate_child_creation_authority`,
  `child_creation_capacity`, `validate_charged_child_record_size`,
  `validate_child_install_authority`, `child_install_charged_entry_bytes`,
  `child_install_capacity` and `validate_child_install_effect_record`; and
- `validate_child_allocation_record`.

The exact baseline ranges are lines 5588–6131, 9051–9110, 9656–10079 and
13004–13022 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. The two
one-line public phase adapters contain no constructor of their own; their
shared helper is counted once. Calls into validators and the already
classified allocation commit adapter do not count their inner constructors a
second time.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 8 | Root Component Registry meta authority is absent | self; existing exact identity | Complete root Registry preparation; retry only after readiness changes | public |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 8 | The exact Component Registry partition is absent | self | Complete/recover the protected partition; retry only after it exists | public |
| `COMPONENT_CHILD_OPERATION_UNRESERVED` | 7 | Exact child-allocation operation record is absent | self | Reserve/query the exact operation first; absence is not commitment | public |
| `COMPONENT_CHILD_OPERATION_CONFLICT` | 1 | Operation ID is already bound to different immutable child intent | self | Replay only the exact original request | public |
| `COMPONENT_CHILD_REGISTRY_AUTHORITY_STALE` | 1 | Partition Spec, release, lifecycle or head changed before reservation commit | self; existing exact identity | Refresh the exact Component Registry head and retry with the same operation | public |
| `COMPONENT_CHILD_CREATION_AUTHORITY_INVALID` | 1 | Creation controller, active partition authority or Store artifact differs from the reservation | `COMPONENT_CHILD_AUTHORITY_INVALID` | Re-resolve protected creation authority; never substitute controller or artifact | recent failure |
| `COMPONENT_CHILD_INSTALL_AUTHORITY_INVALID` | 1 | Created Canister, binding, release set, artifact source or limit differs from the reservation | `COMPONENT_CHILD_AUTHORITY_INVALID` | Re-resolve protected install authority; never substitute binding or artifact | recent failure |
| `COMPONENT_DESCENDANT_COUNT_OVERFLOW` | 3 | Descendant traversal, reserved or aggregate count cannot advance with checked arithmetic | `COMPONENT_DESCENDANT_CAPACITY_EXHAUSTED` | Stop allocation and inspect descendant accounting; do not blindly retry | recent failure |
| `COMPONENT_CHILD_SUBTREE_REMOVAL_FENCED` | 1 | The selected parent lies inside a nonterminal subtree removal | self | Wait for removal to finish or select an unaffected admitted parent | public |
| `COMPONENT_CHILD_PARENT_ROLE_INDEX_INVALID` | 2 | Parent-role count index has the wrong identity or a retained zero count | `COMPONENT_REGISTRY_STATE_INVALID` | Stop mutation and reconcile the exact parent-role index | recent failure |
| `COMPONENT_CHILD_PARENT_ROLE_COUNT_OVERFLOW` | 1 | Per-parent role count cannot advance | `COMPONENT_CHILD_PARENT_ROLE_CAPACITY_EXHAUSTED`; existing exact identity | Inspect parent-role accounting; do not blindly retry | recent failure |
| `COMPONENT_CHILD_PARENT_ROLE_CAPACITY_EXHAUSTED` | 1 | Parent reached its protected direct-child role ceiling | self; existing exact identity | Free that parent's role capacity; exact retry after state change | public |
| `COMPONENT_DESCENDANT_CAPACITY_EXHAUSTED` | 1 | Component tree reached its protected descendant ceiling | self; existing exact identity | Free Component-tree capacity; exact retry after state change | public |
| `COMPONENT_REGISTRY_MANAGED_DESCENDANT_COUNT_OVERFLOW` | 1 | Root-wide managed-descendant count cannot advance | `COMPONENT_REGISTRY_STATE_INVALID` | Stop mutation and reconcile root/partition count accounting | recent failure |
| `COMPONENT_REGISTRY_BYTE_COUNT_OVERFLOW` | 15 | Checked partition, record, index or root Registry byte arithmetic overflowed | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and inspect byte accounting; no blind retry | recent failure |
| `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | 6 | Reservation, creation or install evidence exceeds its protected Component or root Registry ceiling | self; existing exact identity | Free Registry capacity or reinstall with an admitted larger limit | public |
| `COMPONENT_REGISTRY_BYTE_ACCOUNTING_UNDERFLOW` | 3 | Reservation or precharge would unexpectedly reduce accounted Registry bytes | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and inspect the byte ledger | recent failure |
| `COMPONENT_CHILD_BYTE_ACCOUNTING_NONCONVERGENT` | 3 | Bounded fixed-point calculation cannot stabilize reservation/create/install bytes | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct the encoded-byte model; no unchanged retry | recent failure |
| `COMPONENT_CHILD_CREATION_BYTE_CHARGE_INVALID` | 1 | Creation precharge is smaller than the retained reservation record | `COMPONENT_REGISTRY_STATE_INVALID` | Correct pre-effect capacity derivation before any effect runs | recent failure |
| `COMPONENT_CHILD_BYTE_CHARGE_EXCEEDED` | 1 | Encoded child allocation record exceeds its precharged stable footprint | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; retained evidence exceeded the pre-effect proof | recent failure |
| `COMPONENT_CHILD_CREATION_TRANSITION_INVALID` | 3 | Creation intent/result edge is not admitted from the current child phase | self | Inspect operation status and request only its admitted next edge | public |
| `COMPONENT_CHILD_INSTALL_TRANSITION_INVALID` | 7 | Install intent/result/verification edge is not admitted from the current child phase | self | Inspect operation status and request only its admitted next edge | public |
| `COMPONENT_CHILD_CANISTER_CONFLICT` | 2 | Created principal differs from retained intent or collides with protected/indexed authority | self | Preserve/re-observe the exact created Canister; never substitute a principal | public |
| `COMPONENT_REGISTRY_KNOWN_CREATED_COUNT_OVERFLOW` | 1 | Known-created physical-Canister counter cannot advance | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and inspect physical inventory accounting | recent failure |
| `COMPONENT_REGISTRY_ALLOCATED_COUNT_OVERFLOW` | 1 | Reserved, committed and descendant allocation totals cannot be added | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and inspect allocation accounting | recent failure |
| `COMPONENT_REGISTRY_KNOWN_CREATED_EXCEEDS_ALLOCATED` | 1 | Known-created Canisters exceed all allocated Component-tree capacity | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed; reconcile exact physical and logical inventory | recent failure |
| `COMPONENT_CHILD_INSTALL_INTENT_MISMATCH` | 1 | Durable module, chunk or binding intent differs from the verified install plan | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve first intent and reject contradictory effect evidence | recent failure |
| `COMPONENT_CHILD_ALLOCATION_RECORD_INVALID` | 1 | Persisted operation, Component/head, parent or limit identity is malformed | `COMPONENT_CHILD_AUTHORITY_INVALID` | Fail closed and inspect/reinstall protected child-allocation state | recent failure |

The 83 sites contribute **17 new exact candidates** and reuse 11 existing exact
identities. They add no safe projection: every masked meaning uses an existing
projection or an existing exact capacity identity. Together, both slices have
classified 138 direct sites and qualify 40 new exact candidates, three exact
reuses in the first slice, 11 existing exact identities in the second slice
and one additional safe projection.

## Child Commitment and Activation Persistence Slice

This slice accounts for all 73 direct constructor references in:

- `commit_verified_child`, `mark_child_directory_prepared`,
  `mark_child_runtime_activated`, `activate_child_membership` and
  `mark_child_membership_synchronized`;
- `committed_child_records`, `persist_child_membership_activation` and
  `active_child_membership_records`; and
- `exact_committed_child_partition`, `exact_active_child_partition`,
  `validate_active_child_partition`,
  `validate_child_directory_authority_hash`,
  `validate_child_membership_directory_authority_hash` and
  `validate_child_record`.

The exact baseline ranges are lines 6150–6582, 10197–10637, 10914–11088,
11239–11300 and 12079–12091 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. Calls into
hash constructors, partition validators, the charged-record validator and the
allocation commit adapter remain separately accounted source sites; this table
does not count their constructors transitively.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 5 | Root Component Registry meta authority is absent | self; existing exact identity | Complete root Registry preparation; retry only after readiness changes | public |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 5 | The requested Component Registry partition is absent before commitment | self; existing exact identity | Complete/recover the protected partition; retry only after it exists | public |
| `COMPONENT_CHILD_OPERATION_UNRESERVED` | 5 | Exact child-allocation operation record is absent | self; existing exact identity | Reserve/query the exact operation first; absence is not commitment | public |
| `COMPONENT_REGISTRY_PARTITION_MISSING` | 2 | A committed or active child receipt names a Component whose partition disappeared | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect/reinstall protected Registry state | recent failure |
| `COMPONENT_CHILD_DIRECTORY_TIME_NOT_MONOTONIC` | 2 | Proposed Directory synchronization time does not advance current Component authority | self | Supply a later observed synchronization time against the current head | public |
| `COMPONENT_CHILD_COMMITMENT_TRANSITION_INVALID` | 1 | Child has not reached `Verified` for initial Registry commitment | self | Resume the exact operation through verification before commitment | public |
| `COMPONENT_CHILD_DIRECTORY_PREPARATION_TRANSITION_INVALID` | 1 | Child has no committed Registry receipt to prepare | self | Commit the verified child before Directory preparation | public |
| `COMPONENT_CHILD_RUNTIME_ACTIVATION_TRANSITION_INVALID` | 1 | Child has no committed Registry receipt to activate | self | Commit and prepare the exact Directory before runtime activation | public |
| `COMPONENT_CHILD_MEMBERSHIP_ACTIVATION_TRANSITION_INVALID` | 2 | Child is uncommitted or lacks terminal Directory/runtime receipts | self | Resume the exact commitment/preparation/activation sequence | public |
| `COMPONENT_CHILD_MEMBERSHIP_ROW_TRANSITION_INVALID` | 1 | Membership activation found a normalized child row outside `Prepared` without a membership receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile row/receipt state | recent failure |
| `COMPONENT_CHILD_MEMBERSHIP_SYNCHRONIZATION_TRANSITION_INVALID` | 2 | Child is uncommitted or has no active membership receipt to synchronize | self | Activate exact membership before acknowledging synchronization | public |
| `COMPONENT_CHILD_DIRECTORY_AUTHORITY_STALE` | 1 | Directory-preparation acknowledgement names a different committed authority hash | self | Reload the committed receipt and retry with its exact hash | public |
| `COMPONENT_CHILD_RUNTIME_DIRECTORY_AUTHORITY_UNREADY` | 1 | Runtime activation lacks the exact prepared Directory hash or preparation receipt | self | Finish Directory preparation and retry its exact authority | public |
| `COMPONENT_CHILD_MEMBERSHIP_DIRECTORY_AUTHORITY_STALE` | 1 | Membership synchronization names a different active Directory authority hash | self | Reload active membership and retry with its exact hash | public |
| `COMPONENT_CHILD_COMMITMENT_DIRECTORY_HASH_INVALID` | 1 | Reconstructed committed Directory authority differs from its immutable receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed; inspect Registry/Directory evidence | recent failure |
| `COMPONENT_CHILD_MEMBERSHIP_DIRECTORY_HASH_INVALID` | 1 | Reconstructed active Directory authority differs from its membership receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed; inspect membership evidence | recent failure |
| `COMPONENT_CHILD_RECEIPT_BYTE_FOOTPRINT_CHANGED` | 3 | Boolean Directory/runtime/membership acknowledgement changed its precharged stable footprint | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct the bounded record model | recent failure |
| `COMPONENT_CHILD_ROW_MISSING` | 3 | Committed or active child receipt has no normalized child row | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile the normalized Registry indexes | recent failure |
| `COMPONENT_CHILD_ROW_IDENTITY_INVALID` | 1 | Normalized child row crosses its Component tree boundary | `COMPONENT_CHILD_AUTHORITY_INVALID` | Fail closed and inspect protected tree identity | recent failure |
| `COMPONENT_CHILD_PRINCIPAL_ALREADY_COMMITTED` | 1 | Proposed child principal already has committed membership | self | Replay the owning operation; never commit the principal twice | public |
| `COMPONENT_REGISTRY_REVISION_EXHAUSTED` | 2 | Component Registry revision cannot advance without wrapping | self | Retire/reinstall the root; never wrap or reuse Registry history | public |
| `COMPONENT_DESCENDANT_RESERVATION_UNDERFLOW` | 1 | Commitment has no reserved descendant to consume | `COMPONENT_REGISTRY_STATE_INVALID` | Stop mutation and reconcile reservation/accounting state | recent failure |
| `COMPONENT_DESCENDANT_COUNT_OVERFLOW` | 1 | Committed descendant count cannot advance | `COMPONENT_DESCENDANT_CAPACITY_EXHAUSTED`; existing exact identity | Stop mutation and inspect descendant accounting | recent failure |
| `COMPONENT_REGISTRY_BYTE_COUNT_OVERFLOW` | 7 | Checked terminal-record, row, index or partition byte arithmetic overflowed | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and inspect byte accounting | recent failure |
| `COMPONENT_CHILD_BYTE_CHARGE_EXCEEDED` | 4 | Commitment or membership bytes exceed the frozen pre-install charge | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed; pre-effect capacity proof was insufficient | recent failure |
| `COMPONENT_CHILD_COMMITMENT_COMPONENT_LIMIT_INVALID` | 1 | Terminal commitment exceeds the protected Component Registry limit despite prior reservation | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct capacity derivation | recent failure |
| `COMPONENT_REGISTRY_BYTE_ACCOUNTING_UNDERFLOW` | 4 | Commitment/activation cannot release or replace its exact prior byte charge | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and inspect the byte ledger | recent failure |
| `COMPONENT_CHILD_BYTE_ACCOUNTING_NONCONVERGENT` | 2 | Commitment or active-membership fixed-point bytes do not stabilize | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and correct the encoded-byte model | recent failure |
| `COMPONENT_CHILD_COMMITMENT_RECORD_INVALID` | 6 | Commitment constructor/validator crossed into a noncommitted phase | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect durable operation phase | recent failure |
| `COMPONENT_CHILD_MEMBERSHIP_RECORD_INVALID` | 1 | Active commitment lost its membership receipt during byte convergence | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect durable membership state | recent failure |
| `COMPONENT_CHILD_COMMITMENT_RECEIPT_MISMATCH` | 1 | Normalized row, traversal, partition or release authority differs from immutable commitment | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the receipt; fail closed and reconcile all exact indexes | recent failure |
| `COMPONENT_CHILD_MEMBERSHIP_RECEIPT_MISMATCH` | 1 | Active row or monotonic partition coverage differs from immutable membership | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the receipt; fail closed and reconcile active membership | recent failure |
| `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | 2 | Active membership would exceed protected Component or root Registry capacity | self; existing exact identity | Free Registry capacity or reinstall with an admitted larger limit | public |

The 73 sites contribute **24 new exact candidates** and reuse nine existing
exact identities. They add no safe projection. Across all three slices, 211
direct sites now qualify 64 new exact candidates and one additional safe
projection.

## Root Draining, Final Inventory And Logical Removal Persistence Slice

This slice accounts for all 73 direct constructor references in:

- `begin_root_draining`, `root_draining`, `root_draining_if_present`,
  `validate_published_root_draining` and `require_root_store_admin_open`;
- `prepare_root_final_inventory`, `root_final_inventory_intent_registry`,
  `begin_root_final_inventory`, `root_final_inventory`,
  `root_final_inventory_if_present` and `verify_root_final_inventory_store`;
- `root_removal_publication_if_present` and
  `record_root_removal_publication`; and
- the direct validators, terminal-history reconstruction, byte-ledger,
  Store-evidence and canonical-hash helpers from
  `validate_root_draining_record` through `root_final_inventory_hash`.

The exact baseline ranges are lines 1471–1857 and 7359–7876 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. Calls into the
Fleet Registry reservation hasher, Store status owner and storage commit
adapter remain transparent; their inner constructors are not counted again.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_DRAINING_OPERATION_ID_INVALID` | 1 | Root-draining operation ID is zero | self | Supply one nonzero operation ID before any fence commit | public |
| `ROOT_DRAINING_START_TIME_INVALID` | 1 | Root-draining fence time is zero | self | Supply the positive observed start time before commit | public |
| `ROOT_DRAINING_RESERVATION_HASH_MISSING` | 1 | Coordinator reservation has no nonzero authority hash | self | Re-obtain the exact qualified draining reservation | public |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 10 | Root Component Registry meta authority is absent | self; existing exact identity | Complete root Registry preparation before retry | public |
| `ROOT_DRAINING_REQUEST_CONFLICT` | 1 | A retained draining fence exists under different Registry or reservation authority | self | Replay only the exact original draining request | public |
| `ROOT_DRAINING_PREPARATION_NOT_COVERED` | 1 | Requested draining Registry is not covered by root preparation authority | self | Refresh preparation and use its exact current Registry | public |
| `ROOT_DRAINING_FENCE_COMMIT_CONFLICT` | 1 | Root Registry authority changed before the local admission fence committed | self | Reload current authority and retry the exact operation | public |
| `ROOT_DRAINING_UNSTARTED` | 6 | A later root-retirement edge has no durable draining fence | self | Begin and publish root draining before continuing | public |
| `ROOT_DRAINING_OPERATION_CONFLICT` | 6 | Status, final inventory or publication names a different retained draining operation | self | Query/replay only the exact retained operation | public |
| `ROOT_DRAINING_LOCAL_FENCE_MISSING` | 1 | A draining Fleet Mirror has no corresponding local admission cutoff | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect root Registry/Fleet Mirror authority | recent failure |
| `ROOT_DRAINING_PUBLICATION_NOT_LATER` | 1 | Published draining Registry does not advance the locally fenced Active authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile Coordinator publication order | recent failure |
| `ROOT_STORE_ADMIN_DRAINING_FENCED` | 1 | Store administration was requested after the one-way root draining fence | self | Do not reopen Store administration; finish retirement | public |
| `ROOT_FINAL_INVENTORY_PREPARATION_TIME_INVALID` | 1 | Final-inventory preparation time is zero | self | Supply a positive observed preparation time | public |
| `ROOT_FINAL_INVENTORY_PRECEDES_DRAINING` | 1 | Final-inventory preparation time predates the durable draining fence | self | Re-observe after the fence and retry exact authority | public |
| `ROOT_FINAL_INVENTORY_INTENT_COMMIT_CONFLICT` | 1 | Root Registry authority changed before final-inventory intent commit | self | Reload current authority and exact retry | public |
| `ROOT_FINAL_INVENTORY_INTENT_COMMIT_MISMATCH` | 1 | Reconstructed committed intent differs from its pre-commit terminal plan | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the committed record and inspect storage authority | recent failure |
| `ROOT_FINAL_INVENTORY_UNAVAILABLE` | 3 | A later retirement edge requires final inventory that is not frozen | self | Complete exact final-inventory preparation/finalization first | public |
| `ROOT_FINAL_INVENTORY_STORE_MISMATCH` | 1 | Live Store status differs from retained terminal Store evidence | self | Re-observe the exact Store; do not advance retirement | public |
| `ROOT_REMOVAL_PUBLICATION_CONFLICT` | 1 | A retained logical-removal publication differs from the Coordinator response | self | Replay only the exact original publication response | public |
| `ROOT_REMOVAL_RESPONSE_MISMATCH` | 1 | Coordinator logical-removal response differs from local final inventory | self | Reject the response and reconcile Coordinator/root authority | public |
| `ROOT_REMOVAL_PUBLICATION_COMMIT_CONFLICT` | 1 | Root Registry authority changed before publication receipt commit | self | Reload current authority and retry exact receipt persistence | public |
| `ROOT_REMOVAL_PUBLICATION_RECEIPT_MISSING` | 1 | Storage accepted publication commit but retained no terminal receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect durable root-retirement state | recent failure |
| `ROOT_DRAINING_RECEIPT_INVALID` | 1 | Retained draining receipt differs from protected root authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect/reinstall protected root state | recent failure |
| `ROOT_DRAINING_RESERVATION_HASH_MISMATCH` | 1 | Retained Coordinator reservation content does not match its hash | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and preserve the contradictory receipt | recent failure |
| `ROOT_STORE_RECLAMATION_RECEIPT_HASH_MISMATCH` | 1 | Retained Store-reclamation receipt hash is invalid | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before any later Store boundary | recent failure |
| `ROOT_STORE_BINDING_FINALIZATION_RECEIPT_HASH_MISMATCH` | 1 | Retained Store-binding-finalization receipt hash is invalid | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before Store deletion | recent failure |
| `ROOT_STORE_DELETION_RECEIPT_HASH_MISMATCH` | 1 | Retained Store-deletion receipt hash is invalid | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before root deletion preparation | recent failure |
| `TERMINAL_COMPONENT_ALLOCATION_SEQUENCE_OVERFLOW` | 1 | Removed allocation count cannot advance to the next durable sequence | `COMPONENT_REGISTRY_STATE_INVALID` | Stop retirement and inspect identity history | recent failure |
| `TERMINAL_COMPONENT_ALLOCATION_HISTORY_NONCONTIGUOUS` | 1 | Next allocation sequence differs from complete removed history | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; never repair by reusing an identity | recent failure |
| `TERMINAL_ROOT_UNKNOWN_COMPONENT_HISTORY` | 1 | Registry history retains a Component absent from allocation history | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile retained allocation/Registry history | recent failure |
| `TERMINAL_ROOT_REGISTRY_BYTE_LEDGER_MISMATCH` | 1 | Reconstructed terminal history bytes differ from root meta bytes | `COMPONENT_REGISTRY_STATE_INVALID` | Stop retirement and inspect byte accounting | recent failure |
| `TERMINAL_COMPONENT_ALLOCATION_COUNT_OVERFLOW` | 1 | Complete removed allocation history exceeds the bounded count | `COMPONENT_REGISTRY_STATE_INVALID` | Stop retirement and inspect bounded inventory | recent failure |
| `TERMINAL_COMPONENT_DRAINING_HISTORY_MISMATCH` | 1 | Allocation and draining history cardinalities differ | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile terminal histories before retirement | recent failure |
| `TERMINAL_COMPONENT_DRAINING_HISTORY_MISSING` | 1 | One retained allocation has no exact draining history | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve allocation history and restore exact terminal evidence | recent failure |
| `TERMINAL_ROOT_REGISTRY_BYTE_OVERFLOW` | 2 | Reconstructing terminal Registry byte charges overflows | `COMPONENT_REGISTRY_STATE_INVALID` | Stop retirement and inspect bounded byte accounting | recent failure |
| `TERMINAL_COMPONENT_ALLOCATION_INDEX_OVERFLOW` | 1 | Canonical allocation index cannot convert to its durable sequence | `COMPONENT_REGISTRY_STATE_INVALID` | Stop retirement and inspect impossible inventory scale | recent failure |
| `ROOT_NONTERMINAL_COMPONENT_HISTORY` | 1 | At least one Component allocation has not reached `Removed` | self | Finish that Component's exact draining/removal journey | public |
| `ROOT_FINAL_INVENTORY_OPERATION_ID_INVALID` | 1 | Final-inventory operation ID is zero | self | Supply the nonzero root-draining operation ID | public |
| `ROOT_FINAL_INVENTORY_REGISTRY_NOT_COVERED` | 1 | Final-inventory Registry is not current with or later than the draining fence | self | Converge the root on the current removal Registry | public |
| `ROOT_FINAL_INVENTORY_LIVE_CAPACITY_REMAINS` | 1 | Root counters still report reserved, committed, descendant or physical Canister inventory | self | Drain/recycle all remaining managed capacity first | public |
| `ROOT_FINAL_INVENTORY_LIVE_MEMBERSHIP_REMAINS` | 1 | Partition or principal indexes still contain live membership | self | Finish exact membership removal before finalization | public |
| `TERMINAL_COMPONENT_MEMBERSHIP_REMOVAL_MISSING` | 1 | Terminal Component history lacks its immutable membership-removal receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and restore exact terminal evidence | recent failure |
| `TERMINAL_COMPONENT_HISTORY_ENCODING_FAILED` | 1 | Canonical terminal-history authority cannot be Candid encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not advance | recent failure |
| `ROOT_FINAL_INVENTORY_STORE_CATALOG_COUNT_OVERFLOW` | 1 | Retained Store catalog count cannot fit its bounded terminal field | `COMPONENT_REGISTRY_STATE_INVALID` | Stop retirement and inspect Store inventory bounds | recent failure |
| `ROOT_FINAL_INVENTORY_STORE_TEMPLATE_COUNT_OVERFLOW` | 1 | Live Store template count cannot fit its bounded terminal field | `COMPONENT_REGISTRY_STATE_INVALID` | Stop retirement and inspect Store inventory bounds | recent failure |
| `ROOT_FINAL_INVENTORY_STORE_GC_UNPREPARED` | 1 | Store lacks the exact prepared GC write fence required by final inventory | self | Prepare Store GC and re-observe its exact status | public |
| `ROOT_FINAL_INVENTORY_STORE_AUTHORITY_INVALID` | 1 | Store binding, catalog accounting or GC lineage is not exact terminal authority | self | Repair/converge the Store and retry observation | public |
| `ROOT_FINAL_INVENTORY_STORE_CATALOG_ENCODING_FAILED` | 1 | Canonical terminal Store catalog authority cannot be encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not advance | recent failure |
| `ROOT_FINAL_INVENTORY_RECORD_INVALID` | 1 | Retained final inventory differs from reconstructed terminal authority or its hash | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and preserve contradictory evidence | recent failure |
| `ROOT_FINAL_INVENTORY_INTENT_INVALID` | 1 | Retained final-inventory intent differs from its terminal plan or fence time | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before finalization | recent failure |
| `ROOT_FINAL_INVENTORY_ENCODING_FAILED` | 1 | Canonical final-inventory hash authority cannot be encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not advance | recent failure |

The 51 rows sum to all 73 selected sites. Fifty exact identities are new;
`COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` is reused. No safe projection is
added: every masked state contradiction uses the existing
`COMPONENT_REGISTRY_STATE_INVALID` projection.

## Store Reclamation And Publication-Binding Finalization Persistence Slice

This slice accounts for all 45 direct constructor references in:

- `root_store_reclamation_intent_if_present`,
  `begin_root_store_reclamation`, `root_store_reclamation_if_present` and
  `record_root_store_reclamation`;
- `root_store_binding_finalization_intent_if_present`,
  `begin_root_store_binding_finalization`,
  `root_store_binding_finalization_if_present` and
  `record_root_store_binding_finalization`; and
- the direct record/evidence and canonical-hash helpers from
  `root_store_reclamation_record` through
  `root_store_binding_finalization_hash`.

The exact baseline ranges are lines 1858–2173 and 7877–8035 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. The Store GC
and publication-state effects remain outside this pure persistence slice; this
ledger classifies their evidence validation and durable retry boundaries.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 8 | Root Component Registry meta authority is absent | self; existing exact identity | Complete root Registry preparation before retry | public |
| `ROOT_DRAINING_UNSTARTED` | 4 | Store-retirement persistence has no durable root draining fence | self; existing exact identity | Begin root draining before continuing | public |
| `ROOT_DRAINING_OPERATION_CONFLICT` | 4 | Store-retirement status names a different root-draining operation | self; existing exact identity | Query/replay only the exact retained operation | public |
| `ROOT_STORE_RECLAMATION_INVENTORY_HASH_INVALID` | 1 | Requested final-inventory hash is zero | self | Supply the exact nonzero retained final-inventory hash | public |
| `ROOT_STORE_RECLAMATION_PREPARATION_TIME_INVALID` | 1 | Reclamation preparation time is zero | self | Supply a positive observed preparation time | public |
| `ROOT_STORE_RECLAMATION_INTENT_CONFLICT` | 1 | Exact operation already retains a different final-inventory hash | self | Replay only the original reclamation intent | public |
| `ROOT_FINAL_INVENTORY_UNAVAILABLE` | 1 | Reclamation requires final inventory that is not complete | self; existing exact identity | Complete final inventory first | public |
| `ROOT_REMOVAL_PUBLICATION_UNAVAILABLE` | 1 | Store reclamation was requested before logical Coordinator removal publication | self | Complete and retain logical removal first | public |
| `ROOT_STORE_RECLAMATION_FINAL_INVENTORY_MISMATCH` | 1 | Requested hash differs from retained final inventory | self | Use the exact retained final-inventory authority | public |
| `ROOT_STORE_RECLAMATION_INTENT_COMMIT_CONFLICT` | 1 | Root Registry changed before reclamation-intent commit | self | Reload current authority and exact retry | public |
| `ROOT_STORE_RECLAMATION_INTENT_MISSING` | 1 | Storage accepted the intent commit but retained no intent | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before invoking reclamation | recent failure |
| `ROOT_STORE_RECLAMATION_COMPLETION_TIME_INVALID` | 1 | Reclamation completion time is zero | self | Supply the positive observed completion time | public |
| `ROOT_STORE_RECLAMATION_COMMIT_CONFLICT` | 1 | Root Registry changed before terminal reclamation commit | self | Reload current authority and retry exact evidence persistence | public |
| `ROOT_STORE_RECLAMATION_RECEIPT_MISSING` | 1 | Storage accepted reclamation commit but retained no receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the durable operation | recent failure |
| `ROOT_STORE_RECLAMATION_RECEIPT_MISMATCH` | 1 | Committed reclamation receipt differs from exact terminal evidence | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the retained receipt and fail closed | recent failure |
| `ROOT_STORE_BINDING_FINALIZATION_AUTHORITY_INVALID` | 1 | Reclamation hash, binding, generation or preparation time is incomplete | self | Supply complete exact finalization authority | public |
| `ROOT_STORE_BINDING_FINALIZATION_INTENT_CONFLICT` | 1 | Exact operation already retains different binding-finalization authority | self | Replay only the original intent | public |
| `ROOT_STORE_RECLAMATION_UNAVAILABLE` | 1 | Binding finalization requires a terminal reclamation receipt | self | Complete Store reclamation first | public |
| `ROOT_STORE_BINDING_FINALIZATION_RECLAMATION_MISMATCH` | 1 | Requested reclamation hash differs from the retained receipt | self | Use the exact retained reclamation authority | public |
| `ROOT_STORE_BINDING_FINALIZATION_INTENT_COMMIT_CONFLICT` | 1 | Root Registry changed before finalization-intent commit | self | Reload current authority and exact retry | public |
| `ROOT_STORE_BINDING_FINALIZATION_INTENT_MISSING` | 1 | Storage accepted finalization-intent commit but retained no intent | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before the publication-state effect | recent failure |
| `ROOT_STORE_BINDING_FINALIZATION_COMPLETION_TIME_INVALID` | 1 | Binding-finalization completion time is zero | self | Supply the positive observed completion time | public |
| `ROOT_STORE_BINDING_FINALIZATION_COMMIT_CONFLICT` | 1 | Root Registry changed before terminal finalization commit | self | Reload current authority and retry exact evidence persistence | public |
| `ROOT_STORE_BINDING_FINALIZATION_RECEIPT_MISSING` | 1 | Storage accepted finalization commit but retained no receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the durable operation | recent failure |
| `ROOT_STORE_BINDING_FINALIZATION_RECEIPT_MISMATCH` | 1 | Committed finalization receipt differs from exact terminal evidence | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the retained receipt and fail closed | recent failure |
| `ROOT_STORE_RECLAMATION_INTENT_UNPREPARED` | 1 | Terminal Store evidence arrived before durable reclamation intent | self | Prepare the intent before recording effect evidence | public |
| `ROOT_STORE_RECLAMATION_EVIDENCE_MISMATCH` | 1 | Live Store does not prove one exact empty completed GC lineage | self | Re-observe/complete the exact Store reclamation | public |
| `ROOT_STORE_RECLAMATION_ENCODING_FAILED` | 1 | Canonical reclamation receipt authority cannot be encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not advance | recent failure |
| `ROOT_STORE_BINDING_FINALIZATION_INTENT_UNPREPARED` | 1 | Publication evidence arrived before durable finalization intent | self | Prepare the intent before recording effect evidence | public |
| `ROOT_STORE_BINDING_FINALIZATION_GENERATION_OVERFLOW` | 1 | Required exact `source + 3` publication generation overflows | `COMPONENT_REGISTRY_STATE_INVALID` | Stop retirement; never wrap a publication generation | recent failure |
| `ROOT_STORE_BINDING_FINALIZATION_EVIDENCE_MISMATCH` | 1 | Live publication state does not prove the exact binding removal and generation | self | Re-observe/complete the exact publication-state effect | public |
| `ROOT_STORE_BINDING_FINALIZATION_ENCODING_FAILED` | 1 | Canonical finalization receipt authority cannot be encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not advance | recent failure |

The 32 rows sum to all 45 selected sites. Twenty-eight exact identities are
new; four reuse the root readiness/final-inventory identities from the previous
slice. No projection is added.

## Store Deletion And Root-Deletion Preparation Persistence Slice

This slice accounts for all 61 direct constructor references in:

- `root_store_deletion_intent_if_present`, `begin_root_store_deletion`,
  `record_root_store_cycle_reclamation`, `root_store_deletion_if_present` and
  `record_root_store_deletion`;
- `root_deletion_preparation_intent_if_present`,
  `begin_root_deletion_preparation`, `record_root_deletion_cycle_reclamation`,
  `root_deletion_preparation_if_present` and
  `record_root_deletion_preparation`; and
- `root_store_deletion_record`, `root_store_deletion_hash` and
  `validate_root_store_deletion_authority`.

The exact baseline ranges are lines 2174–2720 and 8036–8158 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. Management
stop/delete/status calls and Coordinator readiness publication remain workflow
effects outside this persistence slice. Their durable intent, cycle evidence,
typed terminal absence and exact retry records are classified here.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 10 | Root Component Registry meta authority is absent | self; existing exact identity | Complete root Registry preparation before retry | public |
| `ROOT_DRAINING_UNSTARTED` | 4 | Physical-retirement persistence has no durable root draining fence | self; existing exact identity | Begin root draining before continuing | public |
| `ROOT_DRAINING_OPERATION_CONFLICT` | 4 | Store/root deletion status names a different draining operation | self; existing exact identity | Query/replay only the exact retained operation | public |
| `ROOT_STORE_DELETION_INTENT_CONFLICT` | 1 | Exact operation already retains different Store deletion authority | self | Replay only the original deletion intent | public |
| `ROOT_STORE_BINDING_FINALIZATION_UNAVAILABLE` | 1 | Store deletion requires a terminal binding-finalization receipt | self | Complete exact binding finalization first | public |
| `ROOT_STORE_DELETION_FINALIZATION_MISMATCH` | 1 | Requested finalization hash differs from its retained receipt | self | Use the exact retained finalization authority | public |
| `ROOT_STORE_DELETION_BINDING_MISMATCH` | 1 | Requested Store binding differs from finalized publication authority | self | Re-observe/use the finalized binding | public |
| `ROOT_STORE_DELETION_CANISTER_MISMATCH` | 1 | Requested Store principal differs from finalized publication authority | self | Never substitute a Store principal; use exact authority | public |
| `ROOT_STORE_DELETION_CONTROLLER_AUTHORITY_MISSING` | 1 | Observed Store controllers omit the protected Fleet Subnet Root | self | Restore/re-observe exact root authority before deletion | public |
| `ROOT_STORE_DELETION_INTENT_COMMIT_CONFLICT` | 1 | Root Registry changed before Store-deletion intent commit | self | Reload current authority and exact retry | public |
| `ROOT_STORE_DELETION_INTENT_MISSING` | 1 | Storage accepted deletion-intent commit but retained no intent | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before cycle transfer or deletion | recent failure |
| `ROOT_STORE_DELETION_INTENT_UNPREPARED` | 2 | Cycle or deletion evidence arrived before durable Store-deletion intent | self | Prepare the intent before any effect evidence is recorded | public |
| `ROOT_STORE_CYCLE_RECLAMATION_RECEIPT_CONFLICT` | 1 | Replayed Store cycle evidence differs from its durable receipt | self | Replay only the exact observed result | public |
| `ROOT_STORE_CYCLE_RECLAMATION_EVIDENCE_INVALID` | 1 | Store post-transfer cycles or time exceed durable deletion authority | self | Reject contradictory evidence and re-observe the exact Store | public |
| `ROOT_STORE_CYCLE_RECLAMATION_COMMIT_CONFLICT` | 1 | Root Registry changed before Store cycle evidence committed | self | Reload and retry exact evidence persistence | public |
| `ROOT_STORE_CYCLE_RECLAMATION_RECEIPT_MISSING` | 1 | Storage accepted Store cycle evidence but retained no receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before physical deletion | recent failure |
| `ROOT_STORE_DELETION_COMPLETION_TIME_INVALID` | 1 | Store-deletion completion time is zero | self | Supply the positive typed-absence observation time | public |
| `ROOT_STORE_DELETION_COMMIT_CONFLICT` | 1 | Root Registry changed before terminal Store-deletion commit | self | Reload and retry exact terminal evidence persistence | public |
| `ROOT_STORE_DELETION_RECEIPT_MISSING` | 1 | Storage accepted deletion commit but retained no terminal receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before root deletion preparation | recent failure |
| `ROOT_STORE_DELETION_RECEIPT_MISMATCH` | 1 | Committed Store-deletion receipt differs from exact absence authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the receipt and fail closed | recent failure |
| `ROOT_DELETION_PREPARATION_AUTHORITY_INVALID` | 1 | Store hash, Coordinator, cycles observations, derived target or time is incomplete | self | Supply one complete exact preparation authority | public |
| `ROOT_DELETION_PREPARATION_INTENT_CONFLICT` | 1 | Exact operation already retains different root-deletion authority | self | Replay only the original preparation intent | public |
| `ROOT_FINAL_INVENTORY_UNAVAILABLE` | 1 | Root-deletion preparation requires terminal final inventory | self; existing exact identity | Complete final inventory first | public |
| `ROOT_STORE_DELETION_UNAVAILABLE` | 1 | Root-deletion preparation requires terminal Store deletion | self | Complete and retain Store deletion first | public |
| `ROOT_DELETION_PREPARATION_STORE_RECEIPT_MISMATCH` | 1 | Requested Store-deletion hash differs from retained receipt | self | Use the exact retained Store-deletion authority | public |
| `ROOT_DELETION_PREPARATION_COORDINATOR_MISMATCH` | 1 | Requested Coordinator differs from protected Fleet authority | self | Use the exact protected Coordinator | public |
| `ROOT_DELETION_PREPARATION_INTENT_COMMIT_CONFLICT` | 1 | Root Registry changed before deletion-preparation intent commit | self | Reload current authority and exact retry | public |
| `ROOT_DELETION_PREPARATION_INTENT_MISSING` | 1 | Storage accepted preparation-intent commit but retained no intent | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before Coordinator readiness or cycle transfer | recent failure |
| `ROOT_DELETION_PREPARATION_INTENT_UNPREPARED` | 2 | Root cycle/readiness evidence arrived before durable preparation intent | self | Prepare the intent before recording later evidence | public |
| `ROOT_CYCLE_RECLAMATION_RECEIPT_CONFLICT` | 1 | Replayed root cycle/Coordinator intent evidence differs from its receipt | self | Replay only the exact observed result | public |
| `ROOT_CYCLE_RECLAMATION_EVIDENCE_INVALID` | 1 | Coordinator intent, post-transfer cycles or time exceeds durable authority | self | Reject contradictory evidence and re-observe exact authority | public |
| `ROOT_CYCLE_RECLAMATION_COMMIT_CONFLICT` | 1 | Root Registry changed before root cycle evidence committed | self | Reload and retry exact evidence persistence | public |
| `ROOT_CYCLE_RECLAMATION_RECEIPT_MISSING` | 1 | Storage accepted root cycle evidence but retained no receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before readiness commit | recent failure |
| `ROOT_DELETION_READINESS_AUTHORITY_INVALID` | 1 | Coordinator readiness hash or completion time is missing | self | Supply the complete exact readiness receipt | public |
| `ROOT_CYCLE_RECLAMATION_AMOUNT_MISSING` | 1 | Root deletion intent lacks observed post-transfer cycles | self | Complete and record exact root cycle reclamation | public |
| `ROOT_CYCLE_RECLAMATION_TIME_MISSING` | 1 | Root deletion intent lacks cycle-reclamation time | self | Complete and record exact root cycle reclamation | public |
| `ROOT_DELETION_COORDINATOR_INTENT_MISSING` | 1 | Root deletion intent lacks Coordinator execution-intent hash | self | Complete Coordinator readiness intent first | public |
| `ROOT_DELETION_READINESS_COMMIT_CONFLICT` | 1 | Root Registry changed before readiness receipt commit | self | Reload and retry exact receipt persistence | public |
| `ROOT_DELETION_READINESS_RECEIPT_MISSING` | 1 | Storage accepted readiness commit but retained no receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; physical root deletion is not authorized | recent failure |
| `ROOT_STORE_CYCLE_RECLAMATION_AMOUNT_MISSING` | 1 | Store deletion intent lacks observed post-transfer cycles | self | Complete and record exact Store cycle reclamation | public |
| `ROOT_STORE_CYCLE_RECLAMATION_TIME_MISSING` | 1 | Store deletion intent lacks cycle-reclamation time | self | Complete and record exact Store cycle reclamation | public |
| `ROOT_STORE_DELETION_EVIDENCE_MISMATCH` | 1 | Typed Store absence evidence differs from durable deletion/cycle authority | self | Reject it; re-observe exact typed absence | public |
| `ROOT_STORE_DELETION_ENCODING_FAILED` | 1 | Canonical Store-deletion receipt authority cannot be encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not advance | recent failure |
| `ROOT_STORE_DELETION_AUTHORITY_INVALID` | 1 | Store binding/module/controllers/cycles/time deletion authority is incomplete | self | Supply canonical controllers and complete exact authority | public |

The 44 rows sum to all 61 selected sites. Forty exact identities are new; four
reuse root readiness/final-inventory identities. No projection is added.

## Final And Initial Root Inventory Persistence Slice

This slice accounts for all 35 direct constructor references in:

- `finalize_root_inventory`;
- `seal_initial_inventory`, `validate_sealed_initial_inventory`,
  `initial_inventory`, `mark_initial_inventory_directories_converged` and
  `mark_initial_inventory_root_runtime_activated`; and
- `complete_initial_inventory`, `initial_inventory_hash_entry`,
  `initial_inventory_hash`, `validate_initial_inventory_receipt` and
  `update_initial_inventory_receipt`.

The exact baseline ranges are lines 2721–2928 and 7121–7358 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`.
`registry_covers_preparation` and the view converter contain no constructor.
The shared atomic meta commit adapter remains classified at its concrete
variants rather than receiving an inventory aggregate code.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_FINAL_INVENTORY_RETRY_REGISTRY_CONFLICT` | 1 | Terminal retry supplies a different Fleet Registry than the retained inventory | self | Replay only the exact original Registry authority | public |
| `ROOT_FINAL_INVENTORY_FINALIZATION_TIME_INVALID` | 1 | Final-inventory completion time is zero | self | Supply the positive observed completion time | public |
| `ROOT_FINAL_INVENTORY_INTENT_UNPREPARED` | 1 | Finalization was requested before durable final-inventory intent | self | Prepare the exact intent first | public |
| `ROOT_FINAL_INVENTORY_INTENT_CONFLICT` | 1 | Finalization Registry differs from durable intent | self | Replay only the exact retained intent | public |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 5 | Root Component Registry meta authority is absent | self; existing exact identity | Complete root Registry preparation before retry | public |
| `ROOT_FINAL_INVENTORY_PRECEDES_DRAINING` | 1 | Finalization time predates the durable draining fence | self; existing exact identity | Re-observe after the fence and retry exact authority | public |
| `ROOT_FINAL_INVENTORY_COMMIT_CONFLICT` | 1 | Root Registry changed before final-inventory receipt committed | self | Reload current authority and retry exact persistence | public |
| `ROOT_FINAL_INVENTORY_COMMIT_MISMATCH` | 1 | Committed final inventory differs from the prepared terminal record | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the committed record and fail closed | recent failure |
| `ROOT_INITIAL_INVENTORY_SEAL_TIME_INVALID` | 1 | Initial-inventory seal time is zero | self | Supply a positive observed seal time | public |
| `ROOT_INITIAL_INVENTORY_UNSEALED` | 3 | Initial inventory is required but has no durable seal receipt | self | Seal the complete initial inventory first | public |
| `ROOT_INITIAL_INVENTORY_ACTIVATION_CONFLICT` | 3 | Fleet activation operation or inventory hash differs from the sealed receipt | self | Replay only the exact sealed activation authority | public |
| `ROOT_INITIAL_INVENTORY_NONTERMINAL_ALLOCATIONS` | 1 | Reserved Component allocations remain at the one-way seal | self | Finish every initial allocation before sealing | public |
| `ROOT_INITIAL_INVENTORY_COUNT_OVERFLOW` | 1 | Initial Component count cannot fit its bounded receipt field | `COMPONENT_REGISTRY_STATE_INVALID` | Stop activation and inspect inventory bounds | recent failure |
| `ROOT_INITIAL_INVENTORY_COUNTERS_MISMATCH` | 1 | Committed count or next sequence differs from allocation inventory | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile Registry counters | recent failure |
| `ROOT_INITIAL_INVENTORY_CANISTER_COUNT_OVERFLOW` | 1 | Top-level plus descendant count overflows during inventory proof | `COMPONENT_REGISTRY_STATE_INVALID` | Stop activation and inspect Canister accounting | recent failure |
| `ROOT_INITIAL_INVENTORY_KNOWN_CREATED_MISMATCH` | 1 | Known-created counter lies outside complete initial inventory bounds | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile physical/logical inventory | recent failure |
| `ROOT_INITIAL_INVENTORY_PARTITION_CARDINALITY_MISMATCH` | 1 | Allocation and Registry-partition counts differ | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile normalized Registry state | recent failure |
| `COMPONENT_REGISTRY_BYTE_COUNT_OVERFLOW` | 1 | Initial partition-byte reconstruction overflows | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop activation and inspect byte accounting | recent failure |
| `ROOT_INITIAL_INVENTORY_BYTE_LEDGER_MISMATCH` | 1 | Reconstructed partition bytes differ from root Registry bytes | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile byte authority | recent failure |
| `ROOT_INITIAL_INVENTORY_ALLOCATION_SEQUENCE_INVALID` | 1 | Canonical initial allocation sequence is not consecutive | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; never repair by reusing identity | recent failure |
| `ROOT_INITIAL_INVENTORY_COMMITMENT_UNAVAILABLE` | 1 | Initial allocation lacks Registry commitment | self | Finish exact commitment before sealing | public |
| `ROOT_INITIAL_INVENTORY_MEMBERSHIP_UNAVAILABLE` | 1 | Initial allocation lacks active membership | self | Finish exact membership activation before sealing | public |
| `ROOT_INITIAL_INVENTORY_TERMINAL_EVIDENCE_UNAVAILABLE` | 1 | Directory, runtime or membership convergence is incomplete | self | Complete the missing activation/convergence edge | public |
| `ROOT_INITIAL_INVENTORY_ENCODING_FAILED` | 1 | Canonical initial-inventory authority cannot be encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not seal | recent failure |
| `ROOT_INITIAL_INVENTORY_RECEIPT_INVALID` | 1 | Retained sealed count, hash or time differs from current protected authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the receipt and fail closed | recent failure |
| `ROOT_INITIAL_INVENTORY_RUNTIME_BEFORE_DIRECTORIES` | 1 | Requested root runtime activation precedes Directory convergence | self | Commit Directory convergence first | public |
| `ROOT_INITIAL_INVENTORY_RUNTIME_RECEIPT_INVALID` | 1 | Retained root-runtime receipt lacks Directory convergence evidence | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect activation receipt state | recent failure |

The 27 rows sum to all 35 selected sites. Twenty-four exact identities are new;
three reuse Registry readiness/byte and final-inventory ordering identities. No
projection is added.

## Directory And Protected-Status Persistence Slice

This slice accounts for all 15 direct constructor references in:

- `directory_page`;
- `prepared_partition` and `committed_child_authority`;
- `registered_parent`; and
- `subtree_removal`.

The exact baseline range is lines 3423–3675 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. The partition,
child, traversal and subtree validators remain separately accounted source
sites; this table classifies only the direct lookup and paging boundary. The
thin `partition`, `component_for_principal`, `child_allocation` and
`component_draining` views contain no direct constructor of their own.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_DIRECTORY_PAGE_LIMIT_INVALID` | 1 | Ops received a zero examined-row bound | self; existing exact identity | Supply the positive protocol-bounded page limit | public |
| `COMPONENT_DIRECTORY_CURSOR_QUERY_CONFLICT` | 2 | Cursor parent or parent-role identity differs from the selected filter | self; existing exact identity | Restart the exact query; never reuse a cursor across filters | public |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 1 | The requested Component has no committed Registry partition | self; existing exact identity | Commit or recover the exact partition before reading | public |
| `COMPONENT_DIRECTORY_TRAVERSAL_CHILD_MISSING` | 1 | A retained traversal index names no normalized child row | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile traversal/child indexes | recent failure |
| `COMPONENT_DIRECTORY_TRAVERSAL_AUTHORITY_INVALID` | 1 | Traversal, normalized child and parent-principal index disagree | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile exact tree/index authority | recent failure |
| `COMPONENT_ALLOCATION_OPERATION_UNRESERVED` | 1 | Prepared-partition lookup has no retained top-level allocation | self; existing exact identity | Reserve or query the exact operation first | public |
| `COMPONENT_ALLOCATION_COMMITMENT_TRANSITION_INVALID` | 1 | Prepared-partition lookup is requested before Registry commitment | self; existing exact identity | Complete exact verification and Registry commitment first | public |
| `COMPONENT_CHILD_OPERATION_UNRESERVED` | 1 | Committed-child lookup has no retained child allocation | self; existing exact identity | Reserve or query the exact child operation first | public |
| `COMPONENT_CHILD_COMMITMENT_TRANSITION_INVALID` | 1 | Committed-child lookup is requested before Registry commitment | self; existing exact identity | Complete exact child verification and commitment first | public |
| `COMPONENT_REGISTRY_PARTITION_MISSING` | 1 | A principal index retains membership after its owning partition disappeared | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile protected Registry persistence | recent failure |
| `COMPONENT_CHILD_PRINCIPAL_INDEX_MISSING` | 1 | An indexed child principal has no normalized child row | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile the principal/child indexes | recent failure |
| `COMPONENT_CHILD_PRINCIPAL_INDEX_INVALID` | 1 | Indexed child, immediate parent and traversal index disagree | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile normalized parentage/index authority | recent failure |
| `COMPONENT_SUBTREE_ROOT_AUTHORITY_MISSING` | 1 | A retained subtree-removal operation has no root Registry authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile root meta with retained removal state | recent failure |
| `COMPONENT_SUBTREE_FENCE_PARTITION_MISSING` | 1 | A retained subtree-removal operation has no owning Component partition | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile partition/fence persistence | recent failure |

The 14 rows sum to all 15 selected sites. Three exact identities are new;
eleven existing workflow, allocation or Registry identities are reused. No
safe projection is added. Reusing the paging, commitment and principal-index
identities is deliberate: the ops boundary owns the same failed action and
retry decision rather than a second layer-specific code.

## Top-Level Draining And Removal Transition Persistence Slice

This slice accounts for all 50 direct constructor references in:

- `begin_component_draining`;
- `prepare_component_quiescence` and `mark_component_quiescent`;
- `advance_component_draining` and `finalize_component_inventory`; and
- `prepare_component_deletion`, `mark_component_deleted` and
  `remove_component_membership`.

The exact baseline range is lines 3698–4329 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. Calls into the
partition, draining, Directory and terminal-inventory validators remain
separately accounted source sites. The exact storage commit adapter retains its
already-classified typed variants rather than receiving one aggregate
transition code.

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 4 | Root Component Registry meta authority is absent | self; existing exact identity | Complete root Registry preparation before retry | public |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 8 | The target Component has no committed Registry partition | self; existing exact identity | Commit or recover the exact partition before lifecycle work | public |
| `COMPONENT_DRAINING_OPERATION_CONFLICT` | 3 | Existing drain, quiescence or final-inventory state names another operation | self; existing exact identity | Replay only the exact retained operation | public |
| `COMPONENT_DRAINING_OPERATION_ID_INVALID` / `COMPONENT_DRAINING_LIFECYCLE_UNAVAILABLE` / `COMPONENT_DRAINING_AUTHORITY_MISSING` / `COMPONENT_DRAINING_REQUEST_AUTHORITY_CONFLICT` | 1 | One branch merges a zero operation, a valid non-Active phase, a `Draining` partition with no fence, and stale expected Registry authority | first two and fourth self; authority missing projects to `COMPONENT_REGISTRY_STATE_INVALID`; last two are existing exact identities | Split predicates; correct invalid/stale input, wait for a valid lifecycle, or fail closed on missing protected authority | public except missing authority is recent failure |
| `COMPONENT_DRAINING_DIRECTORY_TIME_NOT_MONOTONIC` | 1 | Draining fence time does not advance the current Component Directory time | self | Re-observe a later time against the current partition | public |
| `COMPONENT_DRAINING_CHILD_LIFECYCLE_INCOMPLETE` | 2 | Reserved descendants or a nonterminal child allocation remain at the drain fence | self | Finish every admitted child operation before exact retry | public |
| `COMPONENT_DRAINING_SUBTREE_REMOVAL_INCOMPLETE` | 1 | A retained subtree-removal operation is still nonterminal at the drain fence | self | Finish the exact subtree removal before retry | public |
| `COMPONENT_REGISTRY_REVISION_EXHAUSTED` | 1 | Draining cannot advance the Component Registry revision without wrapping | self; existing exact identity | Retire or reinstall the root; never wrap Registry history | public |
| `COMPONENT_DRAINING_UNPREPARED` | 7 | A later transition has no durable Component draining fence | self; existing exact identity | Begin or query the exact draining operation first | public |
| `COMPONENT_QUIESCENCE_REQUEST_AUTHORITY_CONFLICT` | 1 | Quiescence operation or expected Registry differs from the draining fence | self; existing exact identity | Reload the fence and replay only its exact request | public |
| `COMPONENT_QUIESCENCE_DESCENDANT_REMOVAL_STARTED` | 1 | Partition descendant authority changed before quiescence intent was prepared | self | Prepare quiescence before beginning descendant removal | public |
| `COMPONENT_QUIESCENCE_MODULE_AUTHORITY_MISSING` / `COMPONENT_QUIESCENCE_PREPARATION_TIME_INVALID` | 1 | Quiescence preparation has a zero module hash or predates the draining fence | self | Supply qualified module evidence observed no earlier than the fence | public |
| `COMPONENT_QUIESCENCE_DIRECTORY_AUTHORITY_CONFLICT` | 1 | Prepared Directory evidence does not cover the exact draining Registry | self | Re-converge and supply the exact draining Directory authority | public |
| `COMPONENT_QUIESCENCE_RECEIPT_CONFLICT` | 1 | Exact retry supplies a module different from the retained terminal receipt | self | Replay only the exact observed terminal module evidence | public |
| `COMPONENT_QUIESCENCE_UNPREPARED` | 1 | Terminal quiescence evidence arrived before its durable stop intent | self; existing exact identity | Prepare the stop intent before recording its result | public |
| `COMPONENT_QUIESCENCE_MODULE_MISMATCH` / `COMPONENT_QUIESCENCE_OBSERVATION_TIME_INVALID` | 1 | Observed terminal module differs from the intent or its time predates preparation | self; module identity is an existing exact identity | Re-observe the exact intended module at a valid later time | public |
| `COMPONENT_DRAINING_OPERATION_CONFLICT` / redundant post-validator lifecycle predicate / `COMPONENT_DRAINING_QUIESCENCE_INCOMPLETE` | 1 | Drain advancement merges the wrong operation, a status already proved by the validator and incomplete terminal quiescence | exact existing identities for operation/quiescence; no code for redundant predicate | Split the actionable predicates and delete the redundant status check | public; sediment for redundant predicate |
| `COMPONENT_DRAINING_CURSOR_REMOVAL_MISSING` | 1 | Draining cursor names no retained subtree-removal operation | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile cursor/removal persistence | recent failure |
| `COMPONENT_DRAINING_DESCENDANT_ROOT_MISSING` | 1 | Nonempty descendant authority has no registered direct root child | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile the normalized tree indexes | recent failure |
| `COMPONENT_DRAINING_CURSOR_TARGET_MISMATCH` | 1 | Derived subtree operation is retained for a different direct-child target | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile deterministic cursor authority | recent failure |
| `COMPONENT_FINAL_INVENTORY_REQUEST_AUTHORITY_CONFLICT` | 1 | Exact retry supplies a Registry different from the retained final inventory | self | Replay only the exact retained final-inventory authority | public |
| `COMPONENT_DRAINING_QUIESCENCE_INCOMPLETE` | 2 | Final inventory or deletion is requested before terminal quiescence | self; existing exact identity | Complete the exact quiescence journey first | public |
| `COMPONENT_FINAL_INVENTORY_UNAVAILABLE` | 1 | Deletion preparation has no frozen final inventory | self | Finalize and retain the exact empty inventory first | public |
| `COMPONENT_DELETION_REQUEST_AUTHORITY_CONFLICT` | 1 | Deletion request hash differs from the frozen final inventory | self; existing exact identity | Replay only the exact frozen deletion request | public |
| `COMPONENT_DELETION_PREPARATION_TIME_INVALID` | 1 | Deletion preparation predates final-inventory finalization | self | Re-observe a time at or after finalization | public |
| `COMPONENT_DELETION_UNPREPARED` | 2 | Deletion evidence or membership removal has no durable deletion intent | self; existing exact identity | Prepare the exact deletion intent first | public |
| `COMPONENT_DELETION_OBSERVATION_TIME_INVALID` | 1 | Deletion observation predates its durable intent | self | Supply the exact later effect observation | public |
| `COMPONENT_DELETION_RECYCLING_UNREADY` | 1 | Membership removal was requested before independent deletion/recycling evidence | self; existing exact identity | Complete physical recycling before membership removal | public |
| `COMPONENT_MEMBERSHIP_REMOVAL_TIME_INVALID` | 1 | Membership-removal observation predates deletion authority | self | Supply a removal time at or after terminal deletion evidence | public |

The 29 rows sum to all 50 selected sites. Compound branches qualify 35 exact-
label occurrences: 19 exact identities are new and 16 reuse existing workflow,
Registry or allocation meanings. The post-validator lifecycle predicate in
`advance_component_draining` is sediment and receives no code. No safe
projection is added.

The begin-draining compound branch must distinguish a valid non-Active phase
from a `Draining` partition that has lost its durable fence; only the latter is
a masked protected-state contradiction. Likewise, quiescence module/time
evidence and advancement operation/quiescence state must become independent
predicates before B4 replaces their strings.

## Top-Level Draining And Removal Protected-Validation Slice

This slice accounts for 58 direct constructor references in:

- `component_draining_state`, `component_quiescence_terminal_entry_bytes` and
  `component_quiescence_intent_state`;
- `RootComponentFinalInventoryAuthority::invalid`,
  `RootComponentDeletionAuthority::invalid`,
  `require_ordinary_component_lifecycle`,
  `validate_ordinary_component_lifecycle` and
  `validate_component_draining_record`;
- the final-inventory, deletion, allocation-history, membership-removal and
  removed-authority helpers from `ensure_component_final_inventory_candidate`
  through `component_has_terminal_quiescence`;
- `ensure_component_lifecycle_history_is_terminal` and
  `component_final_inventory_hash`; and
- `removed_component_descendant_content_hash`.

The exact baseline ranges are lines 392–568, 9230–9459, 11354–12032,
12906–12933, 13063–13095 and 13190–13238 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. The two shared
`invalid` constructors each currently hide several independently actionable
predicates. Their source references count once apiece, but their rows expand
every predicate that B4 must encode separately.

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_BYTE_ACCOUNTING_UNDERFLOW` | 4 | Component or root bytes cannot subtract the partition/draining entries being replaced | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and reconcile the byte ledger | recent failure |
| `COMPONENT_REGISTRY_BYTE_COUNT_OVERFLOW` | 6 | Draining or quiescence replacement bytes overflow checked arithmetic | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and inspect bounded byte accounting | recent failure |
| `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | 4 | Draining or quiescence state exceeds the protected Component or root ceiling | self; existing exact identity | Free Registry capacity or reinstall with larger admitted limits | public |
| `COMPONENT_DRAINING_BYTE_ACCOUNTING_NONCONVERGENT` | 1 | Draining partition fixed-point bytes do not stabilize | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct the encoded-byte model | recent failure |
| `COMPONENT_QUIESCENCE_TERMINAL_BYTE_RESERVATION_NONCONVERGENT` | 1 | Worst-case quiescence/removal receipt reservation does not stabilize | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before persisting an insufficient charge | recent failure |
| `COMPONENT_QUIESCENCE_BYTE_ACCOUNTING_NONCONVERGENT` | 1 | Quiescence partition fixed-point bytes do not stabilize | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct the encoded-byte model | recent failure |
| `COMPONENT_GROUP_AGGREGATE_LIFECYCLE_REQUIRED` | 1 | A grouped Component was sent through ordinary per-Component removal | self; existing exact identity | Resume through its aggregate Component Group lifecycle | public |
| `COMPONENT_GROUP_ORDINARY_DRAINING_STATE_INVALID` | 1 | Persisted grouped authority has entered the ordinary draining validator | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile grouped lifecycle authority | recent failure |
| `COMPONENT_DRAINING_RECEIPT_INVALID` | 1 | Draining identity, Registry, descendant, Directory, quiescence, cursor or partition coverage is not exact | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve the receipt and reconcile protected Registry authority | recent failure |
| `COMPONENT_FINAL_INVENTORY_QUIESCENCE_MISSING` / `COMPONENT_FINAL_INVENTORY_AUTHORITY_INVALID` / `COMPONENT_FINAL_INVENTORY_FLEET_COVERAGE_INVALID` / `COMPONENT_FINAL_INVENTORY_DIRECTORY_AUTHORITY_INVALID` / `COMPONENT_FINAL_INVENTORY_RECEIPT_TIME_INVALID` / `COMPONENT_FINAL_INVENTORY_HASH_INVALID` / `COMPONENT_FINAL_INVENTORY_EMPTY_STATE_INVALID` / `COMPONENT_FINAL_INVENTORY_MEMBERSHIP_INVALID` / `COMPONENT_FINAL_INVENTORY_CURSOR_INVALID` | 1 | One shared constructor hides nine final-inventory receipt predicates | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Split quiescence, snapshot, Fleet coverage, Directory, time, hash, empty-state, index and cursor validation | recent failure |
| `COMPONENT_DELETION_FINAL_INVENTORY_MISSING` / `COMPONENT_DELETION_QUIESCENCE_MISSING` / `COMPONENT_DELETION_FINAL_INVENTORY_AUTHORITY_INVALID` / `COMPONENT_DELETION_QUIESCENCE_AUTHORITY_INVALID` / `COMPONENT_DELETION_PREPARATION_RECEIPT_TIME_INVALID` / `COMPONENT_DELETION_RECEIPT_TIME_INVALID` / `COMPONENT_MEMBERSHIP_REMOVAL_RECEIPT_TIME_INVALID` / `COMPONENT_DELETION_BYTE_CHARGE_EXCEEDED` | 1 | One shared constructor hides eight deletion/removal receipt predicates | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Split missing authority, retained evidence, each time edge and byte precharge validation | recent failure |
| `COMPONENT_FINAL_INVENTORY_REQUEST_AUTHORITY_CONFLICT` | 1 | Requested final-inventory Registry differs from the current partition head | self; existing exact identity | Reload the current head and retry exact authority | public |
| `COMPONENT_FINAL_INVENTORY_LIVE_STATE_REMAINS` | 1 | Partition is not yet empty and `Draining` | self | Finish descendant removal and draining convergence first | public |
| `COMPONENT_FINAL_INVENTORY_TIME_INVALID` | 1 | Proposed finalization predates quiescence or current Directory authority | self | Re-observe a time covering both terminal authorities | public |
| `COMPONENT_FINAL_INVENTORY_LIVE_MEMBERSHIP_REMAINS` | 1 | Empty counters/hash disagree with retained child or principal indexes | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile normalized membership indexes | recent failure |
| `COMPONENT_FINAL_INVENTORY_FLEET_DIRECTORY_ROOT_MISMATCH` | 1 | Fleet Directory provenance names another Fleet Subnet Root | self | Supply the exact local root's authenticated Directory | public |
| `COMPONENT_FINAL_INVENTORY_FLEET_REGISTRY_VERSION_INVALID` | 1 | Fleet Directory provenance has revision zero | self | Supply versioned Fleet Registry authority | public |
| `COMPONENT_FINAL_INVENTORY_FLEET_REGISTRY_HASH_INVALID` | 1 | Fleet Directory provenance has an empty content hash | self | Supply exact nonempty Fleet Registry authority | public |
| `COMPONENT_DRAINING_OPERATION_CONFLICT` | 1 | Deletion operation differs from the retained draining fence | self; existing exact identity | Replay only the exact draining operation | public |
| `COMPONENT_DELETION_REQUEST_AUTHORITY_CONFLICT` | 1 | Deletion progress differs from the expected frozen inventory hash | self; existing exact identity | Replay only the exact retained deletion request | public |
| `COMPONENT_ALLOCATION_HISTORY_MISSING` | 1 | Live Component partition has no retained top-level allocation | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and restore/reconcile immutable allocation history | recent failure |
| `COMPONENT_ALLOCATION_HISTORY_DUPLICATE` | 1 | More than one retained allocation claims the same Component | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; never select one duplicate as authority | recent failure |
| `COMPONENT_ALLOCATION_HISTORY_AUTHORITY_INVALID` | 1 | Retained committed allocation identity differs from the draining partition | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both records and reconcile protected authority | recent failure |
| `COMPONENT_ALLOCATION_TERMINAL_ACTIVATION_MISSING` | 1 | Draining allocation lacks terminal Directory/runtime/membership activation evidence | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile immutable activation history | recent failure |
| redundant postcondition in `removed_component_allocation` | 1 | Private helper rechecks the committed phase already proved by its sole caller | no code | Delete the unreachable defensive branch after retaining the caller proof | sediment |
| `COMPONENT_ALLOCATION_TERMINAL_RECORD_BOUND_EXCEEDED` | 1 | Removed allocation history exceeds its bounded stable record allowance | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before committing oversized terminal history | recent failure |
| `COMPONENT_MEMBERSHIP_REMOVAL_SPEC_COUNT_INVALID` | 1 | Per-Spec committed count cannot decrement into its bounded field | `COMPONENT_REGISTRY_STATE_INVALID` | Stop settlement and reconcile Spec counters | recent failure |
| `COMPONENT_MEMBERSHIP_REMOVAL_ROOT_COMMITTED_COUNT_UNDERFLOW` | 1 | Root committed-Component count is already zero | `COMPONENT_REGISTRY_STATE_INVALID` | Stop settlement and reconcile root counters | recent failure |
| `COMPONENT_MEMBERSHIP_REMOVAL_ROOT_KNOWN_CREATED_COUNT_UNDERFLOW` | 1 | Root known-created Canister count is already zero | `COMPONENT_REGISTRY_STATE_INVALID` | Stop settlement and reconcile physical inventory accounting | recent failure |
| `COMPONENT_MEMBERSHIP_REMOVAL_BYTE_CHARGE_EXCEEDED` | 1 | Terminal removal receipt exceeds its precharged stable footprint | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct pre-effect reservation | recent failure |
| `COMPONENT_ALLOCATION_HISTORY_MISSING` | 1 | Terminal receipt has no retained allocation history | `COMPONENT_REGISTRY_STATE_INVALID`; existing identity from this slice | Fail closed and preserve the terminal receipt | recent failure |
| `COMPONENT_MEMBERSHIP_REMOVAL_ALLOCATION_HISTORY_INVALID` | 1 | Terminal receipt does not bind one unique allocation operation | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile immutable allocation history | recent failure |
| `COMPONENT_REGISTRY_BYTE_ACCOUNTING_UNDERFLOW` / `COMPONENT_REGISTRY_BYTE_COUNT_OVERFLOW` | 1 | Membership settlement merges subtract-underflow with removed-record add-overflow | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identities | Split checked subtraction and addition before mapping | recent failure |
| `COMPONENT_MEMBERSHIP_REMOVAL_RECEIPT_MISSING` | 2 | Absent live partition lacks a terminal membership-removal receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; physical absence is not logical removal authority | recent failure |
| `COMPONENT_MEMBERSHIP_REMOVAL_ALLOCATION_AUTHORITY_INVALID` | 1 | Removed allocation binding differs from reconstructed terminal partition | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both records and reconcile terminal authority | recent failure |
| `COMPONENT_REMOVED_MEMBERSHIP_REMAINS` | 1 | Removed Component retains a partition, child/traversal row or principal index | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and settle every exact index atomically | recent failure |
| `COMPONENT_MEMBERSHIP_REMOVAL_HASH_INVALID` | 1 | Retained terminal receipt hash is noncanonical | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the receipt and inspect canonical authority | recent failure |
| `COMPONENT_MEMBERSHIP_REMOVAL_ALLOCATION_HISTORY_INVALID` | 1 | Terminal receipt points to allocation history outside `Removed` | `COMPONENT_REGISTRY_STATE_INVALID`; existing identity from this slice | Fail closed and reconcile immutable allocation phase | recent failure |
| `COMPONENT_REMOVED_FINAL_INVENTORY_AUTHORITY_INVALID` / `COMPONENT_REMOVED_FINAL_INVENTORY_HASH_INVALID` / `COMPONENT_MEMBERSHIP_REMOVAL_RECEIPT_TIME_INVALID` / `COMPONENT_REMOVAL_CURSOR_NONTERMINAL` | 1 | One branch merges reconstructed inventory, hash, removal time and cursor completion | `COMPONENT_REGISTRY_STATE_INVALID`; receipt-time identity already introduced by deletion validation | Split all four protected predicates | recent failure |
| `COMPONENT_MEMBERSHIP_REMOVAL_HASH_ENCODING_FAILED` | 1 | Canonical terminal membership authority cannot be Candid encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not settle removal | recent failure |
| `COMPONENT_DRAINING_AUTHORITY_MISSING` | 1 | A `Draining` partition has no retained draining record | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile partition/draining persistence | recent failure |
| `COMPONENT_FINAL_INVENTORY_CHILD_HISTORY_INCOMPLETE` | 1 | Final inventory retains a nonterminal child allocation | `COMPONENT_REGISTRY_STATE_INVALID` | Finish or reconcile exact child lifecycle history | recent failure |
| `COMPONENT_FINAL_INVENTORY_SUBTREE_HISTORY_INCOMPLETE` | 1 | Final inventory retains a nonterminal subtree removal | `COMPONENT_REGISTRY_STATE_INVALID` | Finish or reconcile exact subtree-removal history | recent failure |
| `COMPONENT_FINAL_INVENTORY_HASH_ENCODING_FAILED` | 1 | Canonical final-inventory authority cannot be Candid encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not finalize | recent failure |
| `COMPONENT_DESCENDANT_REMOVAL_DIGEST_AUTHORITY_INVALID` | 1 | Removal digest input has invalid prior head, count, revision, child or lifecycle authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before committing descendant removal | recent failure |
| `COMPONENT_DESCENDANT_REMOVAL_DIGEST_ENCODING_FAILED` | 1 | Canonical descendant-removal digest input cannot be Candid encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not commit removal | recent failure |

The 46 rows sum to all 58 selected source sites. Expanded shared constructors
qualify 64 exact-label occurrences: 50 exact identities are new, 14 reuse
existing or earlier-in-slice identities and one unreachable private-helper
postcondition is sediment. No safe projection is added.

Final-inventory and deletion validators must no longer route every failed
predicate through one `invalid()` constructor. Snapshot identity, Fleet and
Directory coverage, ordering, hash, empty-state, index, cursor and byte-charge
failures have different operator investigations even though each safely
projects to the same public Registry-state invariant.

## Subtree Fence And Advancement Persistence Slice

This slice accounts for all 33 direct constructor references in:

- `begin_draining_subtree_removal`, `begin_subtree_removal_with_origin` and
  `advance_subtree_removal`; and
- `subtree_fence_partition` and `subtree_removal_progress_state`.

The exact baseline ranges are lines 4330–4634 and 9111–9230 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. The public
ordinary-removal adapter contains no constructor of its own. Protected record,
root, target and progress validators remain separately accounted source sites;
the storage commit adapter preserves its existing typed variants.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_DRAINING_SUBTREE_TARGET_UNAVAILABLE` | 1 | Draining advancement has no new direct-child subtree ready to fence | self | Re-read draining progress; finalize when descendants are empty | public |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 2 | Root Component Registry meta authority is absent | self; existing exact identity | Complete root Registry preparation before retry | public |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 2 | Owning Component partition is absent | self; existing exact identity | Commit or recover the exact partition before subtree work | public |
| `COMPONENT_SUBTREE_ORIGIN_LIFECYCLE_CONFLICT` | 1 | Ordinary or draining-driver origin is not admitted by current lifecycle/quiescence authority | self | Use the lifecycle's exact removal driver or complete quiescence | public |
| `COMPONENT_SUBTREE_OPERATION_CONFLICT` | 1 | Existing operation is bound to a different target or Registry fence | self; existing exact identity | Replay only the original subtree request | public |
| `COMPONENT_SUBTREE_FENCE_REGISTRY_STALE` | 1 | Requested Registry head differs from the current Component partition | self | Reload the current head and retry the exact operation | public |
| `COMPONENT_SUBTREE_REMOVAL_IN_PROGRESS` | 1 | Ordinary removal was requested while another subtree operation is nonterminal | self | Finish the retained operation before starting another | public |
| `COMPONENT_SUBTREE_TARGET_UNAVAILABLE` | 1 | Requested pre-fence target is not a registered child | self | Select an exact currently registered descendant | public |
| `COMPONENT_SUBTREE_TARGET_INACTIVE` | 1 | Requested pre-fence target is not `Active` | self | Select an Active target or finish its current lifecycle | public |
| `COMPONENT_DESCENDANT_COUNT_OVERFLOW` | 1 | Traversal bound cannot add its root step | `COMPONENT_DESCENDANT_CAPACITY_EXHAUSTED`; existing exact identity | Stop and inspect descendant accounting | recent failure |
| `COMPONENT_SUBTREE_CHILD_LIFECYCLE_INCOMPLETE` | 1 | A nonterminal child allocation lies inside the proposed subtree | self | Finish that exact child operation before fencing removal | public |
| `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | 4 | Fence or traversal progress exceeds protected Component or root bytes | self; existing exact identity | Free Registry capacity or reinstall with larger admitted limits | public |
| `COMPONENT_REGISTRY_BYTE_COUNT_OVERFLOW` | 7 | Fence/progress byte derivation overflows checked arithmetic | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and inspect bounded byte accounting | recent failure |
| `COMPONENT_DRAINING_AUTHORITY_MISSING` | 1 | Draining-driver fence has no retained Component draining record | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile partition/draining persistence | recent failure |
| `COMPONENT_SUBTREE_REMOVAL_UNPREPARED` | 1 | Advancement has no durable subtree fence | self; existing exact identity | Begin or query the exact removal operation first | public |
| `COMPONENT_SUBTREE_TRAVERSAL_EXPECTATION_AHEAD` | 1 | Caller expects traversal beyond durable progress | self | Reload status and retry from the retained step | public |
| `COMPONENT_SUBTREE_TRAVERSAL_STEP_EXHAUSTED` | 1 | Durable traversal step cannot advance without wrapping | self | Stop the operation and inspect impossible traversal history | public |
| `COMPONENT_REGISTRY_BYTE_ACCOUNTING_UNDERFLOW` | 3 | Fence delta or current traversal entries cannot be subtracted from Component/root bytes | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and reconcile the byte ledger | recent failure |
| `COMPONENT_SUBTREE_FENCE_BYTE_ACCOUNTING_NONCONVERGENT` | 1 | Fence fixed-point bytes do not stabilize | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before committing the fence | recent failure |
| `COMPONENT_SUBTREE_PROGRESS_BYTE_ACCOUNTING_NONCONVERGENT` | 1 | Traversal-progress fixed-point bytes do not stabilize | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before committing progress | recent failure |

The 20 rows sum to all 33 selected sites. Eleven exact identities are new and
nine reuse existing Registry, draining, capacity or subtree-workflow meanings.
No safe projection is added.

Pre-fence target absence is intentionally distinct from the existing masked
`COMPONENT_SUBTREE_TARGET_UNREGISTERED`: the former rejects caller-selected
input before authority is frozen, while the latter detects disappearance of a
protected nonterminal target. Their actions and retry safety differ.

## Subtree Stop And Deletion Persistence Slice

This slice accounts for all 31 direct constructor references in:

- `prepare_subtree_leaf_stop` and `mark_subtree_leaf_stopped`; and
- `prepare_subtree_leaf_delete` and `mark_subtree_leaf_deleted`.

The exact baseline range is lines 4635–5076 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. Management stop,
live observation and pool recycling remain workflow/effect owners. This slice
classifies only durable intent/result transition and exact-retry persistence;
shared record/progress validators and byte-state derivation remain separately
accounted source sites.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_SUBTREE_REMOVAL_UNPREPARED` | 4 | Stop or deletion transition has no durable subtree fence | self; existing exact identity | Begin or query the exact removal operation first | public |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 4 | Root Component Registry meta authority is absent | self; existing exact identity | Complete root Registry preparation before retry | public |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 4 | Owning Component partition is absent | self; existing exact identity | Commit or recover the exact partition before leaf effects | public |
| `COMPONENT_SUBTREE_STOP_LEAF_UNSELECTED` | 1 | Traversal has not produced a leaf for stop preparation | self | Advance the bounded traversal before retry | public |
| `COMPONENT_SUBTREE_STOP_AUTHORITY_MISSING` | 1 | Later progress retains no stop intent/authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile phase/stop persistence | recent failure |
| `COMPONENT_SUBTREE_STOP_REQUEST_CONFLICT` | 2 | Stop preparation differs from retained or selected leaf authority | self; existing exact identity | Replay only the exact selected stop request | public |
| `COMPONENT_SUBTREE_STOP_CONTROLLER_AUTHORITY_INVALID` | 1 | Stop preparation derives an anonymous root controller | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and repair protected root authority | recent failure |
| `COMPONENT_SUBTREE_STOPPED_HISTORY_CONFLICT` | 1 | Exact retry supplies a module different from completed leaf history | self | Replay only the terminal observed module evidence | public |
| `COMPONENT_SUBTREE_STOP_UNPREPARED` | 1 | Stopped evidence arrived before a durable stop intent | self; existing exact identity | Prepare the exact stop intent first | public |
| `COMPONENT_SUBTREE_STOPPED_RECEIPT_MISSING` | 1 | Later progress retains no stopped receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile phase/stopped persistence | recent failure |
| `COMPONENT_SUBTREE_STOPPED_OBSERVATION_CONFLICT` | 2 | Observed stopped leaf/module differs from durable stop or later receipt authority | self | Re-observe and replay only the exact stopped evidence | public |
| `COMPONENT_SUBTREE_DELETION_REQUEST_CONFLICT` | 5 | Deletion preparation differs from stopped or later retained leaf authority | self; existing exact identity | Replay only the exact retained deletion request | public |
| `COMPONENT_SUBTREE_DELETION_STOP_UNREADY` | 1 | Deletion preparation has no durable stopped receipt | self | Complete and retain exact stop observation first | public |
| `COMPONENT_SUBTREE_DELETION_UNPREPARED` | 1 | Deleted evidence arrived before a durable deletion intent | self; existing exact identity | Prepare the exact deletion intent first | public |
| `COMPONENT_SUBTREE_DELETED_OBSERVATION_CONFLICT` | 2 | Deleted observation differs from prepared or later retained authority | self | Replay only the exact terminal deletion evidence | public |

The 15 rows sum to all 31 selected sites. Seven exact identities are new and
eight reuse existing Registry, subtree workflow or authority meanings. No safe
projection is added.

Missing stop/deletion intent is a correctable transition error. A later phase
that cannot reconstruct the already-crossed stop/stopped authority is instead a
masked persistence contradiction; the two must not share one `unprepared`
code.

## Subtree Membership, Directory And Leaf-Finalization Slice

This slice accounts for all 31 direct constructor references in:

- `remove_subtree_leaf_membership`;
- `mark_subtree_leaf_directory_synchronized`; and
- `finalize_subtree_leaf`.

The exact baseline range is lines 5077–5587 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. It covers the
atomic membership/index settlement after physical recycling, convergence of
the affected owner and immediate-parent Directory recipients, and bounded
advancement to the next post-order leaf. The called protected validators,
canonical hashes and fixed-point byte helpers remain independently accounted
source sites.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_SUBTREE_REMOVAL_UNPREPARED` | 3 | Membership, Directory or finalization transition has no durable subtree fence | self; existing exact identity | Begin or query the exact removal operation first | public |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 3 | Root Component Registry meta authority is absent | self; existing exact identity | Complete root Registry preparation before retry | public |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 3 | Owning Component partition is absent | self; existing exact identity | Commit or recover the exact partition before leaf settlement | public |
| `COMPONENT_SUBTREE_MEMBERSHIP_REMOVAL_REQUEST_CONFLICT` | 3 | Requested leaf selection differs from deleted or later durable membership-removal authority | self | Reload status and replay only the exact retained selection | public |
| `COMPONENT_SUBTREE_DELETION_RECEIPT_MISSING` | 1 | Membership removal is requested before the exact leaf deletion receipt exists | self | Complete and retain exact deletion evidence first | public |
| `COMPONENT_CHILD_DIRECTORY_TIME_NOT_MONOTONIC` | 1 | Membership settlement does not advance the Component Directory authority time | self; existing exact identity | Supply a later observed synchronization time against the current head | public |
| `COMPONENT_SUBTREE_SELECTED_LEAF_NOT_CHILDLESS` | 1 | The selected post-order leaf still has registered children | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile traversal plus immediate-parent indexes | recent failure |
| `COMPONENT_CHILD_PARENT_ROLE_INDEX_INVALID` | 2 | Deleted membership has no positive parent-role count | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop settlement and reconcile the exact parent-role index | recent failure |
| `COMPONENT_REGISTRY_REVISION_EXHAUSTED` | 1 | Membership settlement cannot advance the Registry revision without wrapping | self; existing exact identity | Retire or reinstall the root; never wrap Registry history | public |
| `COMPONENT_SUBTREE_MEMBERSHIP_COMMITTED_DESCENDANT_COUNT_UNDERFLOW` | 1 | Component committed-descendant count is already zero | `COMPONENT_REGISTRY_STATE_INVALID` | Stop settlement and reconcile partition membership accounting | recent failure |
| `COMPONENT_SUBTREE_MEMBERSHIP_ROOT_MANAGED_DESCENDANT_COUNT_UNDERFLOW` | 1 | Root managed-descendant count is already zero | `COMPONENT_REGISTRY_STATE_INVALID` | Stop settlement and reconcile root/partition descendant accounting | recent failure |
| `COMPONENT_MEMBERSHIP_REMOVAL_ROOT_KNOWN_CREATED_COUNT_UNDERFLOW` | 1 | Root known-created Canister count is already zero | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop settlement and reconcile physical inventory accounting | recent failure |
| `COMPONENT_SUBTREE_DIRECTORY_REQUEST_CONFLICT` | 2 | Directory synchronization selection differs from membership-removed or later durable authority | self; existing exact identity | Reload status and replay only the exact retained Directory request | public |
| `COMPONENT_SUBTREE_MEMBERSHIP_REMOVAL_UNREADY` | 1 | Directory synchronization is requested before membership removal | self; existing exact identity | Complete exact membership settlement before the later phase | public |
| `COMPONENT_SUBTREE_DIRECTORY_OWNER_COVERAGE_CONFLICT` | 1 | Observed owner evidence does not cover the requested protected authority | self | Re-observe the owner and retry with covering evidence | public |
| `COMPONENT_SUBTREE_DIRECTORY_OWNER_LIFECYCLE_INVALID` | 1 | Owner evidence does not match admitted Active or terminally quiescent Draining authority | self; existing exact identity | Resume only from the admitted owner lifecycle path | public |
| `COMPONENT_SUBTREE_DIRECTORY_SYNCHRONIZATION_UNREADY` | 1 | Leaf finalization is requested before Directory synchronization | self | Complete and retain exact Directory convergence first | public |
| `COMPONENT_SUBTREE_FINALIZATION_REQUEST_CONFLICT` | 1 | Finalization selection differs from synchronized durable authority | self | Reload status and replay only the synchronized leaf selection | public |
| `COMPONENT_SUBTREE_COMPLETED_LEAF_COUNT_OVERFLOW` | 1 | Protected completed-leaf count cannot advance without wrapping | `COMPONENT_REGISTRY_STATE_INVALID` | Stop finalization and inspect bounded traversal history | recent failure |
| `COMPONENT_SUBTREE_COMPLETED_LEAF_HISTORY_EXHAUSTED` | 1 | Finalization would exceed the operation's frozen completed-leaf bound | self | Stop and inspect the admitted traversal bound before retry | public |
| `COMPONENT_SUBTREE_OPERATION_MISSING_AFTER_FINALIZATION` | 1 | The just-committed subtree operation cannot be read back | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile the atomic finalization commit | recent failure |

The 21 rows sum to all 31 selected sites. Eleven exact identities are new and
ten reuse existing Registry, Directory, accounting or subtree-workflow
meanings. No safe projection is added.

The childless check is a protected post-order invariant after the one-way
draining fence; exposing it as a normal retry conflict would misclassify
corrupt traversal/index state. Directory coverage supplied by a caller may
instead be stale and remains a correctable conflict. Completed-leaf arithmetic
overflow is likewise distinct from reaching the frozen, public history bound.

## Protected Subtree Authority And History Validators

This slice accounts for all 33 direct constructor references in:

- the subtree Directory coverage and recipient-convergence helpers;
- `validate_subtree_removal_record`, `validate_subtree_removal_root` and
  `validate_subtree_removal_progress`;
- the membership-removal, Directory-synchronized and completed-removal
  validators;
- completed-leaf construction, hashing, validation and exact-retry lookup;
- finalized-parent selection and first-child traversal; and
- bounded subtree ancestry plus traversal-record validation.

The exact baseline ranges are lines 12140–12905 and 12934–12976 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. The four
final-inventory history sites at 12906–12933 and the descendant digest helpers
after 13095 were classified in the top-level protected-validation slice and are
not counted again.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_SUBTREE_DIRECTORY_COVERAGE_CONFLICT` | 1 | Submitted Directory authority/hash does not exactly cover the current partition | self | Reload the current head and re-observe exact Directory authority | public |
| `COMPONENT_SUBTREE_DIRECTORY_EVIDENCE_CONFLICT` | 1 | Recipient evidence has a zero operation, wrong binding or incomplete activation | self | Re-observe and submit exact recipient evidence | public |
| `COMPONENT_SUBTREE_DIRECTORY_TOP_LEVEL_PARENT_EVIDENCE_CONFLICT` | 1 | A top-level owner is accompanied by duplicate parent evidence | self | Omit the impossible duplicate recipient evidence | public |
| `COMPONENT_SUBTREE_PARENT_MISSING` | 2 | A surviving immediate parent has no retained membership row | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile post-order membership history | recent failure |
| `COMPONENT_SUBTREE_PARENT_INACTIVE` | 2 | A surviving immediate parent is not Active when traversal or convergence resumes | self; existing exact identity | Restore or finish the parent's exact lifecycle before retry | public |
| `COMPONENT_SUBTREE_PARENT_DIRECTORY_EVIDENCE_MISSING` | 1 | A surviving parent has no submitted Directory convergence evidence | self | Re-observe the parent and retry with exact evidence | public |
| `COMPONENT_SUBTREE_DIRECTORY_PARENT_COVERAGE_CONFLICT` | 1 | Parent and owner recipients cover different Directory authority | self | Re-observe both recipients against the same protected head | public |
| `COMPONENT_SUBTREE_RECORD_OPERATION_ID_INVALID` / `COMPONENT_SUBTREE_RECORD_TARGET_AUTHORITY_INVALID` / `COMPONENT_SUBTREE_RECORD_REGISTRY_FENCE_INVALID` / `COMPONENT_SUBTREE_RECORD_FENCED_PROGRESS_INVALID` / `COMPONENT_SUBTREE_RECORD_TRAVERSAL_PROGRESS_INVALID` / `COMPONENT_SUBTREE_RECORD_LEAF_SELECTION_PROGRESS_INVALID` / `COMPONENT_SUBTREE_RECORD_STOP_PROGRESS_INVALID` / `COMPONENT_SUBTREE_RECORD_DELETION_PROGRESS_INVALID` / `COMPONENT_SUBTREE_RECORD_MEMBERSHIP_PROGRESS_INVALID` / `COMPONENT_SUBTREE_RECORD_DIRECTORY_PROGRESS_INVALID` / `COMPONENT_SUBTREE_RECORD_COMPLETED_PROGRESS_INVALID` / `COMPONENT_SUBTREE_RECORD_COMPLETION_COUNT_INVALID` | 1 | One shared constructor hides operation, target, Registry-fence, every phase-specific progress and completed-count predicates | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Split the phase predicates and preserve the malformed record for investigation | recent failure |
| `COMPONENT_SUBTREE_STOP_CONTROLLER_AUTHORITY_INVALID` | 1 | Retained stop authority differs from the protected Fleet Subnet Root | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile stop/root authority | recent failure |
| `COMPONENT_SUBTREE_TARGET_UNREGISTERED` | 2 | A protected nonterminal target disappeared from membership | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed; absence is not terminal recycling evidence | recent failure |
| `COMPONENT_SUBTREE_FENCE_AUTHORITY_INVALID` | 2 | Current target row differs from the immutable subtree fence | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve the fence and reconcile protected target authority | recent failure |
| `COMPONENT_SUBTREE_CURSOR_UNREGISTERED` | 1 | A protected nonterminal cursor disappeared from membership | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile traversal/index persistence | recent failure |
| `COMPONENT_SUBTREE_CURSOR_AUTHORITY_INVALID` | 1 | Current cursor row differs from durable progress | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both records and reconcile exact cursor authority | recent failure |
| `COMPONENT_DESCENDANT_COUNT_OVERFLOW` | 1 | Traversal bound cannot add its root step | `COMPONENT_DESCENDANT_CAPACITY_EXHAUSTED`; existing exact identity | Stop and inspect descendant accounting | recent failure |
| `COMPONENT_SUBTREE_CURSOR_OUTSIDE_FENCE` | 1 | Durable cursor ancestry escapes the fenced target subtree | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile parentage plus cursor history | recent failure |
| `COMPONENT_SUBTREE_SELECTED_LEAF_NOT_CHILDLESS` | 1 | Protected post-order selection still has a registered child | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile traversal plus immediate-parent indexes | recent failure |
| `COMPONENT_SUBTREE_MEMBERSHIP_COMMITTED_DESCENDANT_COUNT_UNDERFLOW` | 1 | Historical membership receipt has no previous descendant to remove | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop settlement and reconcile partition membership accounting | recent failure |
| `COMPONENT_SUBTREE_MEMBERSHIP_RECEIPT_AUTHORITY_INVALID` / `COMPONENT_SUBTREE_MEMBERSHIP_REGISTRY_COVERAGE_INVALID` / `COMPONENT_SUBTREE_MEMBERSHIP_INDEX_SETTLEMENT_INVALID` / `COMPONENT_SUBTREE_SELECTED_LEAF_NOT_CHILDLESS` | 1 | One shared constructor merges canonical receipt, current-or-later head, removed-index and post-order-child predicates | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Split receipt authority, head coverage, index settlement and childless proof | recent failure |
| `COMPONENT_SUBTREE_DIRECTORY_RECEIPT_OWNER_INVALID` / `COMPONENT_SUBTREE_DIRECTORY_RECEIPT_REGISTRY_COVERAGE_INVALID` / `COMPONENT_SUBTREE_DIRECTORY_RECEIPT_MEMBERSHIP_COVERAGE_INVALID` / `COMPONENT_SUBTREE_DIRECTORY_RECEIPT_PARENT_INVALID` | 1 | One shared constructor merges surviving owner, current Registry, removed membership and immediate-parent authority | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Split every recipient and coverage predicate before mapping | recent failure |
| `COMPONENT_SUBTREE_COMPLETED_HISTORY_MISSING` | 1 | Completed progress has no terminal completed-leaf history | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile operation/history persistence | recent failure |
| `COMPONENT_SUBTREE_COMPLETION_AUTHORITY_INVALID` / `COMPONENT_SUBTREE_COMPLETION_TARGET_MEMBERSHIP_RETAINED` | 1 | One constructor merges terminal receipt authority with a target row that survived removal | `COMPONENT_REGISTRY_STATE_INVALID` for both exact leaves | Split immutable completion evidence from membership settlement | recent failure |
| `COMPONENT_SUBTREE_COMPLETED_LEAF_RECEIPT_ENCODING_FAILED` | 1 | Canonical synchronized receipt cannot be encoded for completed-leaf hashing | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as an implementation/state failure; do not finalize | recent failure |
| `COMPONENT_SUBTREE_COMPLETED_LEAF_OPERATION_AUTHORITY_INVALID` / `COMPONENT_SUBTREE_COMPLETED_LEAF_TRAVERSAL_INVALID` / `COMPONENT_SUBTREE_COMPLETED_LEAF_IDENTITY_INVALID` / `COMPONENT_SUBTREE_COMPLETED_LEAF_MODULE_INVALID` / `COMPONENT_SUBTREE_COMPLETED_LEAF_REGISTRY_AUTHORITY_INVALID` / `COMPONENT_SUBTREE_COMPLETED_LEAF_DIRECTORY_AUTHORITY_INVALID` / `COMPONENT_SUBTREE_COMPLETED_LEAF_RECEIPT_HASH_INVALID` | 1 | One shared constructor hides operation, traversal, leaf, module, Registry, Directory and receipt-hash history predicates | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Split each immutable completed-history authority predicate | recent failure |
| `COMPONENT_SUBTREE_FINALIZATION_REQUEST_CONFLICT` | 1 | Exact retry leaf selection differs from completed history | self; existing exact identity | Replay only the terminal retained selection | public |
| `COMPONENT_DIRECTORY_TRAVERSAL_CHILD_MISSING` | 1 | Traversal index references no normalized child row | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile traversal/child indexes | recent failure |
| `COMPONENT_DIRECTORY_TRAVERSAL_AUTHORITY_INVALID` | 2 | Traversal identity disagrees with its Component or normalized child row | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile exact tree/index authority | recent failure |
| `COMPONENT_SUBTREE_ANCESTRY_CHILD_MISSING` | 1 | Bounded ancestry walk reaches an unregistered intermediate child | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile immutable immediate-parent membership | recent failure |
| `COMPONENT_SUBTREE_ANCESTRY_BOUND_EXCEEDED` | 1 | Ancestry does not reach target or root within committed-descendant bounds | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect cyclic or corrupt parentage | recent failure |

The 28 rows sum to all 33 selected source sites. Their candidate columns name
52 exact-label occurrences: 40 new identities and twelve deliberate uses of
eleven existing identities. `COMPONENT_SUBTREE_SELECTED_LEAF_NOT_CHILDLESS`
appears in two independently evaluated source predicates. No safe projection
is added.

Five shared constructors currently hide 29 exact protected predicates. B4 must
replace those boolean aggregates with named phase/authority validation so no
single compact code continues to obscure the broken record, receipt or history
edge. This is not diagnostic fragmentation: each leaf identifies a different
immutable authority that an operator must reconcile.

## Top-Level Commitment And Activation Persistence Slice

This slice accounts for all 61 direct constructor references in:

- `commit_verified`, ordinary and grouped Directory preparation,
  `mark_runtime_activated`, ordinary and grouped membership activation and
  `mark_membership_synchronized`;
- prepared and active record construction plus fixed-point byte accounting;
- exact prepared/active partition reconstruction and validation; and
- committed and active Directory-authority hash validation.

The exact baseline ranges are lines 6583–7111, 10638–10913, 11089–11238 and
11301–11353 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. The workflow
boundary already classifies response-shape and orchestration failures; this
slice owns the durable transition, reconstruction and atomic-commit facts.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 6 | Root Component Registry meta authority is absent | self; existing exact identity | Complete Registry preparation before retry | public |
| `COMPONENT_ALLOCATION_OPERATION_UNRESERVED` | 6 | Exact top-level allocation record is absent | self; existing exact identity | Reserve or query the exact operation first | public |
| `COMPONENT_DIRECTORY_TIME_INVALID` | 1 | Initial committed Directory synchronization time is zero | self | Supply a positive observed synchronization time | public |
| `COMPONENT_ALLOCATION_COMMITMENT_TRANSITION_INVALID` | 1 | Allocation has not reached verified installation before commitment | self; existing exact identity | Complete exact verification before commitment | public |
| `COMPONENT_ALLOCATION_BYTE_CHARGE_EXCEEDED` | 2 | Prepared or active partition exceeds its frozen pre-install charge | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed; the pre-effect capacity proof was insufficient | recent failure |
| `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | 3 | Prepared/active partition exceeds protected Component or root Registry capacity | self; existing exact identity | Free capacity or reinstall with an admitted larger bound | public |
| `COMPONENT_REGISTRY_BYTE_ACCOUNTING_UNDERFLOW` | 2 | Root cannot replace the exact prepared/active partition byte charge | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and reconcile the byte ledger | recent failure |
| `COMPONENT_COMMITMENT_ROOT_BYTE_RESERVATION_INVALID` | 1 | Pre-install reservation would exceed the protected root limit at commitment | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct pre-effect root capacity derivation | recent failure |
| `COMPONENT_COMMITMENT_RESERVED_COUNT_UNDERFLOW` | 1 | Root has no reserved Component count to consume | `COMPONENT_REGISTRY_STATE_INVALID` | Stop commitment and reconcile allocation accounting | recent failure |
| `COMPONENT_COMMITTED_COUNT_OVERFLOW` | 1 | Root committed-Component count cannot advance | `COMPONENT_ALLOCATION_CAPACITY_EXHAUSTED` | Stop commitment and inspect root capacity accounting | recent failure |
| `COMPONENT_DIRECTORY_PREPARATION_TRANSITION_INVALID` | 2 | Ordinary or grouped allocation is not committed for Directory preparation | self; existing exact identity | Commit the verified Component first | public |
| `COMPONENT_DIRECTORY_PREPARATION_AUTHORITY_STALE` | 1 | Expected Directory hash differs from immutable commitment | self | Reload the commitment and replay its exact hash | public |
| `COMPONENT_RECEIPT_BYTE_FOOTPRINT_CHANGED` | 3 | Directory, runtime or membership acknowledgement changes its precharged footprint | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct the bounded record model | recent failure |
| `COMPONENT_GROUP_DIRECTORY_AUTHORITY_INVALID` | 1 | Proposed grouped Directory authority hash is zero | self | Supply the exact nonzero group Directory authority | public |
| `COMPONENT_GROUP_DIRECTORY_ORIGIN_CONFLICT` | 1 | Group Directory transition targets a nongrouped allocation | self | Use the operation's exact provisioning mode | public |
| `COMPONENT_GROUP_DIRECTORY_PREVIOUS_AUTHORITY_INVALID` | 1 | Previous group authority is zero or equals the proposed authority | self | Supply the distinct retained prior authority | public |
| `COMPONENT_GROUP_DIRECTORY_AUTHORITY_CONFLICT` | 1 | Retained unpublished commitment differs from the requested group transition | self | Reload and replay only the exact retained transition | public |
| `COMPONENT_RUNTIME_ACTIVATION_TRANSITION_INVALID` | 1 | Allocation is not committed for runtime activation | self | Complete Registry commitment first | public |
| `COMPONENT_RUNTIME_DIRECTORY_AUTHORITY_UNREADY` | 1 | Runtime activation lacks the exact prepared Directory receipt/hash | self; existing exact identity | Finish exact Directory preparation before retry | public |
| `COMPONENT_MEMBERSHIP_ACTIVATION_TRANSITION_INVALID` | 2 | Membership activation is uncommitted or lacks terminal Directory/runtime receipts | self; existing exact identity | Complete the exact preceding transitions first | public |
| `COMPONENT_MEMBERSHIP_ACTIVATION_MODE_CONFLICT` | 1 | Grouped/ordinary activation mode differs from provisioning origin | self | Invoke the exact activation mode frozen by allocation | public |
| `COMPONENT_DIRECTORY_TIME_NOT_MONOTONIC` | 1 | Active Directory time does not advance prepared authority | self | Supply a later observation against the committed head | public |
| `COMPONENT_MEMBERSHIP_SYNCHRONIZATION_TRANSITION_INVALID` | 2 | Allocation is uncommitted or membership has not been activated | self | Complete membership activation before synchronization | public |
| `COMPONENT_MEMBERSHIP_DIRECTORY_AUTHORITY_STALE` | 1 | Synchronization hash differs from active membership authority | self | Reload active membership and replay its exact hash | public |
| `COMPONENT_REGISTRY_BYTE_COUNT_OVERFLOW` | 2 | Prepared/active record or index byte totals overflow checked arithmetic | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and inspect bounded byte accounting | recent failure |
| `COMPONENT_COMMITMENT_RECORD_INVALID` | 2 | Prepared-record construction/reconstruction unexpectedly leaves committed phase | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the record and inspect the commitment transition | recent failure |
| `COMPONENT_COMMITMENT_BYTE_ACCOUNTING_NONCONVERGENT` | 1 | Prepared partition fixed-point bytes do not stabilize | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct the encoded-byte model | recent failure |
| `COMPONENT_MEMBERSHIP_ACTIVATION_RECORD_INVALID` | 2 | Active-record construction unexpectedly lacks or leaves committed authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the record and inspect activation construction | recent failure |
| `COMPONENT_REGISTRY_REVISION_EXHAUSTED` | 2 | Active membership revision cannot advance without wrapping | self; existing exact identity | Retire or reinstall the root; never wrap Registry history | public |
| `COMPONENT_MEMBERSHIP_RECORD_INVALID` | 1 | Active record loses the membership receipt during byte convergence | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and inspect durable membership state | recent failure |
| `COMPONENT_MEMBERSHIP_BYTE_ACCOUNTING_NONCONVERGENT` | 1 | Active partition fixed-point bytes do not stabilize | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct the encoded-byte model | recent failure |
| `COMPONENT_REGISTRY_PARTITION_MISSING` | 2 | Committed or active allocation has no Registry partition | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile allocation/partition persistence | recent failure |
| `COMPONENT_COMMITMENT_RECEIPT_MISMATCH` | 2 | Prepared partition identity/current state differs from immutable commitment | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve both records and reconcile exact commitment authority | recent failure |
| `COMPONENT_MEMBERSHIP_ACTIVATION_EVIDENCE_INVALID` / `COMPONENT_MEMBERSHIP_PARTITION_IDENTITY_INVALID` / `COMPONENT_MEMBERSHIP_PARTITION_PROGRESSION_INVALID` | 1 | One shared constructor merges immutable activation receipt, current Component identity and monotonic partition progression | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Split receipt, identity and progression predicates before mapping | recent failure |
| `COMPONENT_MEMBERSHIP_RECEIPT_MISMATCH` | 1 | Active Directory hash differs from immutable membership receipt | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve the receipt and reconcile active Directory evidence | recent failure |
| `COMPONENT_COMMITMENT_DIRECTORY_HASH_INVALID` | 1 | Prepared Directory hash differs from current committed partition authority | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve the receipt and reconcile Registry/Directory evidence | recent failure |

The 36 rows sum to all 61 selected source sites. Their candidate columns name
38 exact identities: 22 are new and sixteen reuse existing allocation,
capacity, workflow or Registry meanings. No safe projection is added.

The active-partition validator must split receipt validity, current identity
and monotonic progression. Those facts have different corruption journeys even
though all remain masked Registry-state failures. Public stale hashes and
out-of-order transitions remain independently correctable and must not inherit
that protected-state projection.

## Fleet-Service Directory Refresh Persistence Slice

This slice accounts for all 21 direct constructor references in
`root_component_canisters`, `directory_synchronization_targets`,
`prepare_directory_refresh`, `commit_directory_refresh` and
`directory_refresh_plan_for_intent` at lines 1169–1469 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`. It owns target
selection and the pre-runtime partition commit; workflow observation and the
Fleet-service publication barrier remain separately classified.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_TOP_LEVEL_PRINCIPAL_DUPLICATE` | 1 | Two partitions bind the same top-level Canister principal | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile partition/principal authority | recent failure |
| `COMPONENT_DIRECTORY_REFRESH_TARGETS_NONCANONICAL` | 1 | Requested Component identities are duplicate or out of canonical order | self | Sort, deduplicate and retry the exact target set | public |
| `COMPONENT_ALLOCATION_HISTORY_DUPLICATE` | 1 | Selected Component has duplicate top-level allocation history | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed; never choose one record as authority | recent failure |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 4 | Selected or journalled Component has no Registry partition | self; existing exact identity | Recover the exact committed partition before refresh | public |
| `COMPONENT_DIRECTORY_REFRESH_TARGET_INACTIVE` | 1 | Initially selected service Component is not Active | self | Select an Active target or finish its lifecycle first | public |
| `COMPONENT_ALLOCATION_HISTORY_MISSING` | 2 | Selected or journalled Component has no retained allocation | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile immutable allocation history | recent failure |
| `COMPONENT_ALLOCATION_HISTORY_AUTHORITY_INVALID` | 1 | Selected allocation differs from current partition authority | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve both records and reconcile protected authority | recent failure |
| `COMPONENT_DIRECTORY_REFRESH_TARGET_UNREADY` | 1 | Selected allocation lacks terminal runtime and Directory membership evidence | self | Finish activation and synchronization before refresh | public |
| `COMPONENT_DIRECTORY_REFRESH_TARGET_AUTHORITY_STALE` | 1 | Captured lifecycle, Canister or baseline Registry authority changed before planning | self | Reload the exact target and rebuild the intent | public |
| `COMPONENT_DIRECTORY_TIME_NOT_MONOTONIC` | 1 | Refresh time does not advance current Directory authority | self; existing exact identity | Supply a later observation against the current head | public |
| `COMPONENT_REGISTRY_REVISION_EXHAUSTED` | 1 | Refresh cannot advance the Registry revision without wrapping | self; existing exact identity | Retire or reinstall the root; never wrap history | public |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 1 | Root Component Registry meta authority is absent before commit | self; existing exact identity | Complete Registry preparation before retry | public |
| `COMPONENT_DIRECTORY_REFRESH_PARTITION_CONFLICT` | 1 | Partition changed after exact Directory intent persistence | self | Reconstruct status and preserve the original intent | public |
| `COMPONENT_DIRECTORY_REFRESH_ALLOCATION_CONFLICT` | 1 | Retained allocation belongs to another Component than the intent | self | Reject substitution and replay only the exact intent | public |
| `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | 2 | Refresh exceeds protected Component or root Registry capacity | self; existing exact identity | Free capacity or reinstall with an admitted larger bound | public |
| `COMPONENT_DIRECTORY_REFRESH_INTENT_CONFLICT` | 1 | Reconstructed plan or committed partition differs from durable refresh intent | self | Preserve the intent and retry only its exact authority | public |

The 16 rows sum to all 21 selected source sites. Eight exact identities are new
and eight reuse existing allocation, Registry, Directory or capacity meanings.
No safe projection is added.

## Remaining Registry, Accounting And Hash Adapters

This final ops slice accounts for all 42 direct constructor references not
owned by an earlier bounded slice. The exact baseline ranges are lines
2929–3028, 8159–8175, 8975–9050, 9459–9655, 10080–10196, 12032–12078,
12092–12139, 13023–13062 and 13096–13189 of
`crates/canic-control-plane/src/ops/component_registry/mod.rs`.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_PREPARATION_CONFLICT` | 1 | Registry is already prepared under different root/release authority | self | Reinstall or replay only the exact preparation | public |
| `COMPONENT_ALLOCATION_OPERATION_UNRESERVED` | 1 | Active-partition lookup has no retained allocation | self; existing exact identity | Reserve or query the exact operation first | public |
| `COMPONENT_ALLOCATION_COMMITMENT_TRANSITION_INVALID` | 1 | Active-partition lookup is requested before commitment | self; existing exact identity | Complete verified commitment first | public |
| `COMPONENT_MEMBERSHIP_ACTIVATION_TRANSITION_INVALID` | 1 | Active-partition lookup has no membership receipt | self; existing exact identity | Complete membership activation first | public |
| `COMPONENT_SPEC_RESERVED_COUNT_OVERFLOW` | 1 | Per-Spec reserved count cannot fit the bounded response | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile the Spec allocation index | recent failure |
| `COMPONENT_SPEC_COMMITTED_COUNT_OVERFLOW` | 1 | Per-Spec committed count cannot fit the bounded response | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile the Spec allocation index | recent failure |
| `COMPONENT_PEER_RESERVED_COUNT_OVERFLOW` | 1 | Per-requester peer reservation count cannot fit its bounded response | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile the peer allocation index | recent failure |
| `COMPONENT_PEER_COMMITTED_COUNT_OVERFLOW` | 1 | Per-requester peer committed count cannot fit its bounded response | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile the peer allocation index | recent failure |
| `ROOT_DRAINING_ALLOCATION_ADMISSION_CLOSED` | 1 | Root draining fence rejects a new top-level allocation | self | Select an open root; draining never reopens | public |
| `COMPONENT_REGISTRY_BYTE_ACCOUNTING_UNDERFLOW` / `COMPONENT_REGISTRY_BYTE_COUNT_OVERFLOW` | 2 | Shared byte replacement merges subtract-underflow with add-overflow | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identities | Split the checked arithmetic before mapping | recent failure |
| `COMPONENT_DIRECTORY_REFRESH_INTENT_CONFLICT` | 1 | Reconstructed refresh plan differs from durable intent | self; existing exact identity | Preserve and replay only the exact intent | public |
| `COMPONENT_REGISTRY_BYTE_COUNT_OVERFLOW` | 9 | Subtree finalization/removal or install charging overflows checked bytes | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and inspect bounded byte accounting | recent failure |
| `COMPONENT_REGISTRY_BYTE_ACCOUNTING_UNDERFLOW` | 4 | Subtree finalization/removal cannot subtract current authority bytes | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and reconcile the byte ledger | recent failure |
| `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | 5 | Subtree finalization/removal or install charge exceeds protected capacity | self; existing exact identity | Free capacity or reinstall with a larger admitted bound | public |
| `COMPONENT_SUBTREE_FINALIZATION_BYTE_ACCOUNTING_NONCONVERGENT` | 1 | Completed-leaf history fixed-point bytes do not stabilize | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct the encoded-byte model | recent failure |
| `COMPONENT_SUBTREE_MEMBERSHIP_RECEIPT_MISSING_DURING_BYTE_CONVERGENCE` | 1 | Membership-removal byte convergence loses its receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve state and inspect phase/receipt construction | recent failure |
| `COMPONENT_SUBTREE_MEMBERSHIP_BYTE_ACCOUNTING_NONCONVERGENT` | 1 | Membership-removal fixed-point bytes do not stabilize | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct the encoded-byte model | recent failure |
| `COMPONENT_ALLOCATION_INSTALL_TRANSITION_INVALID` | 1 | Install-charge derivation runs before creation | self; existing exact identity | Complete exact creation first | public |
| `COMPONENT_REGISTRY_PARTITION_PRINCIPAL_INDEX_INVALID` | 1 | Partition binding differs from its top-level principal index | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile normalized partition indexes | recent failure |
| `COMPONENT_REGISTRY_PARTITION_REVISION_INVALID` / `COMPONENT_REGISTRY_PARTITION_DIRECTORY_TIME_INVALID` / `COMPONENT_REGISTRY_PARTITION_DESCENDANT_HASH_INVALID` / `COMPONENT_REGISTRY_PARTITION_CONTENT_HASH_INVALID` | 1 | One shared constructor merges version, Directory time, descendant digest/count and canonical content-hash predicates | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Split each protected partition-head predicate | recent failure |
| `COMPONENT_CHILD_PRINCIPAL_INDEX_INVALID` | 1 | Registered child differs from principal or traversal indexes | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile normalized child indexes | recent failure |
| `COMPONENT_REGISTRY_PARTITION_HASH_ENCODING_FAILED` | 1 | Canonical partition authority cannot be Candid encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not commit | recent failure |
| `COMPONENT_DESCENDANT_COMMIT_DIGEST_AUTHORITY_INVALID` | 1 | Commit digest has an empty prior hash or non-Prepared child | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before committing descendant membership | recent failure |
| `COMPONENT_DESCENDANT_COMMIT_DIGEST_ENCODING_FAILED` | 1 | Canonical descendant-commit authority cannot be encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not commit | recent failure |
| `COMPONENT_DESCENDANT_ACTIVATION_DIGEST_AUTHORITY_INVALID` | 1 | Activation digest has invalid prior/revision/Active-child authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before activating descendant membership | recent failure |
| `COMPONENT_DESCENDANT_ACTIVATION_DIGEST_ENCODING_FAILED` | 1 | Canonical descendant-activation authority cannot be encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as implementation/state failure; do not activate | recent failure |

The 26 rows sum to all 42 selected source sites. Their candidate columns name
30 exact-label occurrences representing 28 identities: nineteen are new and
nine reuse existing Registry, transition, Directory or child-index meanings.
The two shared byte identities each occur in both the split adapter and a
bounded owner row. No safe projection is added.

These two slices close every direct constructor in
`ops/component_registry/mod.rs`. The completion claim remains conditional on a
fresh whole-file rescan and the mechanical source-range manifest; the prose
tables themselves are not proof.

## Grouping Decisions

- Repeated “authority not prepared” and “operation not reserved” sites share
  one identity because owner, action and retry are identical across each
  transition.
- Missing creation intent and invalid creation phase share one transition code;
  they do not imply that creation was or was not performed.
- The five install-phase failures share one transition code. The separate
  durable intent mismatch remains an invariant because it proves contradictory
  effect authority rather than an out-of-order request.
- Arithmetic overflow, underflow and insufficient precharge remain distinct
  exact internal facts, even though all project to the same safe Registry-state
  identity.
- Protected capacity exhaustion is public and distinct from arithmetic
  overflow. Capacity may be relieved; overflow means accounting is invalid.
- The commit adapter is transparent where it reproduces an already mapped
  operation, parent or unprepared-authority meaning. It does not receive one
  aggregate “commit failed” code.
- Descendant arithmetic overflow remains an exact state fact even though it
  safely projects to descendant capacity; a caller must not infer that the
  counter is valid from the public capacity code.
- Component and root Registry byte ceilings share one identity because the
  blocked action and recovery are the same. Overflow, reduction and failure to
  converge remain separate exact state facts.
- Creation and install authority mismatches remain distinct because their
  protected fields, effect boundary and operator investigation differ.
- Missing pre-commit state remains `Unavailable`, while a partition or row
  missing after an immutable receipt is a masked Registry invariant.
- Each public activation edge has its own transition identity. Corrupt durable
  phase/receipt state is not exposed as an operator-retryable transition.
- A supplied stale authority hash is public and correctable; recomputing a
  different hash from protected durable evidence is a masked state failure.
- A root that still has live work is retryably unready. A root whose claimed
  terminal history, hash or byte ledger contradicts protected state is not
  merely unready and projects to the Registry-state invariant.
- Logical Coordinator removal publication remains separate from physical
  Store/root deletion. Recording its exact response authorizes no later effect
  by itself.
- Reclamation and publication-binding finalization keep separate durable
  intents, live evidence and terminal receipts. Neither successful effect
  response is commitment, and neither later boundary absorbs the earlier code.
- Store cycle reclamation, typed Store absence, root cycle reclamation and
  Coordinator readiness remain four separate boundaries. Missing evidence is
  retryable; contradictory or lost committed evidence is not merged with it.
- Initial-inventory incompleteness is retryable before the one-way seal.
  Counter, sequence, partition, byte or retained-receipt contradictions are
  protected-state failures and remain distinct from that readiness class.
- Directory paging input reuses the public workflow cursor and page-limit
  identities. Normalized traversal, child and principal-index contradictions
  remain separately actionable protected-state facts.
- A stale or premature draining request remains caller-correctable. Missing
  fences, cursor targets or normalized descendant roots after durable state are
  protected contradictions and never become ordinary lifecycle conflicts.
- Final-inventory and deletion validation may share one public projection but
  not one exact internal identity. Every protected predicate retains its own
  numeric observation and host action.
- Subtree origin, target and Registry-head rejection before fence commitment
  remain public and correctable. The same facts becoming contradictory after a
  durable fence are protected-state failures owned by the validator slice.
- Stop and deletion retries preserve the original leaf/effect authority.
  Missing pre-effect intent remains public; missing evidence after a later
  durable phase projects to Registry-state invalidity.

## Required Tests

- one exhaustive mapping over all nine storage commit variants;
- exact retry for identical operation intent and rejection of a conflicting
  payload;
- creation and installation transition matrices including terminal replay;
- allocation sequence exhaustion and stale-sequence separation;
- byte capacity versus overflow, underflow and insufficient-charge separation;
- principal/identity conflict without substituting another Canister;
- every masked storage contradiction recorded numerically before projection;
- a constructor-site manifest proving all 55 selected baseline sites remain
  accounted for after source movement;
- direct-child retry versus conflicting intent, including a removed or
  concurrently draining parent;
- checked parent, descendant and root count overflow versus configured
  capacity exhaustion;
- fixed-point byte convergence, reduction, overflow and insufficient precharge
  as independent adversarial cases;
- creation/install authority substitution for controller, Store artifact,
  release, binding and raw module; and
- a constructor-site manifest proving all 83 selected direct-child sites
  remain accounted for after source movement;
- commitment, Directory preparation, runtime activation, membership activation
  and synchronization transition matrices with exact terminal replay;
- post-receipt partition/row disappearance and protected identity mismatch;
- stale supplied authority versus contradictory reconstructed authority;
- terminal-byte precharge, limit, underflow and convergence adversarial cases;
  and
- a constructor-site manifest proving all 73 selected commitment/activation
  sites remain accounted for after source movement;
- root draining request/retry, local-fence and later-publication separation;
- final-inventory rejection for every nonterminal counter, membership index,
  allocation-history and byte-ledger contradiction;
- exact Store catalog/GC observation versus retained final inventory; and
- exact logical-removal publication retry, conflicting response and lost
  local-commit receipt behavior, plus a 73-site manifest;
- Store reclamation intent/effect/commit interruption, exact empty-GC lineage
  and conflicting retry evidence; and
- publication-binding `source + 3` generation overflow, incomplete effect,
  exact replay and lost terminal receipt, plus a 45-site manifest;
- Store-deletion intent, cycle evidence, typed absence and terminal receipt
  interruption/replay with controller/module/binding substitution; and
- root-deletion preparation, Coordinator intent, cycle evidence and readiness
  receipt interruption/replay, plus a 61-site manifest;
- final-inventory intent/commit exact replay and contradictory receipt; and
- initial-inventory counter, partition, sequence, byte and activation-receipt
  adversarial cases, plus a 35-site manifest; and
- zero scan bounds, cross-filter cursors, missing traversal/child rows,
  inconsistent immediate-parent indexes and missing subtree root/partition
  authority, plus a 15-site manifest; and
- draining-fence, quiescence, final-inventory, deletion and membership-removal
  transition matrices, including every compound-predicate split and a 50-site
  manifest; and
- draining/quiescence byte fixed points, final-inventory and deletion predicate
  expansion, allocation-history uniqueness, atomic counter/index settlement,
  canonical receipt hashes and a 58-site protected-validator manifest; and
- ordinary/draining-driver fence admission, exact retry, stale/ahead traversal,
  checked capacity and byte fixed points, plus a 33-site manifest; and
- leaf stop/deletion intent and result transition matrices, including terminal
  replay, conflicting observed modules/targets and lost intermediate authority,
  plus a 31-site manifest; and
- membership/index settlement, immediate-parent/owner Directory convergence,
  completed-leaf finalization and exact replay, plus a 31-site manifest; and
- every protected subtree phase, receipt, history, ancestry and traversal
  predicate split, plus a 33-site manifest; and
- top-level commitment, grouped/ordinary Directory preparation, runtime and
  membership activation/synchronization, immutable partition reconstruction
  and byte convergence, plus a 61-site manifest; and
- Fleet-service refresh target selection, intent persistence, pre-runtime
  partition commit and reconstruction, plus a 21-site manifest; and
- Registry preparation/read adapters, count projections, draining admission,
  shared byte arithmetic, subtree byte convergence, partition validation and
  descendant digest construction, plus a 42-site manifest.

## Next Component Registry Slice

Continue the whole-program frontier with Component provisioning and Fleet
Coordinator ops/workflows. Preserve an
existing meaning where owner, action and retry are identical and split caller
staleness from protected-state contradiction.
