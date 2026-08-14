# Canic 0.102 Component Provisioning Workflow Direct-Constructor Leaves

Date: 2026-08-13

## Status

This B1 ledger reconciles the 56 production `InternalError::*` references in
`crates/canic-control-plane/src/workflow/component_provisioning.rs`. It assigns
no number and changes no runtime behavior. The separate ops ledger owns the 177
persistence constructors; workflow rows describe authentication, observation,
external-effect and orchestration failures only.

## Acceptance, Publication And Runtime Activation

This first slice accounts for all 18 direct constructor references at lines
65–531. It covers Coordinator acceptance, Fleet Registry observation,
Component/Component Group Directory publication and both fresh-root and
already-active-root activation paths.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `GROUP_ACCEPTANCE_ROOT_INACTIVE` / `GROUP_ACCEPTANCE_REGISTRY_MISMATCH` | 1 | Acceptance merges a non-Active local root with a request that names a different Fleet Registry version | self | Wait for Active root or refresh the exact Registry independently | public |
| `GROUP_ACCEPTANCE_AUTHORITY_CHANGED` | 1 | Root Component Registry or runtime mode changed across Store observation | self | Restart acceptance from a fresh protected observation | public |
| `GROUP_PUBLICATION_REGISTRY_UNAVAILABLE` | 1 | Root has no prepared Component Registry authority for publication | self | Finish root Registry preparation first | public |
| `GROUP_PUBLICATION_MIRROR_ROOT_MISMATCH` / `GROUP_PUBLICATION_MIRROR_REGISTRY_MISMATCH` | 1 | Advanced Fleet Mirror names another root or Registry version | self | Preserve mirror/request evidence and refresh exact local authority | public |
| `GROUP_ACTIVATION_ALLOCATION_MISSING` | 1 | Selected activation member has no retained allocation | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Reconcile aggregate and allocation persistence | recent failure |
| `GROUP_ACTIVATION_DEPLOYMENT_KIND_INVALID` | 1 | Activation selected an ordinary/non-group protected deployment | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fix the internal derivation path; never reinterpret deployment kind | recent failure |
| `GROUP_ACTIVATION_CREDENTIAL_MISSING` | 1 | Prepared fresh root has no activation credential | self | Re-run exact preparation/status recovery | public |
| `GROUP_ACTIVATION_ROOT_TIME_MISSING` | 2 | Active root status has no activation time | self; both fresh and scale-out paths share one exact meaning | Re-observe exact Fleet activation status | public |
| `GROUP_ACTIVATION_ROOT_INACTIVE` / `GROUP_ACTIVATION_INITIAL_DIRECTORIES_UNCONVERGED` / `GROUP_ACTIVATION_INITIAL_ROOT_RUNTIME_INACTIVE` / `GROUP_ACTIVATION_INITIAL_COMPONENT_COUNT_MISMATCH` | 1 | Fresh-root terminal check merges Fleet phase, initial Directory convergence, root runtime activation and frozen Component count | self | Continue the exact incomplete boundary; never infer one proof from another | public |
| `GROUP_ACTIVATION_SCALE_OUT_ROOT_INACTIVE` / `GROUP_ACTIVATION_SCALE_OUT_ROOT_TIME_MISSING` / `GROUP_ACTIVATION_SCALE_OUT_ROOT_TIME_AFTER_ACCEPTANCE` / `GROUP_ACTIVATION_SCALE_OUT_DIRECTORIES_UNCONVERGED` / `GROUP_ACTIVATION_SCALE_OUT_ROOT_RUNTIME_INACTIVE` | 1 | Scale-out terminal check merges pre-existing Active phase, qualified prior activation time and initial Directory/runtime evidence | self | Re-observe the pre-existing root authority or reject the batch | public |
| `GROUP_PUBLICATION_PARTITION_MISSING` | 1 | Selected published Component has no Registry partition | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve aggregate member and Registry evidence and fail closed | recent failure |
| `GROUP_PUBLICATION_ALLOCATION_MISSING` | 1 | Selected published Component has no retained allocation authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve aggregate member and Registry evidence and fail closed | recent failure |
| `GROUP_PUBLICATION_DEPLOYMENT_MISMATCH` / `GROUP_PUBLICATION_COMPONENT_GROUP_MISMATCH` | 1 | Retained runtime authority differs from aggregate deployment or Component Group Directory | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Preserve both authorities and identify the failed field | recent failure |
| `GROUP_PUBLICATION_DELIVERY_INTENT_MISSING` | 1 | External Directory call has no durable pre-call intent | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Persist or recover the exact intent before invoking | recent failure |
| `GROUP_PUBLICATION_INTENT_CURSOR_CONFLICT` / `GROUP_PUBLICATION_INTENT_CANISTER_CONFLICT` / `GROUP_PUBLICATION_INTENT_DIRECTORY_HASH_CONFLICT` | 1 | Retained intent differs from derived member index, Canister or Directory authority hash | `COMPONENT_REGISTRY_STATE_INVALID`; first two exact identities already exist | Preserve intent and derived authority; never retarget the effect | recent failure |
| `GROUP_PUBLICATION_DIRECTORY_RECEIPT_UNPREPARED` / `GROUP_PUBLICATION_DIRECTORY_RECEIPT_HASH_MISMATCH` | 1 | Post-call root receipt merges missing preparation with wrong Directory authority hash | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Recover exact target status and commit only matching evidence | recent failure |
| `GROUP_PUBLICATION_PARTITION_STATUS_INVALID` / `GROUP_PUBLICATION_PARTITION_BINDING_MISMATCH` / `GROUP_PUBLICATION_PARTITION_REVISION_MISMATCH` / `GROUP_PUBLICATION_PARTITION_HASH_MISMATCH` | 1 | Publication partition validation merges lifecycle, protected binding, revision and content hash | `COMPONENT_REGISTRY_STATE_INVALID` for every leaf | Preserve partition/member evidence and fail closed | recent failure |

