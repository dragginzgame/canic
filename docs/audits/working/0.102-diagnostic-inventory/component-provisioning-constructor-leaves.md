# Canic 0.102 Component Provisioning Direct-Constructor Leaves

Date: 2026-08-13

## Status

This B1 ledger completes site-level reconciliation of the root-local Component
Group provisioning authority in
`crates/canic-control-plane/src/ops/component_provisioning.rs`. It allocates no
number and changes no runtime behavior.

The ops and workflow files contain 233 production `InternalError::*`
references:

| Owner | References |
| --- | ---: |
| `ops/component_provisioning.rs` | 177 |
| `workflow/component_provisioning.rs` | 56 |

## Acceptance, Member Progress, Publication And Activation Persistence

This first slice accounts for all 64 direct constructor references at lines
247–1378 of `ops/component_provisioning.rs`. It covers exact acceptance replay,
the four member cursors, protected deployment reconstruction, publication
intent/delivery, runtime activation and allocation/draining exclusion fences.
The shared record, cursor, result and receipt validators remain later source
sites and are not counted transitively.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `GROUP_PROVISIONING_OPERATION_CONFLICT` | 2 | Acceptance operation is already bound to different request, plan or runtime mode | self | Replay only the exact accepted operation | public |
| `GROUP_PROVISIONING_PLACEMENT_COUNT_OVERFLOW` | 1 | Root tracked-placement count cannot add the accepted batch | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop acceptance and inspect aggregate accounting | recent failure |
| `GROUP_PROVISIONING_PLACEMENT_CAPACITY_EXHAUSTED` | 1 | Accepted placements exceed the root's protected placement ceiling | `COMPONENT_PROVISIONING_ROOT_GROUP_PLACEMENT_CAPACITY_EXCEEDED`; existing exact identity | Reduce the batch or select capacity on another admitted root | public |
| `GROUP_PROVISIONING_PLACEMENT_ALREADY_RESERVED` | 1 | A placement identity is already retained by another operation | self | Replay its owning operation or choose a new admitted placement identity | public |
| `GROUP_PROVISIONING_OPERATION_UNACCEPTED` | 7 | Status, member transition, publication or activation has no durable accepted operation | self | Accept or query the exact operation first | public |
| `GROUP_PROVISIONING_STATUS_PLAN_CONFLICT` | 1 | Status request plan differs from retained operation | self | Query with the exact retained plan hash | public |
| `GROUP_PROVISIONING_ADVANCE_AUTHORITY_CONFLICT` | 1 | Advance request names another operation or plan | self | Replay only the exact accepted authority | public |
| `GROUP_PROVISIONING_ADVANCE_CURSOR_CONFLICT` | 1 | Expected four-cursor progress is neither current nor an exact one-step replay | self | Reload status and retry its exact cursor tuple | public |
| `GROUP_PROVISIONING_RESERVATION_COMPLETE` | 1 | Reservation cursor has no remaining member | self | Advance to claiming or return the terminal phase | public |
| `GROUP_PROVISIONING_CLAIM_PREREQUISITE_UNREADY` | 1 | Claim requested before every member identity is reserved | self | Finish bounded reservation first | public |
| `GROUP_PROVISIONING_CLAIM_COMPLETE` | 1 | Claim cursor has no remaining member | self | Advance to installation | public |
| `GROUP_PROVISIONING_INSTALL_PREREQUISITE_UNREADY` | 1 | Install requested before every prepaid Canister is claimed | self | Finish bounded claiming first | public |
| `GROUP_PROVISIONING_INSTALL_COMPLETE` | 1 | Install cursor has no remaining member | self | Advance to Registry commitment | public |
| `GROUP_PROVISIONING_REGISTRY_PREREQUISITE_UNREADY` | 1 | Registry commit requested before every member is installed | self | Finish verified Store-backed installation first | public |
| `GROUP_PROVISIONING_REGISTRY_COMMIT_COMPLETE` | 1 | Registry cursor has no remaining member | self | Finalize the provisioned result | public |
| `GROUP_DEPLOYMENT_OPERATION_MISSING` | 1 | Retained grouped origin has no provisioning operation | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile origin/operation persistence | recent failure |
| `GROUP_DEPLOYMENT_PLAN_AUTHORITY_INVALID` | 1 | Grouped origin plan hash differs from retained operation | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both authorities and fail closed | recent failure |
| `GROUP_DEPLOYMENT_PLACEMENT_MISSING` | 1 | Grouped origin names no accepted placement | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile accepted placement authority | recent failure |
| `GROUP_DEPLOYMENT_BINDING_AUTHORITY_INVALID` | 1 | Component binding differs from retained Fleet/root/Spec member authority | `COMPONENT_REGISTRY_STATE_INVALID` | Reject substitution and reconcile protected binding | recent failure |
| `GROUP_DEPLOYMENT_RESULT_MISSING` | 1 | Grouped deployment has no frozen provisioned result | `COMPONENT_REGISTRY_STATE_INVALID` | Finish/reconcile aggregate provisioning before runtime use | recent failure |
| `GROUP_DEPLOYMENT_DIRECTORY_MEMBER_MISSING` | 1 | Derived group Directory has no retained member path | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile result/member Directory construction | recent failure |
| `GROUP_DEPLOYMENT_DIRECTORY_BINDING_INVALID` | 1 | Derived Directory member binding differs from protected Component binding | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both records and fail closed | recent failure |
| `GROUP_PROVISIONING_RESERVATION_STEP_CONFLICT` | 1 | Reservation cursor step is already committed or not current | self | Reload and replay only the exact next member | public |
| `GROUP_PROVISIONING_CLAIM_STEP_CONFLICT` | 1 | Claim cursor step is already committed or prerequisites changed | self | Reload and replay only the exact next member | public |
| `GROUP_PROVISIONING_INSTALL_STEP_CONFLICT` | 1 | Install cursor step is already committed or prerequisites changed | self | Reload and replay only the exact next member | public |
| `GROUP_PROVISIONING_REGISTRY_STEP_CONFLICT` | 1 | Registry cursor step is already committed or prerequisites changed | self | Reload and replay only the exact next member | public |
| `GROUP_PROVISIONING_RESERVATION_PHASE_INVALID` | 1 | Reservation mutation reached a terminal aggregate phase | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile record/cursor phase | recent failure |
| `GROUP_PROVISIONING_CLAIM_PHASE_INVALID` | 1 | Claim mutation reached a terminal aggregate phase | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile record/cursor phase | recent failure |
| `GROUP_PROVISIONING_INSTALL_PHASE_INVALID` | 1 | Install mutation reached a terminal aggregate phase | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile record/cursor phase | recent failure |
| `GROUP_PROVISIONING_REGISTRY_PHASE_INVALID` | 1 | Registry mutation reached a terminal aggregate phase | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile record/cursor phase | recent failure |
| `GROUP_PUBLICATION_START_TIME_INVALID` | 1 | Publication start predates terminal provisioning | self | Supply an observation at or after provisioning | public |
| `GROUP_PUBLICATION_FLEET_DIRECTORY_CONFLICT` | 1 | Supplied Fleet Directory differs from requested Registry or local root | self | Refresh and supply the exact local Fleet Directory | public |
| `GROUP_PUBLICATION_PROVISIONED_RESULT_UNREADY` | 1 | Publication begins without terminal provisioned result | self | Finish and retain exact provisioning first | public |
| `GROUP_PUBLICATION_IN_FLIGHT_CONFLICT` | 1 | A different member/directory delivery intent is already in flight | self | Recover or replay the retained exact delivery | public |
| `GROUP_PUBLICATION_DELIVERY_AUTHORITY_INVALID` | 1 | Delivery has zero Directory hash or predates publication start | self | Supply qualified hash/time before invoking the member | public |
| `GROUP_PUBLICATION_DELIVERY_INTENT_MISSING` | 1 | Delivery observation has no durable pre-call intent | self | Persist the exact delivery intent before any call | public |
| `GROUP_PUBLICATION_DELIVERY_OBSERVATION_CONFLICT` | 1 | Observed member/hash differs from pre-call intent | self | Re-observe and replay only the exact intent | public |
| `GROUP_PUBLICATION_COMPONENT_COUNT_OVERFLOW` | 1 | Published-member cursor cannot advance | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect publication accounting | recent failure |
| `GROUP_PUBLICATION_INCOMPLETE` | 1 | Finalization sees unpublished members or an in-flight delivery | self | Finish/recover every bounded delivery first | public |
| `GROUP_PUBLICATION_COMPLETION_TIME_INVALID` | 1 | Publication completion predates provisioning | self | Supply a terminal observation at or after provisioning | public |
| `GROUP_ACTIVATION_PUBLICATION_UNREADY` | 1 | Runtime activation begins before terminal Directory publication | self | Finalize exact publication first | public |
| `GROUP_ACTIVATION_START_TIME_INVALID` | 1 | Activation start predates publication | self | Supply an observation at or after publication | public |
| `GROUP_ACTIVATION_MEMBER_CURSOR_CONFLICT` | 1 | Activated member index differs from durable cursor | self | Reload and activate only the exact next member | public |
| `GROUP_ACTIVATION_ALLOCATION_MISSING` | 1 | Selected activation member has no Component allocation | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile aggregate/Registry persistence | recent failure |
| `GROUP_ACTIVATION_COMPONENT_COUNT_OVERFLOW` | 1 | Activated-member cursor cannot advance | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect activation accounting | recent failure |
| `GROUP_ACTIVATION_INCOMPLETE` | 1 | Terminal receipt requested before every runtime is active | self | Complete and verify each bounded member first | public |
| `GROUP_PROVISIONING_ORDINARY_ALLOCATION_FENCED` | 1 | Active aggregate operation owns root top-level allocation capacity | self | Finish the aggregate batch before ordinary allocation | public |
| `GROUP_PROVISIONING_OTHER_OPERATION_ACTIVE` | 1 | Another aggregate operation owns the root | self | Finish/recover that exact operation first | public |
| `GROUP_PROVISIONING_ACTIVE_OPERATION_RECORD_MISSING` | 1 | Aggregate index names the requested active operation but its record is absent | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile aggregate index/record persistence | recent failure |
| `GROUP_PROVISIONING_ROOT_DRAINING_FENCED` | 1 | Root retains an active aggregate operation or group placement | self | Remove all group authority before root draining | public |
| `GROUP_RUNTIME_MEMBER_MISSING` | 1 | Stored group origin has no accepted member | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile accepted member authority | recent failure |
| `GROUP_RUNTIME_PLACEMENT_INDEX_OVERFLOW` | 1 | Placement index cannot fit the bounded cursor type | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect bounded accepted batch state | recent failure |
| `GROUP_RUNTIME_MEMBER_INDEX_OVERFLOW` | 1 | Member index cannot fit the bounded cursor type | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect bounded accepted batch state | recent failure |
| `GROUP_RUNTIME_RESULT_MISSING` | 1 | Stored group origin has no provisioned result | `COMPONENT_REGISTRY_STATE_INVALID` | Finish/reconcile aggregate provisioning before runtime use | recent failure |
| `GROUP_RUNTIME_ORIGIN_KIND_INVALID` | 1 | Group-origin validator received an ordinary origin | `COMPONENT_REGISTRY_STATE_INVALID` | Fix the internal call path; never reinterpret ordinary origin | recent failure |
| `GROUP_RUNTIME_MEMBER_AUTHORITY_INVALID` | 1 | Stored origin Spec/hash differs from accepted member | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both authorities and fail closed | recent failure |
| `GROUP_RUNTIME_PLACEMENT_MISSING` | 1 | Stored group origin names no accepted placement | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile origin/placement authority | recent failure |

