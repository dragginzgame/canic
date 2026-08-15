# Canic 0.102 Fleet Coordinator Receipt-Invariant Frontier

Date: 2026-08-15

## Status

This B1 ledger expands the shared `receipt_invariant(&'static str)` funnel in
`crates/canic-control-plane/src/ops/fleet_coordinator/mod.rs`. It assigns no
number and changes no runtime behavior.

The parent constructor ledger correctly counts the funnel's one
`InternalError::invariant` definition, but that definition has **235 production
call sites in 102 functions**. Those call sites describe distinct protected
state, receipt, cursor, time and authority failures. The adapter itself must
receive no generic code; each call site must map to an exact typed leaf or an
explicit transparent/sediment disposition.

All 235 call sites in the parent source now have dispositions below. A
whole-repository semantic rescan also finds 10 calls in the dedicated
`root_deletion` module and 47 in `deployment_ledger`; those 57 submodule calls
remain open in their owning file ledgers. This document therefore closes the
parent-file funnel inventory, not B1 or the whole-program constructor frontier.

## Mechanical Frontier

The static call census excludes the function definition and partitions every
call by consecutive source range:

| Inclusive source lines | Calls |
| --- | ---: |
| 1–1739 | 16 |
| 1740–3435 | 85 |
| 3436–5060 | 26 |
| 5061–6805 | 70 |
| 6806–7995 | 38 |
| **Total** | **235 in 102 functions** |

The source uses a `&'static str`, so every current call is statically
enumerable. A future typed implementation must delete the string selector
rather than retain it beside numeric diagnostics.

## Public-Transition Persistence Calls

This first slice accounts for all 16 calls at lines 1–1739. These are state
contradictions discovered while public transitions recover or commit an
already-classified effect boundary; they project to the existing guarded state
diagnostic rather than exposing storage detail.

| Exact candidate | Calls / producer function | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_ROOT_PROVISION_RECONCILIATION_INTENT_MISSING` | 1; `advance_component_provisioning_root` | Reconcile disposition has no retained root-provision intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve operation state and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_RESPONSE_INTENT_LOST` | 1; `record_component_provisioning_root` | Recording path loses the pre-call root-provision intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve before/after operation evidence and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_PREDECESSOR_MISSING` | 1; `record_component_provisioning_root` | Root-provision response has no durable previous progress | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve response/progress and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_ROOTS_ACCEPTED_TIME_MISSING` | 1; `record_component_provisioning_root` | Provisioning state loses immutable RootsAccepted time | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve state and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_COUNT_UNREPRESENTABLE` | 1; `record_component_provisioning_root` | Durable provisioned-root receipt count cannot fit `u32` | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve bounded state and fail closed | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_RECONCILIATION_INTENT_MISSING` | 1; `advance_component_directory_confirmation` | Reconcile disposition has no retained Directory intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve operation state and fail closed | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_CANONICAL_ROOT_MISMATCH` | 1; `advance_component_directory_confirmation` | Selected Directory predecessor root differs from canonical plan order | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan/progress and fail closed | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_RESPONSE_INTENT_LOST` | 1; `record_component_directory_confirmation` | Fresh Directory response path loses its pre-call intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve before/after operation evidence and fail closed | recent failure |
| `FLEET_SCALE_OUT_SYNCHRONIZATION_RESPONSE_INTENT_LOST` | 1; `record_component_scale_out_directory_synchronization` | Scale-out synchronization response path loses its intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve before/after operation evidence and fail closed | recent failure |
| `FLEET_SCALE_OUT_PUBLICATION_RESPONSE_INTENT_LOST` | 1; `record_component_scale_out_directory_publication` | Scale-out publication response path loses its intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve before/after operation evidence and fail closed | recent failure |
| `FLEET_SCALE_OUT_PUBLICATION_SYNCHRONIZATION_MISSING` | 1; `record_component_scale_out_directory_publication` | Publication has no retained synchronization record | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve publication/progress and fail closed | recent failure |
| `FLEET_SCALE_OUT_PUBLICATION_SYNCHRONIZATION_INCOMPLETE` | 1; `record_component_scale_out_directory_publication` | Publication begins while retained synchronization is nonterminal | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve state; do not infer synchronization completion | recent failure |
| `FLEET_RUNTIME_ACTIVATION_RECONCILIATION_INTENT_MISSING` | 1; `advance_component_runtime_activation` | Reconcile disposition has no retained runtime-activation intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve operation state and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_CANONICAL_ROOT_MISMATCH` | 1; `advance_component_runtime_activation` | Selected activation progress root differs from canonical plan order | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan/progress and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_RESPONSE_INTENT_LOST` | 1; `record_component_runtime_activation` | Runtime-activation response path loses its pre-call intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve before/after operation evidence and fail closed | recent failure |
| `COORDINATOR_DEPLOYMENT_CONFIGURATION_INVALID` | 1; `validate_current_registry` | Stored Component deployment configuration fails its canonical digest validation | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve configuration and fail closed | recent failure |

The 16 rows sum to all 16 selected calls and introduce 16 unique exact labels.
No safe projection is added: every row reuses the already qualified guarded
state projection.

## Provisioning Record And Retired Scale-Out Authority

This slice accounts for all 32 calls at lines 1740–2170. It covers canonical
fresh-operation state, active scale-out state and the permanent retired
scale-out receipt ledger. Compound receipt validators deliberately expand every
immutable identity, count, time, Registry, placement and publication field.

