# Canic 0.102 Fleet Coordinator Direct-Constructor Leaves

Date: 2026-08-13

## Status

This B1 ledger begins site-level reconciliation of the Fleet Coordinator's
Registry, Component provisioning and root-lifecycle persistence authority in
`crates/canic-control-plane/src/ops/fleet_coordinator/mod.rs`. It assigns no
number and changes no runtime behavior. The dedicated `root_deletion` module
and Coordinator workflow remain separate frontiers.

The parent ops file contains 154 production `InternalError::*` references. The
first four sections below classify its 67 public-transition and current-state
sites at lines 110–1739.

## Genesis, Joining, Snapshot And Activation

This slice accounts for all 11 direct constructor references at lines 110–378.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COORDINATOR_INIT_PRINCIPAL_MISMATCH` | 1 | Protected Coordinator principal differs from the installed Canister | self | Reinstall with the exact protected init authority | public |
| transparent deployment-configuration digest cause | 1 | Genesis wraps the exact canonical deployment-configuration rejection as text | preserve the exact nested configuration diagnostic | Remove the formatted wrapper and fix the rejected configuration field | nested configuration owner |
| `COORDINATOR_GENESIS_CONFLICT` | 1 | Genesis storage already contains different protected Registry state | self | Replay only the exact genesis record | public |
| `FLEET_ROOT_JOIN_IDENTITY_CONFLICT` | 1 | Subnet or root principal is already joined under different protected authority | self | Replay the exact entry or select a nonconflicting Fleet/Subnet root identity | public |
| `FLEET_ROOT_JOIN_AFTER_ACTIVATION` | 1 | Initial Registry activation is already committed | self | Do not append an initial Joining root after activation | public |
| `FLEET_ROOT_JOIN_REGISTRY_STALE` | 1 | Join request expected a different current Fleet Registry version | self | Reload current Registry and rebuild the exact join request | public |
| `FLEET_ROOT_JOIN_RECEIPT_MISSING` | 1 | Compiled Registry contains the root but durable join receipt does not | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve Registry state and fail closed | recent failure |
| `FLEET_SNAPSHOT_ACK_REGISTRY_STALE` | 1 | Root acknowledgement names a different current Registry version | self | Fetch and acknowledge the current complete snapshot | public |
| `FLEET_SNAPSHOT_ACK_CONFLICT` | 1 | Root already acknowledged different Registry authority | self | Replay the exact retained acknowledgement | public |
| `FLEET_REGISTRY_ACTIVATION_CONFLICT` | 1 | Registry activation is already committed against different authority | self | Replay only the exact activation request | public |
| `FLEET_REGISTRY_ACTIVATION_REGISTRY_STALE` | 1 | Activation expected a different current Registry version | self | Reload the current all-Joining snapshot and retry exactly | public |

The 11 rows sum to all 11 selected source sites. One formatted configuration
adapter remains transparent; the other ten rows introduce exact candidates.
No safe projection is added.

## Component Provisioning Preparation And Root Progress

This slice accounts for all 23 direct constructor references at lines 379–913.
It covers fresh and scale-out plan ownership, durable status, root acceptance
and the pre-call/response boundary for root provisioning.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_COMPONENT_OPERATION_ID_INVALID` | 1 | Provisioning operation ID is all zeroes | self | Generate a qualified nonzero operation ID | public |
| `FLEET_COMPONENT_PLANNED_TIME_INVALID` | 1 | Provisioning planned time is zero | self | Supply a positive trusted observation time | public |
| `FLEET_COMPONENT_FRESH_PLAN_CONFLICT` | 1 | Fresh provisioning already retains another operation or plan | self | Replay the exact retained plan | public |
| `FLEET_COMPONENT_SERVICE_PUBLICATION_ALREADY_STARTED` | 1 | Fresh provisioning is proposed after Fleet-service publication receipts exist | self | Reinstall or use the admitted scale-out path | public |
| `FLEET_COMPONENT_SOURCE_REGISTRY_CHANGED` | 1 | Fresh provisioning is proposed after a later Registry transition | self | Reinstall or compile against the admitted current lifecycle path | public |
| `FLEET_COMPONENT_SCALE_OUT_RETIRED_PLAN_CONFLICT` | 1 | Retired scale-out operation ID is replayed with a different plan | self | Replay only the exact retired operation | public |
| `FLEET_COMPONENT_SCALE_OUT_PLAN_CONFLICT` | 1 | Current scale-out operation ID retains a different protected plan | self | Replay only the exact active operation | public |
| `FLEET_COMPONENT_OTHER_SCALE_OUT_ACTIVE` | 1 | Another nonterminal scale-out operation owns the Coordinator | self | Finish/recover the active scale-out first | public |
| `FLEET_COMPONENT_FRESH_PROVISIONING_MISSING` | 1 | Scale-out has no retained fresh-provisioning authority | self | Complete fresh installation first | public |
| `FLEET_COMPONENT_OPERATION_ID_REUSED` | 1 | Scale-out operation ID is already owned by fresh provisioning | self | Allocate a distinct nonzero operation ID | public |
| `FLEET_COMPONENT_FRESH_PROVISIONING_INCOMPLETE` | 1 | Retained fresh provisioning has not reached terminal runtime activation | self | Finish/recover the fresh operation first | public |
| `FLEET_COMPONENT_OPERATION_UNPREPARED` | 1 | Status names neither an active operation nor a retired scale-out receipt | self | Prepare the exact operation first | public |
| `FLEET_COMPONENT_STATUS_PLAN_CONFLICT` | 1 | Retired status lookup reuses an operation with another plan hash | self | Query with the exact retained plan hash | public |
| `FLEET_ROOT_ACCEPTANCE_START_TIME_INVALID` | 1 | Root-acceptance start time is zero | self | Supply a positive trusted observation time | public |
| `FLEET_ROOT_ACCEPTANCE_CURSOR_CONFLICT` | 1 | Response cursor differs from durable accepted-root progress | self | Reload progress and replay the exact root index | public |
| `FLEET_ROOT_ACCEPTANCE_INTENT_MISSING` | 1 | Root-acceptance response has no durable pre-call intent | self | Persist or recover the exact intent before recording a response | public |
| `FLEET_ROOT_ACCEPTANCE_COUNT_OVERFLOW` | 1 | Accepted-root receipt count cannot fit its durable field | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact projection | Stop and inspect bounded root-batch accounting | recent failure |
| `FLEET_ROOT_PROVISION_START_TIME_INVALID` | 1 | Root-provisioning start time is zero | self | Supply a positive trusted observation time | public |
| `FLEET_ROOT_PROVISION_ACCEPTANCE_INCOMPLETE` | 1 | Root provisioning begins before every root accepts its batch | self | Finish exact root acceptance first | public |
| `FLEET_ROOT_PROVISION_START_TIME_REGRESSED` | 1 | Root-provisioning start precedes the prior durable observation | self | Supply a monotonic trusted observation time | public |
| `FLEET_ROOT_PROVISION_CURSOR_TERMINAL` | 1 | Begin path has no current root response because its cursor is terminal | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve progress and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_INTENT_MISSING` | 1 | Root-provisioning response has no exact durable pre-call intent | self | Persist/recover the exact intent before recording | public |
| `FLEET_ROOT_PROVISION_OBSERVATION_TIME_REGRESSED` | 1 | Root-provisioning observation precedes its intent | self | Re-observe at a monotonic trusted time | public |

The 23 rows sum to all 23 selected source sites and introduce 23 unique exact
labels. No safe projection is added.

## Service Publication, Directory Confirmation And Runtime Activation

This slice accounts for all 14 direct constructor references at lines 914–1401.
It covers Fleet-service publication and fresh/scale-out Directory and runtime
effect intent/response boundaries.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_SERVICE_PUBLICATION_PROVISIONING_INCOMPLETE` | 1 | Fleet-service publication begins before every root is provisioned | self | Finish/recover root provisioning first | public |
| `FLEET_SERVICE_PUBLICATION_TIME_INVALID` | 1 | Publication time is zero | self | Supply a positive trusted observation time | public |
| `FLEET_SERVICE_PUBLICATION_TIME_BEFORE_PROVISIONING` | 1 | Publication predates complete root provisioning | self | Supply a monotonic observation after provisioning | public |
| `FLEET_DIRECTORY_CONFIRMATION_START_TIME_INVALID` | 1 | Directory confirmation is zero or predates service publication | self | Supply a monotonic trusted observation time | public |
| `FLEET_DIRECTORY_CONFIRMATION_MODE_INVALID` | 1 | Fresh-publication response endpoint receives a scale-out operation | self | Use the typed scale-out synchronization/publication endpoint | public |
| `FLEET_DIRECTORY_CONFIRMATION_INTENT_MISSING` | 1 | Fresh Directory response has no exact durable pre-call intent | self | Persist/recover the exact intent before recording | public |
| `FLEET_DIRECTORY_CONFIRMATION_OBSERVATION_TIME_REGRESSED` | 1 | Fresh Directory observation predates its intent | self | Re-observe at a monotonic trusted time | public |
| `FLEET_DIRECTORY_CONFIRMATION_OPERATION_CONFLICT` | 1 | Retained Directory intent operation differs from the advance operation | self | Replay only the exact retained operation | public |
| `FLEET_SCALE_OUT_SYNCHRONIZATION_INTENT_MISSING` | 1 | Scale-out synchronization response has no durable pre-call intent | self | Persist/recover the exact synchronization intent | public |
| `FLEET_SCALE_OUT_PUBLICATION_INTENT_MISSING` | 1 | Scale-out publication response has no durable pre-call intent | self | Persist/recover the exact publication intent | public |
| `FLEET_SCALE_OUT_PUBLICATION_ROOT_INDEX_CONFLICT` / `FLEET_SCALE_OUT_PUBLICATION_ROOT_CONFLICT` / `FLEET_SCALE_OUT_PUBLICATION_OPERATION_CONFLICT` / `FLEET_SCALE_OUT_PUBLICATION_PLAN_CONFLICT` / `FLEET_SCALE_OUT_PUBLICATION_REGISTRY_CONFLICT` / `FLEET_SCALE_OUT_PUBLICATION_CURSOR_CONFLICT` / `FLEET_SCALE_OUT_PUBLICATION_START_TIME_INVALID` / `FLEET_SCALE_OUT_PUBLICATION_OBSERVATION_TIME_REGRESSED` | 1 | One response branch merges root index/principal, operation, plan, Registry, member cursor and two time edges | self for every exact leaf | Preserve intent/progress/response evidence and identify the failed field | public |
| `FLEET_RUNTIME_ACTIVATION_START_TIME_INVALID` | 1 | Runtime activation is zero or predates Directory confirmation | self | Supply a monotonic trusted observation time | public |
| `FLEET_RUNTIME_ACTIVATION_INTENT_MISSING` | 1 | Runtime activation response has no exact durable pre-call intent | self | Persist/recover the exact activation intent | public |
| `FLEET_RUNTIME_ACTIVATION_OBSERVATION_TIME_REGRESSED` | 1 | Runtime activation observation predates its intent | self | Re-observe at a monotonic trusted time | public |

