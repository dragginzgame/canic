# Canic 0.102 Fleet Subnet Root Workflow Constructor Leaves

Date: 2026-08-15

## Status

This B1 ledger classifies direct `InternalError::*` references in
`crates/canic-control-plane/src/workflow/fleet_subnet_root.rs`. It allocates no
number and changes no runtime behavior. Typed inner calls remain transparent;
the workflow receives an identity only for an independently actionable local
authority, interruption or response-validation decision.

The baseline file contains 69 production references. The four slices below
classify all 69 sites.

## Sibling Store Adoption Boundary

This slice accounts for all seven direct constructors in
`wasm_store_adoption_status`, `protected_sibling_wasm_store_authority`,
`observe_sibling_wasm_store`,
`require_sibling_wasm_store_controller_phase` and
`require_final_sibling_wasm_store_controllers`, the exact baseline range at
lines 103–249. The `adopt_wasm_store` orchestration itself preserves typed
storage and management-call failures and contains no direct constructor.

| Exact candidate | Producer function/branch | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_STORE_ADOPTION_UNAVAILABLE` | `wasm_store_adoption_status` / terminal receipt absent | Status is requested before the terminal sibling Store adoption receipt | self | Resume/query the exact adoption operation | public |
| `ROOT_STORE_ADOPTION_OPERATION_ID_INVALID` | `protected_sibling_wasm_store_authority` / zero operation ID | Adoption operation ID is zero | self | Supply the nonzero protected install operation | public |
| `ROOT_STORE_ADOPTION_OPERATION_CONFLICT` | `protected_sibling_wasm_store_authority` / operation differs from activation identity | Adoption operation differs from protected root-install identity | self | Replay only the exact install operation | public |
| `ROOT_STORE_ADOPTION_AUTHORITY_CONFLICT` | `protected_sibling_wasm_store_authority` / supplied authority differs from root authority | Caller-supplied Store authority differs from protected root authority | self | Use the exact installed sibling Store binding | public |
| `ROOT_STORE_ADOPTION_LIVE_MODULE_MISMATCH` | `observe_sibling_wasm_store` / running-module predicate | Store is not Running or its module differs from protected infrastructure artifact | self | Restore/reinstall exact Store module before adoption | public |
| `ROOT_STORE_ADOPTION_CONTROLLER_PHASE_INVALID` | `require_sibling_wasm_store_controller_phase` / neither admitted controller set | Live controllers are neither the frozen installer-plus-root set nor final root-only set | self | Restore one exact admitted controller phase | public |
| `ROOT_STORE_ADOPTION_FINAL_CONTROLLERS_UNCONVERGED` | `require_final_sibling_wasm_store_controllers` / controllers differ from final set | Post-update observation has not converged to sole root control | self | Re-observe/retry the exact update; do not commit adoption | public |

The seven rows sum to all seven selected sites and add seven exact identities.
No projection is added. Management status/update transport failures and the
storage intent/receipt identities remain transparent.

## Draining, Final Inventory And Logical Removal Boundary

This slice accounts for all 36 direct constructors in:

- `begin_draining`, `exact_draining_retry`,
  `fetch_root_draining_reservation`, `validate_root_draining_reservation`,
  `finalize_inventory`, `publish_removal` and `removal_status`;
- `canister_summary`, `validated_root_state`, `validate_protected_root`,
  `validate_component_registry`, `validate_draining_evidence`,
  `ensure_root_is_published_draining`, `removed_root_inventory`,
  `verify_store_before_reclamation` and `summary`; and
- `publish_removed_to_coordinator`,
  `validate_removal_publication_response` and
  `removal_publication_response`.

The exact baseline ranges are lines 250–502 and 828–1145. Read-only status
wrappers and response converters without constructors remain transparent.

| Exact candidate or disposition | Producer function/branch | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_DRAINING_REQUIRES_ACTIVE` | `begin_draining` / root status is not `Active` | Local root is not `Active` when the one-way draining fence is requested | self | Begin draining only from exact Active mirror authority | public |
| `ROOT_DRAINING_REGISTRY_NOT_COVERED` | `begin_draining` / requested Registry not covered by mirror | Requested source Registry is not covered by the active local mirror | self | Refresh the mirror and retry its exact current authority | public |
| `ROOT_DRAINING_REQUEST_CONFLICT` | `exact_draining_retry` / Registry differs from retained request | Exact retry names a different source Registry | self; existing exact identity | Replay only the retained draining request | public |
| transparent Coordinator reservation transport | `fetch_root_draining_reservation` / typed remote result | Typed Coordinator result is returned without a workflow aggregate | preserve exact nested diagnostic | Retry according to the Coordinator/call diagnostic | reservation operation owner |
| `ROOT_DRAINING_RESERVATION_MISMATCH` | `validate_root_draining_reservation` / qualified reservation predicate | Coordinator reservation differs from operation, root, Registry, time or canonical hash authority | self | Reject it and obtain the exact qualified reservation | public |
| `ROOT_FINAL_INVENTORY_RETRY_REGISTRY_CONFLICT` | `finalize_inventory` / retained Registry differs from retry | Final-inventory retry names a different Registry | self; existing exact identity | Replay only the original Registry authority | public |
| `ROOT_FINAL_INVENTORY_RETAINED_ASSETS` | `finalize_inventory` / non-Store asset count is nonzero | Pool, allocation or workload Canisters remain and would be orphaned | self | Recycle or hand off every retained asset before retry | public; count remains in pool status |
| `ROOT_FINAL_INVENTORY_POOL_WORK_PENDING` | `finalize_inventory` / pending pool lifecycle work | Pool lifecycle work remains nonterminal | self | Reconcile every pending pool operation first | public |
| `ROOT_FINAL_INVENTORY_INTENT_CONFLICT` | `finalize_inventory` / request Registry differs from retained intent | Final-inventory request differs from its durable intent | self; existing exact identity | Replay only the exact retained intent | public |
| `ROOT_FINAL_INVENTORY_REGISTRY_MISMATCH` | `finalize_inventory` / request Registry differs from active mirror | First final-inventory request differs from the active Registry mirror | self | Refresh and submit the exact current mirror version | public |
| `ROOT_FINAL_INVENTORY_STORE_PRINCIPAL_MISMATCH` | `finalize_inventory` / write-fenced Store differs from catalog | Write-fenced Store differs from the root release-set catalog Store | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile Store/bootstrap authority | recent failure |
| `ROOT_REMOVAL_RETRY_REGISTRY_CONFLICT` | `publish_removal` / retained previous Registry differs from retry | Logical-removal retry names a different previous Registry | self | Replay only the exact original publication request | public |
| `ROOT_REMOVAL_REGISTRY_MISMATCH` | `publish_removal` / request Registry differs from active mirror | First logical-removal request differs from the active mirror | self | Refresh and submit the exact current Registry | public |
| `ROOT_REMOVAL_STORE_PRINCIPAL_MISMATCH` | `publish_removal` / verified Store differs from catalog | Removal-verified Store differs from the root release-set catalog Store | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile Store/bootstrap authority | recent failure |
| `ROOT_REMOVAL_PUBLICATION_UNAVAILABLE` | `removal_status` and `removed_root_inventory` / publication absent | Status or later retirement requires a retained logical-removal publication | self; existing exact identity | Complete and retain Coordinator publication first | public |
| `ROOT_SUMMARY_STORE_COUNT_OVERFLOW` | `canister_summary` / Store count does not fit `u32` | Root-local Store inventory cannot fit the bounded summary count | `COMPONENT_REGISTRY_STATE_INVALID` | Inspect bounded Store inventory; do not emit a partial summary | recent failure |
| `ROOT_SUMMARY_STORE_CARDINALITY_INVALID` | `canister_summary` / Store count differs from one | Active root does not have exactly one known sibling Store | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile root/Store inventory authority | recent failure |
| `ROOT_SUMMARY_STORE_INVENTORY_MISMATCH` | `canister_summary` / pool Store count differs from root Store count | Runtime Store and physical Canister inventories disagree | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile the two protected indexes | recent failure |
| transparent protected Fleet-activation cause | `validated_root_state` / typed root-authority and Active-state adapters | Root authority or Active-state rejection is already typed by Fleet activation | preserve exact nested diagnostic | Follow the exact activation diagnostic | guarded runtime status |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | `validated_root_state` / Component Registry authority absent | Root Component Registry meta authority is absent | self; existing exact identity | Complete root Registry preparation before retry | public |
| `ROOT_PROTECTED_AUTHORITY_MISMATCH` | `validate_protected_root` / protected root differs from receiver | Protected root binding names another Canister | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reinstall exact protected authority | recent failure |
| `COMPONENT_REGISTRY_PREPARATION_AUTHORITY_INVALID` | `validate_component_registry` / source authority or coverage mismatch | Registry root/release or preparation coverage differs from protected mirror authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile/reinstall protected state | recent failure |
| `COMPONENT_REGISTRY_ALLOCATED_COUNT_OVERFLOW` | `validate_component_registry` / checked allocation sum overflow | Root Component allocation counters overflow | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Stop mutation and inspect counter accounting | recent failure |
| `COMPONENT_REGISTRY_KNOWN_CREATED_EXCEEDS_ALLOCATED` | `validate_component_registry` / known-created exceeds allocated | Known-created workload Canisters exceed logical capacity | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Reconcile physical and logical inventory | recent failure |
| `ROOT_DRAINING_PUBLICATION_UNAVAILABLE` | `ensure_root_is_published_draining` / root status is not `Draining` | Final inventory is requested before the root is published `Draining` | self | Complete Coordinator draining publication and mirror convergence | public |
| `ROOT_REMOVAL_PUBLICATION_RECEIPT_INVALID` | `removed_root_inventory` and `removal_publication_response` / final-inventory hash mismatch | Retained logical-removal publication names a different final inventory | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both records and fail closed | recent failure |
| `ROOT_STORE_RECLAMATION_STORE_PRINCIPAL_MISMATCH` | `verify_store_before_reclamation` / verified Store differs from catalog | Reclamation verification observes a Store other than the release-set catalog Store | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile exact Store/bootstrap authority | recent failure |
| `ROOT_STORE_RECLAMATION_INVENTORY_EVIDENCE_MISMATCH` | `verify_store_before_reclamation` / verified evidence differs from final inventory | Live verified Store evidence differs from retained final inventory | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve terminal inventory and fail closed | recent failure |
| `ROOT_SUMMARY_INFRASTRUCTURE_COUNT_OVERFLOW` | `fleet_subnet_root::summary` / root-plus-Store checked addition fails | Root plus Store count overflows the bounded summary | `COMPONENT_REGISTRY_STATE_INVALID` | Inspect infrastructure inventory; emit no partial total | recent failure |
| `ROOT_SUMMARY_WORKLOAD_INVENTORY_MISMATCH` | `fleet_subnet_root::summary` / physical and Registry workload counts differ | Physical workload count differs from Registry principal accounting | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile physical and logical inventory | recent failure |
| `ROOT_SUMMARY_TOTAL_COUNT_OVERFLOW` | `fleet_subnet_root::summary` / checked total addition fails | Infrastructure, workload and pool total overflows | `COMPONENT_REGISTRY_STATE_INVALID` | Inspect bounded counters; emit no partial total | recent failure |
| transparent Coordinator removal transport | `publish_removed_to_coordinator` / typed remote result | Typed Coordinator publication result is returned without a workflow aggregate | preserve exact nested diagnostic | Retry through the exact publication operation | Coordinator/root publication owner |
| `ROOT_REMOVAL_RESPONSE_MISMATCH` | `validate_removal_publication_response` / qualified response predicate | Coordinator response differs from request, transition revision or nonzero head | self; existing exact identity | Reject it and query/replay the exact publication | public |

