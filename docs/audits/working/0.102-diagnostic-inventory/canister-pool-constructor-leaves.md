# Canic 0.102 Canister Pool Constructor Leaves

Date: 2026-08-15

## Status

This B1 evidence ledger classifies the direct constructors in
`crates/canic-control-plane/src/ops/canister_pool/mod.rs` by consecutive source
range. It assigns no number and changes no runtime behavior.

The file contains 69 production `InternalError::*` references across four
consecutive ranges. They reduce to 56 distinct producer-coverage labels after
exact reuse and one required semantic split; these are not 56 proposed codes.
Every label now names its producer
function/branch and each range has an independent completeness guard. This
closes the ops file at the symbolic-anchor level. The adjacent pool workflow
is also source-addressed; both families still require many-to-one compression.

## Inventory Initialization, Reset And Claim Range

| Exact candidate | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_STORE_INVENTORY_CONFLICT` | 1 | `CanisterPoolOps::initialize_store`; existing principal is not the exact infrastructure Store | self | Reconcile the exact Store principal and inventory owner | public |
| `CANISTER_POOL_IMPORT_MAXIMUM_EXCEEDED` | 1 | `CanisterPoolOps::initialize_imports`; `imports.len()` exceeds `maximum_size` | self | Reduce imports or increase the immutable pool policy | public |
| `CANISTER_POOL_IMPORT_PRINCIPAL_DUPLICATE` | 1 | `CanisterPoolOps::initialize_imports`; canonical principal-set length differs from input length | self | Remove duplicate principals | public |
| `CANISTER_POOL_IMPORT_ASSET_CONFLICT` | 1 | `CanisterPoolOps::initialize_imports`; existing principal is not an Imported asset | self | Remove the conflicting import or reconcile the existing asset | public |
| `CANISTER_POOL_RECYCLE_WORKLOAD_REQUIRED` | 1 | `CanisterPoolOps::register_recycled_pending`; status is neither exact Workload nor Recycled/Recycling replay | self | Use the exact registered workload asset | public |
| `CANISTER_POOL_RESET_COMPLETION_STATUS_INVALID` | 1 | `CanisterPoolOps::mark_ready`; status rejects the reset-completion transition | self | Query the asset and complete only an admitted reset | public |
| `CANISTER_POOL_RESET_FAILURE_STATUS_INVALID` | 1 | `CanisterPoolOps::mark_failed`; status rejects the reset-failure transition | self | Query the asset and record failure only for an admitted reset | public |
| `CANISTER_POOL_RESET_RETRY_STATUS_INVALID` | 1 | `CanisterPoolOps::retry_reset`; status has no retained failed reset | self | Retry only the exact failed asset | public |
| `CANISTER_POOL_CLAIM_DUPLICATE` | 2 | `CanisterPoolOps::claim_oldest_ready` and `CanisterPoolOps::claimed_canister`; more than one row matches one exact claim | `COMPONENT_REGISTRY_STATE_INVALID`; both lookups share one exact meaning | Preserve inventory and fail closed before selecting/finalizing another asset | recent failure |
| `CANISTER_POOL_CLAIM_ALLOCATION_MISMATCH` | 1 | `CanisterPoolOps::finalize_claim`; status is neither exact Claimed nor Workload replay | self | Replay with the exact claim and principal | public |

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

| Exact candidate | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_CREATION_ALREADY_PENDING_CONFLICT` | 1 | `CanisterPoolOps::begin_creation`; pending authority differs from the exact retry | self | Reconcile or complete the pending creation before starting another | public |
| `CANISTER_POOL_CREATION_TIMESTAMP_NONMONOTONIC` | 1 | `CanisterPoolOps::begin_creation`; proposed deduplication time does not exceed the high-water mark | self | Allocate the next monotonic timestamp | public |
| `CANISTER_POOL_CREATION_NOT_PENDING` | 9 | missing `state.creation` in `begin_creation_attempt`, `finish_creation_attempt`, `mark_creation_created`, `settle_creation_attempt`, `block_creation`, `register_created_pending_reset`, `commit_creation`, `cancel_known_unapplied_creation` or `rollover_known_expired_creation` | self; all sites share one exact absence fact | Inspect status and begin/recover the exact creation | public |
| `CANISTER_POOL_CREATION_ATTEMPT_OPERATION_MISMATCH` | 1 | `CanisterPoolOps::begin_creation_attempt`; operation ID differs from pending creation | self | Replay with the exact pending operation ID | public |
| `CANISTER_POOL_CREATION_ATTEMPT_COST_AUTHORITY_PENDING` | 1 | `CanisterPoolOps::begin_creation_attempt`; cost settlement is already retained | self | Settle the previous attempt before another paid effect | public |
| `CANISTER_POOL_CREATION_ATTEMPT_PHASE_INVALID` | 1 | `CanisterPoolOps::begin_creation_attempt`; progress is not Intent | self | Query and reconcile the retained terminal/blocked progress | public |
| `CANISTER_POOL_CREATION_RECEIPT_OPERATION_MISMATCH` | 1 | `CanisterPoolOps::mark_creation_created`; result operation differs from pending creation | self | Record only the exact operation's result | public |
| `CANISTER_POOL_CREATION_TERMINAL_EVIDENCE_CONFLICT` | 1 | `CanisterPoolOps::mark_creation_created`; retained terminal progress is not the exact Created replay | self | Preserve the first terminal evidence and reconcile the exact retry | public |
| `CANISTER_POOL_CREATION_COST_SETTLEMENT_MISMATCH` | 1 | `CanisterPoolOps::settle_creation_attempt`; supplied settlement differs from retained cost authority | self | Settle only the exact retained cost operation | public |
| `CANISTER_POOL_CREATION_BLOCK_COST_AUTHORITY_PENDING` | 1 | `CanisterPoolOps::block_creation`; cost settlement remains pending | self | Settle cost authority before recording a terminal block | public |
| `CANISTER_POOL_CREATION_TERMINAL_PROGRESS_OVERWRITE` | 1 | `CanisterPoolOps::block_creation`; progress is neither Intent nor the exact Blocked replay | self | Preserve terminal progress and reconcile the original result | public |
| `CANISTER_POOL_CREATION_RECEIPT_PRINCIPAL_MISMATCH` | 1 | `CanisterPoolOps::register_created_pending_reset`; principal differs from Created progress | self | Adopt only the exact created principal | public |
| `CANISTER_POOL_CREATION_INVENTORY_CONFLICT` | 1 | `CanisterPoolOps::register_created_pending_reset`; existing asset is not the exact Created/PendingReset adoption | self | Reconcile the existing row; do not overwrite it | public |
| `CANISTER_POOL_CREATION_PRINCIPAL_MISSING` | 1 | `CanisterPoolOps::commit_creation`; progress is not Created | self | Recover/query the creation result before commit | public |
| `CANISTER_POOL_CREATION_SEQUENCE_EXHAUSTED` | 4 | checked sequence advance in `commit_creation`, `retry_blocked_creation`, `cancel_known_unapplied_creation` or `rollover_known_expired_creation` | self; all four paths share one exhausted authority | Stop autonomous creation for this root | public |
| `CANISTER_POOL_BLOCKED_CREATION_NOT_PENDING` | 1 | `CanisterPoolOps::retry_blocked_creation`; no pending creation exists | self | Inspect creation status before retry | public |
| `CANISTER_POOL_CREATION_RETRY_BLOCKED_REQUIRED` | 1 | `CanisterPoolOps::retry_blocked_creation`; progress is not Blocked | self | Retry only retained Blocked progress | public |
| `CANISTER_POOL_CREATION_RETRY_UNRESOLVED_EXPIRED_FORBIDDEN` | 1 | `CanisterPoolOps::retry_blocked_creation`; failure is UnresolvedAfterLedgerWindow | self | Resolve manually; never repeat the paid creation | public |
| `CANISTER_POOL_CREATION_CANCEL_KNOWN_UNAPPLIED_REQUIRED` | 1 | `CanisterPoolOps::cancel_known_unapplied_creation`; `creation_is_known_unapplied` is false | self | Cancel only a typed known-unapplied outcome | public |
| `CANISTER_POOL_CREATION_ROLLOVER_KNOWN_UNAPPLIED_REQUIRED` | 1 | `CanisterPoolOps::rollover_known_expired_creation`; `creation_is_known_unapplied_intent` is false | self | Roll over only the proved-unapplied intent | public |
| `CANISTER_POOL_CREATION_TIMESTAMP_EXHAUSTED` | 1 | `CanisterPoolOps::next_creation_timestamp`; high-water-mark `checked_add(1)` fails | self | Stop autonomous creation for this root | public |