The 14 rows sum to all 14 selected source sites. Candidate-column extraction
finds 21 unique exact labels because the scale-out authority branch expands all
eight independent predicates. No safe projection is added.

## Root Lifecycle, Current State And Commit

This slice accounts for all 19 direct constructor references at lines
1402–1739. It covers draining reservation/publication, logical removal,
Coordinator state restoration and optimistic transition commit.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_DRAINING_RESERVATION_OPERATION_ID_INVALID` | 1 | Reservation operation ID is all zeroes | self | Generate a qualified nonzero operation ID | public |
| `ROOT_DRAINING_RESERVATION_TIME_INVALID` | 1 | Reservation preparation time is zero | self | Supply a positive trusted observation time | public |
| `ROOT_DRAINING_RESERVATION_IDENTITY_CONFLICT` | 1 | Reservation identity is already retained under different authority | self | Replay only the exact reservation | public |
| `FLEET_REGISTRY_INACTIVE` | 2 | Draining reservation or publication is attempted before active Registry genesis | self; both sites share the same prerequisite and action | Complete exact initial Registry activation | public |
| `ROOT_DRAINING_RESERVATION_UNAVAILABLE` | 1 | Status names no prepared draining reservation | self | Prepare or query the exact reservation identity | public |
| `ROOT_DRAINING_RESERVATION_STATUS_CONFLICT` | 1 | Status operation/root differs from retained reservation | self | Query the exact retained identity | public |
| `ROOT_DRAINING_PUBLICATION_IDENTITY_CONFLICT` | 1 | Draining publication identity is retained under different authority | self | Replay only the exact publication | public |
| `ROOT_DRAINING_PUBLICATION_REGISTRY_STALE` | 1 | Draining publication expects a different current Registry version | self | Reload current Registry and rebuild the request | public |
| transparent draining-publication validation adapter | 1 | String validation result is converted to `InvalidInput` | preserve exact validation leaf allocated by the lifecycle-helper slice | Correct the exact rejected request field | nested lifecycle-validation owner |
| `ROOT_REMOVAL_CALLER_MISMATCH` | 1 | Caller differs from the root named by terminal final inventory | self | Invoke from the exact root principal | public |
| `ROOT_REMOVAL_PUBLICATION_IDENTITY_CONFLICT` | 1 | Removal publication identity is retained under different authority | self | Replay only the exact removal publication | public |
| `ROOT_REMOVAL_PUBLICATION_REGISTRY_STALE` | 1 | Removal publication expects a different current Registry version | self | Reload current Registry and rebuild the request | public |
| transparent removal-publication validation adapter | 1 | String validation result is converted to `InvalidInput` | preserve exact validation leaf allocated by the lifecycle-helper slice | Correct the exact rejected request field | nested lifecycle-validation owner |
| `COORDINATOR_GENESIS_UNINITIALIZED` | 2 | Read or transition commit has no Coordinator genesis state | self; both sites share one exact prerequisite | Initialize genesis before any Registry operation | public |
| `COORDINATOR_REGISTRY_AUTHORITY_MISMATCH` | 1 | Stored Coordinator authority differs from Fleet Registry authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both authorities and fail closed | recent failure |
| `COORDINATOR_APP_AUTHORITY_MISMATCH` | 1 | Stored configured App differs from protected Fleet authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both authorities and fail closed | recent failure |
| `COORDINATOR_COMMIT_STATE_CHANGED` | 1 | Registry state changed before an optimistic transition committed | self | Reload current state and recompute/retry the exact transition | public |

The 17 rows sum to all 19 selected source sites. Two validation adapters remain
transparent, two sites share `FLEET_REGISTRY_INACTIVE`, and two share
`COORDINATOR_GENESIS_UNINITIALIZED`. The section introduces 15 unique exact
labels and no safe projection.

## Test Adapters, Retired Receipt Encoding And Status Lookup

This slice accounts for all five direct constructor references at lines
1740–3435. Three occur only in `#[cfg(test)]` helper code and receive no runtime
code. The remaining two cover canonical retired scale-out receipt encoding and
active-operation status lookup. The many protected validator failures in this
range call the shared `receipt_invariant` constructor and are owned by the later
receipt-invariant expansion, not silently counted as constructor-free success.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| test-only deployment-configuration compilation adapter | 1 | `#[cfg(test)]` helper compiles checked-in configuration and formats its rejection | no runtime code | Fix the test configuration | test only |
| test-only uninitialized Coordinator helper | 1 | `#[cfg(test)]` helper reads Coordinator state before genesis | no runtime code | Initialize the test fixture | test only |
| test-only deployment-configuration mismatch | 1 | `#[cfg(test)]` helper compares fixture and durable configuration | no runtime code | Align the fixture with retained test state | test only |
| `FLEET_COMPONENT_SCALE_OUT_RECEIPT_ENCODING_FAILED` | 1 | Canonical retired scale-out receipt cannot be Candid encoded for hashing | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the receipt and stop retirement | recent failure |
| `FLEET_COMPONENT_STATUS_PLAN_CONFLICT` | 1 | Active status lookup reuses an operation with another plan hash | self; existing exact identity | Query with the exact retained plan hash | public |