The 33 rows sum to all 36 selected sites. Twenty-two exact identities are new,
eight reuse Component Registry persistence identities and three rows preserve
typed nested causes. No projection is added.

## Store Reclamation And Binding-Finalization Boundary

This slice accounts for all seven direct constructors in `reclaim_store`,
`store_reclamation_status`, `finalize_store_binding` and
`store_binding_finalization_status`, the exact baseline range at lines
503–638.

| Exact candidate | Producer function/branch | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_STORE_RECLAMATION_FINAL_INVENTORY_MISMATCH` | `reclaim_store` / requested final-inventory hash mismatch | Reclamation request names a different final inventory | self; existing exact identity | Use the exact retained final-inventory hash | public |
| `ROOT_STORE_RECLAMATION_INTENT_CONFLICT` | `reclaim_store` / retained intent authority mismatch | Reclamation retry differs from its durable intent | self; existing exact identity | Replay only the original reclamation intent | public |
| `ROOT_STORE_RECLAMATION_UNAVAILABLE` | `store_reclamation_status` and `finalize_store_binding` / receipt absent | Reclamation status/finalization requires a terminal reclamation receipt | self; existing exact identity | Complete exact Store reclamation first | public |
| `ROOT_STORE_BINDING_FINALIZATION_RECLAMATION_MISMATCH` | `finalize_store_binding` / requested reclamation hash mismatch | Finalization request names a different reclamation receipt | self; existing exact identity | Use the exact retained reclamation hash | public |
| `ROOT_STORE_BINDING_FINALIZATION_INTENT_CONFLICT` | `finalize_store_binding` / retained intent authority mismatch | Finalization retry differs from its durable intent | self; existing exact identity | Replay only the original finalization intent | public |
| `ROOT_STORE_BINDING_FINALIZATION_UNAVAILABLE` | `store_binding_finalization_status` / receipt absent | Status/deletion requires terminal binding finalization | self; existing exact identity | Complete exact binding finalization first | public |

The six rows sum to all seven selected sites. Every identity reuses the
Component Registry persistence ledger; this workflow slice adds no exact or
projection identity. Store publication/GC effect calls retain their typed inner
diagnostics.

## Store Deletion And Root-Deletion Readiness Boundary

This slice accounts for all 19 direct constructors in:

- `delete_store`, `store_deletion_status`, `prepare_deletion` and
  `deletion_preparation_status` (lines 639–827); and
- `validate_deletion_preparation_retry`,
  `validate_root_deletion_cycle_reserve`, `root_deletion_readiness_request`,
  the two Coordinator readiness call adapters and response validators, and
  `reclaim_root_deletion_cycles` (lines 1169–1452).

Cost Guard reservation/recovery, management cycle transfer and typed
Coordinator calls remain transparent. The workflow owns the live balance,
reserve and response-authority decisions around them.

| Exact candidate or disposition | Producer function/branch | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_STORE_BINDING_FINALIZATION_UNAVAILABLE` | `delete_store` / finalization receipt absent | Store deletion requires terminal binding finalization | self; existing exact identity | Complete finalization first | public |
| `ROOT_STORE_DELETION_FINALIZATION_MISMATCH` | `delete_store` / requested finalization hash mismatch | Store-deletion request names a different finalization receipt | self; existing exact identity | Use the exact retained finalization hash | public |
| `ROOT_STORE_DELETION_INTENT_CONFLICT` | `delete_store` / retained intent authority mismatch | Store-deletion retry differs from durable intent | self; existing exact identity | Replay only the original deletion intent | public |
| `ROOT_STORE_DELETION_UNAVAILABLE` | `store_deletion_status` and `prepare_deletion` / deletion receipt absent | Status or root deletion requires terminal Store-deletion receipt | self; existing exact identity | Complete typed Store absence and receipt commit | public |
| `ROOT_DELETION_PREPARATION_STORE_RECEIPT_MISMATCH` | `prepare_deletion` / requested Store-deletion hash mismatch | Root-deletion preparation names a different Store-deletion receipt | self; existing exact identity | Use the exact retained Store-deletion hash | public |
| `ROOT_DELETION_READINESS_UNAVAILABLE` | `deletion_preparation_status` / readiness receipt absent | Status requires a terminal root-deletion readiness receipt | self | Complete Coordinator readiness and local receipt commit | public |
| `ROOT_DELETION_PREPARATION_RECEIPT_CONFLICT` | `validate_deletion_preparation_retry` / retained receipt authority mismatch | Terminal readiness retry differs from its durable receipt | self | Replay only the exact original preparation request | public |
| `ROOT_DELETION_PREPARATION_AUTHORITY_INVALID` | `validate_root_deletion_cycle_reserve` / retained-target authority predicate | Retained-cycle target or reserved-cycle observation differs from live freezing authority | self; existing exact identity | Recompute from exact live metrics before intent | public |
| `ROOT_DELETION_COORDINATOR_INTENT_MISSING` | `root_deletion_readiness_request` / intent hash absent | Readiness request lacks Coordinator execution-intent hash | self; existing exact identity | Prepare and retain Coordinator intent first | public |
| `ROOT_CYCLE_RECLAMATION_AMOUNT_MISSING` | `root_deletion_readiness_request` / post-transfer observation absent | Readiness request lacks post-transfer cycle observation | self; existing exact identity | Complete and retain exact root cycle reclamation | public |
| `ROOT_CYCLE_RECLAMATION_TIME_MISSING` | `root_deletion_readiness_request` / reclamation time absent | Readiness request lacks cycle-reclamation time | self; existing exact identity | Complete and retain exact root cycle reclamation | public |
| transparent Coordinator readiness transports | `prepare_root_deletion_readiness_with_coordinator` and `record_root_deletion_readiness_with_coordinator` / typed remote results | Typed intent/readiness call results retain their exact diagnostic | preserve exact nested diagnostic | Retry through the corresponding Coordinator operation | Coordinator/root readiness owners |
| `ROOT_DELETION_COORDINATOR_INTENT_RESPONSE_MISMATCH` | `validate_root_deletion_readiness_intent_response` / qualified response predicate | Coordinator intent response differs from root request, principal, time or nonzero hash | self | Reject and query/replay the exact intent | public |
| `ROOT_DELETION_COORDINATOR_READINESS_RESPONSE_MISMATCH` | `validate_root_deletion_readiness_response` / qualified response predicate | Coordinator readiness response differs from complete local deletion authority | self | Reject and query/replay exact readiness | public |
| `ROOT_CYCLE_BALANCE_INCREASED_AFTER_INTENT` | `reclaim_root_deletion_cycles` / live balance exceeds frozen observation | Root balance exceeds the pre-transfer observation frozen in deletion intent | self | Stop; inspect foreign funding before any transfer | public |
| `ROOT_DELETION_CYCLE_RESERVE_INSUFFICIENT` | `reclaim_root_deletion_cycles` / refund-headroom and call-cost subtraction fails | Frozen retained target cannot cover refund headroom and exact call cost | self | Recompute valid authority; never transfer below the reserve | public |
| `ROOT_CYCLE_RECLAMATION_INCOMPLETE` | `reclaim_root_deletion_cycles` / post-transfer balance predicate fails | Observed post-transfer balance still exceeds durable retained target | self | Preserve transfer evidence and reconcile before readiness | public |

