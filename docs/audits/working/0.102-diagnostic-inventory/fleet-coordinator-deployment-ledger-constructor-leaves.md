# Canic 0.102 Fleet Coordinator Deployment-Ledger Constructor Leaves

Date: 2026-08-15

## Status

This B1 evidence ledger classifies
`crates/canic-control-plane/src/ops/fleet_coordinator/deployment_ledger/mod.rs`.
It assigns no number and changes no runtime behavior.

The conservative source frontier contains two direct `InternalError::*`
references and 47 calls to the parent `receipt_invariant` funnel. Two funnel
calls live in a `#[cfg(test)]` helper and are test-only dispositions. One
production call currently discards an already typed plan-compilation error and
must become transparent rather than receive a broad diagnostic.

## Direct Public Plan Boundaries

| Exact candidate | Producer function/branch | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_COMPONENT_SCALE_OUT_DEPLOYMENT_UNKNOWN` | `scale_out_plan_hash` / durable deployment lookup miss | A scale-out plan selects no configured durable deployment | self | Select a configured deployment before hashing or reserving | public |
| `FLEET_COMPONENT_SCALE_OUT_OPERATION_REQUIRED` | `scale_out_deployment` / non-Scale-Out operation | An operation that must be Scale Out has another operation kind | self | Submit an exact Scale Out operation | public |

Both direct references are classified and add two exact candidates.

## Fresh Authority And Reservation

This table assigns the first nine funnel calls: two fresh-root authority checks
and seven reservation checks. Compound count and ceiling validation is expanded
by predicate.

| Exact candidate or disposition | Producer function/branch | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_COMPONENT_FRESH_ROOT_AUTHORITY_OPERATION_INVALID` | `fresh_install_root_authority` / non-Fresh-Install operation | Installed-root authority does not originate in the terminal fresh-install plan | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve fresh provisioning and fail closed | recent failure |
| `FLEET_COMPONENT_FRESH_ROOT_AUTHORITY_DUPLICATE` | `fresh_install_root_authority` / non-increasing sorted root pair | The fresh plan contains the same installed root more than once | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the plan and reject duplicate root authority | recent failure |
| `FLEET_COMPONENT_SCALE_OUT_OPERATION_REQUIRED` | `reserve_scale_out` / non-Scale-Out operation | Reservation receives an operation other than Scale Out | self; reuses the direct public-plan identity | Submit the exact Scale Out operation | public |
| `FLEET_COMPONENT_SCALE_OUT_DEPLOYMENT_ABSENT` | `reserve_scale_out` / deployment lookup miss | The selected deployment has no durable ledger row | self | Select or restore the exact configured deployment | public |
| `FLEET_COMPONENT_COMMITTED_PLACEMENT_COUNT_OVERFLOW` | `reserve_scale_out` / `usize`-to-`u32` conversion failure | Durable placement cardinality does not fit its canonical `u32` boundary | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the ledger and fail closed | recent failure |
| `FLEET_COMPONENT_SCALE_OUT_PLACEMENT_COUNT_NONMONOTONIC` | `reserve_scale_out` / positive checked difference absent | Requested placement count does not strictly exceed the previous count | self | Increase the desired count within the configured ceiling | public |
| `FLEET_COMPONENT_SCALE_OUT_PREVIOUS_PLACEMENT_COUNT_MISMATCH` / `FLEET_COMPONENT_SCALE_OUT_MAXIMUM_PLACEMENTS_EXCEEDED` | `reserve_scale_out` / committed-count or maximum-placement predicate | Reservation disagrees with committed cardinality or exceeds the deployment ceiling | self for both exact leaves | Refresh deployment status or lower the requested count | public |
| `FLEET_COMPONENT_SCALE_OUT_PLACEMENT_ORDINAL_EXHAUSTED` | `reserve_scale_out` / `next_placement_ordinal.checked_add` failure | Advancing the durable placement ordinal overflows `u32` | self | No further placement can be reserved for this deployment | public |
| `FLEET_COMPONENT_SCALE_OUT_RESERVED_RANGE_MISMATCH` | `reserve_scale_out` / planned end differs from reserved end | The compiled plan does not occupy the exact next durable ordinal range | self | Recompile from the current deployment head | public |