| Exact candidate | Calls / producer function | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_SERVICE_PUBLICATION_OPERATION_MISSING` | 2; `validate_component_provisioning_record`, `validate_service_publication_receipt_owners` | A Fleet-service publication receipt has no active or retired provisioning owner | `COMPONENT_REGISTRY_STATE_INVALID`; both owner scans share one exact meaning | Preserve receipt/operation ledgers and fail closed | recent failure |
| `FLEET_COMPONENT_RECORD_OPERATION_ID_INVALID` | 1; `validate_component_provisioning_record` | Retained fresh-provisioning operation ID is zero | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve record and fail closed | recent failure |
| `FLEET_COMPONENT_RECORD_PLAN_HASH_INVALID` | 1; `validate_component_provisioning_record` | Retained fresh-provisioning plan hash is zero | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve record and fail closed | recent failure |
| `FLEET_COMPONENT_RECORD_OPERATION_KIND_INVALID` | 1; `validate_component_provisioning_record` | Fresh-provisioning slot contains a non-fresh operation | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve record and fail closed | recent failure |
| `FLEET_COMPONENT_RECORD_PLAN_INVALID` | 1; `validate_component_provisioning_record` | Retained plan no longer validates against canonical configuration/Registry authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan and its source authority | recent failure |
| `FLEET_COMPONENT_RECORD_PLAN_HASH_REDERIVATION_FAILED` | 1; `validate_component_provisioning_record` | Canonical retained-plan hash cannot be recomputed | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan and stop restoration | recent failure |
| `FLEET_COMPONENT_RECORD_PLAN_HASH_MISMATCH` | 1; `validate_component_provisioning_record` | Retained plan hash differs from canonical bytes | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve record and fail closed | recent failure |
| `FLEET_SCALE_OUT_RECORD_OPERATION_KIND_INVALID` | 1; `validate_component_scale_out_progress` | Active scale-out slot contains a non-scale-out operation | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve record and fail closed | recent failure |
| `FLEET_SCALE_OUT_RECORD_PHASE_INVALID` | 1; `validate_component_scale_out_progress` | Active scale-out record contains an unavailable state phase | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve record and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_RECEIPT_LIMIT_EXCEEDED` | 1; `validate_component_scale_out_receipts` | Retired receipt count exceeds the bounded placement-derived ceiling | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve ledger and stop mutation | recent failure |
| `COORDINATOR_DEPLOYMENT_CONFIGURATION_INVALID` | 1; `validate_component_scale_out_receipts` | Current deployment configuration digest cannot be rederived | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve configuration and fail closed | recent failure |
| `FLEET_SCALE_OUT_ACTIVE_PLAN_TIME_BEFORE_RETIRED_HISTORY` | 1; `validate_component_scale_out_receipts` | Active scale-out planned time predates last retired completion | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve active/retired journals and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_OPERATION_KIND_INVALID` | 1; `retired_scale_out_authority` | Retired receipt is not a scale-out operation | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_PLACEMENT_COUNT_NONMONOTONIC` | 1; `retired_scale_out_authority` | Requested placement total does not strictly exceed its previous total | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and reject invalid range | recent failure |
| `FLEET_SCALE_OUT_RETIRED_PLACEMENT_COUNT_UNREPRESENTABLE` | 1; `retired_scale_out_authority` | Retired placement delta cannot index the platform collection | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve bounded receipt and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_OPERATION_ID_INVALID` / `FLEET_SCALE_OUT_RETIRED_PLAN_HASH_INVALID` / `FLEET_SCALE_OUT_RETIRED_OPERATION_REUSES_FRESH` / `FLEET_SCALE_OUT_RETIRED_OPERATION_REUSES_ACTIVE` / `FLEET_SCALE_OUT_RETIRED_OPERATION_DUPLICATE` / `FLEET_SCALE_OUT_RETIRED_CONFIGURATION_MISMATCH` | 1; `validate_retired_scale_out_identity` | One identity predicate merges zero operation/plan, fresh/active reuse, retired duplication and configuration digest | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve all journals and identify the exact collision | recent failure |
| `FLEET_SCALE_OUT_RETIRED_RECEIPT_HASH_MISSING` / `FLEET_SCALE_OUT_RETIRED_RECEIPT_HASH_MISMATCH` | 1; `validate_retired_scale_out_content_hash` | Retired receipt hash is zero or differs from canonical bytes | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve receipt and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_ROOT_COUNT_UNREPRESENTABLE` | 2; `validate_retired_scale_out_counts`, `validate_retired_scale_out_placements` | Retired root count cannot index platform collections | `COMPONENT_REGISTRY_STATE_INVALID`; placement validator reuses the exact meaning | Preserve receipt and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_CONFIRMATION_COUNT_UNREPRESENTABLE` | 1; `validate_retired_scale_out_counts` | Retired Directory-confirmation root count cannot index platform collections | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_COMPONENT_COUNT_UNREPRESENTABLE` | 1; `validate_retired_scale_out_counts` | Retired Component count cannot index platform collections | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_PLACEMENT_LENGTH_MISMATCH` / `FLEET_SCALE_OUT_RETIRED_ROOT_COUNT_ZERO` / `FLEET_SCALE_OUT_RETIRED_ROOT_COUNT_LIMIT_EXCEEDED` / `FLEET_SCALE_OUT_RETIRED_CONFIRMATION_COUNT_BELOW_ROOTS` / `FLEET_SCALE_OUT_RETIRED_CONFIRMATION_COUNT_LIMIT_EXCEEDED` / `FLEET_SCALE_OUT_RETIRED_COMPONENT_COUNT_BELOW_PLACEMENTS` / `FLEET_SCALE_OUT_RETIRED_COMPONENT_COUNT_LIMIT_EXCEEDED` | 1; `validate_retired_scale_out_counts` | One bounded-count predicate merges placement coverage, nonzero/maximum roots, confirmation coverage/maximum and Component minimum/maximum | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve receipt and identify the exact invalid bound | recent failure |
| `FLEET_SCALE_OUT_RETIRED_PLAN_BEFORE_PREVIOUS_COMPLETION` / `FLEET_SCALE_OUT_RETIRED_PLAN_TIME_INVALID` / `FLEET_SCALE_OUT_RETIRED_ROOTS_ACCEPTED_TIME_REGRESSED` / `FLEET_SCALE_OUT_RETIRED_COMPONENTS_PROVISIONED_TIME_REGRESSED` / `FLEET_SCALE_OUT_RETIRED_SERVICE_PUBLICATION_TIME_REGRESSED` / `FLEET_SCALE_OUT_RETIRED_DIRECTORIES_CONFIRMED_TIME_REGRESSED` / `FLEET_SCALE_OUT_RETIRED_RUNTIMES_ACTIVATED_TIME_REGRESSED` | 1; `validate_retired_scale_out_times` | One ordering predicate merges prior-history fence, nonzero planned time and five terminal transition edges | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve ordered receipt history and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_SOURCE_REGISTRY_AUTHORITY_MISMATCH` / `FLEET_SCALE_OUT_RETIRED_PUBLISHED_REGISTRY_AUTHORITY_MISMATCH` / `FLEET_SCALE_OUT_RETIRED_REGISTRY_REVISION_REGRESSED` | 1; `validate_retired_scale_out_registry` | Registry validator merges source authority, published authority and monotonic revision | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve both Registry versions and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_PLACEMENT_OFFSET_UNREPRESENTABLE` | 1; `validate_retired_scale_out_placements` | Placement offset cannot fit the canonical ordinal field | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_PLACEMENT_ORDINAL_OVERFLOW` | 1; `validate_retired_scale_out_placements` | Previous placement count plus offset overflows | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_PLACEMENT_DEPLOYMENT_MISMATCH` / `FLEET_SCALE_OUT_RETIRED_PLACEMENT_ORDINAL_MISMATCH` / `FLEET_SCALE_OUT_RETIRED_PLACEMENT_OPERATION_MISMATCH` / `FLEET_SCALE_OUT_RETIRED_PLACEMENT_PLAN_MISMATCH` / `FLEET_SCALE_OUT_RETIRED_PLACEMENT_ROOT_RECEIPT_HASH_INVALID` | 1; `validate_retired_scale_out_placements` | Placement authority merges deployment, ordinal, operation, plan and nonzero root receipt hash | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve placement/receipt and identify the failed field | recent failure |
| `FLEET_SCALE_OUT_RETIRED_SELECTED_ROOT_EVIDENCE_MISMATCH` | 1; `validate_retired_scale_out_placements` | Unique `(root, receipt hash)` coverage differs from retained root count | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_PUBLICATION_MISSING` | 1; `validate_retired_scale_out_publication` | Retired scale-out has no Fleet-service publication receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve retired receipt and publication ledger | recent failure |
| `FLEET_SCALE_OUT_RETIRED_PUBLICATION_OPERATION_MISMATCH` / `FLEET_SCALE_OUT_RETIRED_PUBLICATION_PLAN_MISMATCH` / `FLEET_SCALE_OUT_RETIRED_PUBLICATION_CONFIGURATION_MISMATCH` / `FLEET_SCALE_OUT_RETIRED_PUBLICATION_SOURCE_REGISTRY_MISMATCH` / `FLEET_SCALE_OUT_RETIRED_PUBLICATION_TARGET_REGISTRY_MISMATCH` | 1; `validate_retired_scale_out_publication` | Publication authority merges operation, plan, configuration and both Registry versions | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve both receipts and identify the failed field | recent failure |
| `FLEET_SCALE_OUT_CURRENT_ROOT_CROSSED_PROVISIONING_FENCE` | 1; `validate_scale_out_service_publication_fence` | Active scale-out current root progressed beyond `Accepted` before Coordinator publication | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve root/Coordinator progress and fail closed | recent failure |

The 30 rows sum to all 32 selected calls. Candidate-column extraction expands
them to 58 exact-label occurrences and 58 unique labels in this slice. One
label reuses the preceding configuration identity, so this slice adds 57 exact
meanings. No safe projection is added.

## Runtime Activation Barrier And Stored Evidence

This slice accounts for all 19 calls at lines 2171–2443. It covers the
Directory-to-runtime barrier, ordered completed/current activation history,
stored root progress, terminal activation authority and the retained pre-call
intent. Compound predicates are split by protected field and operation-specific
time rule.