The 57 rows sum to all 64 selected source sites. Candidate-column extraction
finds 57 unique exact labels, all new to the preceding qualified ledgers. Broad
provisioning identities named only as public projections are not exact-label
collisions. No safe projection is added.

## Protected Request And Stable Phase Validation

This second slice accounts for all 36 direct constructor references at lines
1379–2386 of `ops/component_provisioning.rs`. It covers protected publication
and activation requests, member selection, accepted authority, aggregate-state
ownership and every persisted phase from `Accepted` through `RuntimesActive`.
Compound rows deliberately allocate one candidate per independently actionable
or independently corrupt predicate; a future implementation must not retain
the shared broad constructor merely to keep one source site.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `GROUP_PROVISIONING_OPERATION_UNACCEPTED` | 2 | Publication or activation names no durable accepted operation | self; existing exact identity | Accept or query the exact operation first | public |
| `GROUP_PUBLICATION_PHASE_INVALID` | 1 | Publication advance reached an operation outside the publishing phase | self | Reload status; publish only from the retained publishing phase | public |
| `GROUP_ACTIVATION_PHASE_INVALID` | 1 | Activation advance reached an operation outside the activating phase | self | Reload status; activate only from the retained activating phase | public |
| `GROUP_PROVISIONING_RESULT_MISSING` | 1 | Member selection has no durable provisioned result | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the operation and reconcile phase/result persistence | recent failure |
| `GROUP_PROVISIONING_COMPONENT_CURSOR_OVERFLOW` | 1 | Flattened member traversal cannot advance its bounded cursor | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect accepted member accounting | recent failure |
| `GROUP_PROVISIONING_COMPONENT_CURSOR_OUT_OF_RANGE` | 1 | Requested flattened member index is outside the provisioned result | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile cursor/result authority | recent failure |
| `GROUP_PUBLICATION_OPERATION_CONFLICT` / `GROUP_PUBLICATION_PLAN_CONFLICT` / `GROUP_PUBLICATION_CURSOR_CONFLICT` / `GROUP_PUBLICATION_REGISTRY_AUTHORITY_CONFLICT` / `GROUP_PUBLICATION_REGISTRY_REVISION_STALE` / `GROUP_PUBLICATION_REGISTRY_HASH_INVALID` | 1 | One request predicate merges operation, plan, current-or-replayed cursor, Registry authority, monotonic revision and nonzero content hash | self for every exact leaf | Reload exact operation and current Registry evidence; retry only a current or one-step replay | public |
| `GROUP_PUBLICATION_REGISTRY_REVISION_CONFLICT` | 1 | A Registry revision is reused with different authority or content hash | self | Preserve both observations and fail closed | public |
| `GROUP_PUBLICATION_REGISTRY_CONFLICT` | 1 | Request names a Registry different from the publication already retained | self | Replay only the retained publication authority | public |
| `GROUP_ACTIVATION_OPERATION_CONFLICT` / `GROUP_ACTIVATION_PLAN_CONFLICT` / `GROUP_ACTIVATION_CURSOR_CONFLICT` | 1 | One activation request predicate merges operation, plan and current-or-one-step-replayed progress | self for every exact leaf | Reload exact activation status and replay only its accepted progress | public |
| `GROUP_ACTIVATION_MEMBER_UNCOMMITTED` | 1 | Selected member has not reached Registry commitment | self | Finish exact member Registry commitment first | public |
| `GROUP_ACTIVATION_MEMBERSHIP_MISSING` | 1 | Committed member lacks its Active membership receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve commitment and reconcile membership evidence | recent failure |
| `GROUP_ACTIVATION_RUNTIME_EVIDENCE_MISSING` / `GROUP_ACTIVATION_DIRECTORY_EVIDENCE_MISSING` | 1 | One branch merges absent runtime activation with absent Directory synchronization | `COMPONENT_REGISTRY_STATE_INVALID` for either exact leaf | Reconcile the missing terminal evidence; do not infer one from the other | recent failure |
| `GROUP_ACTIVATION_BINDING_AUTHORITY_INVALID` / `GROUP_ACTIVATION_MEMBERSHIP_INACTIVE` / `GROUP_ACTIVATION_DIRECTORY_TIME_CONFLICT` | 1 | Active partition validation merges protected binding, lifecycle and exact Directory timestamp | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Preserve both records and reconcile the exact failed authority | recent failure |
| `GROUP_ACTIVATION_DEPLOYMENT_KIND_INVALID` | 1 | Group activation derived an ordinary/non-group protected deployment | `COMPONENT_REGISTRY_STATE_INVALID` | Fix the internal derivation path; never reinterpret deployment kind | recent failure |
| `GROUP_ACTIVATION_MEMBER_OPERATION_CONFLICT` / `GROUP_ACTIVATION_MEMBER_SPEC_CONFLICT` / `GROUP_ACTIVATION_MEMBER_SPEC_HASH_CONFLICT` / `GROUP_ACTIVATION_MEMBER_ORIGIN_CONFLICT` / `GROUP_ACTIVATION_MEMBER_RELEASE_SET_CONFLICT` | 1 | One predicate merges member operation, Spec, Spec hash, group origin and active release-set authority | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Preserve allocation and aggregate records and reconcile the named authority | recent failure |
| `GROUP_PROVISIONING_ACCEPTANCE_TIME_INVALID` | 1 | Acceptance observation time is zero | self | Supply a positive trusted observation time | public |
| `GROUP_PROVISIONING_PLACEMENT_COUNT_UNREPRESENTABLE` | 1 | Accepted placement count cannot fit the durable `u32` field | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Reduce the bounded batch | public |
| `GROUP_PROVISIONING_MEMBER_COUNT_UNREPRESENTABLE` | 1 | One placement member count cannot fit the durable `u32` field | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Reduce the bounded group | public |
| `GROUP_PROVISIONING_MEMBER_COUNT_OVERFLOW` | 1 | Accepted member counts cannot be summed in the durable field | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Reduce the batch or inspect failed bound enforcement | recent failure |
| `GROUP_PROVISIONING_VALIDATED_PLACEMENT_COUNT_MISMATCH` / `GROUP_PROVISIONING_VALIDATED_MEMBER_COUNT_MISMATCH` | 1 | One invariant merges independently recomputed placement and member counts | `COMPONENT_REGISTRY_STATE_INVALID` for either exact leaf | Preserve validation and batch evidence and fail closed | recent failure |
| `GROUP_PROVISIONING_OPERATION_ID_INVALID` | 1 | Operation identity is all zeroes | self | Generate a qualified nonzero operation ID | public |
| `GROUP_PROVISIONING_PLAN_HASH_INVALID` | 1 | Canonical plan hash is all zeroes | self | Compile and bind a qualified plan hash | public |
| `GROUP_PROVISIONING_PLACEMENT_ACCOUNTING_INVALID` | 1 | Aggregate tracked-placement count differs from the durable placement index | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both authorities and reconcile accounting | recent failure |
| `GROUP_PROVISIONING_ACTIVE_OPERATION_MISMATCH` / `GROUP_PROVISIONING_TERMINAL_ACTIVE_OPERATION_RETAINED` | 1 | Aggregate-state validation merges a live phase owned by another operation with terminal state that still retains an active owner | `COMPONENT_REGISTRY_STATE_INVALID` for either exact leaf | Reconcile record and aggregate ownership without selecting either as truth | recent failure |
| `GROUP_PROVISIONING_ACCEPTANCE_RECEIPT_HASH_INVALID` | 1 | Accepted record differs from its canonical receipt hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the record and reject restoration | recent failure |
| `GROUP_PROVISIONING_COMPLETION_TIME_INVALID` | 1 | Provisioning completion is zero or predates acceptance | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve phase evidence and reject restoration | recent failure |
| `GROUP_PROVISIONING_RECEIPT_HASH_INVALID` | 1 | Provisioned terminal record differs from its canonical receipt hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the record and reject restoration | recent failure |
| `GROUP_PUBLICATION_PROVISIONED_RESULT_UNREADY` | 1 | Publishing-state reconstruction lacks the result guaranteed by provisioned-state validation | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Reconcile the phase postcondition; do not synthesize a result | recent failure |
| `GROUP_PUBLICATION_START_TIME_INVALID` | 1 | Persisted publication start predates provisioning | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve the record and reject restoration | recent failure |
| `GROUP_PUBLICATION_COMPLETION_TIME_INVALID` | 1 | Persisted publication completion predates provisioning | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve the record and reject restoration | recent failure |
| `GROUP_PUBLICATION_RECEIPT_HASH_INVALID` | 1 | Published record differs from its canonical receipt hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the record and reject restoration | recent failure |
| `GROUP_ACTIVATION_START_TIME_INVALID` / `GROUP_ACTIVATION_CURSOR_EXCEEDS_COMPONENT_COUNT` | 1 | Activating-state reconstruction merges start-before-publication with a cursor beyond the frozen member count | `COMPONENT_REGISTRY_STATE_INVALID` for either exact leaf; start-time identity already exists | Split temporal and count corruption before mapping | recent failure |
| `GROUP_ACTIVATION_START_TIME_INVALID` / `GROUP_ACTIVATION_COMPLETION_TIME_INVALID` / `GROUP_ACTIVATION_EVIDENCE_COMPONENT_COUNT_MISMATCH` / `GROUP_ACTIVATION_FLEET_OPERATION_ID_INVALID` / `GROUP_ACTIVATION_INITIAL_INVENTORY_HASH_INVALID` / `GROUP_ACTIVATION_FRESH_ROOT_TIME_MISMATCH` / `GROUP_ACTIVATION_ACTIVE_ROOT_TIME_MISSING` / `GROUP_ACTIVATION_ACTIVE_ROOT_TIME_AFTER_ACCEPTANCE` | 1 | One terminal predicate merges two ordering edges, frozen count, Fleet activation identity, inventory identity and distinct fresh/active-root time rules | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Split all eight predicates and preserve the invalid terminal evidence | recent failure |
| `GROUP_ACTIVATION_RECEIPT_HASH_INVALID` | 1 | Runtimes-active record differs from its canonical receipt hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the record and reject restoration | recent failure |

