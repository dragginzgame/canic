# Canic 0.102 Intent Store Diagnostic Leaves

Date: 2026-08-15

## Status

This provisional B1 ledger covers all 51 production-reachable
`IntentStoreOpsError` variants at immutable baseline `v0.101.53`. It allocates
no numbers. Intent IDs, resource keys, operation IDs, timestamps, schema
versions, counters and index values never enter the compact error.

## Public Projection Rule

The store has two audiences:

- request/state-machine conflicts whose compact identity tells an authorized
  caller how to correct or resume an operation; and
- contradictions among primary records, derived indexes and metadata that
  must fail closed and project publicly to `INTENT_STATE_INVALID`.

Every source variant retains its exact internal numeric identity. The masking
rule below does not permit a caller to interpret `INTENT_STATE_INVALID` as
absence, expiry, completion or permission to repeat an effect. Exact masked
codes require the approved bounded numeric runtime observation.

## Request And State-Machine Leaves

These labels are safe as their own public projection:

| Candidate label | Current typed source | Action and retry |
| --- | --- | --- |
| `INTENT_CONFLICT` | `Conflict` | Replay the exact existing intent payload |
| `INTENT_EXPIRED` | `Expired` | Start a new admitted operation; do not revive the expired intent |
| `INTENT_ID_EXHAUSTED` | `IdOverflow` | Stop allocation; operator intervention is required |
| `INTENT_TRANSITION_INVALID` | `InvalidTransition` | Inspect the exact state and request only its admitted next transition |
| `INTENT_NOT_FOUND` | `NotFound` | Use an existing intent; absence is not success |
| `INTENT_SETTLEMENT_DUPLICATED` | `RepeatedSettlementIntent` | Deduplicate the settlement batch |
| `INTENT_RESOURCE_TOTAL_CAPACITY_REACHED` | `ResourceTotalRecordCapacityReached` | Free/reclaim a resource-total slot before retry |
| `INTENT_EXPIRY_DEADLINE_OVERFLOW` | `ExpiryDeadlineOverflow` | Use a bounded creation time/TTL |
| `INTENT_RECEIPT_CONFLICT` | `ReceiptBackedConflict` | Replay only the exact operation payload |
| `INTENT_RECEIPT_EVIDENCE_CONFLICT` | `ReceiptBackedEvidenceConflict` | Preserve the first terminal evidence; reject contradictory evidence |
| `INTENT_RECEIPT_OWNERSHIP_MISMATCH` | `ReceiptBackedOwnershipMismatch` | Use the application/Canic-owned resource path matching the intent |
| `INTENT_APPLICATION_RECEIPT_CAPACITY_UNAVAILABLE` | `ApplicationReceiptEligibilityCapacityUnavailable` | Reclaim eligible application receipts before reserving more terminal capacity |

The current special conversion of only
`ResourceTotalRecordCapacityReached` to broad `ResourceExhausted` must become
an exhaustive typed projection. No class decision may depend on formatted
`IntentStoreOpsError` text.

## Aggregate, Pending And Expiry Invariants

All labels in this section project to `INTENT_STATE_INVALID`. Stop mutation and
reconstruct/check the canonical primary records; do not repair by guessing
from a derived counter or index.

| Exact identity | Current typed owner |
| --- | --- |
| `INTENT_AGGREGATE_UNDERFLOW` | `IntentStoreOpsError::AggregateUnderflow` |
| `INTENT_AGGREGATE_OVERFLOW` | `IntentStoreOpsError::AggregateOverflow` |
| `INTENT_PENDING_INDEX_MISSING` | `IntentStoreOpsError::PendingIndexMissing` |
| `INTENT_PENDING_INDEX_EXISTS` | `IntentStoreOpsError::PendingIndexExists` |
| `INTENT_PENDING_INDEX_MISMATCH` | `IntentStoreOpsError::PendingIndexMismatch` |
| `INTENT_PENDING_TOTAL_MISMATCH` | `IntentStoreOpsError::PendingTotalMismatch` |
| `INTENT_RESOURCE_TOTAL_LIMIT_EXCEEDED` | `IntentStoreOpsError::ResourceTotalRecordLimitExceeded` |
| `INTENT_EXPIRY_INDEX_EXISTS` | `IntentStoreOpsError::ExpiryIndexExists` |
| `INTENT_EXPIRY_INDEX_MISSING` | `IntentStoreOpsError::ExpiryIndexMissing` |
| `INTENT_EXPIRY_INDEX_VALUE_MISMATCH` | `IntentStoreOpsError::ExpiryIndexValueMismatch` |
| `INTENT_EXPIRY_INDEX_KEY_MISMATCH` | `IntentStoreOpsError::ExpiryIndexKeyMismatch` |
| `INTENT_TTL_FREE_IN_EXPIRY_INDEX` | `IntentStoreOpsError::TtlFreeIntentInExpiryIndex` |
| `INTENT_TOTALS_MISSING` | `IntentStoreOpsError::TotalsMissing` |

Capacity reached is an admitted resource condition and is listed above;
persisted record count above the limit is a contradictory state invariant and
remains distinct here.

## Placement Acknowledgement Invariants

These six exact internal leaves project to `INTENT_STATE_INVALID`:

| Exact identity | Current typed owner |
| --- | --- |
| `INTENT_PLACEMENT_ACK_INDEX_EXISTS` | `IntentStoreOpsError::PlacementAcknowledgementIndexExists` |
| `INTENT_PLACEMENT_ACK_INDEX_MISSING` | `IntentStoreOpsError::PlacementAcknowledgementIndexMissing` |
| `INTENT_PLACEMENT_ACK_INDEX_VALUE_MISMATCH` | `IntentStoreOpsError::PlacementAcknowledgementIndexValueMismatch` |
| `INTENT_PLACEMENT_ACK_INDEX_UNEXPECTED` | `IntentStoreOpsError::PlacementAcknowledgementUnexpectedIndex` |
| `INTENT_PLACEMENT_ACK_PRIMARY_MISSING` | `IntentStoreOpsError::PlacementAcknowledgementPrimaryMissing` |
| `INTENT_PLACEMENT_ACK_PRIMARY_MISMATCH` | `IntentStoreOpsError::PlacementAcknowledgementPrimaryMismatch` |

