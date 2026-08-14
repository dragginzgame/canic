# Canic 0.102 Canister Pool Constructor Leaves

Date: 2026-08-13

## Status

This B1 evidence ledger classifies the direct constructors in
`crates/canic-control-plane/src/ops/canister_pool/mod.rs` by consecutive source
range. It assigns no number and changes no runtime behavior.

The file contains 69 production `InternalError::*` references. The first range,
lines 45-346 at the current baseline, contains 11 references and owns Store/
import initialization, recycling/reset transitions and pool claims. Later
creation, handoff, Store-deletion, configuration and helper ranges remain open.

## Inventory Initialization, Reset And Claim Range

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_STORE_INVENTORY_CONFLICT` | 1 | Sibling Wasm Store principal already belongs to a non-Store physical inventory row | self | Reconcile the exact Store principal and inventory owner | public |
| `CANISTER_POOL_IMPORT_MAXIMUM_EXCEEDED` | 1 | Initial import count exceeds configured `maximum_size` | self | Reduce imports or increase the immutable pool policy | public |
| `CANISTER_POOL_IMPORT_PRINCIPAL_DUPLICATE` | 1 | Initial imports repeat one physical Canister principal | self | Remove duplicate principals | public |
| `CANISTER_POOL_IMPORT_ASSET_CONFLICT` | 1 | Imported principal already belongs to a non-imported physical inventory row | self | Remove the conflicting import or reconcile the existing asset | public |
| `CANISTER_POOL_RECYCLE_WORKLOAD_REQUIRED` | 1 | Recycling begins for an asset that is neither the exact workload nor its idempotent recycling replay | self | Use the exact registered workload asset | public |
| `CANISTER_POOL_RESET_COMPLETION_STATUS_INVALID` | 1 | Reset completion targets an asset outside pending, recycling or failed reset state | self | Query the asset and complete only an admitted reset | public |
| `CANISTER_POOL_RESET_FAILURE_STATUS_INVALID` | 1 | Reset failure targets an asset outside pending, recycling or failed reset state | self | Query the asset and record failure only for an admitted reset | public |
| `CANISTER_POOL_RESET_RETRY_STATUS_INVALID` | 1 | Reset retry targets an asset without a retained failed reset | self | Retry only the exact failed asset | public |
| `CANISTER_POOL_CLAIM_DUPLICATE` | 2 | One Component allocation owns more than one claimed physical asset | `COMPONENT_REGISTRY_STATE_INVALID`; both lookups share one exact meaning | Preserve inventory and fail closed before selecting/finalizing another asset | recent failure |
| `CANISTER_POOL_CLAIM_ALLOCATION_MISMATCH` | 1 | Claim finalization targets an asset not claimed/workloaded by the exact Component operation | self | Replay with the exact claim and principal | public |

The ten rows sum to all 11 references in this range and add ten exact meanings.
No safe projection is added.

## Dynamic Public Context

Two conflict messages interpolate a physical Canister principal: the import
asset conflict and recycled-workload requirement. Both principals came from the
exact caller request and are therefore caller-derivable. Discard them from the
compact diagnostic; the request and exact code retain the required action.

The free-form reset failure `reason` is not constructed by this range as an
error message. It is persisted operational state and remains part of the
separate durable-string ownership audit.

## Autonomous Creation Intent Through Rollover

This range assigns all 32 references from `begin_creation` through
`next_creation_timestamp`. Repeated state facts retain one identity across the
exact commands that inspect them.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_CREATION_ALREADY_PENDING_CONFLICT` | 1 | New refill authority differs from an already pending creation; exact authority retry succeeds | self | Reconcile or complete the pending creation before starting another | public |
| `CANISTER_POOL_CREATION_TIMESTAMP_NONMONOTONIC` | 1 | New Cycles Ledger deduplication time does not exceed the durable high-water mark | self | Allocate the next monotonic timestamp | public |
| `CANISTER_POOL_CREATION_NOT_PENDING` | 9 | Attempt, result, settlement, block, adoption, commit, cancel or rollover has no durable creation | self; all sites share one exact absence fact | Inspect status and begin/recover the exact creation | public |
| `CANISTER_POOL_CREATION_ATTEMPT_OPERATION_MISMATCH` | 1 | Begin-attempt operation differs from the pending creation | self | Replay with the exact pending operation ID | public |
| `CANISTER_POOL_CREATION_ATTEMPT_COST_AUTHORITY_PENDING` | 1 | Begin-attempt would overwrite unsettled replay-cost authority | self | Settle the previous attempt before another paid effect | public |
| `CANISTER_POOL_CREATION_ATTEMPT_PHASE_INVALID` | 1 | Begin-attempt is no longer at the creation-intent phase | self | Query and reconcile the retained terminal/blocked progress | public |
| `CANISTER_POOL_CREATION_RECEIPT_OPERATION_MISMATCH` | 1 | Cycles Ledger creation result belongs to another refill operation | self | Record only the exact operation's result | public |
| `CANISTER_POOL_CREATION_TERMINAL_EVIDENCE_CONFLICT` | 1 | A retained Created or Blocked result differs from a later creation result | self | Preserve the first terminal evidence and reconcile the exact retry | public |
| `CANISTER_POOL_CREATION_COST_SETTLEMENT_MISMATCH` | 1 | Settlement differs from the replay-cost authority retained before invocation | self | Settle only the exact retained cost operation | public |
| `CANISTER_POOL_CREATION_BLOCK_COST_AUTHORITY_PENDING` | 1 | Creation is blocked while a paid-attempt settlement remains pending | self | Settle cost authority before recording a terminal block | public |
| `CANISTER_POOL_CREATION_TERMINAL_PROGRESS_OVERWRITE` | 1 | Block transition would overwrite Created or different Blocked progress | self | Preserve terminal progress and reconcile the original result | public |
| `CANISTER_POOL_CREATION_RECEIPT_PRINCIPAL_MISMATCH` | 1 | Inventory adoption principal differs from the durable Cycles Ledger receipt | self | Adopt only the exact created principal | public |
| `CANISTER_POOL_CREATION_INVENTORY_CONFLICT` | 1 | Created principal already belongs to incompatible physical inventory | self | Reconcile the existing row; do not overwrite it | public |
| `CANISTER_POOL_CREATION_PRINCIPAL_MISSING` | 1 | Commit is attempted without a retained Created principal | self | Recover/query the creation result before commit | public |
| `CANISTER_POOL_CREATION_SEQUENCE_EXHAUSTED` | 4 | Commit, explicit blocked retry, cancellation or rollover cannot advance the durable operation sequence | self; all four paths share one exhausted authority | Stop autonomous creation for this root | public |
| `CANISTER_POOL_BLOCKED_CREATION_NOT_PENDING` | 1 | Explicit blocked retry has no retained blocked creation | self | Inspect creation status before retry | public |
| `CANISTER_POOL_CREATION_RETRY_BLOCKED_REQUIRED` | 1 | Explicit blocked retry targets creation progress that is not Blocked | self | Retry only retained Blocked progress | public |
| `CANISTER_POOL_CREATION_RETRY_UNRESOLVED_EXPIRED_FORBIDDEN` | 1 | An unresolved result beyond the Cycles Ledger window attempts another paid effect | self | Resolve manually; never repeat the paid creation | public |
| `CANISTER_POOL_CREATION_CANCEL_KNOWN_UNAPPLIED_REQUIRED` | 1 | Cancellation lacks exact known-unapplied evidence | self | Cancel only a typed known-unapplied outcome | public |
| `CANISTER_POOL_CREATION_ROLLOVER_KNOWN_UNAPPLIED_REQUIRED` | 1 | Expired-operation rollover lacks exact unapplied intent evidence | self | Roll over only the proved-unapplied intent | public |
| `CANISTER_POOL_CREATION_TIMESTAMP_EXHAUSTED` | 1 | Durable Cycles Ledger timestamp high-water mark cannot advance | self | Stop autonomous creation for this root | public |