The 35 rows sum to all 36 selected source sites. The two terminal compound
rows are intentionally not compressed: start-time validity is one exact meaning
reused at two sites, while every other predicate retains a separately testable
identity. Candidate-column extraction finds 59 occurrences and 58 unique exact
labels. Six occurrences reuse five labels from the first slice; the other 53
labels are new to the preceding qualified ledgers. No safe projection is added.

## Publication And Provisioned-Result Integrity

This third slice accounts for all 35 direct constructor references at lines
2387–2991 of `ops/component_provisioning.rs`. It covers partial-publication
integrity, extraction and canonical freezing of the provisioned result,
Component Group Directory derivation and the complete protected member/result
authority. The qualified-identity helper's four boolean leaves are part of its
single caller predicate and are therefore expanded here.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `GROUP_PUBLICATION_REGISTRY_AUTHORITY_CONFLICT` / `GROUP_PUBLICATION_REGISTRY_REVISION_STALE` / `GROUP_PUBLICATION_REGISTRY_HASH_INVALID` / `GROUP_PUBLICATION_FLEET_DIRECTORY_HASH_INVALID` | 1 | Persisted publication authority merges wrong Fleet Registry authority, regressed revision, zero Registry hash and zero Fleet Directory hash | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf; first three exact identities already exist | Preserve publication and operation records and reconcile the named authority | recent failure |
| `GROUP_PUBLICATION_REGISTRY_REVISION_CONFLICT` | 1 | Persisted publication reuses the accepted Registry revision with different authority/content | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve both Registry observations and fail closed | recent failure |
| `GROUP_PUBLICATION_COUNT_UNREPRESENTABLE` | 1 | Published-member count cannot index the platform collection | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect bounded cursor representation | recent failure |
| `GROUP_PUBLICATION_CURSOR_EXCEEDS_RESULT` | 1 | Published-member cursor exceeds the frozen provisioned result | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve cursor and result evidence and fail closed | recent failure |
| `GROUP_PUBLICATION_DIRECTORY_COUNT_MISMATCH` | 1 | Retained Component Directory evidence count differs from the published cursor | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile evidence/cursor persistence | recent failure |
| `GROUP_PUBLICATION_DIRECTORY_COMPONENT_MISMATCH` / `GROUP_PUBLICATION_DIRECTORY_HASH_MISMATCH` | 1 | One predicate merges wrong Component identity with wrong Component Directory content hash | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve result and Directory evidence and identify the failed field | recent failure |
| `GROUP_PUBLICATION_GROUP_DIRECTORY_COUNT_MISMATCH` | 1 | Component Group Directory evidence does not cover every placement | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile placement/evidence persistence | recent failure |
| `GROUP_PUBLICATION_GROUP_PLACEMENT_MISMATCH` / `GROUP_PUBLICATION_GROUP_DIRECTORY_HASH_MISMATCH` | 1 | Group Directory evidence merges wrong placement identity with wrong canonical content hash | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve derived and retained evidence and fail closed | recent failure |
| `GROUP_PUBLICATION_INTENT_CURSOR_CONFLICT` / `GROUP_PUBLICATION_INTENT_CANISTER_CONFLICT` / `GROUP_PUBLICATION_INTENT_DIRECTORY_HASH_INVALID` / `GROUP_PUBLICATION_START_TIME_INVALID` | 1 | Durable in-flight intent merges next-member cursor, Canister, nonzero Directory hash and not-before-publication time | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf; start-time identity already exists | Recover only the exact canonical intent; never retarget it | recent failure |
| `GROUP_PROVISIONING_RESULT_PHASE_INVALID` / `GROUP_PROVISIONING_RESULT_REGISTRY_INCOMPLETE` | 1 | Result extraction merges a non-Accepted phase with incomplete Registry commitment | self | Finish exact member commitment in the accepted phase | public |
| `GROUP_PROVISIONING_RESULT_COUNT_UNREPRESENTABLE` | 2 | Frozen member count cannot size a platform collection | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; second site reuses this exact identity | Stop and inspect configured/result bounds | recent failure |
| `GROUP_PROVISIONING_RESULT_PLACEMENT_INDEX_UNREPRESENTABLE` | 1 | Accepted placement index cannot fit the durable cursor | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect accepted placement bounds | recent failure |
| `GROUP_PROVISIONING_RESULT_MEMBER_INDEX_UNREPRESENTABLE` | 1 | Accepted member index cannot fit the durable cursor | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect accepted member bounds | recent failure |
| `GROUP_PROVISIONING_RESULT_ALLOCATION_MISSING` | 1 | Registry-committed group member has no allocation receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve cursor/Registry evidence and reconcile allocation persistence | recent failure |
| `GROUP_PROVISIONING_RESULT_PARTITION_MISSING` | 1 | Allocated group member has no Component Registry partition | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve allocation and reconcile partition persistence | recent failure |
| `GROUP_PROVISIONING_RESULT_EVIDENCE_COUNT_MISMATCH` | 1 | Collected member evidence does not cover the frozen batch count | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile extraction count before freezing a result | recent failure |
| `GROUP_PROVISIONING_RESULT_EVIDENCE_TRUNCATED` | 1 | Evidence iterator ends before the accepted member sequence | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve partial evidence and fail closed | recent failure |
| `GROUP_PROVISIONING_RESULT_MEMBER_ORDER_INVALID` | 1 | Extracted evidence is not in canonical placement/member order | `COMPONENT_REGISTRY_STATE_INVALID` | Rebuild from exact cursor order; do not sort contradictory evidence | recent failure |
| `GROUP_PROVISIONING_RESULT_EVIDENCE_SURPLUS` | 1 | Evidence remains after every accepted member is consumed | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve surplus evidence and reconcile accepted scope | recent failure |
| `GROUP_PROVISIONING_OPERATION_UNACCEPTED` | 1 | Result commit names no accepted operation | self; existing exact identity | Accept or query the exact operation first | public |
| `GROUP_PROVISIONING_RESULT_ADVANCE_CONFLICT` / `GROUP_PROVISIONING_RESULT_PHASE_INVALID` / `GROUP_PROVISIONING_RESULT_REGISTRY_INCOMPLETE` | 1 | Commit readiness merges non-advance replay disposition, wrong phase and incomplete Registry cursor | self for every leaf; latter two exact identities already exist | Reload exact status; commit only the current complete accepted result | public |
| `GROUP_PROVISIONING_COMPLETION_OBSERVATION_TIME_INVALID` | 1 | Proposed completion time is zero or predates acceptance | self | Supply a positive trusted observation after acceptance | public |
| `GROUP_DIRECTORY_ACCEPTED_PLACEMENT_MISSING` | 1 | Directory derivation index is outside the accepted placement batch | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve batch/index evidence and fail closed | recent failure |
| `GROUP_DIRECTORY_PROVISIONED_PLACEMENT_MISSING` | 1 | Directory derivation index is outside the frozen result | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve result/index evidence and fail closed | recent failure |
| `GROUP_DIRECTORY_PLACEMENT_IDENTITY_MISMATCH` / `GROUP_DIRECTORY_COMPONENT_GROUP_MISMATCH` / `GROUP_DIRECTORY_MEMBER_COUNT_MISMATCH` | 1 | Planned/result placement validation merges placement identity, group identity and member cardinality | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve both placements and identify the failed authority | recent failure |
| `GROUP_DIRECTORY_MEMBER_PATH_MISMATCH` / `GROUP_DIRECTORY_MEMBER_SPEC_MISMATCH` / `GROUP_DIRECTORY_MEMBER_PURPOSE_MISMATCH` | 1 | Planned/result Directory member validation merges path, Spec and purpose | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve both member records and fail closed | recent failure |
| `GROUP_PUBLICATION_CURSOR_UNREPRESENTABLE` | 1 | Component publication cursor cannot index the result collection | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect bounded cursor representation | recent failure |
| `GROUP_PUBLICATION_CURSOR_OUT_OF_RANGE` | 1 | Component publication cursor has no frozen result member | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve cursor/result evidence and fail closed | recent failure |
| `GROUP_PROVISIONING_RESULT_PLACEMENT_COUNT_MISMATCH` | 1 | Frozen result does not cover the exact accepted placement batch | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve batch/result evidence and fail closed | recent failure |
| `GROUP_PROVISIONING_RESULT_PLACEMENT_IDENTITY_MISMATCH` / `GROUP_PROVISIONING_RESULT_COMPONENT_GROUP_MISMATCH` / `GROUP_PROVISIONING_RESULT_PLACEMENT_MEMBER_COUNT_MISMATCH` | 1 | Result placement validation merges placement identity, group identity and member cardinality | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve both placement authorities and identify the failed field | recent failure |
| `GROUP_PROVISIONING_RESULT_COMPONENT_ID_REUSED` / `GROUP_PROVISIONING_RESULT_CANISTER_ID_REUSED` | 1 | One uniqueness predicate merges duplicate Component identity with duplicate Canister principal | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve the result and reject duplicate authority | recent failure |
| `GROUP_PROVISIONING_RESULT_COUNT_OVERFLOW` | 1 | Result member count cannot advance its bounded accumulator | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect result/bound enforcement | recent failure |
| `GROUP_PROVISIONING_RESULT_COMPONENT_COUNT_MISMATCH` | 1 | Recomputed result member count differs from accepted count | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both counts and fail closed | recent failure |
| `GROUP_PROVISIONING_RESULT_MEMBER_PATH_MISMATCH` / `GROUP_PROVISIONING_RESULT_MEMBER_SPEC_MISMATCH` / `GROUP_PROVISIONING_RESULT_MEMBER_PURPOSE_MISMATCH` / `GROUP_PROVISIONING_RESULT_MEMBER_LIMITS_MISMATCH` / `GROUP_PROVISIONING_RESULT_BINDING_AUTHORITY_MISMATCH` / `GROUP_PROVISIONING_RESULT_BINDING_SPEC_MISMATCH` / `GROUP_PROVISIONING_RESULT_BINDING_SPEC_HASH_MISMATCH` / `GROUP_PROVISIONING_RESULT_BINDING_SUBNET_MISMATCH` / `GROUP_PROVISIONING_RESULT_BINDING_ROOT_MISMATCH` / `GROUP_PROVISIONING_RESULT_COMPONENT_ID_INVALID` / `GROUP_PROVISIONING_RESULT_CANISTER_ID_INVALID` / `GROUP_PROVISIONING_RESULT_REGISTRY_REVISION_INVALID` / `GROUP_PROVISIONING_RESULT_REGISTRY_HASH_INVALID` | 1 | One protected-member predicate merges nine accepted/binding fields with four qualified committed-identity leaves | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Split every authority field and preserve the rejected member | recent failure |

