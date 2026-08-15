# Canic 0.102 Component Directory Synchronization Ops Constructor Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger classifies all twenty-three production
`InternalError` constructor references in the root-owned durable Component
Directory synchronization journal. It assigns no number and changes no runtime
behavior.

| Production owner | Sites |
| --- | ---: |
| `ops/component_directory_synchronization/mod.rs` | 23 |
| **Total** | **23** |

The separately qualified workflow owns live Mirror, Registry, runtime and
Coordinator observations. This ops owner validates and commits the exact local
request, targets, cursor, pre-call intent, observation and terminal receipt.

## Progress And Observation Journal

Six constructor branches add seven exact meanings:

| Exact candidate | Sites | Producer function/branch | Action and retry |
| --- | ---: | --- | --- |
| `FLEET_DIRECTORY_SYNC_OPERATION_NOT_PREPARED` | 1 | `RootComponentDirectorySynchronizationOps::status`; status names no accepted durable synchronization operation | Prepare the exact operation before status or advancement |
| `FLEET_DIRECTORY_SYNC_PROGRESS_EXPECTATION_AHEAD` | 1 | `RootComponentDirectorySynchronizationOps::status`; request expects more synchronized Components than durable state contains | Reload status and retry from the durable cursor |
| `FLEET_DIRECTORY_SYNC_NEXT_TARGET_MISSING` | 1 | `RootComponentDirectorySynchronizationOps::advance`; advancement needs a target intent but the workflow supplied none | Reconstruct the exact next target from the accepted plan |
| `FLEET_DIRECTORY_SYNC_RESPONSE_INTENT_MISSING` | 1 | `RootComponentDirectorySynchronizationOps::record_synchronized`; a response arrives without a durable pre-call intent | Recover or prepare the intent; never infer an effect from the response |
| `FLEET_DIRECTORY_SYNC_OBSERVATION_INTENT_MISMATCH` / `FLEET_DIRECTORY_SYNC_OBSERVATION_TIME_INVALID` | 1 | `RootComponentDirectorySynchronizationOps::record_synchronized`; observed response differs from the full intent or predates its invocation | Preserve the intent and re-observe the exact target |
| `FLEET_DIRECTORY_SYNC_CURSOR_OVERFLOW` | 1 | `RootComponentDirectorySynchronizationOps::record_synchronized`; committed synchronized-Component count cannot advance | Stop and inspect durable cursor accounting |

The response branch must split payload equality from observation time. A broad
"observation differs" identity would merge contradictory authority with a
clock/order violation.

## Request And Acceptance Authority

The request-authority branch expands into six exact fields:

| Exact candidates | Sites | Producer function/authority group |
| --- | ---: | --- |
| `FLEET_DIRECTORY_SYNC_OPERATION_ID_INVALID` / `FLEET_DIRECTORY_SYNC_PLAN_HASH_INVALID` | 1 | `validate_request`; nonzero operation and protected plan identities |
| `FLEET_DIRECTORY_SYNC_REGISTRY_AUTHORITY_MISMATCH` / `FLEET_DIRECTORY_SYNC_REGISTRY_REVISION_REGRESSION` / `FLEET_DIRECTORY_SYNC_SOURCE_REGISTRY_HASH_INVALID` / `FLEET_DIRECTORY_SYNC_PUBLISHED_REGISTRY_HASH_INVALID` | same site | `validate_request`; one authority with a non-regressing, nonzero-hash Registry transition |

Acceptance adds four exact authority and capacity meanings:

| Exact candidates | Sites | Producer function/authority group |
| --- | ---: | --- |
| `FLEET_DIRECTORY_SYNC_ROOT_INVALID` / `FLEET_DIRECTORY_SYNC_FLEET_DIRECTORY_HASH_INVALID` / `FLEET_DIRECTORY_SYNC_PLANNED_TIME_INVALID` | 1 | `validate_acceptance`; real root, nonzero Directory authority and positive planning time |
| `FLEET_DIRECTORY_SYNC_TARGET_COUNT_LIMIT_EXCEEDED` | 1 | `validate_acceptance`; target count exceeds the Fleet Component plan bound |

The two target-validation branches expand into eight meanings:

| Exact candidates | Sites | Producer function/authority group |
| --- | ---: | --- |
| `FLEET_DIRECTORY_SYNC_TARGET_COMPONENT_ORDER_INVALID` / `FLEET_DIRECTORY_SYNC_TARGET_CANISTER_DUPLICATE` / `FLEET_DIRECTORY_SYNC_TARGET_ALLOCATION_OPERATION_DUPLICATE` | 1 | `validate_acceptance`; canonical Component order and globally unique target Canister/allocation operation |
| `FLEET_DIRECTORY_SYNC_TARGET_CANISTER_INVALID` / `FLEET_DIRECTORY_SYNC_TARGET_ALLOCATION_OPERATION_INVALID` / `FLEET_DIRECTORY_SYNC_TARGET_SOURCE_COMPONENT_MISMATCH` / `FLEET_DIRECTORY_SYNC_TARGET_SOURCE_REVISION_INVALID` / `FLEET_DIRECTORY_SYNC_TARGET_SOURCE_HASH_INVALID` | same site | `validate_acceptance`; every target binds one real Canister and allocation operation to an exact positive source Registry head |

The current adjacent-window check is sufficient for Component ordering but not
for Canister or allocation-operation uniqueness because those fields are not
the sort key. B4 must use a bounded whole-set uniqueness check and reject a
duplicate separated by another Component. It must not weaken the global
uniqueness requirement to match the present predicate.

## Exact Retry Authority

Two retry branches expand into seven exact meanings:

| Exact candidates | Sites | Producer function/authority group |
| --- | ---: | --- |
| `FLEET_DIRECTORY_SYNC_RETRY_ROOT_MISMATCH` / `FLEET_DIRECTORY_SYNC_RETRY_DIRECTORY_HASH_MISMATCH` / `FLEET_DIRECTORY_SYNC_RETRY_TARGETS_MISMATCH` | 1 | `require_exact_authority`; accepted retry preserves root, Fleet Directory and complete target set |
| `FLEET_DIRECTORY_SYNC_REQUEST_OPERATION_MISMATCH` / `FLEET_DIRECTORY_SYNC_REQUEST_PLAN_HASH_MISMATCH` / `FLEET_DIRECTORY_SYNC_REQUEST_SOURCE_REGISTRY_MISMATCH` / `FLEET_DIRECTORY_SYNC_REQUEST_PUBLISHED_REGISTRY_MISMATCH` | 1 | `require_request_authority`; every later command preserves the complete accepted request authority |

Zero/malformed initial authority is distinct from a valid but conflicting
retry. The invalid-input and conflict identities therefore do not collapse.

## Cursor And Next-Intent Authority

The cursor conversion reuses existing
`FLEET_DIRECTORY_SYNC_CURSOR_UNREPRESENTABLE`. The missing indexed target and
the next-intent compound expand into fourteen exact meanings:

| Exact candidates | Sites | Producer function/authority group |
| --- | ---: | --- |
| `FLEET_DIRECTORY_SYNC_CURSOR_TARGET_MISSING` | 1 | `validate_next_intent`; durable cursor selects no accepted target |
| `FLEET_DIRECTORY_SYNC_INTENT_INDEX_MISMATCH` / `FLEET_DIRECTORY_SYNC_INTENT_COMPONENT_MISMATCH` / `FLEET_DIRECTORY_SYNC_INTENT_CANISTER_MISMATCH` / `FLEET_DIRECTORY_SYNC_INTENT_ALLOCATION_OPERATION_MISMATCH` | 1 | `validate_next_intent`; exact next target identity and cursor |
| `FLEET_DIRECTORY_SYNC_INTENT_PREVIOUS_COMPONENT_MISMATCH` / `FLEET_DIRECTORY_SYNC_INTENT_PREVIOUS_REVISION_NOT_COVERED` | same site | `validate_next_intent`; previous observed head covers the accepted target source |
| `FLEET_DIRECTORY_SYNC_INTENT_REGISTRY_COMPONENT_MISMATCH` / `FLEET_DIRECTORY_SYNC_INTENT_REGISTRY_REVISION_NOT_ADVANCED` / `FLEET_DIRECTORY_SYNC_INTENT_REGISTRY_HASH_NOT_CHANGED` | same site | `validate_next_intent`; new head belongs to the target and advances revision and content |
| `FLEET_DIRECTORY_SYNC_INTENT_DIRECTORY_TIME_INVALID` / `FLEET_DIRECTORY_SYNC_INTENT_AUTHORITY_HASH_INVALID` / `FLEET_DIRECTORY_SYNC_INTENT_STARTED_TIME_MISMATCH` / `FLEET_DIRECTORY_SYNC_INTENT_STARTED_BEFORE_PLAN` | same site | `validate_next_intent`; Directory observation and invocation times plus authority hash bind the pre-call intent |