| Exact candidate | Calls / producer function | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_DIRECTORY_CONFIRMATION_ROOT_COUNT_UNREPRESENTABLE` | 1; `validate_component_directory_confirmation_state` | Selected root-batch count cannot fit the durable Directory barrier field | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan/progress and fail closed | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_SCALE_OUT_ROOT_COUNT_EXCEEDS_BARRIER` / `FLEET_DIRECTORY_CONFIRMATION_SCALE_OUT_SELECTED_ROOT_OUTSIDE_BARRIER` / `FLEET_DIRECTORY_CONFIRMATION_FRESH_ROOT_COUNT_MISMATCH` / `FLEET_DIRECTORY_CONFIRMATION_CONFIRMED_COUNT_EXCEEDS_BARRIER` | 1; `validate_component_directory_confirmation_state` | One barrier predicate merges scale-out count/coverage, fresh exact count and completed-cursor bounds | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve plan and barrier; identify the exact failed bound | recent failure |
| `FLEET_RUNTIME_ACTIVATION_ROOT_COUNT_UNREPRESENTABLE` | 1; `validate_component_runtime_activation_state` | Selected activation root count cannot fit the durable progress field | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan/progress and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_ROOT_COUNT_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_ACTIVATED_COUNT_EXCEEDS_ROOTS` | 1; `validate_component_runtime_activation_state` | Activation barrier merges selected-root equality and completed-cursor bounds | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve activation progress and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_ROOT_INDEX_UNREPRESENTABLE` | 1; `validate_component_runtime_activation_state` | Completed activation history index cannot fit the canonical root ordinal | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve bounded history and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_COMPLETED_START_BEFORE_HISTORY` / `FLEET_RUNTIME_ACTIVATION_COMPLETED_OBSERVATION_BEFORE_START` | 1; `validate_component_runtime_activation_state` | A completed activation starts before its predecessor or is recorded before it starts | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve ordered history and reject the regressed edge | recent failure |
| `FLEET_RUNTIME_ACTIVATION_CURRENT_START_BEFORE_HISTORY` / `FLEET_RUNTIME_ACTIVATION_CURRENT_OBSERVATION_BEFORE_START` | 1; `validate_component_runtime_activation_state` | Current activation starts before completed history or is recorded before it starts | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve current/completed progress and reject the regressed edge | recent failure |
| `FLEET_RUNTIME_ACTIVATION_START_TIME_MISSING` | 1; `validate_stored_runtime_activation` | Stored root activation lacks its durable start time | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified response identity | Preserve progress and recover/query exact activation evidence | recent failure |
| `FLEET_ROOT_PUBLICATION_COMPLETION_TIME_MISSING` | 1; `validate_stored_runtime_activation` | Stored Published predecessor lacks its completion time | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve publication evidence and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_STORED_ROOT_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_STORED_COMPONENT_COUNT_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_COMPONENT_COUNT_INCOMPLETE` / `FLEET_RUNTIME_ACTIVATION_ROOT_INACTIVE` / `FLEET_RUNTIME_ACTIVATION_IN_PROGRESS_COUNT_ZERO` / `FLEET_RUNTIME_ACTIVATION_IN_PROGRESS_COUNT_EXCEEDED` / `FLEET_RUNTIME_ACTIVATION_START_BEFORE_PUBLICATION` / `FLEET_RUNTIME_ACTIVATION_OBSERVATION_BEFORE_START` | 1; `validate_stored_runtime_activation` | Stored progress merges root/count authority, terminal root/count state, nonterminal cursor bounds and two time edges | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf; five identities reuse qualified response leaves | Preserve progress/publication and identify the exact failed field | recent failure |
| `FLEET_RUNTIME_ACTIVATION_IN_PROGRESS_EVIDENCE_PRESENT` / `FLEET_RUNTIME_ACTIVATION_IN_PROGRESS_COMPLETION_TIME_PRESENT` / `FLEET_RUNTIME_ACTIVATION_IN_PROGRESS_RECEIPT_CHANGED` | 1; `validate_stored_runtime_activation` | In-progress stored state carries terminal evidence/time or changes immutable publication receipt authority | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf; all reuse qualified response identities | Preserve the Published predecessor and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_EVIDENCE_MISSING` | 1; `validate_stored_terminal_runtime_activation` | Stored terminal activation lacks its activation evidence | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified response identity | Preserve terminal progress and query/recover exact evidence | recent failure |
| `FLEET_RUNTIME_ACTIVATION_COMPLETION_TIME_MISSING` | 2; `validate_stored_terminal_runtime_activation`, `validate_terminal_runtime_activation_state` | Stored terminal activation or aggregate terminal progress lacks its completion time | `COMPONENT_REGISTRY_STATE_INVALID`; both calls reuse one qualified response identity | Preserve terminal progress and query/recover exact evidence | recent failure |
| `FLEET_RUNTIME_ACTIVATION_EVIDENCE_COMPONENT_COUNT_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_FLEET_OPERATION_ID_INVALID` / `FLEET_RUNTIME_ACTIVATION_INITIAL_INVENTORY_HASH_INVALID` / `FLEET_RUNTIME_ACTIVATION_COMPLETION_BEFORE_START` / `FLEET_RUNTIME_ACTIVATION_FRESH_ROOT_TIME_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_SCALE_OUT_ROOT_TIME_MISSING` / `FLEET_RUNTIME_ACTIVATION_SCALE_OUT_ROOT_TIME_AFTER_ACCEPTANCE` / `FLEET_RUNTIME_ACTIVATION_OBSERVATION_BEFORE_COMPLETION` | 1; `validate_stored_terminal_runtime_activation` | Terminal stored evidence merges Component identity, operation/inventory authority, operation-specific timing and observation ordering | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf; all reuse qualified response identities | Preserve terminal evidence and identify the exact failed field | recent failure |
| `FLEET_RUNTIME_ACTIVATION_RECEIPT_HASH_INVALID` | 1; `validate_stored_terminal_runtime_activation` | Stored terminal activation receipt hash differs from canonical bytes | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified response identity | Preserve receipt and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_INTENT_ROOT_INDEX_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_INTENT_ROOT_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_INTENT_OPERATION_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_INTENT_PLAN_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_INTENT_COMPONENT_CURSOR_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_INTENT_ROOT_STATE_MISMATCH` / `FLEET_RUNTIME_ACTIVATION_INTENT_START_TIME_REGRESSED` | 1; `validate_runtime_activation_intent` | Retained pre-call intent merges root ordinal/identity, operation/plan, expected progress and time authority | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve intent/progress and identify the exact changed field | recent failure |
| `FLEET_RUNTIME_ACTIVATION_TERMINAL_ROOT_COUNT_INCOMPLETE` / `FLEET_RUNTIME_ACTIVATION_TERMINAL_CURRENT_PROGRESS_PRESENT` / `FLEET_RUNTIME_ACTIVATION_TERMINAL_INTENT_PRESENT` / `FLEET_RUNTIME_ACTIVATION_TERMINAL_COMPLETION_TIME_REGRESSED` | 1; `validate_terminal_runtime_activation_state` | Aggregate terminal state merges complete root coverage, absence of current/intent state and completion ordering | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve terminal state and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_NONTERMINAL_AFTER_ALL_ROOTS` | 1; `validate_terminal_runtime_activation_state` | Nonterminal state has already exhausted every selected root | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve progress and fail closed | recent failure |

The 18 rows sum to all 19 selected calls. The candidate column contains 49
unique labels; weighting the completion-time label's two calls produces 50
exact-label occurrences. Twenty-one occurrences reuse 20 qualified response
identities, so this slice adds 29 exact meanings. No safe projection is added.

## Stored Directory Progress And Intent Authority

This slice accounts for all 16 calls at lines 2444–2800. It covers ordered
completed/current Directory history, stored scale-out synchronization and
publication evidence, the three retained intent variants and the aggregate
terminal barrier. Nested response validators are transparent future
propagation points: giving those adapters another umbrella identity would
discard the exact typed leaf they already compute.

| Exact candidate or disposition | Calls / producer function | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_DIRECTORY_CONFIRMATION_ROOT_INDEX_UNREPRESENTABLE` | 1; `validate_completed_directory_confirmations` | Completed Directory history index cannot fit the canonical root ordinal | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve bounded history and fail closed | recent failure |
| transparent: exact typed Directory-response validation | 3; `validate_completed_directory_confirmations`, `validate_current_directory_confirmation`, `validate_stored_scale_out_confirmation` | Stored fresh completed/current and scale-out publication adapters currently erase the nested response diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | nested operation status |
| `FLEET_DIRECTORY_STORED_COMPLETED_PHASE_INVALID` | 1; `validate_completed_directory_confirmations` | Completed fresh Directory history is not `Published` | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve history and reject nonterminal evidence | recent failure |
| `FLEET_DIRECTORY_COMPLETED_START_BEFORE_HISTORY` / `FLEET_DIRECTORY_COMPLETED_OBSERVATION_BEFORE_START` | 1; `validate_completed_directory_confirmations` | A completed confirmation starts before its predecessor or is recorded before it starts | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve ordered history and reject the regressed edge | recent failure |
| `FLEET_DIRECTORY_STORED_CURRENT_PHASE_INVALID` | 1; `validate_current_directory_confirmation` | Current fresh Directory progress has already crossed from `Provisioned` to terminal | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve current progress and fail closed | recent failure |
| `FLEET_DIRECTORY_CURRENT_START_BEFORE_HISTORY` / `FLEET_DIRECTORY_CURRENT_OBSERVATION_BEFORE_START` | 1; `validate_current_directory_confirmation` | Current confirmation starts before completed history or is recorded before it starts | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve current/completed progress and reject the regressed edge | recent failure |
| `FLEET_SCALE_OUT_SYNC_RESPONSE_OPERATION_CONFLICT` / `FLEET_SCALE_OUT_SYNC_RESPONSE_PLAN_CONFLICT` / `FLEET_SCALE_OUT_SYNC_RESPONSE_SOURCE_REGISTRY_CONFLICT` / `FLEET_SCALE_OUT_SYNC_RESPONSE_PUBLISHED_REGISTRY_CONFLICT` / `FLEET_SCALE_OUT_SYNC_RESPONSE_ROOT_CONFLICT` / `FLEET_SCALE_OUT_SYNC_DIRECTORY_AUTHORITY_MISMATCH` / `FLEET_SCALE_OUT_SYNC_RESPONSE_COUNT_EXCEEDS_AFFECTED` | 1; `validate_stored_scale_out_confirmation` | Stored synchronization authority merges operation, plan, both Registries, root, Directory hash and cursor bound | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf; all reuse qualified response identities | Preserve synchronization and identify the exact changed field | recent failure |
| `FLEET_SCALE_OUT_SYNC_COMPLETE_COUNT_MISMATCH` / `FLEET_SCALE_OUT_SYNC_COMPLETE_TIME_MISSING` / `FLEET_SCALE_OUT_SYNC_COMPLETE_TIME_REGRESSED` / `FLEET_SCALE_OUT_SYNC_COMPLETE_RECEIPT_HASH_INVALID` / `FLEET_SCALE_OUT_SYNC_INCOMPLETE_COUNT_INVALID` / `FLEET_SCALE_OUT_SYNC_INCOMPLETE_TIME_PRESENT` / `FLEET_SCALE_OUT_SYNC_INCOMPLETE_RECEIPT_HASH_PRESENT` | 1; `validate_stored_scale_out_confirmation` | Stored synchronization merges terminal/in-progress count, time and receipt rules | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf; all reuse qualified response identities | Preserve evidence and identify the exact failed terminality rule | recent failure |
| `FLEET_SCALE_OUT_PUBLICATION_SYNCHRONIZATION_INCOMPLETE` | 1; `validate_stored_scale_out_confirmation` | Stored scale-out publication exists before synchronization is terminal | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the preceding receipt identity | Preserve synchronization/publication and fail closed | recent failure |
| `FLEET_DIRECTORY_STORED_SCALE_OUT_TERMINAL_PUBLICATION_PHASE_INVALID` / `FLEET_DIRECTORY_STORED_SCALE_OUT_CURRENT_PUBLICATION_PHASE_INVALID` | 1; `validate_stored_scale_out_confirmation` | Stored scale-out publication terminality differs from completed/current history position | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve publication/history and reject the wrong phase | recent failure |
| `FLEET_DIRECTORY_UNSELECTED_ROOT_PUBLICATION_PRESENT` / `FLEET_DIRECTORY_UNSELECTED_TERMINAL_SYNCHRONIZATION_INCOMPLETE` / `FLEET_DIRECTORY_UNSELECTED_CURRENT_SYNCHRONIZATION_COMPLETE` / `FLEET_DIRECTORY_SELECTED_TERMINAL_PUBLICATION_MISSING` | 1; `validate_stored_scale_out_confirmation` | Root-evidence matcher merges selection, synchronization terminality and publication presence | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve root evidence and identify the exact invalid combination | recent failure |
| `FLEET_DIRECTORY_SCALE_OUT_PUBLICATION_INTENT_SYNCHRONIZATION_MISSING` | 1; `validate_directory_confirmation_intent` | Retained scale-out publication intent has no current synchronization evidence | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve intent and fail closed | recent failure |
| `FLEET_DIRECTORY_FRESH_INTENT_ROOT_INDEX_MISMATCH` / `FLEET_DIRECTORY_FRESH_INTENT_ROOT_MISMATCH` / `FLEET_DIRECTORY_FRESH_INTENT_OPERATION_MISMATCH` / `FLEET_DIRECTORY_FRESH_INTENT_PLAN_MISMATCH` / `FLEET_DIRECTORY_FRESH_INTENT_REGISTRY_MISMATCH` / `FLEET_DIRECTORY_FRESH_INTENT_COMPONENT_CURSOR_MISMATCH` / `FLEET_DIRECTORY_FRESH_INTENT_START_TIME_REGRESSED` / `FLEET_DIRECTORY_SCALE_OUT_SYNC_INTENT_ROOT_INDEX_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_SYNC_INTENT_ROOT_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_SYNC_INTENT_OPERATION_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_SYNC_INTENT_PLAN_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_SYNC_INTENT_SOURCE_REGISTRY_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_SYNC_INTENT_PUBLISHED_REGISTRY_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_SYNC_INTENT_COMPONENT_CURSOR_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_SYNC_INTENT_START_TIME_REGRESSED` / `FLEET_DIRECTORY_SCALE_OUT_PUBLICATION_INTENT_SYNCHRONIZATION_INCOMPLETE` / `FLEET_DIRECTORY_SCALE_OUT_PUBLICATION_INTENT_ROOT_INDEX_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_PUBLICATION_INTENT_ROOT_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_PUBLICATION_INTENT_OPERATION_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_PUBLICATION_INTENT_PLAN_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_PUBLICATION_INTENT_REGISTRY_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_PUBLICATION_INTENT_COMPONENT_CURSOR_MISMATCH` / `FLEET_DIRECTORY_SCALE_OUT_PUBLICATION_INTENT_START_TIME_REGRESSED` | 1; `validate_directory_confirmation_intent` | One variant match merges every immutable root, operation, plan, Registry, cursor, synchronization and time field across fresh publication, scale-out synchronization and scale-out publication intents | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve the typed intent and identify its exact changed field | recent failure |
| `FLEET_DIRECTORY_TERMINAL_ROOT_COUNT_INCOMPLETE` / `FLEET_DIRECTORY_TERMINAL_COMPLETION_TIME_REGRESSED` | 1; `validate_terminal_directory_confirmation` | Aggregate terminal Directory state merges full root coverage and completion ordering | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve terminal state and fail closed | recent failure |