The 34 rows sum to all 35 selected source sites. Candidate-column extraction
finds 64 occurrences and 62 unique exact labels. Six occurrences reuse six
labels from the preceding slices; two pairs repeat inside this slice and the
other 56 unique labels are new to the preceding qualified ledgers. No safe
projection is added.

## Cursor, Member Authority And Commit Mapping

This final ops slice accounts for all 42 direct constructor references at lines
2992–3820 of `ops/component_provisioning.rs`. It covers every persisted cursor,
cursor advancement, member lookup and boundary, protected Registry-committed
authority, canonical hashing and the stable-store commit adapter. The current
dynamic `cursor_kind` message is not an acceptable substitute for exact codes:
the four cursor phases have different recovery prerequisites and therefore
receive distinct candidates where the failed predicate is phase-specific.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `GROUP_PROVISIONING_PLACEMENT_INDEX_INVALID` | 1 | Accepted placement index does not bind the operation and plan that own the batch | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve index and operation records and fail closed | recent failure |
| `GROUP_PROVISIONING_RESERVATION_CURSOR_HASH_INVALID` | 1 | Reservation cursor differs from its canonical protected hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the cursor and reject restoration | recent failure |
| `GROUP_PROVISIONING_CLAIM_CURSOR_HASH_INVALID` | 1 | Claim cursor differs from its canonical protected hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the cursor and reject restoration | recent failure |
| `GROUP_PROVISIONING_CLAIM_BEFORE_RESERVATION` | 1 | Claim progress exists before every identity is reserved | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile cursor ordering; do not infer reservation | recent failure |
| `GROUP_PROVISIONING_INSTALL_CURSOR_HASH_INVALID` | 1 | Install cursor differs from its canonical protected hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the cursor and reject restoration | recent failure |
| `GROUP_PROVISIONING_INSTALL_BEFORE_CLAIM` | 1 | Install progress exists before every prepaid Canister is claimed | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile cursor ordering; do not infer claim | recent failure |
| `GROUP_PROVISIONING_REGISTRY_CURSOR_HASH_INVALID` | 1 | Registry cursor differs from its canonical protected hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the cursor and reject restoration | recent failure |
| `GROUP_PROVISIONING_REGISTRY_BEFORE_INSTALL` | 1 | Registry commitment exists before every member is installed | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile cursor ordering; do not infer installation | recent failure |
| `GROUP_PROVISIONING_RESERVATION_CURSOR_COUNT_INVALID` / `GROUP_PROVISIONING_CLAIM_CURSOR_COUNT_INVALID` / `GROUP_PROVISIONING_INSTALL_CURSOR_COUNT_INVALID` / `GROUP_PROVISIONING_REGISTRY_CURSOR_COUNT_INVALID` | 1 | Dynamic cursor validator merges completed-count overflow for four independently recoverable phases | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve the exact phase cursor and fail closed | recent failure |
| `GROUP_PROVISIONING_CURSOR_PLACEMENT_COUNT_UNREPRESENTABLE` | 1 | Accepted placement count cannot fit the persisted cursor representation | `COMPONENT_REGISTRY_STATE_INVALID` | Reject malformed retained batch/cursor state | recent failure |
| `GROUP_PROVISIONING_RESERVATION_TERMINAL_CURSOR_INVALID` / `GROUP_PROVISIONING_CLAIM_TERMINAL_CURSOR_INVALID` / `GROUP_PROVISIONING_INSTALL_TERMINAL_CURSOR_INVALID` / `GROUP_PROVISIONING_REGISTRY_TERMINAL_CURSOR_INVALID` | 1 | Dynamic validator merges four noncanonical terminal `(placement, member)` cursors | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve the exact phase cursor and fail closed | recent failure |
| `GROUP_PROVISIONING_CURSOR_PLACEMENT_INDEX_UNREPRESENTABLE` | 3 | Cursor placement index cannot address the platform collection | `COMPONENT_REGISTRY_STATE_INVALID`; all three sites reuse this exact identity | Preserve cursor/batch evidence and fail closed | recent failure |
| `GROUP_PROVISIONING_CURSOR_PLACEMENT_OUT_OF_RANGE` | 3 | Cursor placement has no accepted batch entry | `COMPONENT_REGISTRY_STATE_INVALID`; all three sites reuse this exact identity | Preserve cursor/batch evidence and fail closed | recent failure |
| `GROUP_PROVISIONING_RESERVATION_MEMBER_CURSOR_OUT_OF_RANGE` / `GROUP_PROVISIONING_CLAIM_MEMBER_CURSOR_OUT_OF_RANGE` / `GROUP_PROVISIONING_INSTALL_MEMBER_CURSOR_OUT_OF_RANGE` / `GROUP_PROVISIONING_REGISTRY_MEMBER_CURSOR_OUT_OF_RANGE` | 1 | Dynamic validator merges out-of-range member cursors for four phases | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve the exact phase cursor and fail closed | recent failure |
| `GROUP_PROVISIONING_RESERVATION_CURSOR_COUNT_OVERFLOW` / `GROUP_PROVISIONING_CLAIM_CURSOR_COUNT_OVERFLOW` / `GROUP_PROVISIONING_INSTALL_CURSOR_COUNT_OVERFLOW` / `GROUP_PROVISIONING_REGISTRY_CURSOR_COUNT_OVERFLOW` | 1 | Shared advancement merges completed-count arithmetic overflow for four phases | `COMPONENT_PROVISIONING_COUNT_OVERFLOW` for every leaf; existing exact projection | Stop and inspect the exact cursor phase | recent failure |
| `GROUP_PROVISIONING_RESERVATION_MEMBER_CURSOR_OVERFLOW` / `GROUP_PROVISIONING_CLAIM_MEMBER_CURSOR_OVERFLOW` / `GROUP_PROVISIONING_INSTALL_MEMBER_CURSOR_OVERFLOW` / `GROUP_PROVISIONING_REGISTRY_MEMBER_CURSOR_OVERFLOW` | 1 | Shared advancement merges member-index arithmetic overflow for four phases | `COMPONENT_PROVISIONING_COUNT_OVERFLOW` for every leaf; existing exact projection | Stop and inspect the exact cursor phase | recent failure |
| `GROUP_PROVISIONING_CURSOR_PLACEMENT_MEMBER_COUNT_UNREPRESENTABLE` | 1 | Current placement member count cannot fit the durable cursor type | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve accepted batch/cursor evidence and fail closed | recent failure |
| `GROUP_PROVISIONING_RESERVATION_PLACEMENT_CURSOR_OVERFLOW` / `GROUP_PROVISIONING_CLAIM_PLACEMENT_CURSOR_OVERFLOW` / `GROUP_PROVISIONING_INSTALL_PLACEMENT_CURSOR_OVERFLOW` / `GROUP_PROVISIONING_REGISTRY_PLACEMENT_CURSOR_OVERFLOW` | 1 | Shared advancement merges placement-index overflow for four phases | `COMPONENT_PROVISIONING_COUNT_OVERFLOW` for every leaf; existing exact projection | Stop and inspect the exact cursor phase | recent failure |
| `GROUP_PROVISIONING_MEMBER_CURSOR_INDEX_UNREPRESENTABLE` | 1 | Member cursor cannot address the platform collection | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve cursor/batch evidence and fail closed | recent failure |
| `GROUP_PROVISIONING_MEMBER_CURSOR_OUT_OF_RANGE` | 1 | Member cursor has no accepted placement entry | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve cursor/batch evidence and fail closed | recent failure |
| `GROUP_RUNTIME_PLACEMENT_MISSING` | 1 | Group origin names no accepted placement | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve origin/batch evidence and fail closed | recent failure |
| `GROUP_RUNTIME_MEMBER_MISSING` | 1 | Group origin names no accepted member path | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve origin/batch evidence and fail closed | recent failure |
| `GROUP_PROVISIONING_RESERVATION_BOUNDARY_CROSSED` | 1 | Aggregate reservation observes a member beyond `Reserved` | self | Recover the owning aggregate operation; do not reserve again | public |
| `GROUP_PROVISIONING_INSTALL_BOUNDARY_INVALID` | 1 | Aggregate installation does not stop at verified installation | self | Recover exact install status before Registry commitment | public |
| `GROUP_PROVISIONING_REGISTRY_BOUNDARY_UNREACHED` | 1 | Aggregate result extraction sees a member not Registry committed | self | Finish exact Registry commitment first | public |
| `GROUP_PROVISIONING_REGISTRY_BINDING_MISMATCH` / `GROUP_PROVISIONING_REGISTRY_ORIGIN_MISMATCH` / `GROUP_PROVISIONING_REGISTRY_RELEASE_SET_MISMATCH` / `GROUP_PROVISIONING_REGISTRY_STATUS_INVALID` / `GROUP_PROVISIONING_REGISTRY_HEAD_MISMATCH` / `GROUP_PROVISIONING_REGISTRY_BYTE_COUNT_MISMATCH` / `GROUP_PROVISIONING_REGISTRY_DIRECTORY_TIME_MISMATCH` / `GROUP_PROVISIONING_REGISTRY_RESERVED_DESCENDANTS_NONZERO` / `GROUP_PROVISIONING_REGISTRY_COMMITTED_DESCENDANTS_NONZERO` / `GROUP_PROVISIONING_REGISTRY_BYTE_LIMIT_EXCEEDED` | 1 | Registry-committed member validation merges nine protected receipt/partition fields and the accepted byte ceiling | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Split every field, preserve partition/allocation evidence and fail closed | recent failure |
| `GROUP_PROVISIONING_MEMBER_OPERATION_MISMATCH` / `GROUP_PROVISIONING_MEMBER_SPEC_MISMATCH` / `GROUP_PROVISIONING_MEMBER_SPEC_HASH_MISMATCH` / `GROUP_PROVISIONING_MEMBER_ORIGIN_MISMATCH` / `GROUP_PROVISIONING_MEMBER_RELEASE_SET_MISMATCH` | 1 | Reserved-member authority merges operation, Spec, Spec hash, group origin and release set | self for request-visible conflict; `COMPONENT_REGISTRY_STATE_INVALID` when restoring retained contradiction | Reconcile the exact accepted/allocation field | public or recent failure by caller |
| `GROUP_PROVISIONING_CLAIM_INCOMPLETE` | 1 | Member has not completed its prepaid-Canister claim | self | Resume the exact claim operation | public |
| `GROUP_PROVISIONING_MEMBER_REMOVED_DURING_BATCH` | 1 | Member was removed before aggregate provisioning completed | self | Fail the batch and inspect the removal authority | public |
| `GROUP_PROVISIONING_AUTHORITY_ENCODING_FAILED` | 1 | Canonical Candid encoding of protected provisioning authority fails | `COMPONENT_REGISTRY_STATE_INVALID` | Treat as internal schema/authority failure; do not persist a hash | recent failure |
| `GROUP_PROVISIONING_AUTHORITY_BYTE_COUNT_OVERFLOW` | 1 | Canonical authority bytes cannot fit the hashed `u64` length prefix | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact projection | Reject before hashing or persistence | recent failure |
| `GROUP_PROVISIONING_ACCEPTED_PLACEMENT_MISSING` | 1 | Exact placement is absent from the canonically ordered accepted batch | self | Refresh accepted status or reject the foreign placement | public |
| `GROUP_PROVISIONING_ACCEPTED_MEMBER_MISSING` | 1 | Exact member path is absent from the accepted placement | self | Refresh accepted status or reject the foreign member | public |
| `GROUP_PROVISIONING_OTHER_OPERATION_ACTIVE` / `GROUP_PROVISIONING_OPERATION_CONFLICT` / `GROUP_PROVISIONING_ADVANCE_AUTHORITY_CONFLICT` / `GROUP_PROVISIONING_PLACEMENT_ALREADY_RESERVED` / `GROUP_PROVISIONING_PLACEMENT_COUNT_OVERFLOW` | 5 | Stable-store commit adapter preserves five already qualified exact conflict/overflow meanings | self; all five are existing exact identities | Follow the owning operation/placement action and exact retry rule | public |