The nine calls produce ten exact-label occurrences and ten unique labels; one
reuses the direct Scale Out operation requirement, so nine are new.

## Initial Ledger Compilation

| Exact candidate | Producer function/branch | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_COMPONENT_INITIAL_DEPLOYMENT_OPERATION_INVALID` | `compile_initial` / non-Fresh-Install operation | Initial ledger compilation receives an operation other than Fresh Install | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve provisioning and fail closed | recent failure |
| `FLEET_COMPONENT_INITIAL_DEPLOYMENT_RUNTIME_EVIDENCE_MISSING` | `compile_initial` / state is not `RuntimesActivated` | Fresh provisioning has not reached terminal runtime activation | self | Complete runtime activation before materializing the ledger | public |
| `FLEET_COMPONENT_INITIAL_DEPLOYMENT_ROOT_RECEIPT_COUNT_MISMATCH` | `compile_initial` / activation and batch cardinalities differ | Terminal activation lacks exactly one receipt per planned root batch | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve activation evidence and reconcile the missing or extra receipt | recent failure |
| `FLEET_COMPONENT_INITIAL_DEPLOYMENT_ROOT_RECEIPT_ROOT_MISMATCH` | `compile_initial` / activation root differs from batch root | A terminal root receipt belongs to a different planned root batch | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve activation evidence and reject the foreign receipt | recent failure |
| `FLEET_COMPONENT_INITIAL_DEPLOYMENT_ROOT_RECEIPT_HASH_MISSING` | `compile_initial` / zero receipt content hash | A terminal root receipt has a zero content hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve activation evidence and fail closed | recent failure |
| `FLEET_COMPONENT_INITIAL_DEPLOYMENT_UNKNOWN_DEPLOYMENT` | `compile_initial` / unconsumed placement-deployment map | A planned placement remains after every configured deployment is materialized | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the plan and reject undeclared deployment authority | recent failure |

All six calls are classified and add six exact candidates.

## Active Scale-Out Commitment

This table assigns eight calls. The selected-root evidence branch is expanded
into the exact root, progress and terminal-receipt predicates it currently
merges.

| Exact candidate or disposition | Producer function/branch | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_COMPONENT_SCALE_OUT_OPERATION_REQUIRED` | `commit_scale_out` / non-Scale-Out operation | Commitment receives an operation other than Scale Out | self; reuses the public-plan identity | Submit the exact Scale Out operation | public |
| `FLEET_COMPONENT_SCALE_OUT_COMMIT_RUNTIME_EVIDENCE_MISSING` | `commit_scale_out` / state is not `RuntimesActivated` | Commitment is attempted before terminal runtime activation | self | Complete runtime activation before commitment | public |
| `FLEET_COMPONENT_SCALE_OUT_COMMIT_ROOT_RECEIPT_COUNT_MISMATCH` | `commit_scale_out` / activation and batch cardinalities differ | Terminal activation lacks exactly one receipt per selected root | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve activation evidence and reconcile receipt cardinality | recent failure |
| `FLEET_COMPONENT_SCALE_OUT_DEPLOYMENT_ABSENT` | `commit_scale_out` / deployment lookup miss | Commitment cannot find its durable deployment row | self; reuses reservation absence | Restore/query the exact deployment authority | public |
| `FLEET_COMPONENT_COMMITTED_PLACEMENT_COUNT_OVERFLOW` | `commit_scale_out` / `usize`-to-`u32` conversion failure | Committed placement cardinality does not fit `u32` | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the durable count identity | Preserve the ledger and fail closed | recent failure |
| `FLEET_COMPONENT_SCALE_OUT_PREVIOUS_PLACEMENT_COUNT_MISMATCH` / `FLEET_COMPONENT_SCALE_OUT_COMMIT_RESERVED_ORDINAL_MISMATCH` / `FLEET_COMPONENT_SCALE_OUT_MAXIMUM_PLACEMENTS_EXCEEDED` | `commit_scale_out` / inexact durable reservation predicate | Commitment disagrees with the reserved start, reserved end or configured ceiling | self for every exact leaf; two reuse reservation identities | Recover from the exact durable reservation | public |
| `FLEET_COMPONENT_SCALE_OUT_COMMIT_ROOT_MISMATCH` / `FLEET_COMPONENT_SCALE_OUT_COMMIT_ROOT_RUNTIME_INACTIVE` / `FLEET_COMPONENT_SCALE_OUT_COMMIT_COMPONENT_COUNT_MISMATCH` / `FLEET_COMPONENT_SCALE_OUT_COMMIT_ACTIVATION_MISSING` / `FLEET_COMPONENT_SCALE_OUT_COMMIT_ACTIVATION_TIME_MISSING` / `FLEET_COMPONENT_SCALE_OUT_COMMIT_RECEIPT_HASH_MISSING` | `commit_scale_out` / selected-root terminal authority predicates | A selected-root result fails one exact terminal authority predicate | self for root-visible progress leaves; `COMPONENT_REGISTRY_STATE_INVALID` for malformed retained receipt fields | Complete or reconcile the exact selected-root evidence | public or recent failure as stated |
| `FLEET_COMPONENT_SCALE_OUT_COMMIT_FOREIGN_PLACEMENT` | `commit_scale_out` / planned deployment differs from target deployment | A selected root returns a placement for another deployment | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the result and reject foreign placement authority | recent failure |