An index entry never proves that its primary intent committed. Missing or
contradictory primary state cannot be treated as an acknowledged placement.

## Receipt Replay And Eligibility Invariants

All labels below project to `INTENT_STATE_INVALID` and stop reclamation,
replay or settlement until the exact same-release state is reconciled:

| Exact identity | Current typed owner |
| --- | --- |
| `INTENT_RECEIPT_RECORD_LIMIT_EXCEEDED` | `IntentStoreOpsError::ReceiptBackedRecordLimitExceeded` |
| `INTENT_APPLICATION_REPLAY_MISSING` | `IntentStoreOpsError::ApplicationReceiptReplayMissing` |
| `INTENT_APPLICATION_REPLAY_PRIMARY_MISSING` | `IntentStoreOpsError::ApplicationReceiptReplayPrimaryMissing` |
| `INTENT_APPLICATION_REPLAY_UNEXPECTED` | `IntentStoreOpsError::ApplicationReceiptReplayUnexpected` |
| `INTENT_APPLICATION_REPLAY_IDENTITY_MISMATCH` | `IntentStoreOpsError::ApplicationReceiptReplayIdentityMismatch` |
| `INTENT_APPLICATION_REPLAY_SCHEMA_MISMATCH` | `IntentStoreOpsError::ApplicationReceiptReplaySchemaMismatch` |
| `INTENT_APPLICATION_ELIGIBILITY_MISSING` | `IntentStoreOpsError::ApplicationReceiptEligibilityMissing` |
| `INTENT_APPLICATION_ELIGIBILITY_EXISTS` | `IntentStoreOpsError::ApplicationReceiptEligibilityExists` |
| `INTENT_APPLICATION_ELIGIBILITY_PRIMARY_MISMATCH` | `IntentStoreOpsError::ApplicationReceiptEligibilityPrimaryMismatch` |
| `INTENT_APPLICATION_ELIGIBILITY_IDENTITY_MISMATCH` | `IntentStoreOpsError::ApplicationReceiptEligibilityIdentityMismatch` |
| `INTENT_APPLICATION_ELIGIBILITY_SCHEMA_MISMATCH` | `IntentStoreOpsError::ApplicationReceiptEligibilitySchemaMismatch` |
| `INTENT_APPLICATION_ELIGIBILITY_BINDING_MISMATCH` | `IntentStoreOpsError::ApplicationReceiptEligibilityBindingMismatch` |
| `INTENT_APPLICATION_ELIGIBILITY_REVISION_MISMATCH` | `IntentStoreOpsError::ApplicationReceiptEligibilityRevisionMismatch` |
| `INTENT_APPLICATION_ELIGIBILITY_TIMESTAMP_OVERFLOW` | `IntentStoreOpsError::ApplicationReceiptEligibilityOverflow` |
| `INTENT_APPLICATION_ELIGIBILITY_RESERVATION_OVERFLOW` | `IntentStoreOpsError::ApplicationReceiptEligibilityReservationOverflow` |
| `INTENT_APPLICATION_RECLAMATION_COUNT_OVERFLOW` | `IntentStoreOpsError::ApplicationReceiptReclamationCountOverflow` |
| `INTENT_RECEIPT_RECORD_SCHEMA_MISMATCH` | `IntentStoreOpsError::ReceiptBackedRecordSchemaMismatch` |
| `INTENT_PAYLOAD_BINDING_SCHEMA_UNSUPPORTED` | `IntentStoreOpsError::UnsupportedPayloadBindingSchema` |
| `INTENT_TERMINAL_EVIDENCE_SCHEMA_UNSUPPORTED` | `IntentStoreOpsError::UnsupportedTerminalEvidenceSchema` |

`ReceiptBackedRecordLimitExceeded` represents an already-invalid count above
the hard limit; the separate application eligibility capacity leaf is the
admitted pre-reservation resource condition. Unsupported schema tags are
current-layout corruption at the reinstall-only boundary, not migration
requests and not permission to add compatibility decoders.

## Store Schema

| Exact identity | Current typed source | Public projection |
| --- | --- | --- |
| `INTENT_STORE_SCHEMA_MISMATCH` | `IntentStoreOpsError::SchemaMismatch` | `INTENT_STATE_INVALID` |

Pre-1.0 release transitions remain reinstall-only; B5 must not add a legacy
decoder, state migration or fallback tag.

## Current Count

This pass contributes **51 exact semantic candidates** and one additional safe
projection, `INTENT_STATE_INVALID`:

- 12 request/state-machine/resource leaves exposed exactly;
- 13 aggregate, pending, expiry and total invariants;
- six placement-acknowledgement invariants;
- 19 receipt replay, eligibility and schema invariants; and
- one store-schema invariant.

## Required Tests

- exhaustive 51-variant mapping with no formatted classification;
- exact conflict, expiry, not-found and capacity public identities;
- every primary/index/metadata contradiction masks to
  `INTENT_STATE_INVALID` while retaining exact numeric observability;
- missing index or primary state never implies success or absence of an effect;
- record-limit corruption remains distinct from pre-reservation capacity;
- unsupported schema tags fail closed without compatibility decoding; and
- interruption/retry tests continue to prove that diagnostic projection does
  not alter intent commitment or receipt replay decisions.