The 33 rows sum to all 42 selected source sites. Candidate-column extraction
finds 69 occurrences and 69 unique exact labels. Seven reuse preceding exact
identities; the other 62 are new. Phase-expanded cursor leaves count separately
even though four shared constructors currently receive a free-form
`cursor_kind`. No safe projection is added.

## Mechanical Ops Closure

The immutable source partition is exact:

| Inclusive source lines | Constructors | Owner section |
| --- | ---: | --- |
| 247–1378 | 64 | Acceptance, member progress, publication and activation persistence |
| 1379–2386 | 36 | Protected request and stable phase validation |
| 2387–2991 | 35 | Publication and provisioned-result integrity |
| 2992–3820 | 42 | Cursor, member authority and commit mapping |
| **Total** | **177** | **Complete ops file production frontier** |

A mechanical constructor scan and the four table sums independently produce
177. The ranges are consecutive, do not overlap and cover every production
`InternalError::*` reference in the file. The separate 56-site workflow remains
open; it is not inferred from this persistence closure.

## Required Tests

- exact acceptance replay and conflicting plan/runtime-mode rejection;
- all four cursor transition matrices, including one-step response-loss replay;
- protected deployment reconstruction with wrong operation, placement, member,
  Spec, binding and Directory member;
- publication intent before delivery, response loss, conflicting observation
  and terminal receipt replay;