The five rows sum to all five selected source sites. One exact label is new,
one is reused and three test-only sites receive no runtime code. No safe
projection is added.

## Publication And Progress Classification

This slice accounts for all 18 direct constructor references at lines
3436–5060. It covers publication source authority and the current/replay/
reconcile classifiers for root provisioning, Directory synchronization/
publication and runtime activation.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_SERVICE_PUBLICATION_SOURCE_REGISTRY_STALE` | 1 | Operation's immutable source Registry is no longer the current Registry | self | Reconcile the lifecycle transition; do not publish against changed authority | public |
| `FLEET_SERVICE_PUBLICATION_REGISTRY_STALE` | 1 | Current Registry version differs from the provisioning plan's publication fence | self | Reload exact operation/Registry status | public |
| `FLEET_DIRECTORY_CONFIRMATION_PHASE_INVALID` | 1 | Directory confirmation is requested before published Fleet-service topology | self | Finish exact service publication first | public |
| `FLEET_RUNTIME_ACTIVATION_PHASE_INVALID` | 1 | Runtime activation is requested before confirmed Directories | self | Finish exact Directory confirmation first | public |
| `FLEET_RUNTIME_ACTIVATION_TERMINAL_CURSOR_CONFLICT` | 1 | Request is neither current nor the admitted terminal activation replay | self | Reload terminal status and replay its exact cursor | public |
| `FLEET_RUNTIME_ACTIVATION_ROOT_CURSOR_CONFLICT` | 2 | Activated-root count is neither current nor an admitted one-step replay | self; both branches share one exact meaning | Reload status and replay the exact root cursor | public |
| `FLEET_RUNTIME_ACTIVATION_COMPONENT_CURSOR_CONFLICT` | 1 | Current root's Component activation progress is not current or replayable | self | Reload status and replay the exact Component/root progress | public |
| `FLEET_DIRECTORY_CONFIRMATION_TERMINAL_CURSOR_CONFLICT` | 1 | Request is neither current nor the admitted terminal Directory replay | self | Reload terminal status and replay its exact cursor | public |
| `FLEET_DIRECTORY_CONFIRMATION_ROOT_CURSOR_CONFLICT` | 2 | Confirmed-root count is neither current nor an admitted one-step replay | self; both branches share one exact meaning | Reload status and replay the exact root cursor | public |
| `FLEET_DIRECTORY_SYNCHRONIZATION_CURSOR_CONFLICT` | 1 | Current synchronization Component cursor is neither current nor replayable | self | Reload exact synchronization progress | public |
| `FLEET_DIRECTORY_PUBLICATION_CURSOR_CONFLICT` | 1 | Current publication Component cursor is neither current nor replayable | self | Reload exact publication progress | public |
| `FLEET_ROOT_PROVISION_EXPECTED_CURSOR_CONFLICT` | 2 | Expected current-root progress differs from durable progress outside one-step replay | self; both branches share one exact meaning | Reload exact root progress | public |
| `FLEET_ROOT_PROVISION_INDEX_UNREPRESENTABLE` | 1 | Response-loss replay root index cannot address the platform collection | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact projection | Stop and inspect bounded root-count state | recent failure |
| `FLEET_ROOT_PROVISION_COUNT_CONFLICT` | 1 | Provisioned-root count is neither current nor an exact one-step replay | self | Reload exact root-count progress | public |
| `FLEET_DIRECTORY_CONFIRMATION_START_TIME_INVALID` | 1 | Scale-out Directory confirmation is zero or predates service publication | self; existing exact identity | Supply a monotonic trusted observation time | public |

The 15 rows sum to all 18 selected source sites. Fifteen unique exact labels are
present; one reuses the earlier Directory start-time identity and 14 are new.
No safe projection is added.

## Scale-Out Synchronization And Runtime Response Integrity

This slice accounts for all 16 direct constructor references at lines
5061–5655. It covers typed scale-out routing, synchronization response
authority and terminal/in-progress runtime activation evidence.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_SCALE_OUT_OPERATION_REQUIRED` | 1 | Directory synchronization receives a fresh-install operation | self | Use synchronization only for scale-out | public |
| `FLEET_SCALE_OUT_SYNC_REQUEST_OPERATION_CONFLICT` / `FLEET_SCALE_OUT_SYNC_REQUEST_PLAN_CONFLICT` / `FLEET_SCALE_OUT_SYNC_REQUEST_SOURCE_REGISTRY_CONFLICT` / `FLEET_SCALE_OUT_SYNC_REQUEST_PUBLISHED_REGISTRY_CONFLICT` / `FLEET_SCALE_OUT_SYNC_REQUEST_CURSOR_CONFLICT` / `FLEET_SCALE_OUT_SYNC_RESPONSE_OPERATION_CONFLICT` / `FLEET_SCALE_OUT_SYNC_RESPONSE_PLAN_CONFLICT` / `FLEET_SCALE_OUT_SYNC_RESPONSE_SOURCE_REGISTRY_CONFLICT` / `FLEET_SCALE_OUT_SYNC_RESPONSE_PUBLISHED_REGISTRY_CONFLICT` / `FLEET_SCALE_OUT_SYNC_RESPONSE_ROOT_CONFLICT` / `FLEET_SCALE_OUT_SYNC_RESPONSE_COUNT_EXCEEDS_AFFECTED` / `FLEET_SCALE_OUT_SYNC_RESPONSE_CURSOR_CONFLICT` / `FLEET_SCALE_OUT_SYNC_OBSERVATION_TIME_REGRESSED` | 1 | One synchronization response predicate merges five request, seven response and one observation-time authority fields | self for every exact leaf | Preserve request/response/intent and identify the exact changed field | public |
| `FLEET_SCALE_OUT_SYNC_AFFECTED_COUNT_CHANGED` / `FLEET_SCALE_OUT_SYNC_DIRECTORY_HASH_CHANGED` / `FLEET_SCALE_OUT_SYNC_PUBLICATION_ALREADY_STARTED` | 1 | Response replay merges changed affected count, changed Directory identity and premature publication evidence | self | Preserve retained/current evidence and recover the exact synchronization | public |
| `FLEET_SCALE_OUT_SYNC_DIRECTORY_AUTHORITY_MISMATCH` | 1 | Response Fleet Directory content hash differs from the Coordinator-derived Directory | self | Re-fetch/publish against exact Fleet Directory authority | public |
| `FLEET_SCALE_OUT_SYNC_COMPLETE_COUNT_MISMATCH` / `FLEET_SCALE_OUT_SYNC_COMPLETE_TIME_MISSING` / `FLEET_SCALE_OUT_SYNC_COMPLETE_TIME_REGRESSED` / `FLEET_SCALE_OUT_SYNC_COMPLETE_RECEIPT_HASH_INVALID` / `FLEET_SCALE_OUT_SYNC_INCOMPLETE_COUNT_INVALID` / `FLEET_SCALE_OUT_SYNC_INCOMPLETE_TIME_PRESENT` / `FLEET_SCALE_OUT_SYNC_INCOMPLETE_RECEIPT_HASH_PRESENT` | 1 | One branch merges complete and incomplete count, time and receipt-hash rules | self | Split terminal/in-progress evidence and retry only exact qualified progress | public |
| `FLEET_RUNTIME_ACTIVATION_CURSOR_NOT_ADVANCED` | 1 | Root response does not advance exactly one Component/root activation cursor | self | Reload status and invoke only the exact next step | public |
| `FLEET_RUNTIME_ACTIVATION_START_TIME_MISSING` | 1 | Response lacks durable activation start time | self | Recover/query exact activation intent and response | public |
| `FLEET_RUNTIME_ACTIVATION_START_TIME_CHANGED` | 1 | Response changes the start time retained by prior progress | self | Preserve prior response and reject changed evidence | public |
| `FLEET_RUNTIME_ACTIVATION_START_BEFORE_PUBLICATION` / `FLEET_RUNTIME_ACTIVATION_OBSERVATION_BEFORE_START` | 1 | One time predicate merges activation-before-publication and observation-before-activation | self | Re-observe exact progress at monotonic times | public |
| `FLEET_RUNTIME_ACTIVATION_IN_PROGRESS_ROOT_ACTIVE` / `FLEET_RUNTIME_ACTIVATION_IN_PROGRESS_COUNT_EXCEEDED` / `FLEET_RUNTIME_ACTIVATION_IN_PROGRESS_EVIDENCE_PRESENT` / `FLEET_RUNTIME_ACTIVATION_IN_PROGRESS_COMPLETION_TIME_PRESENT` / `FLEET_RUNTIME_ACTIVATION_IN_PROGRESS_RECEIPT_CHANGED` | 1 | In-progress response merges root state, Component cursor, terminal evidence/time absence and immutable publication receipt | self | Preserve publication authority and reject premature terminal evidence | public |
| `FLEET_RUNTIME_ACTIVATION_RESPONSE_PHASE_INVALID` | 1 | Response is neither `Published` progress nor `RuntimesActive` terminal state | self | Query the exact root operation phase | public |
| `FLEET_RUNTIME_ACTIVATION_PUBLICATION_PHASE_INVALID` / `FLEET_RUNTIME_ACTIVATION_PUBLICATION_AUTHORITY_MISMATCH` | 1 | Authority validator merges non-Published predecessor with changed named publication authority | self | Restore exact Published predecessor or reject changed authority | public |
| `FLEET_RUNTIME_ACTIVATION_EVIDENCE_MISSING` | 1 | Terminal response lacks activation evidence | self | Query/recover exact terminal root evidence | public |
| `FLEET_RUNTIME_ACTIVATION_COMPLETION_TIME_MISSING` | 1 | Terminal response lacks runtimes-activated time | self | Query/recover exact terminal root evidence | public |
| `FLEET_RUNTIME_ACTIVATION_ROOT_INACTIVE` / `FLEET_RUNTIME_ACTIVATION_COMPONENT_COUNT_INCOMPLETE` / `FLEET_RUNTIME_ACTIVATION_EVIDENCE_COMPONENT_COUNT_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_FLEET_OPERATION_ID_INVALID` / `FLEET_RUNTIME_ACTIVATION_INITIAL_INVENTORY_HASH_INVALID` / `FLEET_RUNTIME_ACTIVATION_COMPLETION_BEFORE_START` / `FLEET_RUNTIME_ACTIVATION_FRESH_ROOT_TIME_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_SCALE_OUT_ROOT_TIME_MISSING` / `FLEET_RUNTIME_ACTIVATION_SCALE_OUT_ROOT_TIME_AFTER_ACCEPTANCE` / `FLEET_RUNTIME_ACTIVATION_OBSERVATION_BEFORE_COMPLETION` | 1 | Terminal response collapses root/Component completion, activation identity, four operation-specific timing rules and observation ordering | self | Preserve response and identify the exact terminal predicate | public |
| `FLEET_RUNTIME_ACTIVATION_RECEIPT_HASH_INVALID` | 1 | Terminal runtime activation receipt hash is not canonical | self | Preserve response and reject noncanonical receipt | public |

