# Canic 0.102 Fleet Coordinator Root-Deletion Constructor Leaves

Date: 2026-08-13

## Status

This B1 evidence ledger classifies the dedicated Coordinator root-deletion
owner at
`crates/canic-control-plane/src/ops/fleet_coordinator/root_deletion/mod.rs`.
It assigns no number and changes no runtime behavior.

The source has 21 direct `InternalError::*` constructor references and ten
calls to the parent `receipt_invariant` funnel. Generic record lookup and
response hashing receive typed family dispositions below; neither dynamic
label is allowed to survive as public diagnostic text.

## Public Transition And Lookup Boundaries

This table assigns every one of the 21 direct constructor references. Compound
request/retry equality is expanded by protected field so one broad conflict
cannot hide the authority that changed.

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_DELETION_READINESS_INTENT_RETRY_FINAL_INVENTORY_CONFLICT` / `ROOT_DELETION_READINESS_INTENT_RETRY_STORE_DELETION_CONFLICT` / `ROOT_DELETION_READINESS_INTENT_RETRY_CYCLES_BEFORE_CONFLICT` / `ROOT_DELETION_READINESS_INTENT_RETRY_TARGET_CONFLICT` / `ROOT_DELETION_READINESS_INTENT_RETRY_RESERVED_CYCLES_CONFLICT` / `ROOT_DELETION_READINESS_INTENT_RETRY_IDLE_BURN_CONFLICT` / `ROOT_DELETION_READINESS_INTENT_RETRY_FREEZING_THRESHOLD_CONFLICT` / `ROOT_DELETION_READINESS_INTENT_RETRY_PREPARED_TIME_CONFLICT` | 1 | Exact readiness-intent retry changes one of eight retained request-authority fields after root/operation lookup | self for every exact leaf | Replay only the original intent request | public |
| `ROOT_DELETION_READINESS_INTENT_FINAL_INVENTORY_MISMATCH` / `ROOT_DELETION_READINESS_INTENT_STORE_DELETION_HASH_MISSING` / `ROOT_DELETION_READINESS_INTENT_CYCLES_BEFORE_INVALID` / `ROOT_DELETION_READINESS_INTENT_TARGET_INVALID` / `ROOT_DELETION_READINESS_INTENT_TARGET_MISMATCH` / `ROOT_DELETION_READINESS_INTENT_RESERVED_CYCLES_NONZERO` / `ROOT_DELETION_READINESS_INTENT_PREPARED_BEFORE_FINAL_INVENTORY` / `ROOT_DELETION_READINESS_INTENT_RECORDED_BEFORE_PREPARED` | 1 | Readiness-intent admission merges final inventory, Store deletion, cycles target/reserve and two time edges | self for every exact leaf | Correct the exact request field before any cycle reclamation | public |
| `ROOT_DELETION_READINESS_RETRY_INTENT_HASH_CONFLICT` / `ROOT_DELETION_READINESS_RETRY_CYCLES_AFTER_CONFLICT` / `ROOT_DELETION_READINESS_RETRY_RECLAIMED_TIME_CONFLICT` | 1 | Exact readiness retry changes retained intent hash, reclaimed balance or reclamation time | self for every exact leaf | Replay only the original readiness request | public |
| `ROOT_DELETION_READINESS_INTENT_UNAVAILABLE` | 1 | Readiness recording has no retained Coordinator readiness intent | self | Prepare/query the exact intent first | public |
| `ROOT_DELETION_READINESS_EXPECTED_INTENT_HASH_MISMATCH` / `ROOT_DELETION_READINESS_CYCLES_AFTER_EXCEED_BEFORE` / `ROOT_DELETION_READINESS_CYCLES_AFTER_EXCEED_TARGET` / `ROOT_DELETION_READINESS_RECLAIMED_BEFORE_PREPARED` / `ROOT_DELETION_READINESS_RECORDED_BEFORE_RECLAIMED` | 1 | Readiness recording merges intent linkage, two cycle bounds and two time edges | self for every exact leaf | Preserve the intent and correct the exact observation | public |
| `ROOT_DELETION_EXECUTION_RETRY_EXECUTOR_CONFLICT` / `ROOT_DELETION_EXECUTION_RETRY_READINESS_CONFLICT` / `ROOT_DELETION_EXECUTION_RETRY_MODULE_CONFLICT` / `ROOT_DELETION_EXECUTION_RETRY_CONTROLLERS_CONFLICT` / `ROOT_DELETION_EXECUTION_RETRY_CYCLES_AFTER_CONFLICT` / `ROOT_DELETION_EXECUTION_RETRY_RESERVED_CYCLES_CONFLICT` / `ROOT_DELETION_EXECUTION_RETRY_IDLE_BURN_CONFLICT` / `ROOT_DELETION_EXECUTION_RETRY_FREEZING_THRESHOLD_CONFLICT` | 1 | Exact execution retry changes executor or one of seven retained execution-request fields | self for every exact leaf | Replay only the original execution intent | public |
| `ROOT_DELETION_READINESS_UNAVAILABLE` | 1 | Execution begins without terminal Coordinator readiness | self; reuses the qualified readiness identity | Complete/query exact readiness first | public |
| `ROOT_DELETION_EXECUTION_UNAVAILABLE` | 2 | Execution status or completion has no retained execution intent | self; both sites share one exact meaning | Begin/query exact execution first | public |
| `ROOT_DELETION_COMPLETION_RETRY_EXECUTOR_CONFLICT` / `ROOT_DELETION_COMPLETION_RETRY_EXECUTION_HASH_CONFLICT` / `ROOT_DELETION_COMPLETION_RETRY_ABSENCE_TIME_CONFLICT` | 1 | Exact completion retry changes executor, execution hash or observed-absence time | self for every exact leaf | Replay only the original completion evidence | public |
| `ROOT_DELETION_COMPLETION_EXECUTOR_MISMATCH` / `ROOT_DELETION_COMPLETION_EXECUTION_HASH_MISMATCH` / `ROOT_DELETION_COMPLETION_ABSENCE_BEFORE_EXECUTION` / `ROOT_DELETION_COMPLETION_TIME_BEFORE_ABSENCE` | 1 | Completion admission merges executor/hash authority and two destructive time edges | self for every exact leaf | Preserve execution intent and correct the exact completion evidence | public |
| `ROOT_DELETION_UNAVAILABLE` | 1 | Deletion-status query has no terminal deletion receipt | self | Complete/query the exact deletion operation | public |
| `ROOT_DELETION_CALLER_MISMATCH` | 1 | Readiness caller differs from the root being deleted | self | Invoke as the exact protected root | public |
| `ROOT_DELETION_COORDINATOR_MISMATCH` | 1 | Supplied runtime Coordinator differs from protected Fleet authority | self; generalizes the qualified preparation-only name without allocating a second meaning | Use the exact protected Coordinator | public |
| `ROOT_DELETION_REGISTRY_ROOT_NOT_REMOVED` | 1 | Physical deletion preparation lacks an exact `Removed` Fleet Registry row | self | Complete logical Registry removal first | public |
| `ROOT_REMOVAL_PUBLICATION_RECEIPT_MISSING` | 1 | Removed root lacks its exact logical-removal publication receipt | self | Recover/query the exact logical removal receipt | public |
| `ROOT_DELETION_READINESS_INTENT_IDENTITY_CONFLICT` / `ROOT_DELETION_READINESS_IDENTITY_CONFLICT` / `ROOT_DELETION_EXECUTION_IDENTITY_CONFLICT` / `ROOT_DELETION_RECEIPT_IDENTITY_CONFLICT` | 1 | Generic record lookup finds the requested root or operation under different counterpart authority | self for the exact typed record family | Preserve operation/root and query the owning record family | public |
| `ROOT_DELETION_FREEZING_RESERVE_OVERFLOW` | 1 | Idle burn multiplied by freezing threshold overflows `u128` | self | Supply bounded observed metrics; perform no transfer | public |
| `ROOT_DELETION_EXECUTION_RESERVE_OVERFLOW` | 1 | Freezing reserve plus fixed execution reserve overflows `u128` | self | Supply bounded observed metrics; perform no transfer | public |
| `ROOT_DELETION_EXECUTION_READINESS_HASH_MISMATCH` / `ROOT_DELETION_EXECUTION_MODULE_HASH_MISSING` / `ROOT_DELETION_EXECUTION_CONTROLLERS_NONCANONICAL` / `ROOT_DELETION_EXECUTION_EXECUTOR_NOT_CONTROLLER` / `ROOT_DELETION_EXECUTION_CYCLES_EXCEED_BEFORE` / `ROOT_DELETION_EXECUTION_CYCLES_EXCEED_TARGET` / `ROOT_DELETION_EXECUTION_RESERVED_CYCLES_MISMATCH` / `ROOT_DELETION_EXECUTION_IDLE_BURN_MISMATCH` / `ROOT_DELETION_EXECUTION_FREEZING_THRESHOLD_MISMATCH` / `ROOT_DELETION_EXECUTION_TARGET_MISMATCH` / `ROOT_DELETION_EXECUTION_PREPARED_BEFORE_READINESS` | 1 | Execution admission merges readiness, module/controller authority, cycles metrics/target and preparation time | self for every exact leaf | Preserve readiness and correct the exact execution field | public |
| `ROOT_DELETION_READINESS_INTENT_ENCODING_FAILED` / `ROOT_DELETION_READINESS_ENCODING_FAILED` / `ROOT_DELETION_EXECUTION_ENCODING_FAILED` / `ROOT_DELETION_RECEIPT_ENCODING_FAILED` | 1 | Generic response hashing cannot canonically encode one of four typed root-deletion records | `COMPONENT_REGISTRY_STATE_INVALID` for the exact record family | Preserve the record and fail closed before commit | recent failure |

The 20 rows sum to all 21 direct source references. Candidate-column
extraction produces 68 unique exact labels. Two labels reuse existing exact
meanings: readiness unavailable and the Coordinator-authority mismatch. The
other 66 are new; no safe projection is added.

## Durable History Validation

This table assigns all ten calls to `receipt_invariant`. Compound persisted
record validators are expanded field by field. The execution-request wrapper
is transparent because the public validator already computes the exact leaf.

| Exact candidate or disposition | Calls | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_DELETION_READINESS_INTENT_IDENTITY_DUPLICATE` / `ROOT_DELETION_READINESS_IDENTITY_DUPLICATE` / `ROOT_DELETION_EXECUTION_IDENTITY_DUPLICATE` / `ROOT_DELETION_RECEIPT_IDENTITY_DUPLICATE` | 1 | One typed record collection contains more than one row matching the same root/operation identity | `COMPONENT_REGISTRY_STATE_INVALID` for the exact family | Preserve collection and fail closed | recent failure |
| `ROOT_DELETION_READINESS_INTENT_ORDER_NONCANONICAL` / `ROOT_DELETION_READINESS_ORDER_NONCANONICAL` / `ROOT_DELETION_EXECUTION_ORDER_NONCANONICAL` / `ROOT_DELETION_RECEIPT_ORDER_NONCANONICAL` | 1 | One of four durable record collections is not in strict root-principal order | `COMPONENT_REGISTRY_STATE_INVALID` for the exact family | Preserve collection and fail closed | recent failure |
| `ROOT_DELETION_READINESS_INTENT_COORDINATOR_MISMATCH` / `ROOT_DELETION_READINESS_INTENT_FINAL_INVENTORY_MISMATCH` / `ROOT_DELETION_READINESS_INTENT_STORE_DELETION_HASH_MISSING` / `ROOT_DELETION_READINESS_INTENT_CYCLES_BEFORE_INVALID` / `ROOT_DELETION_READINESS_INTENT_TARGET_INVALID` / `ROOT_DELETION_READINESS_INTENT_TARGET_MISMATCH` / `ROOT_DELETION_READINESS_INTENT_RESERVED_CYCLES_NONZERO` / `ROOT_DELETION_READINESS_INTENT_PREPARED_BEFORE_FINAL_INVENTORY` / `ROOT_DELETION_READINESS_INTENT_RECORDED_BEFORE_PREPARED` / `ROOT_DELETION_READINESS_INTENT_HASH_MISMATCH` | 1 | Stored readiness intent merges Coordinator, final/Store authority, cycles target/reserve, two time edges and canonical hash | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf; eight request leaves reuse public admission identities | Preserve receipt and identify the exact failed field | recent failure |
| `ROOT_DELETION_READINESS_INTENT_MISSING` | 1 | Stored readiness receipt has no retained predecessor intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed | recent failure |
| `ROOT_DELETION_READINESS_EXPECTED_INTENT_HASH_MISMATCH` / `ROOT_DELETION_READINESS_COORDINATOR_MISMATCH` / `ROOT_DELETION_READINESS_FINAL_INVENTORY_MISMATCH` / `ROOT_DELETION_READINESS_STORE_DELETION_MISMATCH` / `ROOT_DELETION_READINESS_CYCLES_BEFORE_MISMATCH` / `ROOT_DELETION_READINESS_TARGET_MISMATCH` / `ROOT_DELETION_READINESS_RESERVED_CYCLES_MISMATCH` / `ROOT_DELETION_READINESS_IDLE_BURN_MISMATCH` / `ROOT_DELETION_READINESS_FREEZING_THRESHOLD_MISMATCH` / `ROOT_DELETION_READINESS_CYCLES_AFTER_EXCEED_BEFORE` / `ROOT_DELETION_READINESS_CYCLES_AFTER_EXCEED_TARGET` / `ROOT_DELETION_READINESS_RECLAIMED_BEFORE_PREPARED` / `ROOT_DELETION_READINESS_RECORDED_BEFORE_RECLAIMED` / `ROOT_DELETION_READINESS_HASH_MISMATCH` | 1 | Stored readiness merges exact intent copy, cycle bounds, time ordering and canonical hash | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf; five admission leaves are reused | Preserve receipt/intent and identify the exact failed field | recent failure |
| `ROOT_DELETION_EXECUTION_READINESS_MISSING` | 1 | Stored execution intent has no terminal readiness predecessor | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve execution and fail closed | recent failure |
| transparent: exact typed root-deletion execution validation | 1 | Durable-history adapter currently replaces the exact public execution-authority leaf | preserve the exact nested projection | Remove the string adapter and propagate the typed leaf | recent failure |
| `ROOT_DELETION_EXECUTION_HASH_MISMATCH` | 1 | Stored execution-intent hash differs from canonical bytes | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve execution intent and fail closed | recent failure |
| `ROOT_DELETION_EXECUTION_INTENT_MISSING` | 1 | Stored deletion receipt has no retained execution intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve deletion receipt and fail closed | recent failure |
| `ROOT_DELETION_RECEIPT_COORDINATOR_MISMATCH` / `ROOT_DELETION_RECEIPT_EXECUTOR_MISMATCH` / `ROOT_DELETION_RECEIPT_READINESS_HASH_MISMATCH` / `ROOT_DELETION_RECEIPT_EXECUTION_HASH_MISMATCH` / `ROOT_DELETION_RECEIPT_MODULE_HASH_MISMATCH` / `ROOT_DELETION_RECEIPT_CONTROLLERS_MISMATCH` / `ROOT_DELETION_RECEIPT_CYCLES_AFTER_MISMATCH` / `ROOT_DELETION_RECEIPT_ABSENCE_BEFORE_EXECUTION` / `ROOT_DELETION_RECEIPT_COMPLETION_BEFORE_ABSENCE` / `ROOT_DELETION_RECEIPT_HASH_MISMATCH` | 1 | Stored terminal receipt merges Coordinator/executor, readiness/execution/module/controller/cycles authority, two time edges and canonical hash | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve receipt/execution and identify the exact failed field | recent failure |