- activation cursor, missing allocation and terminal completeness;
- protected publication and activation request matrices, including every
  independently stale Registry/cursor field;
- accepted, provisioned, publishing, published, activating and runtimes-active
  record reconstruction with each time, count, identity and receipt-hash
  predicate corrupted independently;
- aggregate operation/placement ownership disagreement;
- partial publication with every Registry, Directory, cursor, member, intent
  and time predicate corrupted independently;
- result extraction with missing allocation/partition, truncated, reordered or
  surplus evidence and exact retry of terminal commitment;
- Component Group Directory derivation with independently wrong placement,
  group, cardinality, path, Spec and purpose;
- provisioned-result uniqueness, count and every protected member/binding
  field, including all four qualified-identity leaves;
- each of the four cursor phases with corrupt hash, count, terminal tuple,
  member bounds and independent advancement arithmetic failures;
- wrong accepted placement/member lookup and every reserved/installed/
  Registry-committed boundary;
- Registry-committed partition validation with each protected field and byte
  ceiling corrupted independently;
- canonical authority encoding/length failures and all five typed store-commit
  mappings;
- ordinary-allocation and root-draining fences; and
- a constructor-site manifest proving all 100 selected ops sites remain
  accounted.

## Next Slice

Reconcile the 56-site Component provisioning orchestration workflow without
assigning numbers, then move to Fleet Coordinator ops/workflows.