The 21 rows sum to all 32 references in the range and add 21 exact meanings.
No message in this range interpolates a dynamic value.

## Exclusive Asset Handoff

| Exact candidate | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_HANDOFF_ALREADY_COMPLETE` | 1 | `CanisterPoolOps::begin_handoff`; a terminal receipt already exists for the asset | self | Inspect the retained handoff receipt | public |
| `CANISTER_POOL_HANDOFF_CREATION_PENDING` | 1 | `CanisterPoolOps::begin_handoff`; autonomous creation is still pending | self | Reconcile creation before transferring inventory authority | public |
| `CANISTER_POOL_HANDOFF_JOURNAL_ASSET_MISMATCH` | 1 | `CanisterPoolOps::begin_handoff`; exact journal retry finds a different asset status | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve journal and asset; fail closed | recent failure |
| `CANISTER_POOL_HANDOFF_ALREADY_PENDING` | 1 | `CanisterPoolOps::begin_handoff`; pending journal differs by canister or recipient | self | Complete or reconcile the pending handoff | public |
| `CANISTER_POOL_HANDOFF_ASSET_STATUS_INVALID` | 1 | `CanisterPoolOps::begin_handoff`; asset is neither Ready nor Failed | self | Hand off only a reset-terminal pooled asset | public |
| `CANISTER_POOL_HANDOFF_NOT_PENDING` | 1 | `CanisterPoolOps::complete_handoff`; no pending journal exists | self | Prepare/query the exact handoff first | public |
| `CANISTER_POOL_HANDOFF_COMPLETION_CANISTER_MISMATCH` / `CANISTER_POOL_HANDOFF_COMPLETION_RECIPIENT_MISMATCH` | 1 | `CanisterPoolOps::complete_handoff`; canister or recipient differs from pending authority | self for both leaves | Replay only the exact pending handoff | public |
| `CANISTER_POOL_HANDOFF_ASSET_AUTHORITY_MISMATCH` | 1 | `CanisterPoolOps::complete_handoff`; asset is not HandingOff to the retained recipient | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both records and fail closed | recent failure |
| `CANISTER_POOL_HANDOFF_RECEIPT_EXISTS` | 1 | `CanisterPoolOps::complete_handoff`; terminal receipt already exists | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the first receipt and reconcile exact retry | recent failure |

The nine sites produce ten new exact meanings and no projection or dynamic
public value.

## Store Deletion, Configuration And Shared Helpers

| Exact candidate or disposition | Sites | Current producer/meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_STORE_INVENTORY_CONFLICT` | 1 | `CanisterPoolOps::require_store`; asset is not the exact infrastructure Store in Store/deletion-pending state | self; reuses initialization conflict | Reconcile the exact Store principal and inventory row | public |
| `CANISTER_POOL_STORE_DELETION_ORIGIN_MISMATCH` | 2 | `CanisterPoolOps::begin_store_deletion` origin check and the origin arm of `CanisterPoolOps::complete_store_deletion` | self on begin; `COMPONENT_REGISTRY_STATE_INVALID` on retained completion | Preserve inventory and use only the exact Store asset | public or recent failure as stated |
| `CANISTER_POOL_STORE_DELETION_STATE_CONFLICT` | 1 | `CanisterPoolOps::begin_store_deletion`; status is neither Store nor exact pending-operation replay | self | Query and replay the retained Store deletion operation | public |
| `CANISTER_POOL_STORE_DELETION_PENDING_AUTHORITY_MISMATCH` | 1 | pending-operation arm of `CanisterPoolOps::complete_store_deletion` compound authority check | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the row and reconcile the exact operation | recent failure |
| `CANISTER_POOL_CONFIG_MINIMUM_ZERO` | 1 | `validate_config`; immutable pool `minimum_size` is zero | self | Configure a positive minimum | public |
| `CANISTER_POOL_CONFIG_MAXIMUM_BELOW_MINIMUM` | 1 | `validate_config`; immutable pool `maximum_size` is below its minimum | self | Raise the maximum or lower the minimum | public |
| `CANISTER_POOL_CONFIG_CANISTER_CYCLES_ZERO` | 1 | `validate_config`; per-asset creation cycles are zero | self | Configure a positive creation amount | public |
| `CANISTER_POOL_CREATION_ATTEMPT_COST_AUTHORITY_MISMATCH` | 1 | `require_creation_attempt`; settlement differs from retained cost authority | self | Finish only the exact retained settlement | public |
| `CANISTER_POOL_CREATION_ATTEMPT_PHASE_INVALID` | 1 | `require_creation_attempt`; progress is not Intent; reuses the `begin_creation_attempt` identity | self; reuses begin-attempt phase identity | Query and reconcile terminal/blocked progress | public |
| `CANISTER_POOL_CREATION_OPERATION_MISMATCH` | 1 | `require_creation_operation`; operation ID differs from pending creation | self | Replay with the exact pending operation ID | public |
| `CANISTER_POOL_CREATION_COST_AUTHORITY_PENDING` | 1 | `require_creation_cost_settled`; cost settlement remains retained | self | Settle the exact paid attempt first | public |
| `CANISTER_POOL_CREATION_INVENTORY_ADOPTION_MISSING` | 1 | `require_created_inventory_adoption`; Created principal lacks exact Created/PendingReset inventory | self | Adopt and verify the exact principal before commit | public |
| `CANISTER_POOL_MAXIMUM_EXHAUSTED` | 1 | `validate_new_asset_capacity`; standby capacity is exhausted for a new principal | self | Claim, hand off or raise policy before adding an asset | public |
| `CANISTER_POOL_ASSET_NOT_REGISTERED` | 1 | `required_asset`; requested physical Canister has no pool inventory row | self | Use/query a registered pool asset | public |
| `CANISTER_POOL_RECYCLE_INVENTORY_STATUS_MISMATCH` | 1 | `recycling_completion`; asset is neither exact terminal replay nor Recycling | self | Reconcile the physical asset before membership settlement | public |
| `CANISTER_POOL_RECYCLE_COMPONENT_OWNER_MISMATCH` | 1 | `recycling_completion`; retained claim names another Component | self | Settle only the exact workload owner | public |
| `CANISTER_POOL_RECYCLE_RESET_NOT_TERMINAL` | 1 | `recycling_completion`; reset remains Pending | self | Complete/query reset before membership settlement | public |

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

The adjacent 11-site workflow plus six-site refill owner are source-addressed
in [canister-pool-workflow-constructor-leaves.md](canister-pool-workflow-constructor-leaves.md).
Continue producer anchoring through the Store and Mirror families before the
complete coverage set is compressed many-to-one.