The eight calls produce fifteen exact-label occurrences and ten new unique
labels after five reservation/direct identities are reused.

## Exact Reconstruction And Active Record

| Exact candidate or disposition | Producer function/branch | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_COMPONENT_DEPLOYMENT_LEDGER_WITHOUT_FRESH_INSTALL` | `deployment_ledger::validate` / state exists without fresh provisioning | Deployment or retired scale-out state exists without fresh provisioning authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve state and fail closed | recent failure |
| `FLEET_COMPONENT_DEPLOYMENT_LEDGER_HISTORY_MISMATCH` | `deployment_ledger::validate` / reconstructed ledger differs from stored ledger | Stored ledger differs from deterministic fresh, retired and current scale-out reconstruction | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve all authority records and locate the first divergent head | recent failure |
| `FLEET_COMPONENT_DEPLOYMENT_LEDGER_PREMATURE` | `deployment_ledger::validate` / preterminal fresh state retains deployment authority | Deployment state exists before terminal fresh installation | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve state and complete/recover fresh installation | recent failure |
| `FLEET_COMPONENT_SCALE_OUT_OPERATION_ID_MISSING` / `FLEET_COMPONENT_SCALE_OUT_PLAN_HASH_MISSING` | `validate_active_scale_out_record` / zero operation ID or plan hash | Active Scale Out has a zero operation identity or plan hash | `COMPONENT_REGISTRY_STATE_INVALID` for both leaves | Preserve the record and fail closed | recent failure |
| `FLEET_COMPONENT_SCALE_OUT_REUSES_FRESH_OPERATION_ID` | `validate_active_scale_out_record` / operation ID equals fresh operation ID | Active Scale Out reuses the fresh-install operation identity | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both records and reject identity reuse | recent failure |
| `FLEET_COMPONENT_SCALE_OUT_RUNTIME_BOUNDARY_INVALID` | `validate_active_scale_out_record` / `scale_out_runtime_boundary_is_valid` is false | Active Scale Out has a zero plan time for its current state | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the record and fail closed | recent failure |
| transparent: typed scale-out plan compilation cause | `validate_active_scale_out_record` / `hash_with_next_ordinal` error | Canonical plan recomputation currently replaces the exact typed placement, configuration or Registry error | preserve the exact nested projection | Remove the string adapter and propagate the typed cause | public or recent failure owned by the nested cause |
| `FLEET_COMPONENT_SCALE_OUT_PLAN_HASH_MISMATCH` | `validate_active_scale_out_record` / stored hash differs from canonical hash | Stored Scale Out plan hash differs from canonical authority bytes | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the record and reject the corrupted or foreign plan | recent failure |

The eight calls produce eight exact candidates. One call is transparent; the
other seven calls add eight unique labels.

## Retired Scale-Out Receipt Replay

| Exact candidate or disposition | Producer function/branch | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_COMPONENT_SCALE_OUT_RECEIPT_OPERATION_INVALID` | `commit_scale_out_receipt` / non-Scale-Out operation | A retired receipt contains an operation other than Scale Out | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the receipt and fail closed | recent failure |
| `FLEET_COMPONENT_SCALE_OUT_DEPLOYMENT_ABSENT` | `commit_scale_out_receipt` / deployment binary-search miss | Retired replay cannot find the deployment row | `COMPONENT_REGISTRY_STATE_INVALID`; reuses active absence | Preserve history and restore the exact deployment authority | recent failure |
| `FLEET_COMPONENT_COMMITTED_PLACEMENT_COUNT_OVERFLOW` | `commit_scale_out_receipt` / `usize`-to-`u32` conversion failure | Replayed committed cardinality does not fit `u32` | `COMPONENT_REGISTRY_STATE_INVALID`; reuses durable count overflow | Preserve history and fail closed | recent failure |
| `FLEET_COMPONENT_RETIRED_SCALE_OUT_COMMITTED_COUNT_MISMATCH` / `FLEET_COMPONENT_RETIRED_SCALE_OUT_NEXT_ORDINAL_MISMATCH` / `FLEET_COMPONENT_RETIRED_SCALE_OUT_MAXIMUM_EXCEEDED` | `commit_scale_out_receipt` / prior-authority predicate | Retired receipt disagrees with prior cardinality, ordinal head or configured ceiling | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve history and locate the exact divergent authority | recent failure |
| `FLEET_COMPONENT_RETIRED_SCALE_OUT_COUNT_INVALID` | `commit_scale_out_receipt` / checked difference or `usize` conversion absent | Retired requested-minus-previous count is negative or cannot fit `usize` | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed | recent failure |
| `FLEET_COMPONENT_RETIRED_SCALE_OUT_RANGE_COUNT_MISMATCH` | `commit_scale_out_receipt` / receipt and expected cardinalities differ | Retired receipt does not contain exactly its declared placement count | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and reject incomplete/excess history | recent failure |
| `FLEET_COMPONENT_RETIRED_SCALE_OUT_OFFSET_OVERFLOW` | `commit_scale_out_receipt` / `usize`-to-`u32` offset conversion failure | Placement offset cannot fit the canonical `u32` ordinal | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed | recent failure |
| `FLEET_COMPONENT_RETIRED_SCALE_OUT_ORDINAL_OVERFLOW` | `commit_scale_out_receipt` / `previous_placements.checked_add` failure | Previous ordinal plus receipt offset overflows `u32` | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed | recent failure |
| `FLEET_COMPONENT_RETIRED_SCALE_OUT_DEPLOYMENT_MISMATCH` / `FLEET_COMPONENT_RETIRED_SCALE_OUT_ORDINAL_MISMATCH` | `commit_scale_out_receipt` / noncanonical deployment or ordinal | A retired placement belongs to another deployment or ordinal | `COMPONENT_REGISTRY_STATE_INVALID` for both leaves | Preserve receipt and reject noncanonical history | recent failure |