The 14 rows sum to all 16 selected calls. Three calls are transparent nested
validation adapters. Candidate-column extraction produces 54 unique exact
labels for the other 13 calls: 15 reuse qualified response/receipt identities
and 39 are new. No safe projection is added.

## Retired Response And Atomic Service Publication

This slice accounts for all 18 calls at lines 2801–3435. It covers terminal
scale-out receipt creation/replay, the active publication receipt's exact
provisioning authority, its atomic state pairing and operation-record lookup
after a disposition has already selected a path.

| Exact candidate or disposition | Calls / producer function | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_SCALE_OUT_RETIREMENT_STATE_NONTERMINAL` | 1; `component_scale_out_terminal_receipt` | Receipt retirement is attempted before the active scale-out reaches `RuntimesActivated` | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve active state; retire only exact terminal authority | recent failure |
| `FLEET_SCALE_OUT_RETIRED_OPERATION_KIND_INVALID` | 2; `component_scale_out_terminal_receipt`, `component_scale_out_receipt_response` | Terminal receipt creation or replay encounters a non-scale-out operation | `COMPONENT_REGISTRY_STATE_INVALID`; both calls reuse the qualified retired-receipt identity | Preserve record/receipt and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_PLACEMENT_COUNT_NONMONOTONIC` | 1; `component_scale_out_receipt_response` | Retired response cannot derive a positive placement delta | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified retired-receipt identity | Preserve receipt and fail closed | recent failure |
| `FLEET_SCALE_OUT_RETIRED_OPERATION_DUPLICATE` | 1; `component_scale_out_receipt_for_operation` | More than one retired receipt owns the same operation ID | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified retired-receipt identity | Preserve ledger and reject ambiguous replay | recent failure |
| `FLEET_SERVICE_PUBLICATION_OPERATION_MISMATCH` / `FLEET_SERVICE_PUBLICATION_PLAN_MISMATCH` / `FLEET_SERVICE_PUBLICATION_CONFIGURATION_MISMATCH` | 1; `validate_service_publication_authority` | Atomic publication receipt differs from its active operation, plan or deployment configuration | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve both authorities and identify the exact changed field | recent failure |
| `FLEET_SERVICE_PUBLICATION_TIME_BEFORE_PROVISIONING` | 1; `validate_service_publication_authority` | Publication completion predates complete root provisioning | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified public validation identity | Preserve ordered evidence and fail closed | recent failure |
| transparent: exact typed Component-service compilation | 1; `validate_service_publication_authority` | Publication validator currently erases the exact service-compilation diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | nested operation status |
| `FLEET_SERVICE_PUBLICATION_PREVIOUS_REGISTRY_MISMATCH` / `FLEET_SERVICE_PUBLICATION_PUBLISHED_REGISTRY_MISMATCH` / `FLEET_SERVICE_PUBLICATION_ROOT_RECEIPT_HASHES_MISMATCH` / `FLEET_SERVICE_PUBLICATION_SERVICES_MISMATCH` | 1; `validate_service_publication_authority` | Atomic receipt merges source/target Registry, ordered root receipt hashes and canonical compiled services | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve terminal evidence and identify the exact changed field | recent failure |
| `FLEET_SERVICE_PUBLICATION_ATOMIC_RECEIPT_MISSING` | 1; `paired_service_publication_evidence` | Published operation state exists without its atomically committed receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve state and fail closed | recent failure |
| `FLEET_SERVICE_PUBLICATION_ATOMIC_STATE_MISSING` | 1; `paired_service_publication_evidence` | Publication receipt exists without its atomically committed operation state | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and fail closed | recent failure |
| `FLEET_SERVICE_PUBLICATION_RECEIPT_DUPLICATE` | 1; `service_publication_receipt_for_operation` | More than one service-publication receipt owns one operation ID | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve ledger and reject ambiguous replay | recent failure |
| `FLEET_COMPONENT_ACTIVE_OPERATION_RECORD_MISSING` | 1; `require_component_provisioning_operation_record` | Advance disposition selected an active operation that no longer exists | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve caller/operation evidence and fail closed | recent failure |
| `FLEET_COMPONENT_FRESH_OPERATION_RECORD_MISSING` | 1; `component_provisioning_record_mut` | Fresh-provisioning mutation loses its selected record | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve selected record and fail closed | recent failure |
| `FLEET_COMPONENT_OPERATION_RECORD_MISSING` | 3; `component_provisioning_operation_record`, `component_provisioning_operation_record_mut` | Immutable or mutable active-operation lookup loses the selected fresh/scale-out record | `COMPONENT_REGISTRY_STATE_INVALID`; all three calls share one exact meaning | Preserve operation selection and fail closed | recent failure |
| `FLEET_SERVICE_PUBLICATION_DISPOSITION_STATE_INVALID` | 1; `components_provisioned_state` | Publication disposition no longer owns the required `ComponentsProvisioned` state | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the selected transition state and fail closed | recent failure |

The 15 rows sum to all 18 selected calls. One call is a transparent nested
validation adapter. Candidate-column extraction produces 20 unique exact
labels for the other 17 calls: four reuse qualified identities and 16 are new.
No safe projection is added.

## Progress Projection And Directory Variant Integrity

This slice accounts for all 26 calls at lines 3436–5060. It covers bounded
progress-view conversions, terminal replay cursors, selected-root lookup and
the disjoint fresh-publication/scale-out synchronization/scale-out publication
record variants.