The 16 rows sum to all 16 selected source sites. Compound expansion produces 51
unique exact labels; none occur in preceding qualified constructor ledgers. No
safe projection is added.

## Directory Publication And Root Acceptance Evidence

This slice accounts for all 24 direct constructor references at lines
5656–6362. It covers fresh Directory publication evidence, root acceptance
identity/progress/receipt validation and exact response-loss replay.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_DIRECTORY_RESPONSE_AUTHORITY_MISMATCH` | 1 | Named root response authority differs from the expected provisioning authority | self | Preserve predecessor/response and reject changed authority | public |
| `FLEET_DIRECTORY_RESPONSE_CURSOR_SKIPPED` / `FLEET_DIRECTORY_RESPONSE_COUNT_EXCEEDS_COMPONENTS` | 1 | Response count is not a bounded one-step advance or exceeds the frozen Component count | self | Reload exact root status and advance only one bounded member | public |
| `FLEET_DIRECTORY_RESPONSE_PUBLICATION_MISSING` | 1 | Directory response lacks publication evidence | self | Continue/query exact root publication | public |
| `FLEET_DIRECTORY_RESPONSE_REGISTRY_MISMATCH` / `FLEET_DIRECTORY_RESPONSE_FLEET_DIRECTORY_MISMATCH` | 1 | Publication evidence merges wrong Fleet Registry with wrong Fleet Directory content hash | self | Publish against both exact authorities | public |
| `FLEET_DIRECTORY_IN_PROGRESS_COMPLETION_TIME_PRESENT` / `FLEET_DIRECTORY_IN_PROGRESS_RECEIPT_CHANGED` | 1 | In-progress response contains terminal time or changes predecessor receipt | self | Preserve predecessor authority and reject premature terminal evidence | public |
| `FLEET_DIRECTORY_TERMINAL_COUNT_INCOMPLETE` / `FLEET_DIRECTORY_TERMINAL_TIME_BEFORE_PROVISIONING` / `FLEET_DIRECTORY_TERMINAL_OBSERVATION_TIME_REGRESSED` | 1 | Terminal response merges incomplete publication and two invalid time edges | self | Re-observe exact terminal publication after provisioning | public |
| `FLEET_DIRECTORY_TERMINAL_RECEIPT_HASH_INVALID` | 1 | Published Directory receipt hash is not canonical | self | Preserve response and reject noncanonical receipt | public |
| `FLEET_DIRECTORY_RESPONSE_PHASE_INVALID` | 1 | Directory response is neither `Provisioned` progress nor `Published` terminal state | self | Query exact root phase | public |
| `FLEET_DIRECTORY_COMPONENT_EVIDENCE_COUNT_MISMATCH` / `FLEET_DIRECTORY_GROUP_EVIDENCE_COUNT_MISMATCH` | 1 | Publication evidence count check merges Component cursor coverage and group-placement coverage | self | Preserve result/publication evidence and identify failed cardinality | public |
| `FLEET_DIRECTORY_COMPONENT_IDENTITY_MISMATCH` / `FLEET_DIRECTORY_COMPONENT_HASH_MISMATCH` | 1 | Component Directory evidence merges wrong Component identity with wrong Registry content hash | self | Preserve result/evidence and identify the failed field | public |
| `FLEET_DIRECTORY_GROUP_PLACEMENT_MISMATCH` / `FLEET_DIRECTORY_GROUP_HASH_MISMATCH` | 1 | Component Group evidence merges wrong placement identity with wrong canonical Directory hash | self | Preserve result/evidence and identify the failed field | public |
| `FLEET_ROOT_ACCEPTANCE_INDEX_UNREPRESENTABLE` | 1 | Accepted-root cursor cannot address the platform collection | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact projection | Stop and inspect bounded root progress | recent failure |
| `FLEET_ROOT_PROVISION_INDEX_UNREPRESENTABLE` | 1 | Provisioned-root replay cursor cannot address the platform collection | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect bounded root progress | recent failure |
| `FLEET_ROOT_PROVISION_RETRY_EVIDENCE_CONFLICT` | 1 | Exact retry supplies evidence different from the recorded root response | self | Replay the original response only | public |
| `FLEET_ROOT_ACCEPTANCE_COUNT_CONFLICT` | 1 | Expected accepted-root count is neither current nor one-step replay | self | Reload exact acceptance progress | public |
| `FLEET_COMPONENT_ROOT_INDEX_UNREPRESENTABLE` | 2 | Root-batch lookup cursor cannot address the platform collection | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; both sites share one exact meaning | Stop and inspect bounded batch state | recent failure |
| `FLEET_ROOT_ACCEPTANCE_CURSOR_TERMINAL` | 1 | Root-acceptance cursor has no planned batch | self | Finish/return terminal acceptance rather than selecting another root | public |
| `FLEET_ROOT_ACCEPTANCE_REPLAY_TOO_OLD` | 1 | Response replay is older than one durable acceptance step | self | Reload current status; the requested replay is outside the exact horizon | public |
| `FLEET_ROOT_ACCEPTANCE_RETRY_EVIDENCE_CONFLICT` | 1 | Exact retry supplies evidence different from the recorded acceptance | self | Replay the original response only | public |
| `FLEET_ROOT_ACCEPTANCE_RESPONSE_AUTHORITY_MISMATCH` | 1 | Named response identity differs from operation, plan, Registry, configuration or root authority | self | Preserve request/response and reject changed authority | public |
| `FLEET_ROOT_ACCEPTANCE_RESPONSE_PROGRESS_INVALID` | 1 | Named Accepted-state projection differs from initial zero-progress authority | self | Return the exact initial Accepted response | public |
| `FLEET_ROOT_ACCEPTANCE_RESPONSE_TIME_INVALID` | 1 | Accepted response time is zero | self | Return qualified positive acceptance evidence | public |
| `FLEET_ROOT_ACCEPTANCE_RECEIPT_HASH_INVALID` | 1 | Accepted response receipt hash is not canonical | self | Preserve response and reject noncanonical receipt | public |

The 23 rows sum to all 24 selected source sites. Candidate-column extraction
finds 31 unique exact labels; one reuses the earlier provision-index identity
and the other 30 are new. No safe projection is added.

## Root Provision Response And Current-Progress Validation

This slice accounts for all ten direct constructor references at lines
6363–6805. It covers acceptance/provisioning time edges, bounded progression,
the Coordinator publication barrier and current nonterminal root evidence.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_ROOT_ACCEPTANCE_RESPONSE_BEFORE_INTENT` / `FLEET_ROOT_ACCEPTANCE_OBSERVATION_TIME_REGRESSED` | 1 | Acceptance observation merges root acceptance before intent with Coordinator recording before root acceptance | self | Re-observe exact acceptance at monotonic times | public |
| `FLEET_ROOT_PROVISION_ACCEPTED_TIME_CHANGED` | 1 | Provisioning response changes immutable acceptance time | self | Preserve acceptance and reject changed evidence | public |
| `FLEET_ROOT_PROVISION_CURSOR_NOT_ADVANCED` | 1 | Accepted-phase response does not advance exactly one root-local cursor | self | Reload root status and advance one bounded step | public |
| `FLEET_ROOT_PROVISION_TERMINAL_BEFORE_CURSORS` | 1 | Provisioned response arrives before all root-local cursors are terminal | self | Finish exact root-local provisioning first | public |
| `FLEET_ROOT_PROVISION_INDEX_UNREPRESENTABLE` | 1 | Planned root index cannot address the platform collection | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact identity | Stop and inspect bounded root-batch state | recent failure |
| `FLEET_ROOT_PROVISION_COMPLETION_TIME_MISSING` | 1 | Provisioned response lacks completion time | self | Query/recover exact terminal root response | public |
| `FLEET_ROOT_PROVISION_COMPLETION_BEFORE_INTENT` / `FLEET_ROOT_PROVISION_OBSERVATION_TIME_REGRESSED` | 1 | Terminal response merges completion before intent with Coordinator observation before completion | self | Re-observe exact terminal response at monotonic times | public |
| `FLEET_ROOT_PROVISION_PUBLICATION_BARRIER_CROSSED` | 1 | Provisioning endpoint returns `Published` or `RuntimesActive` | self | Use Coordinator-controlled publication/activation stages | public |
| `FLEET_ROOT_PROVISION_RESPONSE_IDENTITY_MISMATCH` / `FLEET_ROOT_PROVISION_RESPONSE_PHASE_INVALID` / `FLEET_ROOT_PROVISION_RESPONSE_PLACEMENT_COUNT_MISMATCH` / `FLEET_ROOT_PROVISION_RESPONSE_COMPONENT_COUNT_MISMATCH` / `FLEET_ROOT_PROVISION_RESPONSE_RESULT_PRESENT` / `FLEET_ROOT_PROVISION_RESPONSE_COMPLETION_TIME_PRESENT` / `FLEET_ROOT_PROVISION_RESPONSE_CURSOR_INVALID` / `FLEET_ROOT_PROVISION_RESPONSE_ACCEPTED_TIME_MISMATCH` / `FLEET_ROOT_PROVISION_RESPONSE_ACCEPTANCE_RECEIPT_MISMATCH` | 1 | One current-response predicate merges named identity, six Accepted-progress fields and two immutable acceptance fields | self | Preserve acceptance/response and identify the exact failed authority | public |
| `FLEET_ROOT_PROVISION_CURRENT_START_TIME_REGRESSED` / `FLEET_ROOT_PROVISION_CURRENT_OBSERVATION_TIME_REGRESSED` | 1 | Retained current response starts before previous receipt or is recorded before its start | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve current/previous evidence and fail closed | recent failure |