The nine calls produce twelve exact-label occurrences. Two existing active
ledger identities are reused and ten labels are new.

## Reservation Bounds And Canonical Initial Set

| Exact candidate | Producer function/branch | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_COMPONENT_SCALE_OUT_RESERVATION_END_MISSING` | `scale_out_plan_reserved_end` / maximum ordinal or successor absent | A Scale Out plan has no maximum reserved placement ordinal | self | Recompile a nonempty bounded reservation | public |
| `FLEET_COMPONENT_SCALE_OUT_RESERVATION_START_MISSING` | `scale_out_plan_reserved_start` / minimum ordinal absent | A Scale Out plan has no minimum reserved placement ordinal | self | Recompile a nonempty bounded reservation | public |
| `FLEET_COMPONENT_INITIAL_PLACEMENT_COUNT_OVERFLOW` | `validate_contiguous_placement_set` / `u32`-to-`usize` conversion failure | Configured initial placement count cannot fit `usize` | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve configuration authority and fail closed | recent failure |
| `FLEET_COMPONENT_INITIAL_PLACEMENT_COUNT_MISMATCH` | `validate_contiguous_placement_set` / materialized and expected cardinalities differ | Materialized initial placement count differs from configuration | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan/results and reconcile exact cardinality | recent failure |
| `FLEET_COMPONENT_INITIAL_PLACEMENT_DEPLOYMENT_MISMATCH` / `FLEET_COMPONENT_INITIAL_PLACEMENT_ORDINAL_MISMATCH` | `validate_contiguous_placement_set` / deployment or contiguous-ordinal predicate | Initial placement belongs to another deployment or is not at its exact contiguous ordinal | `COMPONENT_REGISTRY_STATE_INVALID` for both leaves | Preserve the set and reject noncanonical placement authority | recent failure |

The five calls produce six exact candidates.

## Test-Only Funnel Calls

The two remaining conservative-frontier calls are in
`#[cfg(test)] fn validate_terminal_ledger`. They compare a supplied ledger to
expected active or reserved authority. Production `validate` performs the
maintained deterministic reconstruction and owns the exact
`FLEET_COMPONENT_DEPLOYMENT_LEDGER_HISTORY_MISMATCH` diagnostic. These two
test-only adapters receive no code and must disappear with the broad funnel;
their tests should assert the maintained production validator instead.