| Exact candidate | Calls / producer function | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_SERVICE_PUBLICATION_ATOMIC_STATE_MISSING` | 1; `compile_service_publication` | A `ComponentsProvisioned` operation already has publication-receipt evidence without published state | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified atomic-pair identity | Preserve receipt/state and fail closed | recent failure |
| `FLEET_ROOT_ACCEPTANCE_BATCH_COUNT_UNREPRESENTABLE` | 1; `component_provisioning_root_acceptance_progress` | Planned root-batch count cannot fit the durable acceptance progress field | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan and fail closed | recent failure |
| `FLEET_ROOT_ACCEPTANCE_COUNT_UNREPRESENTABLE` | 1; `root_acceptance_progress_from_parts` | Retained accepted-root count cannot fit the durable progress field | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve acceptance history and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_COUNT_UNREPRESENTABLE` | 2; `active_root_provision_progress`, `terminal_root_provision_progress` | Active or terminal provisioned-root count cannot fit the durable progress field | `COMPONENT_REGISTRY_STATE_INVALID`; both projections share one exact meaning | Preserve bounded provisioning history and fail closed | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_ROOT_COUNT_UNREPRESENTABLE` | 1; `component_directory_confirmation_progress` | Planned confirmation-root count cannot fit the durable Directory barrier | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified barrier identity | Preserve plan/barrier and fail closed | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_COUNT_UNREPRESENTABLE` | 3; `component_directory_confirmation_progress`, `terminal_downstream_directory_progress` | Active or terminal confirmed-root history cannot fit the durable progress field | `COMPONENT_REGISTRY_STATE_INVALID`; all projections share one exact meaning | Preserve bounded Directory history and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_ROOT_COUNT_UNREPRESENTABLE` | 1; `component_runtime_activation_progress` | Planned activation-root count cannot fit the durable activation barrier | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified barrier identity | Preserve plan/barrier and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATED_COUNT_UNREPRESENTABLE` | 1; `component_runtime_activation_progress` | Completed activation history count cannot fit the durable progress field | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve bounded activation history and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_TERMINAL_ROOT_RECEIPT_MISSING` | 1; `terminal_runtime_activation_replay` | Terminal replay has no last root-activation receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve terminal history and fail closed | recent failure |
| `FLEET_DIRECTORY_TERMINAL_ROOT_RECEIPT_MISSING` | 1; `terminal_directory_confirmation_replay` | Terminal replay has no last Directory-confirmation receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve terminal history and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_TERMINAL_RECEIPT_MISSING` | 1; `classify_root_provision_advance` | One-step terminal replay cursor has no corresponding root receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve terminal history and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_CURRENT_ROOT_MISMATCH` / `FLEET_ROOT_PROVISION_CURRENT_PHASE_INVALID` | 1; `root_provision_call` | Current root-provision call merges selected root identity and required `Accepted` phase | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve batch/response and identify the exact mismatch | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_RECONCILIATION_INTENT_MISSING` | 1; `advance_scale_out_directory_confirmation` | Scale-out reconcile disposition loses its retained Directory intent | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified reconcile identity | Preserve operation state and fail closed | recent failure |
| `FLEET_DIRECTORY_ROOT_INDEX_UNREPRESENTABLE` | 1; `confirmation_root` | Directory root index cannot fit the platform collection index | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve bounded cursor and fail closed | recent failure |
| `FLEET_DIRECTORY_ROOT_INDEX_OUT_OF_BOUNDS` | 1; `confirmation_root` | Directory cursor has no root in the frozen confirmation order | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan/cursor and fail closed | recent failure |
| `FLEET_DIRECTORY_SELECTED_BATCH_MISSING` | 1; `confirmation_root` | Fresh Directory root has no same-ordinal selected batch | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan and fail closed | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_CANONICAL_ROOT_MISMATCH` | 1; `confirmation_root` | Fresh Directory root differs from its selected batch at the same ordinal | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified canonical-root identity | Preserve plan/order and fail closed | recent failure |
| `FLEET_DIRECTORY_PUBLICATION_CALL_MODE_INVALID` | 1; `confirmation_call_publication_request` | Publication request extraction receives synchronization-call authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the typed call variant and fail closed | recent failure |
| `FLEET_DIRECTORY_FRESH_INTENT_MODE_INVALID` | 1; `fresh_confirmation_intent` | Fresh confirmation extraction receives a scale-out intent variant | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the typed intent and fail closed | recent failure |
| `FLEET_DIRECTORY_SCALE_OUT_SYNC_INTENT_MODE_INVALID` | 1; `scale_out_synchronization_intent` | Scale-out synchronization extraction receives another intent variant | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the typed intent and fail closed | recent failure |
| `FLEET_DIRECTORY_SCALE_OUT_PUBLICATION_INTENT_MODE_INVALID` | 1; `scale_out_publication_intent` | Scale-out publication extraction receives another intent variant | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the typed intent and fail closed | recent failure |
| `FLEET_DIRECTORY_FRESH_EVIDENCE_MODE_INVALID` | 1; `fresh_confirmation_response` | Fresh confirmation extraction receives scale-out evidence | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the typed evidence variant and fail closed | recent failure |
| `FLEET_DIRECTORY_SCALE_OUT_EVIDENCE_MODE_INVALID` | 1; `scale_out_confirmation_progress` | Scale-out confirmation extraction receives fresh-publication evidence | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the typed evidence variant and fail closed | recent failure |

The 23 rows sum to all 26 selected calls. Candidate-column extraction produces
24 unique exact labels: five reuse qualified identities and 19 are new. No
safe projection is added.

## Selected Root And Published Directory Evidence

This slice accounts for the first 20 calls at lines 5061–5996. It covers
selected-root lookup, bounded Directory/runtime progress conversion, published
root evidence and exact Component Group placement/member reconstruction.

| Exact candidate | Calls / producer function | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_DIRECTORY_SELECTED_BATCH_MISSING` | 3; `selected_root_batch`, `selected_root_provisioned_response`, `validate_directory_confirmation_response` | Selected Directory publication/confirmation root has no matching planned batch | `COMPONENT_REGISTRY_STATE_INVALID`; all calls reuse the qualified lookup identity | Preserve plan/root and fail closed | recent failure |
| `FLEET_DIRECTORY_ROOT_PROVISIONING_EVIDENCE_MISSING` | 2; `selected_root_provisioned_response`, `root_provisioned_response` | Selected/indexed Directory root has no retained root-provisioning receipt | `COMPONENT_REGISTRY_STATE_INVALID`; both calls share one exact meaning | Preserve provisioning history and fail closed | recent failure |
| `FLEET_DIRECTORY_ROOT_PROVISIONING_ROOT_MISMATCH` | 1; `selected_root_provisioned_response` | Selected Directory root's provisioning receipt names another root | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve root/receipt and fail closed | recent failure |
| `FLEET_DIRECTORY_SCALE_OUT_SYNC_CURSOR_MISMATCH` / `FLEET_DIRECTORY_CONFIRMATION_CANONICAL_ROOT_MISMATCH` | 1; `validate_scale_out_synchronization_response` | Synchronization validator merges durable root cursor and canonical-root identity | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf; canonical-root identity is reused | Preserve cursor/root and identify the failed field | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_COUNT_UNREPRESENTABLE` | 1; `commit_directory_confirmation_progress` | Confirmed-root history cannot fit the durable progress field during commit | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified progress identity | Preserve bounded history and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATED_COUNT_UNREPRESENTABLE` | 1; `commit_runtime_activation_response` | Completed activation history cannot fit the durable progress field during commit | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified progress identity | Preserve bounded history and fail closed | recent failure |
| `FLEET_DIRECTORY_ROOT_INDEX_UNREPRESENTABLE` | 1; `root_provisioned_response` | Directory root cursor cannot fit the platform collection index | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified cursor identity | Preserve bounded cursor and fail closed | recent failure |
| `FLEET_RUNTIME_ACTIVATION_ROOT_PUBLICATION_MISSING` | 1; `root_publication_response` | Activation root has no retained matching Directory publication | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve confirmation/publication history and fail closed | recent failure |
| `FLEET_ROOT_PUBLICATION_COMPLETION_TIME_MISSING` | 1; `validate_runtime_activation_response` | Activation predecessor publication lacks its completion time | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified publication identity | Preserve publication and fail closed | recent failure |
| `FLEET_DIRECTORY_PUBLISHED_RESULT_MISSING` | 2; `validate_directory_confirmation_response`, `validate_root_publication_evidence` | Published Directory validation lacks the provisioned Component result | `COMPONENT_REGISTRY_STATE_INVALID`; both calls share one exact meaning | Preserve response and fail closed | recent failure |
| `FLEET_DIRECTORY_PUBLISHED_PROVISIONING_TIME_MISSING` | 1; `validate_directory_confirmation_response` | Published Directory response lacks root provisioning time | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve response and fail closed | recent failure |
| `FLEET_DIRECTORY_PUBLISHED_PUBLICATION_TIME_MISSING` | 1; `validate_directory_confirmation_response` | Published Directory response lacks publication time | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve response and fail closed | recent failure |
| `FLEET_DIRECTORY_PUBLISHED_COMPONENT_COUNT_UNREPRESENTABLE` | 1; `validate_root_publication_evidence` | Published Component count cannot index the Directory evidence collection | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve bounded response and fail closed | recent failure |
| `FLEET_DIRECTORY_GROUP_PLACEMENT_ID_MISMATCH` / `FLEET_DIRECTORY_GROUP_COMPONENT_MISMATCH` / `FLEET_DIRECTORY_GROUP_MEMBER_COUNT_MISMATCH` | 1; `component_group_directory_from_receipt` | Group reconstruction merges placement identity, Component Group identity and member cardinality | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve plan/result and identify the exact mismatch | recent failure |
| `FLEET_DIRECTORY_GROUP_MEMBER_PATH_MISMATCH` / `FLEET_DIRECTORY_GROUP_MEMBER_SPEC_MISMATCH` / `FLEET_DIRECTORY_GROUP_MEMBER_PURPOSE_MISMATCH` | 1; `component_group_directory_from_receipt` | Member reconstruction merges path, Component Spec and purpose authority | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve plan/result and identify the exact mismatch | recent failure |
| `FLEET_ROOT_PROVISION_ROOTS_ACCEPTED_TIME_MISSING` | 1; `root_provision_previous_observed_at` | Root provisioning progress lacks immutable RootsAccepted time | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified transition identity | Preserve progress and fail closed | recent failure |

