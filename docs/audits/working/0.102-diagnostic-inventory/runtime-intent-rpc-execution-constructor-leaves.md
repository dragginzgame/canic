# Canic 0.102 Runtime Intent And RPC Execution Constructor Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger classifies all 19 production `InternalError`
constructor references in runtime intent orchestration and authorized root RPC
execution. It assigns no number and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/runtime/intent.rs` | 10 |
| `workflow/rpc/request/handler/execute.rs` | 9 |
| **Total** | **19** |

## Runtime Intent Orchestration

The ten workflow sites reduce to seven new exact meanings and three reuses of
the typed intent-store family:

| Exact candidate or disposition | Sites | Producer function/branch | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- | --- |
| `INTENT_CLEANUP_COUNT_OVERFLOW` | 2 | `IntentCleanupBatch::work_count` checked-add and `u64` conversion branches | `Invariant` / bounded cleanup accounting | `INTENT_STATE_INVALID` | Stop cleanup and inspect bounded batch accounting |
| reuse `INTENT_AGGREGATE_OVERFLOW` | 1 | `LocalIntentWorkflow::begin` reservation checked-add branch | typed intent aggregate invariant | `INTENT_STATE_INVALID` | Preserve the exact resource aggregate and do not reserve |
| `INTENT_LOCAL_CAPACITY_EXCEEDED` | 1 | `LocalIntentWorkflow::begin` limit predicate | `ResourceExhausted` / application reservation | self | Settle/release existing reservations or lower the request before retry |
| `INTENT_RESOURCE_NAMESPACE_RESERVED` | 1 | `ensure_consumer_resource_key` | `InvalidInput` / public application namespace | self | Use a resource key outside the protected `canic:` namespace |
| `INTENT_CANIC_RESOURCE_NAMESPACE_REQUIRED` | 1 | `ensure_canic_owned_resource_key` | `Invariant` / runtime authority namespace | `INTENT_STATE_INVALID` | Correct the internal caller to use the protected namespace |
| reuse `INTENT_APPLICATION_RECLAMATION_COUNT_OVERFLOW` | 1 | `IntentCleanupWorkflow::cleanup_due_batch` removed-record conversion | typed reclamation accounting invariant | `INTENT_STATE_INVALID` | Stop reclamation and inspect the bounded receipt page |
| `INTENT_APPLICATION_RECLAMATION_LIMIT_EXCEEDED` | 1 | `IntentCleanupWorkflow::cleanup_due_batch` requested-limit subtraction | `Invariant` / bounded cleanup page | `INTENT_STATE_INVALID` | Preserve records and repair a reclamation result above its requested limit |
| `INTENT_EXPIRY_PRIMARY_MISSING` / `INTENT_EXPIRY_PRIMARY_NOT_PENDING` | 1 | `IntentCleanupWorkflow::cleanup_due_batch` `abort_intent_if_pending` false branch | `Invariant` / expiry-index authority | `INTENT_STATE_INVALID` | Distinguish missing from terminal primary state, stop cleanup and rebuild only from canonical primary records |
| reuse `INTENT_EXPIRY_DEADLINE_OVERFLOW` | 1 | `IntentCleanupWorkflow::deadline_ns` | typed finite-expiry arithmetic | self | Reject an unrepresentable deadline before retaining a reservation |

The two cleanup-count conversions share one meaning because both reduce the
same bounded batch to its timer work count. The due-intent branch must split:
`abort_intent_if_pending` currently collapses a missing primary and a present
non-Pending primary into `false`, but those are different stable-state
contradictions. B4 must return a finite typed decision; it must not classify
the branch by checking error prose.

The deadline site reuses the creation-time expiry identity only if validation
moves before durable reservation. The maintained sequence currently reserves
and then schedules. B4 must prove the overflow rejection is pre-effect or add
an interruption-safe typed result that returns the retained intent identity;
the compact code cannot make a hidden reservation safe.

## Authorized Root RPC Execution

The nine RPC sites reduce to seven exact meanings:

| Exact candidate | Sites | Producer function/branch | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- | --- |
| `INTENT_PLACEMENT_ACK_ACTOR_MISMATCH` | 1 | `execute_placement_receipt_acknowledgement` `ActorMismatch` branch | `Forbidden` / placement receipt ownership | self | Acknowledge only a receipt owned by the transport caller |
| `INTENT_PLACEMENT_ACK_NOT_COMMITTED` | 1 | `execute_placement_receipt_acknowledgement` `NotCommitted` branch | `Conflict` / placement receipt state | self | Complete or reconcile the placement effect before acknowledgement |
| `INTENT_PLACEMENT_ACK_EFFECT_MISMATCH` | 1 | `execute_placement_receipt_acknowledgement` `NotPlacementEffect` branch | `Conflict` / receipt effect identity | self | Supply an operation whose terminal receipt is a placement-child effect |
| `RPC_COMPONENT_CALLER_REQUIRED` | 2 | `component_child_provision_request` and `component_child_recycle_request` missing-Component branches | `Forbidden` / Component-child authority | self | Invoke child provisioning/recycling from the exact registered Component, not the root |
| `RPC_COMPONENT_REGISTRY_AUTHORITY_MISSING` | 2 | `component_child_provision_request` and `component_child_recycle_request` missing-Registry branches | `Invariant` / authorized caller context | `COMPONENT_CHILD_AUTHORITY_INVALID` | Stop before lifecycle work and repair protected caller/Registry derivation |
| `RPC_COMPONENT_PARENT_AUTHORITY_MISSING` | 1 | `resolve_provision_parent` | `Invariant` / immediate-parent authority | `COMPONENT_CHILD_AUTHORITY_INVALID` | Stop before the effect and repair protected parent derivation |
| `RPC_COMPONENT_CHILD_RECYCLE_IN_PROGRESS` | 1 | `execute_recycle` `RootComponentChildRecycleOutcome::InProgress` branch | `Unavailable` / exact replay operation | self | Retry the same operation ID; never begin another recycle |

The provisioning and recycling caller checks share one identity because both
require the same registered Component authority and have the same remediation.
The two missing-Registry checks likewise share one invariant. Placement
acknowledgement decisions remain in the intent namespace because the durable
receipt state, not RPC transport, owns their meaning.

External lifecycle failures and response-stage failures are transparent typed
causes. `preserve_root_provision_recovery_required` and
`preserve_root_recycle_recovery_required` retain the original diagnostic while
requiring the exact replay record to enter recovery; neither helper receives a
wrapper code.

## Dynamic Public Context

The public-message subset is closed by Slice 61 of
[dynamic-public-context.md](dynamic-public-context.md): local capacity exposes
four values and the three placement-acknowledgement branches each expose the
submitted operation ID. The current reserved quantity needs one narrow
request-scoped capacity status; request-owned values are discarded.

Deadline scheduling, due-intent cleanup and abort context are lifecycle/timer
diagnostics, not public `Error.message` values. They remain in the guarded
runtime observation/lifecycle-log migration and do not receive dynamic-public-
context rows. Internal recovery logs likewise do not become public detail merely
because they retain operation, caller, Subnet, role, parent or error metadata.

## Reconciliation

All 19 direct sites now have one disposition. They add fourteen exact meanings,
reuse three intent-store identities and add no projection. The effective
constructor frontier moves from 2,198 to 2,217 classified sites and from 301 to
282 open sites. The qualified semantic set reaches 2,461 exact candidates plus
31 safe projections: 2,492 current symbolic identities.

## Required Tests

- exact cleanup-count reuse and bounded overflow rejection;
- local-capacity rejection with no additional reservation;
- application and Canic-owned namespace tests at both boundaries;
- missing versus non-Pending expiry-primary diagnostics;
- deadline overflow before durable reservation or exact recovery evidence for
  the retained identity;
- exhaustive placement-acknowledgement decision mapping;
- shared Component-caller and Registry-authority identities across provision
  and recycle;
- no lifecycle invocation when parent or Registry authority is absent; and
- exact recycle retry preserving the original recovery-required operation.

## Next Slice

Continue with RPC authorization and runtime-auth orchestration, while adding
the explicit dynamic-value rows identified here.