The 21 rows sum to all 32 references in the range and add 21 exact meanings.
No message in this range interpolates a dynamic value.

## Exclusive Asset Handoff

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_HANDOFF_ALREADY_COMPLETE` | 1 | A completed asset is submitted for another handoff | self | Inspect the retained handoff receipt | public |
| `CANISTER_POOL_HANDOFF_CREATION_PENDING` | 1 | Physical-asset handoff begins while autonomous creation remains unresolved | self | Reconcile creation before transferring inventory authority | public |
| `CANISTER_POOL_HANDOFF_JOURNAL_ASSET_MISMATCH` | 1 | Exact handoff retry finds a physical asset outside its retained HandingOff state | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve journal and asset; fail closed | recent failure |
| `CANISTER_POOL_HANDOFF_ALREADY_PENDING` | 1 | A different canister/recipient handoff already owns the exclusive journal | self | Complete or reconcile the pending handoff | public |
| `CANISTER_POOL_HANDOFF_ASSET_STATUS_INVALID` | 1 | Handoff targets an asset outside Ready or Failed state | self | Hand off only a reset-terminal pooled asset | public |
| `CANISTER_POOL_HANDOFF_NOT_PENDING` | 1 | Completion has no durable handoff intent | self | Prepare/query the exact handoff first | public |
| `CANISTER_POOL_HANDOFF_COMPLETION_CANISTER_MISMATCH` / `CANISTER_POOL_HANDOFF_COMPLETION_RECIPIENT_MISMATCH` | 1 | Completion changes the retained physical canister or recipient authority | self for both leaves | Replay only the exact pending handoff | public |
| `CANISTER_POOL_HANDOFF_ASSET_AUTHORITY_MISMATCH` | 1 | Completion journal and HandingOff asset row disagree | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both records and fail closed | recent failure |
| `CANISTER_POOL_HANDOFF_RECEIPT_EXISTS` | 1 | Completion would overwrite an existing terminal handoff receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the first receipt and reconcile exact retry | recent failure |

The nine sites produce ten new exact meanings and no projection or dynamic
public value.

## Store Deletion, Configuration And Shared Helpers

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_STORE_INVENTORY_CONFLICT` | 1 | Required sibling Store is absent from exact infrastructure Store inventory | self; reuses initialization conflict | Reconcile the exact Store principal and inventory row | public |
| `CANISTER_POOL_STORE_DELETION_ORIGIN_MISMATCH` | 2 | Store deletion begins or completes against a non-infrastructure asset | self on begin; `COMPONENT_REGISTRY_STATE_INVALID` on retained completion | Preserve inventory and use only the exact Store asset | public or recent failure as stated |
| `CANISTER_POOL_STORE_DELETION_STATE_CONFLICT` | 1 | Begin deletion finds neither Store nor exact operation retry | self | Query and replay the retained Store deletion operation | public |
| `CANISTER_POOL_STORE_DELETION_PENDING_AUTHORITY_MISMATCH` | 1 | Completion differs from the pending Store-deletion operation | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the row and reconcile the exact operation | recent failure |
| `CANISTER_POOL_CONFIG_MINIMUM_ZERO` | 1 | Immutable pool `minimum_size` is zero | self | Configure a positive minimum | public |
| `CANISTER_POOL_CONFIG_MAXIMUM_BELOW_MINIMUM` | 1 | Immutable pool `maximum_size` is below its minimum | self | Raise the maximum or lower the minimum | public |
| `CANISTER_POOL_CONFIG_CANISTER_CYCLES_ZERO` | 1 | Per-asset creation cycles are zero | self | Configure a positive creation amount | public |
| `CANISTER_POOL_CREATION_ATTEMPT_COST_AUTHORITY_MISMATCH` | 1 | Finish-attempt settlement differs from the authority retained before invocation | self | Finish only the exact retained settlement | public |
| `CANISTER_POOL_CREATION_ATTEMPT_PHASE_INVALID` | 1 | Finish-attempt no longer observes creation Intent | self; reuses begin-attempt phase identity | Query and reconcile terminal/blocked progress | public |
| `CANISTER_POOL_CREATION_OPERATION_MISMATCH` | 1 | A shared creation transition names another durable operation | self | Replay with the exact pending operation ID | public |
| `CANISTER_POOL_CREATION_COST_AUTHORITY_PENDING` | 1 | Commit, cancel or rollover still has retained replay-cost authority | self | Settle the exact paid attempt first | public |
| `CANISTER_POOL_CREATION_INVENTORY_ADOPTION_MISSING` | 1 | Commit occurs before exact Created principal adoption as PendingReset inventory | self | Adopt and verify the exact principal before commit | public |
| `CANISTER_POOL_MAXIMUM_EXHAUSTED` | 1 | A new import/creation would exceed immutable standby capacity | self | Claim, hand off or raise policy before adding an asset | public |
| `CANISTER_POOL_ASSET_NOT_REGISTERED` | 1 | Requested physical Canister has no pool inventory row | self | Use/query a registered pool asset | public |
| `CANISTER_POOL_RECYCLE_INVENTORY_STATUS_MISMATCH` | 1 | Membership removal finds neither exact terminal replay nor Recycling state | self | Reconcile the physical asset before membership settlement | public |
| `CANISTER_POOL_RECYCLE_COMPONENT_OWNER_MISMATCH` | 1 | Recycling claim belongs to another Component tree | self | Settle only the exact workload owner | public |
| `CANISTER_POOL_RECYCLE_RESET_NOT_TERMINAL` | 1 | Membership removal is attempted before reset reaches Ready or Failed | self | Complete/query reset before removal settlement | public |

