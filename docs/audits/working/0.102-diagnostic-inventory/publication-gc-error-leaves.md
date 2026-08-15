# Canic 0.102 Publication GC Error Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger closes all 56 production
`PublicationWorkflowError` constructions in publication `lifecycle/gc.rs`: one
concurrency fence and 55 `InvalidState(String)` constructions. It assigns no
number and changes no runtime behavior.

The status-number formatter expands one construction into six exact numeric
field meanings. The binding-transition formatter expands one construction
into four exact transition meanings. The result is 64 source-semantic
dispositions. Three catalog-overflow constructions share one identity, two
other dispositions reuse already-qualified lifecycle identities and the other
60 add exact meanings. No safe projection is added.

## Lifecycle Exclusion

| Exact candidate | Producer function/branch | Source predicate | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `WASM_STORE_LIFECYCLE_OPERATION_IN_PROGRESS` | `LifecycleOperationGuard::try_enter` busy branch | another root-local Store lifecycle operation owns the heap exclusion guard | self | Reconcile or await the owning operation before exact retry |

This is an in-process concurrency fence, not durable proof that a GC,
reclamation or deletion effect is in flight. Recovery still comes from the
owning stable intent and live observations.

## Final Inventory And Logical Removal

| Exact candidate | Producer function/branch | Source predicate | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `ROOT_FINAL_INVENTORY_SINGLE_STORE_REQUIRED` | `WasmStorePublicationWorkflow::quiesce_single_root_store_for_final_inventory` cardinality branch | final-inventory quiescence observes zero or multiple root-owned Stores | self | Reconcile the exact sibling Store inventory before fencing writes |
| `ROOT_FINAL_INVENTORY_STORE_GC_LINEAGE_MISMATCH` | `WasmStorePublicationWorkflow::quiesce_single_root_store_for_final_inventory` mode branch | runtime/live modes are outside the allowed Normal-to-Prepared convergence | self | Preserve both observations and converge the exact Store GC lineage |
| `ROOT_FINAL_INVENTORY_STORE_GC_PERSIST_FAILED` | `WasmStorePublicationWorkflow::quiesce_single_root_store_for_final_inventory` transition branch | live Prepared authority cannot be committed to root Store state | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the live receipt and stable state; never infer persistence |
| `ROOT_FINAL_INVENTORY_STORE_GC_AUTHORITY_MISMATCH` | `WasmStorePublicationWorkflow::quiesce_single_root_store_for_final_inventory` post-commit predicate | committed runtime `StoreGcAuthority` differs from the re-observed live authority | `COMPONENT_REGISTRY_STATE_INVALID` | Stop finalization and reconcile the protected root/live authorities |
| `ROOT_REMOVAL_SINGLE_STORE_REQUIRED` | `WasmStorePublicationWorkflow::verify_single_root_store_for_removal` cardinality branch | logical-removal reverification observes zero or multiple root-owned Stores | self | Reconcile root infrastructure before publishing removal |
| `ROOT_REMOVAL_STORE_GC_AUTHORITY_MISMATCH` | `WasmStorePublicationWorkflow::verify_single_root_store_for_removal` authority predicate | removal-time runtime `StoreGcAuthority` differs from live Prepared authority | self | Re-observe and reconcile; do not publish logical removal |

The final-inventory and removal mismatches remain separate because they guard
different irreversible ordering boundaries and are recovered through
different durable operations.

## Reclamation And Binding Finalization