The 17 rows sum to all 18 selected source sites. Compound predicates expand
each independently retryable observation or independently corrupt authority
field. Candidate-column extraction finds 33 unique exact labels. Five reuse
preceding ops identities and the other 28 are new. No safe projection is added.

## Member Claim, Install And Registry Commitment

This second slice accounts for all nine direct constructor references at lines
533–818. It covers reservation recovery across Store observation, install
revalidation and Registry commitment across both Store and Fleet Directory
observation. The current free-form `phase` interpolation in the shared missing-
allocation helper expands to the two exact phases that call it.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `GROUP_CLAIM_RESERVATION_MISSING` | 1 | Claim starts without the reserved Component identity | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile aggregate cursor and allocation persistence | recent failure |
| `GROUP_CLAIM_STORE_AUTHORITY_CHANGED` | 1 | Store bootstrap authority changes across prepaid-Canister claim observation | self | Restart the claim from fresh Registry/Store status | public |
| `GROUP_CLAIM_RESERVATION_LOST` | 1 | Reserved Component identity disappears while Store status is awaited | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve before/after evidence and fail closed | recent failure |
| `GROUP_INSTALL_STORE_AUTHORITY_CHANGED` | 1 | Store bootstrap authority changes across grouped installation observation | self | Restart installation from fresh Registry/Store status | public |
| `GROUP_INSTALL_MEMBER_CHANGED` | 1 | Canonical next install member changes while Store status is awaited | self | Reload aggregate status; never install the stale member | public |
| `GROUP_REGISTRY_COMMIT_STORE_AUTHORITY_CHANGED` | 1 | Store bootstrap authority changes across grouped Registry commitment | self | Restart commitment from fresh Registry/Store status | public |
| `GROUP_REGISTRY_COMMIT_DIRECTORY_AUTHORITY_CHANGED` | 1 | Fleet Directory authority changes across grouped Registry commitment | self | Restart with the current exact Fleet Directory | public |
| `GROUP_REGISTRY_COMMIT_MEMBER_CHANGED` | 1 | Canonical next Registry member changes while Store/Directory status is awaited | self | Reload aggregate status; never commit the stale member | public |
| `GROUP_INSTALL_RESERVATION_MISSING` / `GROUP_REGISTRY_COMMIT_RESERVATION_MISSING` | 1 | Shared dynamic helper merges missing reserved allocation at install and Registry-commit boundaries | `COMPONENT_REGISTRY_STATE_INVALID` for either leaf | Reconcile the exact phase cursor and allocation persistence | recent failure |

The nine rows sum to all nine selected source sites and introduce ten unique
exact labels. No safe projection is added.

## Authority, Capacity And Artifact Gates