The ten rows sum to all ten selected source sites. Candidate-column extraction
finds 21 unique exact labels; one reuses the provision-index identity and the
other 20 are new. No safe projection is added.

## Registry Admission, Snapshot And Root-Lifecycle Gates

This slice accounts for all 14 direct constructor references at lines
6806–7995. Thirteen are ordinary exact gates. The fourteenth is the shared
`receipt_invariant` constructor: it has 235 production call sites and cannot be
assigned one generic code. That adapter receives no code; its call sites form a
separate semantic frontier.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_COMPONENT_PLAN_SELECTS_DRAINING_ROOT` | 1 | Provisioning plan selects a root with retained draining reservation | self | Remove the root from the plan or finish/cancel its lifecycle | public |
| `ROOT_LIFECYCLE_GROUPED_AUTHORITY_FENCED` | 1 | Component operation, placement ledger or Fleet service still references the root | self | Remove every grouped authority reference before root lifecycle mutation | public |
| `FLEET_SNAPSHOT_CALLER_NOT_ROOT` | 1 | Snapshot caller is not a current non-Removed Fleet root | self | Invoke from an exact current Fleet Subnet Root | public |
| `FLEET_SNAPSHOT_CALLER_NOT_JOINING` | 1 | Acknowledgement caller is not a current `Joining` root | self | Invoke from the exact Joining root | public |
| `FLEET_SNAPSHOT_ROOT_SET_EMPTY` / `FLEET_SNAPSHOT_ROOT_NOT_JOINING` | 1 | Snapshot synchronization merges empty root set with any non-Joining root | self | Join at least one root and keep all initial roots Joining | public |
| `FLEET_SNAPSHOT_ACK_COUNT_MISMATCH` / `FLEET_SNAPSHOT_ACK_ROOT_MISSING` / `FLEET_SNAPSHOT_ACK_VERSION_MISMATCH` | 1 | Activation acknowledgement gate merges cardinality, root coverage and exact Registry version | self | Collect one exact acknowledgement from every current root | public |
| `FLEET_REGISTRY_INACTIVE` | 1 | Initial Fleet-service publication lacks active Registry genesis | self; existing exact identity | Complete exact initial Registry activation | public |
| `ROOT_DRAINING_PUBLICATION_RESERVATION_MISSING` | 1 | Draining publication has no retained reservation | self | Prepare/query the exact reservation first | public |
| `ROOT_DRAINING_PUBLICATION_RESERVATION_IDENTITY_CONFLICT` | 1 | Publication root/operation differs from retained reservation identity | self | Replay the exact reserved identity | public |
| `ROOT_DRAINING_RESERVATION_REGISTRY_STALE` | 1 | Draining reservation expects a different current Registry version | self | Reload current Registry and rebuild the reservation | public |
| `ROOT_DRAINING_RESERVATION_ROOT_INACTIVE` | 1 | Requested expected root is not `Active` | self | Reserve only the exact Active root authority | public |
| `ROOT_DRAINING_RESERVATION_TARGET_MISSING` | 1 | Reservation root principal is absent from Fleet Registry | self | Refresh Registry or correct the target | public |
| `ROOT_DRAINING_RESERVATION_ROOT_AUTHORITY_MISMATCH` | 1 | Caller-supplied expected root differs from Registry authority | self | Supply the exact current root entry | public |
| shared `receipt_invariant` adapter | 1 | One constructor receives 235 distinct protected-state call-site meanings | no code at the adapter; preserve each exact call-site identity | Replace the string funnel with exact typed leaves | 235-call semantic frontier |

The 14 rows sum to all 14 selected direct constructor sites. The ordinary rows
contain 16 unique exact labels: `FLEET_REGISTRY_INACTIVE` is reused and 15 are
new. The shared adapter remains explicitly unallocated and blocks semantic
closure until all 235 call sites are reconciled. No safe projection is added.

## Mechanical Direct-Constructor Closure

All 154 direct `InternalError::*` references in the parent file now have one
consecutive range owner with no overlap. This is not semantic closure: the
`receipt_invariant` adapter expands to the separate 235-call frontier.

## Current Coverage

The five consecutive ranges account for 72 of the parent file's 154 production
constructor references with no overlap:

| Inclusive source lines | Constructors | Owner section |
| --- | ---: | --- |
| 110–378 | 11 | Genesis, joining, snapshot and activation |
| 379–913 | 23 | Component provisioning preparation and root progress |
| 914–1401 | 14 | Service publication, Directory confirmation and runtime activation |
| 1402–1739 | 19 | Root lifecycle, current state and commit |
| 1740–3435 | 5 | Test adapters, retired receipt encoding and status lookup |
| 3436–5060 | 18 | Publication and progress classification |
| 5061–5655 | 16 | Scale-out synchronization and runtime response integrity |
| 5656–6362 | 24 | Directory publication and root acceptance evidence |
| 6363–6805 | 10 | Root provision response and current-progress validation |
| 6806–7995 | 14 | Registry admission, snapshot and root-lifecycle gates |
| **Direct sites** | **154** | **0 direct sites; 235 receipt calls remain** |

## Required Tests

- genesis authority and exact-retry conflict;
- root join, snapshot acknowledgement and Registry activation replay/stale
  matrices, including Registry-without-receipt corruption;
- fresh/scale-out plan ownership, operation reuse and terminal-fresh gate;
- root acceptance and provisioning before-intent, response-loss, replay,
  cursor and monotonic-time boundaries;
- service publication, Directory synchronization/publication and runtime
  activation with every compound intent field independently wrong;
- draining reservation/publication and removal exact retry, caller and Registry
  authority; and
- missing genesis, optimistic commit conflict and stored App/Registry authority
  corruption.
- snapshot caller/root-set/acknowledgement gates and root draining reservation
  authority; and
- a mechanical manifest and semantic expansion for every `receipt_invariant`
  call site, with no generic adapter code.

## Next Slice

Classify all 235 `receipt_invariant` call sites by protected state-machine and
receipt authority. Then reconcile the dedicated root-deletion module and
Coordinator workflow separately.