The 16 rows sum to all 20 selected calls. Candidate-column extraction produces
21 unique exact labels: seven reuse qualified identities and 14 are new. No
safe projection is added.

## Root Acceptance History And Terminal Barrier

This slice accounts for the first 19 calls at lines 5997–6249. It covers
accepted-root receipt lookup, monotonic stored history, the retained acceptance
intent and both the `RootsAccepted` and later post-acceptance barriers. Nested
response and observation validators are transparent future propagation points.

| Exact candidate or disposition | Calls / producer function | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_ROOT_ACCEPTANCE_RECEIPT_MISSING` | 2; `component_provisioning_root_acceptance`, `replay_recorded_root_acceptance` | Accepted-root cursor has no matching retained receipt | `COMPONENT_REGISTRY_STATE_INVALID`; both lookups share one exact meaning | Preserve acceptance history and fail closed | recent failure |
| `FLEET_COMPONENT_PROVISIONING_PLAN_TIME_INVALID` | 1; `validate_component_provisioning_root_acceptance_state` | Durable provisioning plan time is zero | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve operation and fail closed | recent failure |
| `FLEET_ROOT_ACCEPTANCE_COUNT_EXCEEDS_PLAN` | 1; `validate_component_provisioning_root_acceptance_state` | Retained accepted-root count exceeds the complete root-batch plan | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan/history and fail closed | recent failure |
| `FLEET_ROOT_ACCEPTANCE_INDEX_UNREPRESENTABLE` | 1; `validate_component_provisioning_root_acceptance_state` | Accepted-root history index cannot fit the canonical root ordinal | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; reuses the qualified cursor identity | Preserve bounded history and fail closed | recent failure |
| transparent: exact typed root-acceptance response validation | 1; `validate_component_provisioning_root_acceptance_state` | Stored-history adapter currently erases the exact protected response diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | nested operation status |
| `FLEET_ROOT_ACCEPTANCE_HISTORY_START_TIME_REGRESSED` | 1; `validate_component_provisioning_root_acceptance_state` | Stored root-acceptance invocation starts before the preceding observation | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve ordered history and fail closed | recent failure |
| transparent: exact typed root-acceptance observation validation | 1; `validate_component_provisioning_root_acceptance_state` | Stored-history adapter currently erases the exact response/recording time diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | nested operation status |
| `FLEET_ROOT_ACCEPTANCE_PROGRESS_AND_INTENT_MISSING` | 1; `validate_root_acceptance_phase` | `AcceptingRoots` owns neither a completed acceptance nor a retained pre-call intent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve phase/progress and fail closed | recent failure |
| `FLEET_ROOT_ACCEPTANCE_NONTERMINAL_AFTER_ALL_ROOTS` | 1; `validate_root_acceptance_phase` | Nonterminal acceptance has already exhausted every planned root | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve progress and fail closed | recent failure |
| `FLEET_ROOT_ACCEPTANCE_INTENT_CURSOR_MISMATCH` | 1; `validate_root_acceptance_phase` | Retained intent root index differs from the durable accepted-root count | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve intent/progress and fail closed | recent failure |
| `FLEET_ROOT_ACCEPTANCE_INTENT_ROOT_MISMATCH` | 1; `validate_root_acceptance_phase` | Retained intent names a root different from its planned batch | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve intent/plan and fail closed | recent failure |
| `FLEET_ROOT_ACCEPTANCE_INTENT_TIME_REGRESSED` | 1; `validate_root_acceptance_phase` | Retained intent starts before the preceding acceptance observation | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve ordered intent/history and fail closed | recent failure |
| `FLEET_ROOT_ACCEPTANCE_TERMINAL_COUNT_INCOMPLETE` | 2; `validate_root_acceptance_phase` | `RootsAccepted` or a later phase lacks complete root-acceptance coverage | `COMPONENT_REGISTRY_STATE_INVALID`; both phase groups share one exact meaning | Preserve terminal progress and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_ROOTS_ACCEPTED_TIME_MISSING` | 2; `validate_root_acceptance_phase` | `RootsAccepted` or a later phase lacks immutable completion time | `COMPONENT_REGISTRY_STATE_INVALID`; both calls reuse the qualified transition identity | Preserve terminal progress and fail closed | recent failure |
| `FLEET_ROOT_ACCEPTANCE_COMPLETION_TIME_BEFORE_EVIDENCE` | 2; `validate_root_acceptance_phase` | RootsAccepted completion time predates the last root-acceptance observation | `COMPONENT_REGISTRY_STATE_INVALID`; both phase groups share one exact meaning | Preserve ordered terminal evidence and fail closed | recent failure |

The 15 rows sum to all 19 selected calls. Two calls are transparent nested
validation adapters. Candidate-column extraction produces 13 unique exact
labels for the other 17 calls: two reuse qualified identities and 11 are new.
No safe projection is added.

## Root Provisioning History And Publication Barrier

This slice accounts for the remaining 31 calls at lines 6250–6805. It covers
pre-acceptance exclusion, active/terminal root-provisioning evidence, canonical
service compilation, publication-state ordering, retained root receipts and
pre-call request authority. Nested receipt/current-response validators are
transparent future propagation points.