| Exact candidate or disposition | Sites | Producer function/branch | Source predicate | Public projection | Action and retry |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_STORE_RECLAMATION_SINGLE_STORE_REQUIRED` | 1 | `WasmStorePublicationWorkflow::reclaim_single_root_store_inner` cardinality branch | reclamation observes zero or multiple runtime Stores | self | Reconcile root infrastructure before GC effects |
| reuse `ROOT_STORE_RECLAMATION_STORE_PRINCIPAL_MISMATCH` | 1 | `WasmStorePublicationWorkflow::reclaim_single_root_store_inner` principal predicate | runtime Store differs from terminal final inventory | self | Use the exact terminal Store; never substitute a principal |
| `ROOT_STORE_RECLAMATION_GC_INCOMPLETE` | 1 | `WasmStorePublicationWorkflow::reclaim_single_root_store_inner` terminal-mode predicate | bounded recovery does not converge live GC to `Complete` | self | Resume the retained GC lineage rather than starting another run |
| `ROOT_STORE_RECLAIMED_CATALOG_COUNT_OVERFLOW` | 3 | `WasmStorePublicationWorkflow::reclaim_single_root_store_inner`, `WasmStorePublicationWorkflow::verify_single_reclaimed_root_store_binding` and `WasmStorePublicationWorkflow::verify_single_finalized_root_store_for_deletion` catalog conversions | reclaimed catalog length cannot fit the terminal `u32` field | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve inventory and stop before finalization/deletion |
| `ROOT_STORE_BINDING_FINALIZATION_SINGLE_STORE_REQUIRED` | 1 | `WasmStorePublicationWorkflow::verify_single_reclaimed_root_store_binding` cardinality branch | binding verification observes zero or multiple runtime Stores | self | Reconcile exact root Store inventory |
| `ROOT_STORE_BINDING_FINALIZATION_ACTIVE_AUTHORITY_MISMATCH` | 1 | `WasmStorePublicationWorkflow::verify_single_reclaimed_root_store_binding` active-authority predicate | runtime Store, final inventory and active/detached/retired publication slots are not one exact active authority | self | Preserve all slots and reconcile before any binding transition |
| `ROOT_STORE_BINDING_FINALIZATION_GC_AUTHORITY_MISMATCH` | 1 | `WasmStorePublicationWorkflow::verify_single_reclaimed_root_store_binding` GC-authority predicate | reclaimed runtime GC authority differs from live status | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile exact Store GC evidence before changing publication slots |
| `ROOT_STORE_BINDING_FINALIZATION_RUNTIME_AUTHORITY_MISMATCH` | 1 | `WasmStorePublicationWorkflow::finalize_single_reclaimed_root_store_binding` runtime predicate | runtime Store differs from the durable binding-finalization intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the intent and runtime row; perform no slot mutation |
| `ROOT_STORE_BINDING_CLEAR_ACTIVE_FAILED` | 1 | `WasmStorePublicationWorkflow::finalize_single_reclaimed_root_store_binding` active transition | active-to-detached transition refuses the exact binding | `COMPONENT_REGISTRY_STATE_INVALID` | Re-read publication state and retry only the same transition |
| `ROOT_STORE_BINDING_RETIRE_DETACHED_FAILED` | 1 | `WasmStorePublicationWorkflow::finalize_single_reclaimed_root_store_binding` detached transition | detached-to-retired transition refuses the exact binding | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve detached authority and retry only after exact observation |
| `ROOT_STORE_BINDING_FINALIZE_RETIRED_FAILED` | 1 | `WasmStorePublicationWorkflow::finalize_single_reclaimed_root_store_binding` retired transition | retired-to-finalized transition refuses the exact binding | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve retired authority; never skip the terminal transition |
| `ROOT_STORE_BINDING_TERMINAL_CONVERGENCE_FAILED` | 1 | `WasmStorePublicationWorkflow::finalize_single_reclaimed_root_store_binding` bounded-loop exhaustion | four bounded driver steps do not reach terminal finalization | `COMPONENT_REGISTRY_STATE_INVALID` | Inspect the retained generation/slots and resume from their exact phase |
| `ROOT_STORE_BINDING_FINALIZATION_PROGRESS_MISMATCH` | 1 | `reclaimed_store_binding_phase` noncanonical-state branch | publication state matches none or several canonical transition phases | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; never guess the next phase |
| `ROOT_STORE_RUNTIME_BINDING_MISSING` | 1 | `WasmStorePublicationWorkflow::runtime_store` missing-binding branch | protected lifecycle binding has no runtime Store row | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile the exact Store inventory and intent |
| `ROOT_STORE_GC_RECONCILIATION_PERSIST_FAILED` | 1 | `WasmStorePublicationWorkflow::reconcile_single_root_store_gc` failed-write branch | live GC authority cannot be reconciled into runtime state | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve live and stable evidence; never infer the write |
| `ROOT_STORE_GC_RECONCILIATION_MISMATCH` | 1 | `WasmStorePublicationWorkflow::reconcile_single_root_store_gc` post-write predicate | post-write runtime GC authority still differs from live status | `COMPONENT_REGISTRY_STATE_INVALID` | Stop the lifecycle journey and inspect contradictory authority |

The free-form transition label is deleted in B4. Each of its four static call
sites selects its exact identity before entering the helper. Catalog overflow
has one representation bound and one repair regardless of which of the three
pre-deletion observations detects it.

## Deletion Preparation And Cycle Reclamation

| Exact candidate or disposition | Producer function/branch | Source predicate | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `ROOT_STORE_DELETION_FINALIZED_RUNTIME_GC_MISMATCH` | `WasmStorePublicationWorkflow::verify_single_finalized_root_store_for_deletion` GC-authority predicate | finalized runtime GC authority differs from live status | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both authorities and stop before deletion intent |
| `ROOT_STORE_DELETION_PREMATURE_PHYSICAL_ABSENCE` | `WasmStorePublicationWorkflow::verify_single_finalized_root_store_for_deletion` absent observation | Store is absent before deletion intent is durable | self | Treat as contradictory destructive state; never fabricate an intent or receipt |
| `ROOT_STORE_DELETION_NOT_RUNNING_AT_PREPARATION` | `WasmStorePublicationWorkflow::verify_single_finalized_root_store_for_deletion` status predicate | exact Store is not Running before deletion intent | self | Reconcile live status before preparing the intent |
| `ROOT_STORE_CYCLE_RECLAMATION_ALREADY_DURABLE` | `WasmStorePublicationWorkflow::reclaim_single_finalized_root_store_cycles` retained-result predicate | another reclamation result is already retained | self | Return/reconcile the retained operation rather than transfer again |
| `ROOT_STORE_CYCLE_RECLAMATION_RUNTIME_INVENTORY_MISSING` | `WasmStorePublicationWorkflow::reclaim_single_finalized_root_store_cycles` inventory predicate | cycle reclamation lacks exact root-owned deletion inventory | `COMPONENT_REGISTRY_STATE_INVALID` | Restore/reconcile inventory before any transfer |
| `ROOT_STORE_CYCLE_RECLAMATION_BALANCE_INCREASED_AFTER_INTENT` | `WasmStorePublicationWorkflow::reclaim_single_finalized_root_store_cycles` pre-call balance predicate | pre-call live balance exceeds the frozen intent observation | self | Reconcile the new funding event; do not use stale transfer authority |
| `ROOT_STORE_CYCLE_RECLAMATION_TARGET_EXCEEDED` | `WasmStorePublicationWorkflow::reclaim_single_finalized_root_store_cycles` post-call balance predicate | post-call balance exceeds the durable retained target or pre-call ceiling | self | Preserve observed progress and resume only from the exact deletion operation |
| `ROOT_STORE_DELETION_RUNTIME_INVENTORY_MISSING` | `WasmStorePublicationWorkflow::delete_single_finalized_root_store` present-without-runtime branch | physical Store remains present after its runtime deletion row disappeared | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed before stop/delete; repair exact inventory authority |
| `ROOT_STORE_DELETION_LINEAGE_MISMATCH` | `validate_store_deletion_lineage` | finalization does not bind exact operation, Fleet Subnet Root, Store, inventory hash, binding, `source + 3` generation, time and hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve finalization and final inventory; perform no deletion effect |
| `ROOT_STORE_DELETION_INTENT_LINEAGE_MISMATCH` | `validate_store_deletion_intent_lineage` | deletion intent does not bind exact finalization, Store, binding, module, canonical controllers, cycle authority and ordering | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both records and reject the effect |
| `ROOT_STORE_DELETION_PUBLICATION_STATE_MISMATCH` | `validate_finalized_publication_state` | live publication slots/generation/time differ from terminal finalization | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile publication authority; never delete through a stale finalization |
| `ROOT_STORE_DELETION_SINGLE_RUNTIME_STORE_REQUIRED` | `single_finalized_runtime_store` cardinality branch | deletion preparation observes zero or multiple runtime Stores | self | Reconcile root Store inventory before intent |
| `ROOT_STORE_DELETION_RUNTIME_AUTHORITY_MISMATCH` | `single_finalized_runtime_store` authority predicate | sole runtime Store differs from terminal binding authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both authorities; do not select a different Store |
| `ROOT_STORE_DELETION_RUNTIME_INVENTORY_MISMATCH` | `validate_deletion_runtime_inventory` | optional runtime inventory is neither empty nor the exact deletion target | `COMPONENT_REGISTRY_STATE_INVALID` | Repair the mutually exclusive inventory state before continuing |
| reuse `ROOT_STORE_DELETION_CONTROLLER_AUTHORITY_MISSING` | `store_management_authority` controller predicate | live controllers omit the protected Fleet Subnet Root | self | Restore exact controller authority before deletion |
| `ROOT_STORE_DELETION_MODULE_HASH_MISSING` | `store_management_authority` module-hash predicate | live Store lacks one nonzero 32-byte installed-module hash | self | Re-observe/reinstall through maintained authority before intent |
| `ROOT_STORE_DELETION_MANAGEMENT_AUTHORITY_MISMATCH` | `require_store_management_authority` | live module or canonical controllers differ from the frozen deletion intent | self | Reject stale authority and reconcile the exact Store |
| `ROOT_STORE_DELETION_BALANCE_ZERO` | `store_deletion_cycle_authority` balance predicate | Store has no positive balance from which to retain deletion execution cycles | self | Re-fund or revise the operation before intent; never invent a target |
| `ROOT_STORE_DELETION_FREEZING_RESERVE_OVERFLOW` | `store_deletion_cycle_authority` freezing-reserve arithmetic | idle burn multiplied by freezing threshold exceeds `u128` | `COMPONENT_REGISTRY_STATE_INVALID` | Stop deletion; repair unrepresentable status authority |
| `ROOT_STORE_DELETION_CYCLE_RESERVE_OVERFLOW` | `store_deletion_cycle_authority` retained-target arithmetic | freezing reserve plus execution reserve exceeds `u128` | `COMPONENT_REGISTRY_STATE_INVALID` | Stop before intent; never wrap the retained target |
| `ROOT_STORE_DELETION_RESERVED_CYCLES_PRESENT` | `require_no_reserved_store_cycles` | live reserved cycles are nonzero | self | Clear/reconcile reserved cycles before reclamation or deletion |
| `ROOT_STORE_CYCLE_RECLAMATION_PREMATURE_ABSENCE` | `present_running_store_for_cycle_reclamation` absent observation | Store becomes absent before cycle reclamation is durable | self | Treat as destructive-state contradiction, not successful reclamation |
| `ROOT_STORE_CYCLE_RECLAMATION_NOT_RUNNING` | `present_running_store_for_cycle_reclamation` status predicate | Store is not Running during the transfer phase | self | Reconcile status; never transfer or stop through the wrong phase |
| `ROOT_STORE_CYCLE_RECLAMATION_RESPONSE_MISMATCH` | `validate_store_cycle_reclamation_response` | response differs from exact root destination, target and bounded before/transferred/after authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve response and intent; never commit it |
| `ROOT_STORE_DELETION_REFUND_HEADROOM_UNDERFLOW` | `store_cycle_reclamation_target` | retained target cannot cover the delayed call-refund headroom | self | Recompute a sufficient target before transfer |
| `ROOT_STORE_CYCLE_RECLAMATION_NOT_DURABLE` | `require_durable_store_cycle_reclamation` | stop/delete is requested without both post-transfer balance and timestamp | self | Commit exact reclamation evidence first |
| `ROOT_STORE_DELETION_BALANCE_INCREASED_AFTER_RECLAMATION` | `require_store_deletion_authority` balance predicate | live deletion balance exceeds the durable post-reclamation observation | self | Reconcile the later funding event before destructive effects |
| `ROOT_STORE_DELETION_NOT_STOPPED` | `delete_store_and_observe_absence` status predicate | delete is attempted while exact Store is not Stopped | self | Complete and re-observe stop before deletion |
| `ROOT_STORE_DELETION_RUNTIME_RECONCILIATION_FAILED` | `reconcile_deleted_store_inventory` failed-removal branch | physically absent Store cannot be removed from the exact runtime row | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve terminal absence evidence and reconcile local state |
| `ROOT_STORE_DELETION_RUNTIME_INVENTORY_REMAINS` | `reconcile_deleted_store_inventory` post-removal predicate | runtime primary/PID indexes remain after deletion reconciliation | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; settle every exact index before terminal receipt |

The two existing physical-stop response identities remain owned by their direct
`InternalError` constructors and are not counted here. Typed IC absence and
transport failures also remain owned by the IC adapter; none of the identities
above proves physical absence.

## Exact Status Numeric Bounds

The `status_cycles(label)` formatter expands to six call-site-selected
identities:

| Exact candidate | Producer function/call-site | Status field | Action and retry |
| --- | --- | --- | --- |
| `ROOT_STORE_DELETION_BALANCE_OVERFLOW` | `status_cycles` from deletion preparation/reclamation | initial/pre-reclamation Store balance | Stop before freezing cycle authority |
| `ROOT_STORE_DELETION_IDLE_BURN_OVERFLOW` | `status_cycles` from `store_deletion_cycle_authority` | idle cycles burned per day | Stop before computing the freezing reserve |
| `ROOT_STORE_DELETION_FREEZING_THRESHOLD_OVERFLOW` | `status_cycles` from `store_deletion_cycle_authority` | freezing threshold | Stop before computing the freezing reserve |
| `ROOT_STORE_DELETION_RESERVED_CYCLES_OVERFLOW` | `status_cycles` from `require_no_reserved_store_cycles` | reserved cycles | Stop before asserting reclaimability |
| `ROOT_STORE_POST_RECLAMATION_BALANCE_OVERFLOW` | `status_cycles` from `WasmStorePublicationWorkflow::reclaim_single_finalized_root_store_cycles` | post-transfer Store balance | Preserve the operation and do not commit evidence |
| `ROOT_STORE_DELETION_FINAL_BALANCE_OVERFLOW` | `status_cycles` from `require_store_deletion_authority` | pre-stop/pre-delete balance | Stop before destructive effects |

The free-form label is not a diagnostic owner and is removed in B4. Protected
status/operation evidence retains the value; compact errors retain only the
failed field identity.

## Shared GC State Validators

| Exact candidate | Producer function | Source predicate | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `ROOT_STORE_GC_PREPARED_AUTHORITY_MISMATCH` | `validate_live_prepared_store` | Prepared mode, nonzero prepared time, changed time, absent start/completion and zero run count are not one exact authority | self | Reconcile the live Store to exact Prepared authority |
| `ROOT_STORE_GC_FINAL_INVENTORY_LINEAGE_MISMATCH` | `validate_live_store_gc_lineage` | live GC phase/timestamps/run count do not descend from the retained final-inventory prepared time | self | Preserve the final inventory and resume only its exact GC lineage |
| `ROOT_STORE_RECLAMATION_EMPTY_INVENTORY_REQUIRED` | `validate_live_reclaimed_store` | Complete Store retains occupied bytes, catalog entries, template/release counts or template rows | self | Continue/reconcile GC; never advance binding finalization |

B4 should express each row as a named authority structure or named predicate,
not retain mixed `&&`/`||` arrays. The fields within a row share one authority,
action and retry boundary; unrelated lifecycle stages remain distinct rows.

## Dynamic Public Context

Slices 9–11 of
[dynamic-public-context.md](dynamic-public-context.md) classify every formatted
GC value. Existing controller-guarded Store overview owns runtime binding,
mode and cardinality. The narrow lifecycle inspection and deletion-progress
responses own live GC and financial progress unavailable through that overview.
No compact diagnostic contains a Store principal, binding, balance, mode,
count, transition label or stable-state prose.

## Reconciliation

All 56 constructions have source dispositions. Helper expansion produces 64
semantic dispositions. Two repeated catalog-overflow sites reuse the one new
catalog identity. `ROOT_STORE_RECLAMATION_STORE_PRINCIPAL_MISMATCH` and
`ROOT_STORE_DELETION_CONTROLLER_AUTHORITY_MISSING` reuse qualified identities.
The remaining 60 identities are new and no safe projection is added.

The qualified semantic set moves from 2,763 to 2,823 exact candidates. The 31
safe projections remain unchanged, producing 2,854 current symbolic identities.

## Required Tests

- exhaustive construction-to-code mapping for all 56 source sites;
- lifecycle exclusion never masquerades as durable operation evidence;
- final-inventory and logical-removal Store checks remain distinct;
- all four binding transition call sites select distinct identities without a
  string label;
- all six status numeric conversions select distinct identities;
- the three catalog overflows reuse one representation-bound identity;
- destructive preparation rejects every lineage, publication, runtime,
  module/controller and cycle-authority mismatch before effects;
- response loss and typed absence remain owned by the IC/effect journals;
- runtime inventory is removed only after typed physical absence;
- Prepared, GC-lineage and empty-inventory validators are named and tested at
  every individual predicate; and
- no generic `InvalidState`, invariant or lifecycle code remains selectable.

## Next Slice

Expand the two remaining `PublicationWorkflowError::TransportUnavailable`
surfaces through every reachable typed IC cause and operation-scoped numeric
owner. Then reconcile the complete aggregate and reassess B1 push readiness.