This final slice accounts for all 29 direct constructor references at lines
819–1247. It covers Coordinator authorization, acceptance/progress Registry
observation, Registry/aggregate accounting, runtime mode, root/Spec/group/pool
capacity and exact Store artifact closure.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `GROUP_PROVISIONING_COORDINATOR_REQUIRED` | 1 | Caller is not the protected Fleet Coordinator | self | Invoke as the exact Coordinator; do not retry unchanged | public |
| `GROUP_ACCEPTANCE_ROOT_INACTIVE` / `GROUP_ACCEPTANCE_REGISTRY_MISMATCH` | 1 | Acceptance Registry observation merges a non-Active root with wrong Fleet Registry version | self; both exact identities already exist | Wait for Active root or refresh the exact Registry independently | public |
| `GROUP_PROVISIONING_REGISTRY_UNAVAILABLE` | 2 | Acceptance or member progress has no prepared root Component Registry authority | self; both sites share one action | Finish root Registry preparation first | public |
| `GROUP_PROGRESS_ROOT_INACTIVE` / `GROUP_PROGRESS_REGISTRY_MISMATCH` | 2 | Registry and Fleet Directory progress observations each merge a non-Active root with wrong Fleet Registry version | self; both sites reuse the same two exact meanings | Wait for Active root or reload exact Registry independently | public |
| `GROUP_RESERVATION_COUNT_OVERFLOW` | 1 | Current-member replay adjustment overflows reservation accounting | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact projection | Stop and inspect aggregate/Registry counts | recent failure |
| `GROUP_RESERVATION_REGISTRY_PROGRESS_MISMATCH` | 1 | Registry reservation count differs from aggregate reservation cursor plus exact replay | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both counters and fail closed | recent failure |
| `GROUP_CLAIM_REGISTRY_PROGRESS_MISMATCH` | 1 | Claim begins before Registry reservation count equals the frozen batch | `COMPONENT_REGISTRY_STATE_INVALID` | Reconcile Registry/aggregate reservation persistence | recent failure |
| `GROUP_REGISTRY_COMMIT_COUNT_OVERFLOW` | 1 | Exact-replay adjustment overflows aggregate Registry commitment count | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact projection | Stop and inspect aggregate commitment accounting | recent failure |
| `GROUP_REGISTRY_COMMIT_COUNT_EXCEEDS_BATCH` | 1 | Reconciled Registry commitment count exceeds the accepted Component count | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both counters and fail closed | recent failure |
| `GROUP_REGISTRY_COMMIT_PROGRESS_MISMATCH` | 1 | Registry reservations differ from remaining aggregate commitments | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve Registry/aggregate counters and fail closed | recent failure |
| `GROUP_DEPLOYMENT_CONTEXT_KIND_INVALID` | 1 | Group workflow derives an ordinary protected deployment context | `COMPONENT_REGISTRY_STATE_INVALID` | Fix the derivation path; never reinterpret deployment kind | recent failure |
| `GROUP_PROVISIONING_PREPARED_INVENTORY_PRESENT` / `GROUP_PROVISIONING_ACTIVE_INVENTORY_MISSING` / `GROUP_PROVISIONING_INITIAL_OPERATION_MISMATCH` / `GROUP_PROVISIONING_INITIAL_DIRECTORIES_UNCONVERGED` / `GROUP_PROVISIONING_INITIAL_RUNTIME_INACTIVE` | 1 | Runtime-mode derivation collapses impossible Prepared/Active inventory combinations and three invalid Active inventory fields | self for incomplete runtime state; retained contradictions project to `COMPONENT_REGISTRY_STATE_INVALID` | Identify the exact runtime/inventory predicate before retry | public or recent failure by leaf |
| `GROUP_PROVISIONING_ROOT_BINDING_MISMATCH` / `GROUP_PROVISIONING_RELEASE_SET_MISMATCH` / `GROUP_PROVISIONING_ROOT_DRAINING_FENCED` / `GROUP_PROVISIONING_REGISTRY_PREPARATION_MISMATCH` | 1 | Registry authority validation merges root binding, release set, draining fence and Fleet Registry preparation coverage | self for current authority/fence; root-draining exact identity already exists | Reload exact authority or finish draining; never select one record as truth | public |
| `GROUP_PROVISIONING_RUNTIME_MODE_CHANGED` | 1 | Current root runtime mode differs from the mode frozen at acceptance | self | Restart/reject the batch unless the explicit fresh-root response replay applies | public |
| `GROUP_ACTIVATION_REGISTRY_UNAVAILABLE` | 1 | Activation has no prepared root Component Registry authority | self | Finish Registry preparation first | public |
| `GROUP_ACTIVATION_PUBLICATION_MISSING` | 1 | Runtime activation begins without retained terminal publication evidence | self | Complete exact Directory publication first | public |
| `GROUP_ACTIVATION_RUNTIME_MODE_CHANGED` | 1 | Runtime mode changes before activation completes outside the one fresh-root response replay | self | Reload status and recover only the admitted replay | public |
| `GROUP_CAPACITY_NONTERMINAL_ALLOCATION` | 1 | Root already has a nonterminal top-level Component allocation | self | Finish/recover that allocation before accepting a group batch | public |
| `GROUP_CAPACITY_ROOT_COUNT_OVERFLOW` | 1 | Existing plus requested root Component count overflows | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact projection | Stop and inspect root accounting | recent failure |
| `GROUP_CAPACITY_ROOT_LIMIT_EXCEEDED` | 1 | Batch would exceed the protected root Component-instance ceiling | self | Reduce/split the batch or select another admitted root | public |
| `GROUP_CAPACITY_SPEC_ADMISSION_MISSING` | 1 | Planned Component Spec has no root admission | self | Correct the plan/topology; do not retry unchanged | public |
| `GROUP_CAPACITY_SPEC_COUNT_OVERFLOW` | 1 | Existing plus requested per-Spec count overflows | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact projection | Stop and inspect Spec accounting | recent failure |
| `GROUP_CAPACITY_SPEC_LIMIT_EXCEEDED` | 1 | Batch would exceed the protected root admission for one Spec | self | Reduce/split the batch or select another admitted root | public |
| `GROUP_CAPACITY_PLACEMENT_COUNT_OVERFLOW` | 1 | Existing plus requested Component Group placement count overflows | `COMPONENT_PROVISIONING_COUNT_OVERFLOW`; existing exact projection | Stop and inspect placement accounting | recent failure |
| `GROUP_CAPACITY_PLACEMENT_LIMIT_EXCEEDED` | 1 | Batch would exceed the protected root group-placement ceiling | self | Reduce/split the batch or select another admitted root | public |
| `GROUP_CAPACITY_READY_POOL_INSUFFICIENT` | 1 | Root lacks enough Ready prepaid Canisters for the atomic batch | self | Refill the local pool or reduce the batch | public |
| `GROUP_STORE_ARTIFACT_MISSING` / `GROUP_STORE_ARTIFACT_DUPLICATE` | 1 | Store Catalog count check merges missing and duplicate artifact for a planned role | self | Publish the exact one-artifact-per-role release set | public |

