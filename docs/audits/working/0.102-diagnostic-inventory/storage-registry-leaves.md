# Canic 0.102 Storage Registry Diagnostic Leaves

Date: 2026-08-13

## Status

This provisional B1 ledger covers the bounded ICP-refill, Placement Index and
feature-gated Sharding storage owners at immutable baseline `v0.101.53`. It
allocates no numbers. `StorageOpsError` is transparent and receives no code.

## ICP Refill Records

Ten current `IcpRefillRecordOpsError` variants reduce to seven semantic leaves:

| Candidate label | Current typed source | Public projection | Action and retry |
| --- | --- | --- | --- |
| `ICP_REFILL_CONCURRENT_OPERATION` | `ConcurrentOperation` | self | Resume/await the existing source-target operation |
| `ICP_REFILL_CYCLES_SENT_OVERFLOW` | `CyclesSentOverflow` | `ICP_REFILL_STATE_INVALID` | Reject the unrepresentable durable value; no blind retry |
| `ICP_REFILL_INDEX_DUPLICATED` | `DuplicateActiveIndex`, `DuplicateOperationIndex` | `ICP_REFILL_STATE_INVALID` | Fail index rebuild; inspect canonical records |
| `ICP_REFILL_INDEX_INVALID` | `IndexRecordMissing`, `IndexRecordMismatch` | `ICP_REFILL_STATE_INVALID` | Fail closed; rebuild/repair only from canonical records |
| `ICP_REFILL_ID_EXHAUSTED` | `IdOverflow` | self | Stop new operations; operator intervention is required |
| `ICP_REFILL_OPERATION_CONFLICT` | `OperationConflict`, `RetryRequestMismatch` | self | Reuse an operation ID only with its exact original request |
| `ICP_REFILL_RECORD_NOT_FOUND` | `RecordNotFound` | `ICP_REFILL_STATE_INVALID` | Stop the transition; do not fabricate or skip the record |

The two duplicate-index variants have the same canonical-record owner and
repair. Missing and mismatched index rows likewise represent one invalid
derived index. Full-operation and selected-field retry checks both enforce the
same immutable operation-ID contract. Record IDs, principals, Nat values,
field names and request/record values do not enter the compact error.

`IcpRefillRecord.error_code` remains separate durable operational state. Its
bounded `error_message` is classified during B5; neither is replaced or
interpreted by these request diagnostics.

## Placement Index Registry

| Candidate label | Current typed source | Public projection | Action and retry |
| --- | --- | --- | --- |
| `PLACEMENT_INDEX_KEY_INVALID` | `InvalidKey(String)` | self | Submit pool/key values within their byte bounds |
| `PLACEMENT_INDEX_KEY_BOUND` | `KeyBound` | self | Reuse the existing bound instance or select another key |
| `PLACEMENT_INDEX_PROVISIONAL_PRINCIPAL_MISMATCH` | `ProvisionalPidMismatch` | self | Resume only the exact pending claim and provisional Canister |

B4 must preserve `BoundedStringError::TooLong` as a typed cause instead of
stringifying it into `InvalidKey`. Pool names, logical keys and principals are
available from status/claim records and are omitted from public errors.

## Sharding Registry

The feature-gated owner has eight live variants:

| Candidate label | Current typed source | Public projection | Action and retry |
| --- | --- | --- | --- |
| `SHARDING_KEY_INVALID` | `InvalidKey(String)` | self | Submit pool/partition values within their byte bounds |
| `SHARDING_POOL_MISMATCH` | `PoolMismatch` | self | Use a shard registered in the requested pool |
| `SHARDING_SHARD_NOT_FOUND` | `ShardNotFound` | self | Register/select an existing shard |
| `SHARDING_SLOT_OCCUPIED` | `SlotOccupied` | self | Select a free slot or replay the exact shard |
| `SHARDING_PARTITION_UNASSIGNED` | `PartitionKeyNotAssigned` | self | Assign the partition before lookup |
| `SHARDING_SHARD_CONFLICT` | `ShardConflict` | self | Replay the exact existing shard declaration |
| `SHARDING_ASSIGNMENT_COUNT_UNDERFLOW` | `AssignmentCountUnderflow` | `SHARDING_STATE_INVALID` | Stop mutation; repair derived count from canonical assignments |
| `SHARDING_ASSIGNMENT_COUNT_OVERFLOW` | `AssignmentCountOverflow` | `SHARDING_STATE_INVALID` | Stop mutation; repair derived count/capacity state |

As with Placement Index, B4 replaces the `InvalidKey(String)` bridge with the
typed bounded-string cause. Pool, partition, slot and principal values never
cross the compact public boundary. Underflow and overflow remain distinct
internal leaves because they identify opposite broken accounting invariants,
but share the safe public state projection.

## Current Count

This pass contributes **18 exact semantic candidates**:

- seven ICP-refill record leaves;
- three Placement Index leaves; and
- eight Sharding leaves.

It introduces two additional safe projections:

- `ICP_REFILL_STATE_INVALID`; and
- `SHARDING_STATE_INVALID`.

The unconditional allocation must still reserve Sharding codes if the feature
is a maintained release-Wasm surface. Feature gating is not permission to
reuse a code with another meaning.

## Required Tests

- exhaustive variant-to-code tables for all three owners;
- identical operation-ID payload conflicts map identically at creation and
  retry-validation boundaries;
- derived index corruption is masked and never treated as absence;
- bounded key failures preserve typed causes and omit submitted values;
- exact pending-claim/principal conflict tests for Placement Index;
- Sharding underflow and overflow remain separately observable internally;
  and
- the disabled Sharding feature neither exposes dead format strings nor
  changes allocated meanings.