## Reconciliation

The source/table counts are exact:

- two direct references are classified;
- all 47 funnel calls are classified, including one transparent typed-cause
  adapter and two test-only adapters; and
- the 45 production funnel calls plus two direct calls produce 59 exact-label
  occurrences, 51 unique exact meanings and no new safe projection.

All 51 unique meanings are new relative to the preceding qualified ledgers.
Eight repeated occurrences are deliberate reuse within this owner: the Scale Out
operation requirement, deployment absence, committed-count overflow, previous-
count mismatch and maximum-ceiling meanings are shared across the exact phases
whose action and authority agree.

No dynamic value is interpolated by this module's maintained constructors. Its
current broad strings are all closed static discriminators, so this slice adds
no dynamic-public-context row.

## Required Tests

- change previous/requested counts, ordinal heads and the deployment ceiling
  independently at reservation and commitment;
- reject every selected-root progress, activation and receipt predicate
  independently;
- reconstruct from fresh plus zero, one and several retired Scale Out receipts,
  then corrupt each retained authority edge independently;
- exercise an in-flight reservation before and after terminal runtime evidence
  and prove exact retry advances no ordinal twice;
- prove empty, overflowing, gapped, reordered and foreign-deployment placement
  ranges reject independently; and
- prove canonical plan recompilation preserves the exact nested typed cause
  rather than returning one generic Registry/configuration diagnostic.

## Next Slice

Classify the Coordinator workflow, then continue by external-effect and
authority risk through Canister pool and bootstrap owners.