The 27 rows sum to all 29 selected source sites. Compound runtime/Registry
predicates remain split rather than inheriting the current shared conflict
message. This section has 37 unique candidate labels: two acceptance labels
repeat from the first workflow slice, `GROUP_PROVISIONING_ROOT_DRAINING_FENCED`
reuses the ops ledger and the other 34 are new. No safe projection is added.

## Mechanical Workflow Closure

The immutable source partition is exact:

| Inclusive source lines | Constructors | Owner section |
| --- | ---: | --- |
| 65–531 | 18 | Acceptance, publication and runtime activation |
| 533–818 | 9 | Member claim, install and Registry commitment |
| 819–1247 | 29 | Authority, capacity and artifact gates |
| **Total** | **56** | **Complete workflow production frontier** |

A mechanical constructor scan and the three table sums independently produce
56. The ranges exclude imports/module prose and the inline test module, do not
overlap and cover every production `InternalError::*` reference in the file.
Candidate-column extraction across all three sections finds 80 occurrences and
78 unique exact labels. Eight occurrences intentionally reuse six preceding ops
identities plus the two repeated acceptance observations; the other 72 unique
labels are new. No safe projection is added.

## Required Tests

- acceptance with inactive root, wrong Registry and authority change across
  Store observation tested independently;
- publication with wrong Mirror root/version, absent partition/allocation,
  retained deployment/group mismatch and each pre-call intent field;
- response loss after Directory preparation and independently missing/wrong
  terminal receipt evidence;
- fresh-root activation with missing credential/time and every initial
  inventory predicate independently incomplete; and
- active-root scale-out with each pre-existing runtime/time predicate
  independently invalid.
- claim recovery with missing/lost reservation and changed Store authority;
- install recovery with changed Store/member and phase-qualified missing
  reservation; and
- Registry commitment with changed Store, Fleet Directory or member plus its
  phase-qualified missing reservation.
- exact Coordinator authorization and acceptance/progress Registry observations
  with root status and Registry version varied independently;
- aggregate/Registry reservation and commitment reconciliation, including both
  checked-arithmetic edges;
- every runtime-mode and protected Registry-authority predicate independently;
- root, per-Spec, group-placement and Ready-pool capacity edges; and
- missing versus duplicate Store artifact for each planned role.

## Next Slice

Move to Fleet Coordinator ops/workflows without assigning numbers.
