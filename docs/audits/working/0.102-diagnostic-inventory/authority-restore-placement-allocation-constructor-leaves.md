# Canic 0.102 Authority Restore And Placement Allocation Constructor Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger classifies all twenty-four production
`InternalError` constructor references in authority-restore fence persistence
and receipt-backed placement allocation. It assigns no number and changes no
runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `ops/storage/authority_restore/mod.rs` | 9 |
| `workflow/placement/allocation.rs` | 15 |
| **Total** | **24** |

## Authority-Restore Fence Persistence

The nine storage sites own nine exact same-release restore identities:

| Exact candidate | Sites | Producer function | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- | --- |
| `AUTHORITY_RESTORE_INIT_AUTHORITY_CONFLICT` | 1 | `AuthorityRestoreFenceOps::initialize`; existing-authority branch | `Conflict` / immutable fence authority | self | Reinstall or invoke the Canister bound to the initialized fence |
| `AUTHORITY_RESTORE_INIT_COMMIT_MISSING` | 1 | `AuthorityRestoreFenceOps::initialize`; post-replace absence | `Invariant` / fresh-state persistence | self | Stop initialization; no fence state was durably retained |
| `AUTHORITY_RESTORE_PREPARE_OPERATION_CONFLICT` | 1 | `AuthorityRestoreFenceOps::prepare`; sealed-operation mismatch | `Conflict` / sealed operation identity | self | Resume or exactly retry the operation that owns the seal |
| `AUTHORITY_RESTORE_RESUME_NOT_SEALED` | 1 | `AuthorityRestoreFenceOps::resume`; open-state branch | `Conflict` / resume phase | self | Prepare the exact snapshot operation before resuming it |
| `AUTHORITY_RESTORE_RESUME_OPERATION_MISMATCH` | 1 | `AuthorityRestoreFenceOps::resume`; sealed-operation mismatch | `Conflict` / sealed operation identity | self | Resume only the operation retained by the seal |
| `AUTHORITY_RESTORE_HISTORY_MISMATCH` | 1 | `AuthorityRestoreFenceOps::resume`; history-count mismatch | `Unavailable` / management history authority | self | Keep mutation fenced; restored or ambiguous history cannot resume unchanged |
| `AUTHORITY_RESTORE_OPERATION_ID_REQUIRED` | 1 | `require_operation_id` | `InvalidInput` / snapshot replay identity | self | Supply one nonzero exact snapshot operation ID |
| `AUTHORITY_RESTORE_AUTHORITY_MISMATCH` | 1 | `require_authority` | `Conflict` / ambient Canister authority | self | Invoke the fence only for its exact bound Canister |
| `AUTHORITY_RESTORE_UNINITIALIZED` | 1 | `fence_uninitialized` | `Invariant` / required stable state | self | Initialize the exact authority runtime before prepare, resume or status use |

The generic current `OperationIdRequired` wire leaf cannot survive the hard
cut: a restore snapshot operation has a different owner, status and recovery
journey from authentication preparation or placement. The authority-specific
identity is therefore new, not a reuse.

Initialization conflict, uninitialized state and missing initial persistence
also remain distinct. Absence before initialization is a reinstall/bootstrap
condition; a failed commit after initialization attempted to write is a stable
state invariant and must not be interpreted as the same retry decision.

## Receipt-Backed Placement Allocation

The fifteen workflow sites reduce to fourteen exact meanings because both
cleanup binding checks enforce one immutable payload authority:

| Exact candidate | Sites | Producer function/branch | Current meaning | Public projection | Required hard cut |
| --- | ---: | --- | --- | --- | --- |
| `PLACEMENT_ALLOCATION_RECOVERY_INTENT_MISSING` | 1 | `PlacementAllocationWorkflow::recover_child` missing-intent branch | Recovery request has no durable pre-effect intent | self | Never invent or repeat a root effect; begin only through admitted creation |
| `PLACEMENT_ALLOCATION_SETTLEMENT_STATE_MISMATCH` | 1 | `settle_allocation` contradictory settled-state branch | Terminal settlement decision differs from retained terminal state | `INTENT_STATE_INVALID` | Preserve both decisions and reject contradictory settlement |
| `PLACEMENT_ALLOCATION_SETTLEMENT_INTENT_MISSING` | 1 | `settle_allocation` `SettleReceiptBackedIntentResult::NotFound` | Admitted intent vanished before domain-owned settlement | `INTENT_STATE_INVALID` | Fail closed; absence is not rollback or commitment |
| `PLACEMENT_ALLOCATION_SETTLEMENT_REVISION_CONFLICT` | 1 | `settle_allocation` `SettleReceiptBackedIntentResult::RevisionConflict` | Settlement observes another durable revision | `INTENT_STATE_INVALID` | Reload exact status and never settle stale evidence |
| `PLACEMENT_ALLOCATION_SETTLEMENT_BINDING_CONFLICT` | 1 | `settle_allocation` `SettleReceiptBackedIntentResult::BindingConflict` | Settlement payload differs from the admitted intent | `INTENT_STATE_INVALID` | Preserve first binding and reject substitution |
| `PLACEMENT_ALLOCATION_CLEANUP_BINDING_CONFLICT` | 2 | `remove_terminal_intent` pre-removal predicate and `remove_exact_terminal_intent` `BindingConflict` branch | Cleanup request or result differs from immutable payload binding | `INTENT_STATE_INVALID` | Remove only the exact terminal intent; never select by operation ID alone |
| `PLACEMENT_ALLOCATION_CLEANUP_NOT_TERMINAL` | 1 | `remove_exact_terminal_intent` `NotTerminal` branch | Receipt cleanup reaches a still-pending allocation | `INTENT_STATE_INVALID` | Complete or recover settlement before removal |
| `PLACEMENT_ALLOCATION_CLEANUP_REVISION_CONFLICT` | 1 | `remove_exact_terminal_intent` `RevisionConflict` branch | Cleanup observes another durable revision | `INTENT_STATE_INVALID` | Reload exact terminal evidence and retry its revision |
| `PLACEMENT_ALLOCATION_DOMAIN_MEMBERSHIP_MISSING` | 1 | `begin_allocation` `ExistingCommitted` branch | Intent is committed while the owning domain has no membership | `INTENT_STATE_INVALID` | Reconcile domain membership; do not allocate a replacement |
| `PLACEMENT_ALLOCATION_ROLLED_BACK` | 1 | `begin_allocation` `ExistingRolledBack` branch | Exact operation has already been durably rolled back | self | Start a newly admitted operation only after policy permits it |
| `PLACEMENT_ALLOCATION_BEGIN_BINDING_CONFLICT` | 1 | `begin_allocation` `BindingConflict` branch | Existing operation is bound to different immutable allocation input | `INTENT_STATE_INVALID` | Replay only the original request |
| `PLACEMENT_ALLOCATION_REPLAY_WINDOW_INVALID` | 1 | `begin_allocation` replay-window branches | Canic-owned placement unexpectedly receives application replay-window policy | `INTENT_STATE_INVALID` | Correct the owner/mode boundary; no unchanged retry |
| `PLACEMENT_ALLOCATION_CAPACITY_EXCEEDED` | 1 | `begin_allocation` `CapacityExceeded` branch | Reserved plus committed quantity exceeds the placement resource ceiling | self | Free placement capacity or lower the request before retry |
| `PLACEMENT_ALLOCATION_INTENT_CAPACITY_REACHED` | 1 | `begin_allocation` `StoreCapacityReached` branch | Bounded receipt-backed intent record capacity is full | self | Drain eligible terminal receipts before admitting more effects |

The settlement and cleanup result enums are finite decisions, not formatter
inputs. B4 must map them exhaustively and preserve the operation-correlated
intent status. A code never proves that a root call occurred, that domain
membership exists or that cleanup is safe.

The application replay-window result currently merges `Closed` and `TooLong`.
They intentionally share one placement invariant because neither is admitted
for Canic-owned intents and both have the same owner, exposure, action and
retry policy. Their underlying application path keeps its own exact replay
diagnostics.

## Dynamic Public Context

Twenty-two values are classified as `DPC-295` through `DPC-316` in
[dynamic-public-context.md](dynamic-public-context.md). Thirteen are exact
operation IDs or requested quantity already owned by the caller's request.
Nine are retained state, revision and capacity values owned by the exact
intent/status and bounded counters.

Authority-restore messages are static. Their operation, Canister, management-
history and timestamp evidence remains in the exact fence status response and
does not need to be interpolated into `Error.message`.

## Reconciliation

All twenty-four direct sites now have one disposition. They add twenty-three
exact meanings and reuse no earlier identity. The effective constructor
frontier moves from 2,355 to 2,379 classified sites and from 144 to 120 open
sites. The qualified semantic set reaches 2,540 exact candidates plus 31 safe
projections: 2,571 current symbolic identities.

## Required Tests

- every authority-fence initialization, prepare and resume phase rejects with
  its exact identity;
- nonzero operation, ambient Canister and management-history predicates reject
  independently;
- missing initial persistence cannot be mistaken for an uninitialized fence;
- placement recovery never calls the root without a durable intent;
- settlement state, presence, revision and binding contradictions reject
  independently;
- both cleanup binding checks share one identity while terminal-state and
  revision conflicts remain distinct;
- committed-without-membership and rolled-back replay have different codes and
  actions;
- Canic-owned placement rejects either application replay-window result; and
- placement quantity and receipt-store capacity remain separate resource
  conditions.

## Next Slice

Continue with Component runtime, ICP-refill replay and Component Directory
synchronization ops.