| Exact candidate or disposition | Calls / producer function | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_ROOT_PROVISION_CURRENT_PHASE_INVALID` | 1; `validate_root_provision_response` | Root-provision response predecessor is not `Accepted` | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified current-phase identity | Preserve predecessor/response and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_PRE_ACCEPTANCE_COUNT_PRESENT` / `FLEET_ROOT_PROVISION_PRE_ACCEPTANCE_CURRENT_RESPONSE_PRESENT` / `FLEET_ROOT_PROVISION_PRE_ACCEPTANCE_INTENT_PRESENT` / `FLEET_ROOT_PROVISION_PRE_ACCEPTANCE_COMPLETION_TIME_PRESENT` | 1; `validate_component_provisioning_root_provision_state` | Planned/accepting state merges premature provision count, current response, intent and RootsAccepted time | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve phase/progress and identify the premature field | recent failure |
| `FLEET_ROOT_PROVISION_ROOTS_ACCEPTED_COUNT_PRESENT` / `FLEET_ROOT_PROVISION_ROOTS_ACCEPTED_CURRENT_PRESENT` / `FLEET_ROOT_PROVISION_ROOTS_ACCEPTED_INTENT_PRESENT` / `FLEET_ROOT_PROVISION_ROOTS_ACCEPTED_RESPONSE_MISSING` | 1; `validate_component_provisioning_root_provision_state` | RootsAccepted state merges nonzero progress/current/intent with missing initial accepted response | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve acceptance/provisioning boundary and identify the failed field | recent failure |
| `FLEET_ROOT_PROVISION_ROOTS_ACCEPTED_TIME_MISSING` | 1; `validate_component_provisioning_root_provision_state` | Provisioning phase lacks immutable RootsAccepted time | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified transition identity | Preserve progress and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_COUNT_EXCEEDS_PLAN` | 1; `validate_component_provisioning_root_provision_state` | Provisioned-root count exceeds complete accepted root batches | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan/history and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_NONTERMINAL_AFTER_ALL_ROOTS` / `FLEET_ROOT_PROVISION_NONTERMINAL_COMPLETION_TIME_PRESENT` | 1; `validate_component_provisioning_root_provision_state` | Nonterminal provisioning has exhausted all roots or already carries terminal completion time | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve progress and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_TERMINAL_COUNT_INCOMPLETE` / `FLEET_ROOT_PROVISION_TERMINAL_CURRENT_RESPONSE_PRESENT` / `FLEET_ROOT_PROVISION_TERMINAL_INTENT_PRESENT` | 1; `validate_terminal_component_provisioning` | ComponentsProvisioned state merges incomplete root coverage with retained current/intent evidence | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve terminal progress and fail closed | recent failure |
| `FLEET_COMPONENTS_PROVISIONED_TIME_MISSING` | 1; `validate_terminal_component_provisioning` | ComponentsProvisioned state lacks completion time | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve terminal progress and fail closed | recent failure |
| `FLEET_COMPONENTS_PROVISIONED_TIME_BEFORE_ROOT_EVIDENCE` | 1; `validate_terminal_component_provisioning` | ComponentsProvisioned completion predates the last root-provision observation | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve ordered terminal history and fail closed | recent failure |
| transparent: exact typed Component-service compilation | 1; `validate_terminal_component_provisioning` | Terminal root-history validator currently erases the exact service-compilation diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | nested operation status |
| `FLEET_SERVICE_PUBLICATION_PREMATURE_REGISTRY_PRESENT` / `FLEET_SERVICE_PUBLICATION_PREMATURE_TIME_PRESENT` | 1; `validate_service_publication_progress` | ComponentsProvisioned state already carries target Registry or publication time | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve pre-publication state and fail closed | recent failure |
| `FLEET_SERVICE_PUBLICATION_COMPLETION_TIME_MISSING` | 1; `validate_service_publication_progress` | ServiceTopologyPublished or later state lacks publication time | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve publication state and fail closed | recent failure |
| `FLEET_SERVICE_PUBLICATION_REGISTRY_MISSING` / `FLEET_SERVICE_PUBLICATION_TIME_BEFORE_COMPONENTS` | 1; `validate_service_publication_progress` | Published state merges missing target Registry with publication before Component completion | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve authority/time and identify the failed field | recent failure |
| `FLEET_ROOT_PROVISION_INDEX_UNREPRESENTABLE` | 1; `validate_root_provision_receipts` | Stored root-provision history index cannot fit the canonical root ordinal | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; reuses the qualified cursor identity | Preserve bounded history and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_HISTORY_START_TIME_REGRESSED` / `FLEET_ROOT_PROVISION_HISTORY_OBSERVATION_BEFORE_START` / `FLEET_ROOT_PROVISION_HISTORY_ACCEPTED_TIME_MISMATCH` | 1; `validate_root_provision_receipts` | Stored receipt merges invocation ordering, observation ordering and immutable accepted time | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve acceptance/provision history and identify the failed field | recent failure |
| `FLEET_ROOT_PROVISION_COMPLETION_TIME_MISSING` | 1; `validate_root_provision_receipts` | Stored Provisioned response lacks completion time | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified response identity | Preserve response and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_COMPLETION_BEFORE_INTENT` / `FLEET_ROOT_PROVISION_OBSERVATION_TIME_REGRESSED` | 1; `validate_root_provision_receipts` | Stored terminal response merges completion before invocation and recording before completion | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf; both reuse qualified response identities | Preserve ordered response evidence and fail closed | recent failure |
| transparent: exact typed compiled root-receipt validation | 1; `validate_root_provision_receipts` | Stored receipt adapter currently erases the exact plan/receipt diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | nested operation status |
| transparent: exact typed current root-response validation | 1; `validate_current_root_provision_record` | Current-response adapter currently erases the exact accepted-plan diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | nested operation status |
| `FLEET_ROOT_PROVISION_INTENT_CURSOR_MISMATCH` / `FLEET_ROOT_PROVISION_START_TIME_REGRESSED` | 1; `validate_root_provision_intent` | Retained intent merges root cursor and start-time ordering | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf; start-time identity is reused | Preserve intent/progress and identify the failed field | recent failure |
| `FLEET_ROOT_PROVISION_CURSOR_TERMINAL` | 1; `validate_root_provision_intent` | Retained intent has no current accepted-root response | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified cursor identity | Preserve intent/progress and fail closed | recent failure |
| `FLEET_ROOT_PROVISION_INTENT_ROOT_MISMATCH` / `FLEET_ROOT_PROVISION_INTENT_OPERATION_MISMATCH` / `FLEET_ROOT_PROVISION_INTENT_PLAN_MISMATCH` / `FLEET_ROOT_PROVISION_INTENT_RESERVED_CURSOR_MISMATCH` / `FLEET_ROOT_PROVISION_INTENT_CLAIMED_CURSOR_MISMATCH` / `FLEET_ROOT_PROVISION_INTENT_INSTALLED_CURSOR_MISMATCH` / `FLEET_ROOT_PROVISION_INTENT_REGISTRY_CURSOR_MISMATCH` | 1; `validate_root_provision_intent` | Retained exact request merges root, operation, plan and four root-local progress cursors | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve intent/expected request and identify the exact changed field | recent failure |
| `FLEET_COMPONENT_PLAN_ROOT_PLACEMENT_COUNT_UNREPRESENTABLE` | 2; `root_batch_counts`, `component_provisioning_plan_counts` | Root-batch placement count cannot fit the durable plan/projection field | `COMPONENT_REGISTRY_STATE_INVALID`; both calculations share one exact meaning | Preserve bounded plan and fail closed | recent failure |
| `FLEET_COMPONENT_PLAN_GROUP_MEMBER_COUNT_UNREPRESENTABLE` | 2; `root_batch_counts`, `component_provisioning_plan_counts` | Component Group placement member count cannot fit the durable plan/projection field | `COMPONENT_REGISTRY_STATE_INVALID`; both calculations share one exact meaning | Preserve bounded plan and fail closed | recent failure |
| `FLEET_COMPONENT_PLAN_ROOT_COMPONENT_COUNT_OVERFLOW` | 1; `root_batch_counts` | Per-root Component total overflows while summing placement members | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan and fail closed | recent failure |
| `FLEET_DIRECTORY_CONFIRMATION_ROOT_COUNT_UNREPRESENTABLE` | 1; `component_provisioning_plan_counts` | Plan confirmation-root count cannot fit the durable summary | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified barrier identity | Preserve bounded plan and fail closed | recent failure |
| `FLEET_ROOT_ACCEPTANCE_BATCH_COUNT_UNREPRESENTABLE` | 1; `component_provisioning_plan_counts` | Plan root-batch count cannot fit the durable summary | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified acceptance identity | Preserve bounded plan and fail closed | recent failure |
| `FLEET_COMPONENT_PLAN_PLACEMENT_COUNT_OVERFLOW` | 1; `component_provisioning_plan_counts` | Fleet-wide Component Group placement total overflows | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan and fail closed | recent failure |
| `FLEET_COMPONENT_PLAN_COMPONENT_COUNT_OVERFLOW` | 1; `component_provisioning_plan_counts` | Fleet-wide Component total overflows | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve plan and fail closed | recent failure |

The 29 rows sum to all 31 selected calls. Three calls are transparent nested
validation adapters. Candidate-column extraction produces 47 unique exact
labels for the other 28 calls: ten reuse qualified identities and 37 are new.
No safe projection is added.

## Canonical Registry And Publication History

This slice accounts for the first 17 calls at lines 6806–7305. It covers
Joining acknowledgement order, activation-history reconstruction, immutable
Component operation source snapshots, service-publication receipt replay and
the root-lifecycle response envelope. Canonical Registry compilation/version
adapters are transparent future propagation points.

| Exact candidate or disposition | Calls / producer function | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_SNAPSHOT_ACK_REGISTRY_STALE` / `FLEET_SNAPSHOT_ACK_ORDER_NONCANONICAL` / `FLEET_SNAPSHOT_ACK_ROOT_NOT_JOINING` | 1; `validate_root_snapshot_acknowledgements` | Stored acknowledgement validation merges Registry version, strict root order and Joining membership | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf; Registry identity is reused | Preserve acknowledgement/Registry and identify the failed field | recent failure |
| `FLEET_REGISTRY_HEAD_HISTORY_MISMATCH` | 1; `validate_registry_lifecycle_history` | Current Fleet Registry differs from the last canonically reconstructed history point | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve head/history receipts and fail closed | recent failure |
| `FLEET_REGISTRY_ACTIVATION_RECEIPT_MISSING_WITH_LIFECYCLE_HISTORY` / `FLEET_REGISTRY_ACTIVATION_RECEIPT_MISSING_WITH_TRANSITIONED_REGISTRY` | 1; `initial_lifecycle_history` | Missing activation receipt is paired with later lifecycle receipts or a non-Joining Registry | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve Registry/receipts and fail closed | recent failure |
| `FLEET_REGISTRY_ACTIVE_ACKNOWLEDGEMENTS_PRESENT` | 1; `initial_lifecycle_history` | Activated Registry retains stale Joining acknowledgements | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve active Registry and fail closed | recent failure |
| transparent: exact typed activation-source version derivation | 1; `initial_lifecycle_history` | History adapter currently erases the exact canonical version diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| transparent: exact typed activation-target Registry compilation | 1; `initial_lifecycle_history` | History adapter currently erases the exact active-Registry diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| transparent: exact typed activation-target version derivation | 1; `initial_lifecycle_history` | History adapter currently erases the exact canonical version diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| `FLEET_REGISTRY_ACTIVATION_REQUEST_SOURCE_MISMATCH` / `FLEET_REGISTRY_ACTIVATION_RESPONSE_SOURCE_MISMATCH` / `FLEET_REGISTRY_ACTIVATION_RESPONSE_TARGET_MISMATCH` | 1; `initial_lifecycle_history` | Activation receipt merges request source, response source and response target versions | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve activation receipt/history and identify the failed field | recent failure |
| `FLEET_COMPONENT_SOURCE_REGISTRY_HISTORY_MISSING` | 1; `registry_snapshot_at_version` | Component operation's frozen source Registry version is absent from canonical history | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve operation/history and fail closed | recent failure |
| `FLEET_SERVICE_PUBLICATION_OPERATION_ID_INVALID` / `FLEET_SERVICE_PUBLICATION_PLAN_HASH_INVALID` / `FLEET_SERVICE_PUBLICATION_CONFIGURATION_INVALID` / `FLEET_SERVICE_PUBLICATION_ROOT_RECEIPTS_EMPTY` / `FLEET_SERVICE_PUBLICATION_ROOT_RECEIPT_LIMIT_EXCEEDED` / `FLEET_SERVICE_PUBLICATION_ROOT_RECEIPT_HASH_INVALID` | 1; `apply_service_publication_receipt` | Publication authority merges nonzero identity/configuration with bounded nonempty root-receipt hashes | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve receipt and identify the exact invalid field | recent failure |
| `FLEET_SERVICE_PUBLICATION_SOURCE_HISTORY_MISMATCH` | 1; `apply_service_publication_receipt` | Publication receipt source version differs from canonical history head | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt/history and fail closed | recent failure |
| transparent: exact typed initial service-publication compilation | 1; `apply_service_publication_receipt` | Initial-history adapter currently erases the exact Registry compilation diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| transparent: exact typed scale-out service-publication compilation | 1; `apply_service_publication_receipt` | Scale-out history adapter currently erases the exact Registry compilation diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| `FLEET_SERVICE_PUBLICATION_TARGET_HISTORY_MISMATCH` | 1; `apply_service_publication_receipt` | Publication receipt target version differs from rederived canonical Registry | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt/history and fail closed | recent failure |
| transparent: exact typed root-lifecycle source version derivation | 1; `apply_lifecycle_receipt` | Lifecycle-history adapter currently erases the exact canonical version diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| `FLEET_ROOT_DRAINING_RESPONSE_AUTHORITY_MISMATCH` / `FLEET_ROOT_DRAINING_RESPONSE_SOURCE_MISMATCH` / `FLEET_ROOT_DRAINING_RESPONSE_TARGET_MISMATCH` / `FLEET_ROOT_REMOVAL_RESPONSE_AUTHORITY_MISMATCH` / `FLEET_ROOT_REMOVAL_RESPONSE_SOURCE_MISMATCH` / `FLEET_ROOT_REMOVAL_RESPONSE_TARGET_MISMATCH` | 1; `apply_lifecycle_receipt` | Lifecycle response equality merges draining/removal authority and source/target Registry versions | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve typed receipt/response and identify the exact changed field | recent failure |
| transparent: exact typed root-lifecycle target version derivation | 1; `apply_lifecycle_receipt` | Lifecycle-history adapter currently erases the exact canonical version diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |

The 17 rows sum to all 17 selected calls. Seven calls are transparent nested
Registry/version adapters. Candidate-column extraction produces 25 unique
exact labels for the other ten calls: one reuses a qualified identity and 24
are new. No safe projection is added.

## Root Lifecycle, Join History And Draining Reservations

This final slice accounts for all 21 calls at lines 7306–7995. It covers
draining/removal replay, canonical lifecycle revision order, permanent join
history and root-draining reservation authority. Nested lifecycle validators,
Registry compilers and grouped-authority gates are transparent future
propagation points.

| Exact candidate or disposition | Calls / producer function | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| transparent: exact typed draining-reservation lookup | 1; `apply_draining_receipt` | Lifecycle replay adapter currently erases the exact reserved identity diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| transparent: exact typed draining-publication request validation | 1; `apply_draining_receipt` | Lifecycle replay adapter currently erases the exact request/history diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| transparent: exact typed Draining Registry compilation | 1; `apply_draining_receipt` | Lifecycle replay adapter currently erases the exact Registry transition diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| transparent: exact typed removal-publication request validation | 1; `apply_removal_receipt` | Lifecycle replay adapter currently erases the exact final-inventory/history diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| transparent: exact typed Removed Registry compilation | 1; `apply_removal_receipt` | Lifecycle replay adapter currently erases the exact Registry transition diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| `FLEET_ROOT_LIFECYCLE_REVISION_DUPLICATE` / `FLEET_ROOT_LIFECYCLE_REVISION_NONMONOTONIC` | 1; `canonical_lifecycle_receipts` | Canonical receipt order contains equal or decreasing Registry revisions | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve receipts and fail closed | recent failure |
| `ROOT_DRAINING_PUBLICATION_IDENTITY_CONFLICT` | 1; `validate_lifecycle_receipt_identities` | Two draining publication receipts reuse a root or operation identity | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified identity | Preserve receipts and reject ambiguous replay | recent failure |
| `ROOT_REMOVAL_PUBLICATION_IDENTITY_CONFLICT` | 1; `validate_lifecycle_receipt_identities` | Two removal publication receipts reuse a root or operation identity | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified identity | Preserve receipts and reject ambiguous replay | recent failure |
| `FLEET_ROOT_JOIN_RECEIPT_MISSING` / `FLEET_ROOT_JOIN_RECEIPT_ORPHANED` | 1; `validate_root_join_receipts` | Registry/root-join cardinality mismatch means a root lacks a receipt or a receipt lacks a root | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf; missing-receipt identity is reused | Preserve Registry/receipts and identify the failed direction | recent failure |
| `FLEET_ROOT_JOIN_RECEIPT_ROOT_MISSING` / `FLEET_ROOT_JOIN_RECEIPT_ROOT_DUPLICATE` / `FLEET_ROOT_JOIN_RECEIPT_AUTHORITY_MISMATCH` | 1; `validate_root_join_receipts` | Join receipt resolves to zero/multiple current roots or a different immutable root authority | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve root/receipt and identify the exact resolution failure | recent failure |
| `FLEET_ROOT_JOIN_HISTORY_INCOMPLETE` | 1; `validate_root_join_receipts` | Canonically replayed join receipts do not reconstruct every current root row | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve join history and fail closed | recent failure |
| transparent: exact typed Fleet Registry genesis compilation | 1; `historical_joining_registry` | Join-history adapter currently erases the exact genesis diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| `FLEET_ROOT_JOIN_RECEIPT_STATUS_CHANGED` | 1; `historical_joining_registry` | Durable join receipt no longer retains its original `Joining` row | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve original receipt and fail closed | recent failure |
| transparent: exact typed Joining Registry compilation | 1; `historical_joining_registry` | Join-history adapter currently erases the exact Registry transition diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| transparent: exact typed join-history version derivation | 1; `historical_joining_registry` | Join-history adapter currently erases the exact canonical version diagnostic | preserve the exact nested public projection | Remove the string adapter and propagate the typed leaf | recent failure |
| `FLEET_ROOT_JOIN_RECEIPT_VERSION_MISMATCH` | 1; `historical_joining_registry` | Join receipt version differs from its canonically replayed snapshot | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt/history and fail closed | recent failure |
| `ROOT_DRAINING_RESERVATION_COUNT_EXCEEDS_ROOTS` | 1; `validate_root_draining_reservations` | Retained reservation count exceeds Fleet Registry root count | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve reservations/Registry and fail closed | recent failure |
| `ROOT_DRAINING_RESERVATION_IDENTITY_CONFLICT` | 1; `validate_root_draining_reservations` | Two reservations reuse a root or operation identity | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified identity | Preserve reservations and reject ambiguous replay | recent failure |
| `ROOT_DRAINING_RESERVATION_TARGET_MISSING` | 1; `validate_root_draining_reservations` | Reservation source Registry lacks the expected root principal | `COMPONENT_REGISTRY_STATE_INVALID`; reuses the qualified target identity | Preserve request/Registry and fail closed | recent failure |
| `ROOT_DRAINING_RESERVATION_OPERATION_ID_INVALID` / `ROOT_DRAINING_RESERVATION_ROOT_INACTIVE` / `ROOT_DRAINING_RESERVATION_ROOT_AUTHORITY_MISMATCH` / `ROOT_DRAINING_RESERVATION_COORDINATOR_MISMATCH` / `ROOT_DRAINING_RESERVATION_TIME_INVALID` / `ROOT_DRAINING_RESERVATION_HASH_MISSING` / `ROOT_DRAINING_RESERVATION_HASH_MISMATCH` | 1; `validate_root_draining_reservations` | Stored reservation merges operation, Active root binding, Coordinator, time and canonical hash authority | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf; operation/root/time identities are reused | Preserve reservation and identify the exact failed field | recent failure |
| transparent: `ROOT_LIFECYCLE_GROUPED_AUTHORITY_FENCED` | 1; `validate_root_draining_reservations` | Reservation validator currently replaces the exact grouped root-lifecycle fence | preserve the exact qualified public projection | Remove the string adapter and propagate the typed leaf | recent failure |

The 21 rows sum to all 21 selected calls. Nine calls are transparent nested
validators/Registry adapters; one of those names the existing grouped-authority
leaf it must preserve. Candidate-column extraction therefore produces 23
unique exact labels: ten reuse qualified identities and 13 are new. No safe
projection is added.

## Required Tests

- remove each retained intent after its classifier selects reconcile;
- lose predecessor, RootsAccepted time or bounded count while preserving the
  rest of the operation record;
- substitute a noncanonical root in Directory/runtime progress;
- begin scale-out publication with missing and nonterminal synchronization
  independently; and
- corrupt stored deployment configuration while retaining Coordinator/Registry
  authority;
- independently corrupt every field in retained Directory/runtime intents and
  terminal evidence, including operation-specific time rules;
- force each transparent nested validator to return a distinct typed leaf and
  prove its adapter preserves that leaf;
- remove, duplicate and reorder root acceptance/provisioning/Directory/runtime
  receipts at their exact cursors;
- corrupt service-publication, activation, join and root-lifecycle history one
  immutable field at a time; and
- corrupt draining-reservation identity, source root, Coordinator, time and
  hash independently while retaining all other authority.

## Next Slice

All 235 parent-file calls are classified. Continue with the 10 root-deletion
and 47 deployment-ledger calls in their owning ledgers. Replace the generic
string funnel with typed exact leaves or transparent propagation during B2; do
not allocate one umbrella diagnostic to `receipt_invariant`.