The ten rows sum to all ten hidden calls. One call is a transparent nested
validator. Candidate-column extraction produces 46 unique exact labels for the
other nine calls. Thirteen reuse public-admission identities from the preceding
table and 33 are new; no safe projection is added.

## Dynamic Public Context

The direct source has two dynamic message selectors:

- `find_root_deletion_record` formats a closed local record-family `label`.
  The affected caller already selected the status or mutation family, so the
  discriminator is caller-derivable. It must become the four exact typed
  identity-conflict leaves above, not a retained text detail.
- `response_hash` formats a closed local record-family `label` plus a Candid
  encoder cause. The record family becomes the four exact encoding leaves
  above. The nested encoder cause is operator-only diagnostic context and must
  remain in a structured log; it must not be copied into the public error.

Neither dynamic value justifies a generic detail field or a global last-error
record.

## Required Tests

- change each retained retry authority field independently while keeping the
  root and operation identity fixed;
- reject each readiness-intent, readiness, execution and completion predicate
  independently;
- exercise all four typed record families through identity conflict, duplicate
  record, noncanonical order and encoding failure;
- corrupt every durable hash, predecessor link and copied authority field one
  at a time;
- prove the execution-history adapter preserves every exact typed execution
  validation leaf; and
- prove no destructive completion is accepted before exact execution intent,
  typed absence time and monotonic completion time.

## Next Slice

Classify the deployment-ledger module's two direct constructors and 47 hidden
receipt-invariant calls, then the Coordinator workflow.