B4 must implement these as named predicates or one typed validation result.
The existing post-call runtime-coverage identities are not reused: a malformed
pre-call intent and a valid intent whose runtime failed to converge have
different authorities and recovery actions.

## Terminal State And Stable Validation

Seven source sites add six exact meanings, with target-count representation
shared by two sites:

| Exact candidate | Sites | Producer function/branch | Action and retry |
| --- | ---: | --- | --- |
| `FLEET_DIRECTORY_SYNC_TARGET_COUNT_UNREPRESENTABLE` | 2 | `target_count` or `validated_record`; target collection cannot fit the canonical `u32` response/state field | Preserve state and inspect bounded target accounting |
| `FLEET_DIRECTORY_SYNC_TERMINAL_TIME_INVALID` / `FLEET_DIRECTORY_SYNC_TERMINAL_COUNT_INCOMPLETE` | 1 | `terminal_record`; terminalization predates planning or cursor does not cover every target | Resume from exact cursor; do not fabricate completion |
| `FLEET_DIRECTORY_SYNC_TERMINAL_RECEIPT_HASH_INVALID` | 1 | `status_response`; recomputed terminal response hash differs from retained receipt authority | Preserve state and fail closed |
| `FLEET_DIRECTORY_SYNC_CURSOR_EXCEEDS_TARGETS` | 1 | `validated_record`; decoded durable cursor is beyond the accepted target set | Preserve state and repair/reinstall |
| `FLEET_DIRECTORY_SYNC_TERMINAL_TIME_MISSING` | 1 | `view_to_record`; complete state lacks its protected synchronization time | Preserve state and repair/reinstall |

## Stable Commit Mapping

The conflict adapter adds three exact meanings for the variants reachable from
Directory synchronization:

| Exact candidate | Producer function/typed cause | Action and retry |
| --- | --- | --- |
| `FLEET_DIRECTORY_SYNC_OTHER_OPERATION_ACTIVE` | `map_commit_error`: `ActiveOperationConflict` | Finish or recover the exact operation currently owning the Directory lane |
| `FLEET_DIRECTORY_SYNC_OPERATION_CONFLICT` | `map_commit_error`: `ConflictingOperation` | Replay only the byte-exact accepted operation |
| `FLEET_DIRECTORY_SYNC_ADVANCE_AUTHORITY_CONFLICT` | `map_commit_error`: `OperationChanged` | Reload durable state and recompute the exact next transition |

`PlacementConflict` and `PlacementCountOverflow` cannot be emitted by either
Directory synchronization store method. Their two generic mapping arms are
compile-time sediment on this path and receive no Directory-sync code. B4 must
narrow the commit error boundary or retain explicit unreachable handling; it
must not allocate misleading placement diagnostics to unreachable branches.

## Dynamic Public Context

Every direct message is static. The earlier zero-row closure in
[dynamic-public-context.md](dynamic-public-context.md) remains complete; this
slice adds no dynamic value.

## Reconciliation

All twenty-three direct sites have one disposition. They contain fifty-five
exact meanings, including one existing exact identity, and exclude one
unreachable constructor branch from allocation. The effective constructor frontier moves
from 2,429 to 2,452 classified sites and from 70 to 47 open sites. The qualified
semantic set reaches 2,661 exact candidates plus 31 safe projections: 2,692
current symbolic identities.

## Required Tests

- independently reject every initial request and Registry-transition field;
- independently reject root, Directory hash, planning time and target bound;
- reject adjacent and non-adjacent duplicate Canisters and allocation operations;
- distinguish invalid initial authority from every changed retry field;
- reconstruct the same accepted response after response loss;
- distinguish missing intent, changed intent and pre-invocation observation;
- independently reject all thirteen next-intent authority predicates;
- prove cursor advancement and target-count conversion are checked;
- reject incomplete terminal count/time and corrupted receipt hash;
- distinguish all three reachable stable commit conflicts; and
- prove unreachable placement commit variants allocate no Directory code.

## Next Slice

Continue with the remaining small runtime/control-plane adapters, starting with
the three-site Fleet activation workflow and placement-scaling workflow.