The 17 source references produce 18 label occurrences: the deletion-origin
identity appears at both begin and completion. Two other labels reuse earlier
qualified pool meanings. Fifteen exact meanings are new and no projection is
added. `CANISTER_POOL_ASSET_NOT_REGISTERED` currently interpolates the requested
principal; it is caller-derivable and must become code-only.

## Mechanical Coverage

| Consecutive source range | Source references | Classified | Open |
| --- | ---: | ---: | ---: |
| Inventory initialization, reset and claims (lines 45-346) | 11 | 11 | 0 |
| Autonomous creation (lines 431-780) | 32 | 32 | 0 |
| Handoff and remaining lifecycle body (lines 781-1,000) | 9 | 9 | 0 |
| Store deletion, configuration and helpers (after line 1,000) | 17 | 17 | 0 |
| **Total** | **69** | **69** | **0** |

Line ranges are review aids; function and candidate identities remain the
stable anchors. A fresh source count must accompany every later range update.

## Required Tests For This Range

- reject Store/import collisions without exposing a principal not already in
  the caller request;
- reject duplicate import principals and import count above the exact ceiling;
- exercise every admitted and rejected reset-state transition independently;
- prove exact recycle retry is idempotent while a foreign workload owner
  rejects; and
- inject two claims for one Component operation and prove both claim lookup
  paths fail closed with the same storage diagnostic.

## Next Slice

Classify the 11-site Canister pool workflow, then root Store bootstrap.