The 17 rows sum to all 19 selected sites. Seven exact identities are new, nine
reuse persistence identities and one two-site row preserves typed Coordinator
causes. No projection is added.

Across all four slices, 69 sites qualify 36 new exact candidates, reuse 23
existing exact identities and preserve six typed transitive constructor sites.

## Grouping Decisions

- A missing logical-removal publication is one readiness meaning whether read
  by status or a later Store boundary.
- A request/mirror mismatch is distinct from a retained receipt contradiction.
  The former is caller-correctable; the latter is masked protected state.
- Store-principal mismatch is phase-specific because final inventory, removal
  and reclamation bind different observations and recovery boundaries.
- Summary failures are not best-effort warnings. Contradictory or overflowing
  inventory returns no fabricated partial total.
- Coordinator and Fleet-activation errors remain typed transitive causes and
  receive no root-workflow wrapper code.
- Store reclamation and binding-finalization workflow decisions are exact
  aliases of their persistence meanings; they deliberately add no code.
- Store deletion, Store cycle reclamation, root cycle reclamation and
  Coordinator readiness remain distinct interruption boundaries. A later
  receipt never erases the earlier operation's diagnostic owner.
- Store adoption admits exactly the frozen temporary or final controller set;
  arbitrary controller status never becomes root authority.

## Required Tests

- a 36-site constructor manifest;
- draining retry, stale mirror and canonical reservation substitution;
- retained pool assets and pending pool operations independently blocking
  final inventory;
- Store principal substitution at final inventory, logical removal and
  reclamation verification;
- response loss around Coordinator removal publication and exact later replay;
- invalid previous/next Registry transition and retained publication/inventory
  contradiction; and
- every summary overflow/cardinality/index mismatch rejecting without partial
  output;
- seven reclamation/binding workflow sites mechanically reusing persistence
  meanings without aggregate wrappers;
- Store-deletion response loss before pool settlement and exact terminal retry;
- Coordinator readiness intent and terminal response substitution; and
- increased root balance, insufficient refund/call reserve and incomplete
  post-transfer reclamation as independent failures, plus a 19-site manifest.
- adoption response loss before local commit, wrong operation/Store/module and
  temporary/final/foreign controller-set observations, plus a seven-site
  manifest.

## Next Slice

Return to the remaining Component Registry Directory, peer and protected-
validation constructors, then proceed through provisioning and Coordinator
owners.
