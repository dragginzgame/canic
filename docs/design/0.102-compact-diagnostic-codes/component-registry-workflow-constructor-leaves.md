# Canic 0.102 Component Registry Workflow Constructor Leaves

Date: 2026-08-13

## Status

This B1 ledger reconciles the direct constructors in the root-owned child
create/install/commit/activate workflow that correspond to the ops persistence
boundaries in
[component-registry-constructor-leaves.md](component-registry-constructor-leaves.md).
It allocates no number and changes no runtime behavior.

The selected functions contain **354 direct `InternalError::*` references**:
61 in the direct-child journey, 25 in top-level Component commitment and
activation, 51 in top-level draining, quiescence, deletion and recycling, and
26 in subtree-removal orchestration, 27 in subtree planning, physical effects
and protected validation, 15 in root activation/initial-inventory convergence,
20 in the public Directory/cursor/protected-member surface, 51 in Directory
convergence/runtime-status validation, 23 in peer/protected-allocation
validation and 55 in Registry preparation, allocation admission and top-level
create/install closure. They contribute 198 new exact candidates, reuse 58
exact identities, preserve twenty-five transparent typed-cause or adapter-
sediment sites and add no safe projection. The source file's complete 354-site
production frontier is classified.

## Selected Workflow Boundary

The site manifest covers every direct constructor in these exact baseline
functions from
`crates/canic-control-plane/src/workflow/component_registry/mod.rs`:

- entry and response boundaries: `create_child_allocation`,
  `install_child_allocation`, `commit_child_allocation`,
  `prepare_child_directories`, `activate_child_runtime`,
  `activate_child_membership`, `activate_and_validate_child_membership` and
  `validate_child_membership_receipt`;
- pool/retry boundaries: `advance_child_creation`,
  `claim_component_child_pool_asset`,
  `reconcile_component_child_pool_claim`,
  `require_component_child_progress_canister` and
  `reconcile_existing_child_creation`;
- install boundaries: `child_component_install_plan`,
  `advance_child_install`, `perform_child_install`,
  `verify_and_mark_child_installed`, `observed_child_install_state` and
  `verify_installed_child`;
- activation planning: `prepared_child_runtime_plan`,
  `validate_requesting_parent_still_active`, `current_child_partition` and
  `child_directory_request`;
- phase/response extraction: `child_allocation_creation_and_canister`,
  `child_install_effect`, `committed_or_verified_child_installation`,
  `committed_child_installation`, `committed_child_directory_receipt`,
  `child_commit_response` and `child_membership_response`; and
- protected authority: `child_creation_plan`, `exact_store_artifact`,
  `validate_child_install_effect`, `validate_child_allocation` and
  `deployment_spawn_grant_maximum`.

Two selected retry/effect adapters contain zero direct constructors. They are
listed because their transparency is part of the boundary proof; an outer
wrapper does not receive a generic workflow code.

## Exact Site Disposition

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_CHILD_CALLER_NOT_PARENT` | 5 | Caller is not the exact registered parent at entry or post-await revalidation | self; existing exact identity | Invoke from the registered immediate parent; do not retry unchanged | public |
| `COMPONENT_CHILD_OPERATION_UNRESERVED` | 4 | Exact child-allocation operation record is absent | self; existing exact identity | Reserve/query the exact operation first | public |
| `COMPONENT_CHILD_DIRECTORY_PREPARATION_RECEIPT_MISSING` | 1 | Ops accepted Directory preparation but returned no terminal receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the durable operation before retry | recent failure |
| `COMPONENT_CHILD_RUNTIME_DIRECTORY_AUTHORITY_UNREADY` | 1 | Runtime activation has no terminal prepared-Directory receipt | self; existing exact identity | Complete Directory preparation and retry exact authority | public |
| `COMPONENT_CHILD_RUNTIME_ACTIVATION_RECEIPT_MISSING` | 1 | Ops accepted runtime activation but returned no terminal receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the durable operation before retry | recent failure |
| `COMPONENT_CHILD_MEMBERSHIP_ACTIVATION_TRANSITION_INVALID` | 1 | Membership activation lacks the terminal runtime receipt | self; existing exact identity | Complete runtime activation before membership activation | public |
| `COMPONENT_CHILD_PRINCIPAL_INDEX_MISSING` | 1 | Active child has no registered-principal index entry | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile normalized Registry indexes | recent failure |
| `COMPONENT_CHILD_PRINCIPAL_INDEX_INVALID` | 1 | Registered-principal index resolves to the wrong protected binding or lifecycle | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile row/index authority | recent failure |
| `COMPONENT_CHILD_MEMBERSHIP_RECORD_INVALID` | 2 | Activated/response state has no required membership record | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and inspect durable membership state | recent failure |
| `COMPONENT_CHILD_MEMBERSHIP_RECEIPT_MISMATCH` | 2 | Membership receipt differs from derived active partition or Directory authority | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve receipt and reconcile active membership evidence | recent failure |
| `CANISTER_POOL_READY_ASSET_UNAVAILABLE` | 1 | No root-local `Ready` prepaid Canister can satisfy the child claim | self | Let bounded pool maintenance/import replenish an asset; exact retry later | public |
| `COMPONENT_CHILD_POOL_CLAIM_INTENT_INVALID` | 1 | Durable child creation-intent commit returned another phase after the pool claim | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile claim/operation state | recent failure |
| `COMPONENT_CHILD_POOL_CLAIM_PHASE_INVALID` | 1 | Claimed asset reconciliation reached a phase with no retained Canister | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the claim plus operation journal | recent failure |
| `COMPONENT_CHILD_POOL_CLAIM_PRINCIPAL_MISMATCH` | 1 | Durable allocation principal differs from the exact claimed pool asset | self | Preserve both records; never substitute or finalize another principal | public |
| `COMPONENT_CHILD_MODULE_SOURCE_STORE_MISMATCH` | 1 | Resolved module source Canister differs from the verified sibling Store | `COMPONENT_CHILD_AUTHORITY_INVALID` | Fail closed and re-resolve the protected Store source | recent failure |
| `COMPONENT_CHILD_MODULE_SOURCE_ARTIFACT_MISMATCH` | 1 | Resolved module hash/size differs from verified Store catalog evidence | `COMPONENT_CHILD_AUTHORITY_INVALID` | Fail closed and re-resolve exact artifact evidence | recent failure |
| `COMPONENT_CHILD_COMPONENT_BINDING_INVALID` | 2 | Derived install binding or owning partition binding fails protected Component authority | `COMPONENT_CHILD_AUTHORITY_INVALID`; existing exact identity | Repair/reinstall protected topology/Registry state | recent failure |
| `COMPONENT_REGISTRY_PARTITION_MISSING` | 3 | Protected child install/activation/allocation authority has lost its owning partition | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and inspect/reinstall protected Registry state | recent failure |
| `COMPONENT_CHILD_INSTALL_TRANSITION_INVALID` | 1 | Install requested before child creation | self; existing exact identity | Complete exact creation before installation | public |
| `COMPONENT_CHILD_UNJOURNALED_INSTALL_DETECTED` | 1 | Created child already has the intended module without durable install intent | self | Stop; inspect unknown-result/foreign installation before any retry | public |
| `COMPONENT_CHILD_INSTALL_INTENT_RECEIPT_INVALID` | 1 | Begin-install commit returned a phase other than durable install intent | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the operation/cost-guard journals | recent failure |
| `COMPONENT_CHILD_INSTALL_VERIFICATION_RECEIPT_INVALID` | 1 | Mark-verified commit returned a phase other than `Verified` | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the durable operation | recent failure |
| `COMPONENT_CHILD_CONTROLLER_MISMATCH` | 1 | Installed child controllers differ from the sole Fleet Subnet Root authority | self | Restore exact root-only controllers before continuing | public |
| `COMPONENT_CHILD_MODULE_HASH_MISMATCH` | 1 | Observed module differs from frozen install intent | self | Preserve evidence and resolve the contradictory installation | public |
| `COMPONENT_CHILD_MODULE_UNAVAILABLE` | 1 | Independently observed child has no module after installation | self | Re-observe; retry install only through the durable intent journey | public |
| `COMPONENT_CHILD_RETAINED_BINDING_MISMATCH` | 1 | Installed child reports a binding different from root install authority | self | Fail closed and reinstall with exact protected binding | public |
| `COMPONENT_CHILD_PARENT_INACTIVE` | 1 | Exact parent ceased to be Active before child lifecycle work | self; existing exact identity | Restore Active parent membership; retry only after state change | public |
| `COMPONENT_CHILD_PARENT_AUTHORITY_CHANGED` | 1 | Parent binding or lifecycle changed across an await | self | Restart from current registered parent authority with the same operation | public |
| `COMPONENT_CHILD_CURRENT_PARTITION_AUTHORITY_INVALID` | 1 | Current owning partition no longer covers the immutable committed child authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile monotonic partition coverage | recent failure |
| `COMPONENT_CHILD_COMMITMENT_DIRECTORY_HASH_INVALID` | 1 | Committed Directory receipt differs from the derived Registry authority | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve receipt and reconcile Registry/Directory evidence | recent failure |
| `COMPONENT_CHILD_CREATION_TRANSITION_INVALID` | 1 | Install planning cannot extract a created Canister | self; existing exact identity | Complete exact creation first | public |
| `COMPONENT_CHILD_COMMITMENT_TRANSITION_INVALID` | 1 | Registry commitment requested before verified installation | self; existing exact identity | Complete exact verification first | public |
| `COMPONENT_CHILD_DIRECTORY_PREPARATION_TRANSITION_INVALID` | 1 | Directory planning requested before Registry commitment | self; existing exact identity | Commit the verified child first | public |
| `COMPONENT_CHILD_DIRECTORY_AUTHORITY_UNAVAILABLE` | 1 | Allocation has no committed Directory authority to extract | self | Complete exact Registry commitment before Directory work | public |
| `COMPONENT_CHILD_COMMITMENT_RECEIPT_MISSING` | 1 | Ops commitment returned a noncommitted allocation | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the durable operation | recent failure |
| `COMPONENT_CHILD_COMMITMENT_RECEIPT_MISMATCH` | 1 | Child commitment response differs from partition/Directory authority | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve receipt and reconcile exact authority | recent failure |
| `COMPONENT_CHILD_STORE_AUTHORITY_STALE` | 1 | Verified Store root/release evidence differs from reserved child authority | self | Refresh exact Store/bootstrap evidence; retry unchanged operation | public |
| `COMPONENT_CHILD_STORE_ARTIFACT_UNAVAILABLE` | 1 | Verified Store catalog lacks the admitted child role artifact | self | Publish/repair the exact root release set before retry | public |
| `COMPONENT_CHILD_STORE_ARTIFACT_DUPLICATE` | 1 | Verified Store catalog contains duplicate entries for one role | `COMPONENT_CHILD_AUTHORITY_INVALID` | Fail closed and repair/reinstall catalog authority | recent failure |
| `COMPONENT_CHILD_INSTALL_INTENT_MISMATCH` | 1 | Durable install effect differs from verified plan | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve first intent; reject contradictory effect plan | recent failure |
| `COMPONENT_CHILD_PARENT_BINDING_INVALID` | 2 | Registered top-level or child parent binding is invalid | `COMPONENT_CHILD_AUTHORITY_INVALID`; existing exact identity | Re-resolve exact parent membership; do not retry unchanged | recent failure |
| `COMPONENT_CHILD_ALLOCATION_PARENT_MISMATCH` | 1 | Allocation belongs to a different registered parent | `COMPONENT_CHILD_PARENT_UNAUTHORIZED` | Invoke from the owning parent; never transfer reservation authority | public |
| `COMPONENT_CHILD_SPEC_MISSING` | 1 | Registered parent Spec is absent from compiled topology | `COMPONENT_CHILD_AUTHORITY_INVALID`; existing exact identity | Repair/reinstall protected topology | recent failure |
| `COMPONENT_CHILD_ROLE_NOT_ADMITTED` | 1 | Reserved child role is absent from its Component Spec | self; existing exact identity | Use an admitted role or change topology and reinstall | public |
| `COMPONENT_CHILD_SPAWN_GRANT_MISSING` | 1 | Reserved parent/child role pair lacks its exact spawn grant | self; existing exact identity | Use a granted pair or change topology and reinstall | public |
| `COMPONENT_CHILD_ALLOCATION_AUTHORITY_INVALID` | 1 | Durable reservation differs from protected tree/release/limit authority | `COMPONENT_CHILD_AUTHORITY_INVALID` | Fail closed and inspect/reinstall protected allocation state | recent failure |
| `COMPONENT_CHILD_OPERATION_CONFLICT` | 1 | Exact operation ID is bound to a different child request | self; existing exact identity | Replay only the exact original request | public |
| `COMPONENT_CHILD_DEPLOYMENT_LIMITS_INVALID` | 1 | Deployment spawn-grant reduction is zero or exceeds the Spec ceiling | `COMPONENT_CHILD_AUTHORITY_INVALID`; existing exact identity | Repair/reinstall protected deployment limits | recent failure |

The 48 rows sum to all 61 selected constructor sites. Twenty-six exact
identities are new; the other 22 already exist in the policy or ops ledgers.

## Top-Level Commitment and Activation Boundary

This second slice accounts for all 25 direct constructors in:

- `commit_allocation`, `prepare_component_directories`,
  `prepare_grouped_component_directories`,
  `synchronize_grouped_component_directory`,
  `activate_component_runtime_with_plan`,
  `activate_component_membership_with_plan` and
  `synchronize_active_membership`;
- `prepared_component_runtime_plan`,
  `prepared_group_component_runtime_plan`,
  `prepared_component_runtime_plan_with_group_authority` and
  `validated_group_component_runtime_authority`; and
- `allocation_creation_and_canister`,
  `committed_or_verified_installation`, `committed_installation`,
  `committed_directory_receipt`, `commit_response` and
  `membership_response`.

The public wrapper functions and the final synchronization adapter contain no
direct constructor and remain transparent.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_ALLOCATION_OPERATION_UNRESERVED` | 2 | Exact top-level allocation operation is absent during commitment/planning | self; existing exact identity | Reserve/query the exact operation first | public |
| `COMPONENT_DIRECTORY_PREPARATION_RECEIPT_MISSING` | 1 | Ops accepted Directory preparation but returned no terminal receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect durable operation state | recent failure |
| `COMPONENT_GROUP_PUBLICATION_BARRIER_BREACHED` | 2 | Group member became `Active` before or during the Directory publication barrier | self | Stop aggregate activation and reconcile every member before retry | public |
| `COMPONENT_DIRECTORY_REFRESH_ACTIVATION_CHANGED` | 1 | Active grouped Component changed immutable activation authority during refresh | self | Preserve original activation; reject the refresh | public |
| `COMPONENT_DIRECTORY_REFRESH_NOT_RETAINED` | 1 | Independent observation does not retain a Directory covering the refresh | self | Re-observe and retry bounded synchronization | public |
| `COMPONENT_RUNTIME_DIRECTORY_AUTHORITY_UNREADY` | 1 | Runtime activation has no terminal prepared-Directory receipt | self | Complete Directory preparation first | public |
| `COMPONENT_RUNTIME_ACTIVATION_RECEIPT_MISSING` | 1 | Ops accepted runtime activation but returned no terminal receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect durable operation state | recent failure |
| `COMPONENT_MEMBERSHIP_ACTIVATION_TRANSITION_INVALID` | 1 | Membership activation lacks the terminal runtime receipt | self | Complete exact runtime activation first | public |
| `COMPONENT_MEMBERSHIP_PARTITION_INVALID` | 1 | Membership activation did not produce an `Active` partition | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect partition/operation state | recent failure |
| `COMPONENT_MEMBERSHIP_RECORD_INVALID` | 2 | Activated or response state has no required membership record | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect durable membership state | recent failure |
| `COMPONENT_MEMBERSHIP_RECEIPT_MISMATCH` | 2 | Membership receipt differs from active Registry/Directory authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and reconcile active evidence | recent failure |
| `COMPONENT_COMMITMENT_PARTITION_INVALID` | 1 | Prepared-partition receipt reconstructs a non-`Prepared` partition | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect immutable commitment evidence | recent failure |
| `COMPONENT_COMMITMENT_DIRECTORY_HASH_INVALID` | 1 | Committed Directory receipt differs from current Registry authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and reconcile Registry/Directory evidence | recent failure |
| `COMPONENT_GROUP_RUNTIME_ORIGIN_CONFLICT` | 1 | Retained grouped provisioning origin differs from aggregate authority | self | Replay only through the exact owning group deployment | public |
| `COMPONENT_GROUP_RUNTIME_AUTHORITY_CONFLICT` | 1 | Retained deployment or group Directory differs from aggregate authority | self | Reload exact aggregate authority; reject substitution | public |
| `COMPONENT_ALLOCATION_CREATION_TRANSITION_INVALID` | 1 | Install planning cannot extract a created top-level Canister | self; existing exact identity | Complete exact creation first | public |
| `COMPONENT_ALLOCATION_COMMITMENT_TRANSITION_INVALID` | 1 | Registry commitment requested before verified installation | self | Complete exact verification first | public |
| `COMPONENT_DIRECTORY_PREPARATION_TRANSITION_INVALID` | 1 | Directory planning requested before Registry commitment | self | Commit the verified Component first | public |
| `COMPONENT_DIRECTORY_AUTHORITY_UNAVAILABLE` | 1 | Allocation has no committed Directory authority to extract | self | Complete exact Registry commitment first | public |
| `COMPONENT_COMMITMENT_RECEIPT_MISSING` | 1 | Ops commitment returned a noncommitted allocation | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect durable operation state | recent failure |
| `COMPONENT_COMMITMENT_RECEIPT_MISMATCH` | 1 | Commitment receipt differs from returned Registry/Directory authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve receipt and reconcile exact authority | recent failure |

The 21 rows sum to all 25 selected top-level sites. Nineteen exact identities
are new and two reuse the ops ledger.

## Top-Level Draining, Quiescence And Recycling Boundary

This third slice accounts for all 51 direct constructors in:

- `terminal_component_membership_removal_response`,
  `begin_component_draining`, `component_draining_status`,
  `quiesce_component`, `component_quiescence_status`,
  `advance_component_draining`, `finalize_component_inventory`,
  `delete_component`, `remove_component_membership`,
  `component_recycling_canister` and `component_deletion_status`;
- `advance_component_draining_boundary` and
  `prepared_component_draining_boundary`;
- `component_deletion_response` and `component_quiescence_response`;
- `prepared_component_quiescence_plan`,
  `prepared_component_deletion_plan`,
  `validate_component_deletion_binding` and
  `component_deletion_store_module`;
- `observe_or_stop_component`, `observed_component_quiescence_status`,
  `observe_or_recycle_component`, `observed_component_for_deletion` and
  `validate_component_deletion_live_status`; and
- `validate_component_draining`.

The exact baseline ranges are lines 473–534, 1061–1557, 6458–6577,
7075–7233, 7453–7497, 7547–7600 and 8403–8458 of
`crates/canic-control-plane/src/workflow/component_registry/mod.rs`.
Calls into ops, Store bootstrap, Directory convergence, management effects,
pool recycling and the already-classified Store artifact resolver remain
transparent; their inner constructors are not counted again here.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_DELETION_REQUEST_AUTHORITY_CONFLICT` | 2 | Deletion request operation, Component or final-inventory hash differs from durable intent or terminal receipt | self | Replay only the exact retained deletion request | public |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 3 | No committed Component partition exists at an entry/preparation boundary | self; existing exact identity | Commit/recover the exact partition before draining | public |
| `COMPONENT_DRAINING_SPEC_MISSING` | 3 | A committed draining Component's Spec is absent from protected topology | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect/reinstall topology plus Registry authority | recent failure |
| `COMPONENT_REGISTRY_PARTITION_MISSING` | 5 | A partition disappears after draining/quiescence/deletion mutation or is absent beneath durable draining status | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and inspect/reinstall retained Registry state | recent failure |
| `COMPONENT_DRAINING_UNPREPARED` | 5 | The exact Component has no durable draining fence | self | Begin/query the draining operation before later phases | public |
| `COMPONENT_DRAINING_OPERATION_CONFLICT` | 4 | Status, quiescence, membership removal or deletion names a different durable draining operation | self | Replay only with the retained operation ID | public |
| `COMPONENT_QUIESCENCE_REQUEST_AUTHORITY_CONFLICT` | 1 | Quiescence operation or expected Registry differs from the draining fence | self | Reload draining status and retry its exact authority | public |
| `COMPONENT_MEMBERSHIP_REMOVAL_CANISTER_MISSING` | 1 | Terminal membership removal returned no physical Canister to settle into the pool | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile deletion receipt with pool settlement | recent failure |
| `COMPONENT_DELETION_UNPREPARED` | 2 | Workload deletion has not yet produced a durable intent for status or membership removal | self | Prepare and finish physical recycling before membership removal | public |
| `COMPONENT_DELETION_RECYCLING_UNREADY` | 1 | Deletion remains at intent and has no terminal recycling evidence | self | Resume the exact recycling journey; do not remove membership | public |
| `COMPONENT_DELETION_AUTHORITY_MISSING` | 1 | Deletion status has neither its live partition nor terminal membership-removal receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile retained deletion authority | recent failure |
| `COMPONENT_QUIESCENCE_UNPREPARED` | 1 | Quiescence status has no durable stop intent | self | Prepare quiescence before requesting its response | public |
| `COMPONENT_QUIESCENCE_PREPARATION_NOT_RETAINED` | 1 | Quiescence preparation returned but the workflow cannot reload its stop intent | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the durable draining record | recent failure |
| `COMPONENT_DELETION_PREPARATION_NOT_RETAINED` | 1 | Deletion preparation returned but the workflow cannot reload its intent | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the durable deletion record | recent failure |
| `COMPONENT_QUIESCENCE_STOP_AUTHORITY_INVALID` | 1 | Durable stop intent differs from draining operation, Component, Registry, Canister or controller | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve first intent and reconcile protected authority | recent failure |
| `COMPONENT_QUIESCENCE_STORE_AUTHORITY_STALE` | 1 | Verified Store root or release set differs from current Component quiescence authority | self | Refresh exact root Store evidence before retry | public |
| `COMPONENT_QUIESCENCE_MODULE_AUTHORITY_INVALID` | 1 | Stop intent or terminal observation differs from the verified Store artifact | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile immutable module authority | recent failure |
| `COMPONENT_DELETION_CANISTER_BINDING_INVALID` | 1 | Deletion intent names a Canister other than the protected Component binding | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; never substitute the workload Canister | recent failure |
| `COMPONENT_DELETION_CONTROLLER_AUTHORITY_INVALID` | 1 | Deletion intent controller differs from the sole Fleet Subnet Root | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and repair protected controller authority | recent failure |
| `COMPONENT_DELETION_STORE_ROOT_INVALID` | 1 | Verified Store belongs to a different Fleet Subnet Root | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and re-establish exact sibling Store authority | recent failure |
| `COMPONENT_DELETION_RELEASE_SET_STALE` | 1 | Verified Store release set differs from deletion partition authority | self | Refresh exact release-set evidence before retry | public |
| `COMPONENT_DELETION_INTENT_MODULE_INVALID` | 1 | Deletion intent expected module differs from the verified Store artifact | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the intent and reconcile module authority | recent failure |
| `COMPONENT_DELETION_QUIESCENCE_MODULE_INVALID` | 1 | Terminal quiescence module differs from the verified Store artifact | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect stop/deletion evidence | recent failure |
| `COMPONENT_QUIESCENCE_STOP_IN_PROGRESS` | 2 | Management status is `Stopping` before or after the stop call | self | Wait and re-observe; never issue a second stop while stopping | public |
| `COMPONENT_QUIESCENCE_STOP_NOT_OBSERVED` | 1 | A successful stop response is followed by an observed Running state | self | Preserve uncertainty and re-observe before any retry | public |
| `COMPONENT_QUIESCENCE_CONTROLLER_MISMATCH` | 1 | Live Component controllers differ from the sole root stop authority | self | Restore exact controllers before quiescence can advance | public |
| `COMPONENT_QUIESCENCE_MODULE_MISMATCH` | 1 | Live Component module differs from verified Store quiescence authority | self | Reconcile/reinstall exact admitted module before continuing | public |
| `COMPONENT_RECYCLING_CANISTER_ABSENT` | 1 | Physical workload is absent before it can be retained in the prepaid pool | self | Fail closed and investigate absence; do not treat it as recycling | public |
| `COMPONENT_DELETION_CONTROLLER_MISMATCH` | 1 | Live deletion target controllers differ from the sole root authority | self | Restore exact controllers before recycling | public |
| `COMPONENT_DELETION_MODULE_MISMATCH` | 1 | Live deletion target module differs from verified Store authority | self | Reconcile exact admitted module before recycling | public |
| `COMPONENT_DELETION_NOT_STOPPED` | 1 | Live deletion target is no longer Stopped | self | Re-quiesce through the durable stop journey before recycling | public |
| `COMPONENT_DRAINING_REQUEST_AUTHORITY_CONFLICT` / `COMPONENT_DRAINING_RECEIPT_INVALID` | 1 | One current compound branch merges a caller request mismatch with retained Registry/draining contradiction | self for request mismatch; `COMPONENT_REGISTRY_STATE_INVALID` for retained contradiction | Split the predicates: refresh a stale request, but fail closed on corrupt receipt coverage | public request code; recent failure for retained state |
| `COMPONENT_DRAINING_DIRECTORY_HASH_INVALID` | 1 | Reconstructed draining Directory authority differs from its durable hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the receipt and reconcile Directory/Registry evidence | recent failure |

The 33 rows sum to all 51 selected source sites. The compound validator row
must split into two exact meanings, so the slice contributes **32 new exact
candidates** and reuses two existing exact identities. It adds no safe
projection. Across all three workflow slices, 137 sites now qualify 77 new
exact candidates and reuse 26 existing exact identities.

The compound draining validator is not allowed to retain one `Conflict` code:
caller-supplied staleness is safely correctable, while a retained receipt that
no longer covers protected Registry authority is a masked state invariant.
The B4 implementation must evaluate and map those predicates independently.

## Subtree-Removal Orchestration Boundary

This fourth slice accounts for all 26 direct constructors in
`advance_subtree_removal_phase`, `subtree_removal_action`,
`component_draining_advance_removal_response`, `begin_subtree_removal`,
`advance_subtree_removal`, `prepare_subtree_leaf_stop`,
`stop_subtree_leaf`, `prepare_subtree_leaf_delete`, `delete_subtree_leaf`,
`remove_subtree_leaf_membership`, `synchronize_subtree_leaf_directory`,
`finalize_subtree_leaf` and `subtree_removal_status`.

The exact baseline range is lines 1558–2398 of
`crates/canic-control-plane/src/workflow/component_registry/mod.rs`. This
slice counts only orchestration constructors. The planning, management-effect,
live-observation and protected-record validators used by these functions form
the next independent slice.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_SUBTREE_CURSOR_MISSING_AFTER_PHASE` | 1 | A nonterminal subtree phase returns without its durable traversal cursor | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile phase/cursor persistence | recent failure |
| `COMPONENT_SUBTREE_CURSOR_RETAINS_COMPLETED_TARGET` | 1 | Draining cursor still names a target whose subtree operation is complete | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile terminal target advancement | recent failure |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 9 | The target Component partition is absent at a subtree-removal entry boundary | self; existing exact identity | Commit/recover the exact partition before subtree work | public |
| `COMPONENT_SUBTREE_TARGET_SPEC_MISSING` | 9 | The target Component's Spec is absent from protected topology | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect/reinstall topology plus Registry authority | recent failure |
| `COMPONENT_SUBTREE_REMOVAL_UNPREPARED` | 5 | The exact subtree-removal operation has no durable fence | self | Begin/query the exact removal before later phases | public |
| `COMPONENT_SUBTREE_MEMBERSHIP_REMOVAL_UNREADY` | 1 | Leaf finalization is requested before terminal membership removal | self | Finish physical recycling and membership removal before finalizing the cursor | public |

The six rows sum to all 26 selected source sites. They contribute **five new
exact candidates** and reuse `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE`; no
safe projection is added. Across all four workflow slices, 163 sites now
qualify 82 new exact candidates and reuse 27 existing exact identities.

The repeated entry guards intentionally share identities across phase methods.
The two cursor contradictions do not: losing a nonterminal cursor and retaining
a completed target indicate different persistence failures and recovery audits.

## Subtree Stop, Recycling And Protected-Authority Boundary

This fifth slice accounts for all 27 direct constructors in
`prepared_subtree_leaf_stop_plan`, `validate_subtree_directory_request`,
`prepared_subtree_leaf_delete_plan`, `observe_or_stop_subtree_leaf`,
`observe_or_recycle_subtree_leaf`, `observed_subtree_leaf_status`,
`observed_subtree_leaf_for_deletion`, `validate_subtree_leaf_live_status`,
`validate_subtree_removal` and `validate_subtree_removal_target`.

The exact baseline ranges are lines 7234–7452, 7498–7546, 7601–7646 and
8068–8203 of
`crates/canic-control-plane/src/workflow/component_registry/mod.rs`.
The range ends before `validate_child_allocation`; that validator was already
counted in the direct-child workflow slice and must not be double-counted.

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_SUBTREE_STOP_UNPREPARED` | 1 | Selected leaf has no durable stop intent | self | Prepare the exact leaf stop before executing it | public |
| `COMPONENT_SUBTREE_STOP_REQUEST_CONFLICT` | 1 | Stop request differs from durable leaf, parent, traversal or operation authority | self | Replay only the retained stop request | public |
| `COMPONENT_SUBTREE_STOP_CONTROLLER_AUTHORITY_INVALID` | 2 | Stop intent controller differs from protected Fleet Subnet Root authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and repair retained stop/root authority | recent failure |
| `COMPONENT_SUBTREE_STORE_ROOT_INVALID` | 2 | Verified Store belongs to another root during stop or deletion planning | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and re-establish exact sibling Store authority | recent failure |
| `COMPONENT_SUBTREE_STOP_ARTIFACT_INVALID` | 1 | Stop intent or stopped module differs from the verified Store artifact | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve intent/receipt and reconcile exact module authority | recent failure |
| `COMPONENT_SUBTREE_DIRECTORY_REQUEST_CONFLICT` | 1 | Directory synchronization request differs from durable leaf authority | self | Reload the removal status and retry its exact Directory request | public |
| `COMPONENT_SUBTREE_DELETION_UNPREPARED` | 1 | Selected leaf has no durable deletion intent | self | Prepare deletion after terminal stop evidence | public |
| `COMPONENT_SUBTREE_DELETION_REQUEST_CONFLICT` | 1 | Deletion request differs from durable stopped-leaf authority | self | Replay only the retained deletion request | public |
| `COMPONENT_SUBTREE_DELETION_CONTROLLER_AUTHORITY_INVALID` | 1 | Deletion intent controller differs from protected root authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and repair retained deletion/root authority | recent failure |
| `COMPONENT_SUBTREE_DELETION_ARTIFACT_INVALID` | 1 | Deletion intent or stopped receipt differs from the verified Store artifact | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve evidence and reconcile exact module authority | recent failure |
| `COMPONENT_SUBTREE_STOP_IN_PROGRESS` | 2 | Management status is `Stopping` before or after the stop call | self | Wait and re-observe; never stop the same leaf twice while stopping | public |
| `COMPONENT_SUBTREE_STOP_NOT_OBSERVED` | 1 | A successful stop response is followed by an observed Running state | self | Preserve uncertainty and re-observe before retry | public |
| `COMPONENT_RECYCLING_CANISTER_ABSENT` | 1 | Physical leaf is absent before it can be retained in the prepaid pool | self; existing exact identity | Fail closed and investigate absence; never treat it as recycling | public |
| `COMPONENT_SUBTREE_DELETION_NOT_STOPPED` | 1 | Live deletion target is no longer Stopped | self | Resume the durable stop journey before recycling | public |
| `COMPONENT_SUBTREE_DELETION_CONTROLLER_MISMATCH` | 1 | Live leaf controllers differ from sole root authority | self | Restore exact controllers before recycling | public |
| `COMPONENT_SUBTREE_DELETION_MODULE_MISMATCH` | 1 | Live leaf module differs from verified Store authority | self | Reconcile exact admitted module before recycling | public |
| `COMPONENT_SUBTREE_FENCE_PARTITION_MISSING` | 1 | Durable subtree fence has no owning Component partition | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile partition/fence persistence | recent failure |
| `COMPONENT_SUBTREE_FENCE_AUTHORITY_INVALID` | 1 | Fence Component, Registry, operation or Store authority differs from its partition | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the fence and reconcile protected authority | recent failure |
| `COMPONENT_SUBTREE_OPERATION_CONFLICT` | 1 | Existing subtree operation is bound to another exact intent | self | Replay only the original operation payload | public |
| `COMPONENT_SUBTREE_TARGET_STILL_REGISTERED` | 1 | A membership-removed receipt exists while target principal remains registered | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile receipt/index settlement | recent failure |
| `COMPONENT_SUBTREE_TARGET_UNREGISTERED` | 1 | Nonterminal subtree target disappeared from registered membership | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; absence is not terminal recycling evidence | recent failure |
| `COMPONENT_SUBTREE_TARGET_TOP_LEVEL_INVALID` | 1 | Subtree fence targets a top-level Component instead of a descendant | `COMPONENT_REGISTRY_STATE_INVALID` | Reject and reconcile cursor/target authority | recent failure |
| transparent protected child-binding cause | 1 | Typed topology validation rejects the registered target binding | preserve exact typed topology diagnostic | Remove the formatted wrapper and retain the nested typed cause | operation/recent failure owned by the exact cause |
| `COMPONENT_SUBTREE_TARGET_AUTHORITY_INVALID` | 1 | Fence target principal, parent, role, depth, install hash or Component binding differs from registered authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile target/fence evidence | recent failure |

The 24 rows sum to all 27 selected sites. They contribute **22 new exact
candidates**, reuse `COMPONENT_RECYCLING_CANISTER_ABSENT` and classify one
formatted wrapper as a transparent typed-cause carrier. No safe projection is
added. Across all five workflow slices, 190 sites now qualify 104 new exact
candidates and reuse 28 existing exact identities.

Physical absence remains a single exact meaning for top-level and descendant
workloads because both are Canic-managed Canisters that must be retained in the
same prepaid pool. It is never inferred from transport prose. Stop and deletion
authority remain phase-specific because their durable intents and interruption
boundaries differ.

## Root Activation And Initial-Inventory Convergence Boundary

This sixth slice accounts for all 15 direct constructors in
`converge_root_activation_inventory`, `mark_root_runtime_activated`,
`active_component_member_authority`, `verify_initial_component_convergence`
and `prepared_initial_component_runtime_plan`.

The exact baseline range is lines 3394–3615 of
`crates/canic-control-plane/src/workflow/component_registry/mod.rs`.
`seal_root_activation_inventory`, the boolean terminal-receipt query and the
binding-only member projection contain no direct constructor and remain
transparent. Store bootstrap, protected Registry validation, runtime-status
transport and grouped-runtime authority retain their separately classified
typed causes.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_INITIAL_INVENTORY_CHANGED_DURING_VERIFICATION` | 1 | Sealed inventory hash or operation roster changed while each initial Component was re-observed | self | Restart convergence from the current sealed authority; do not commit the stale proof | public |
| `ROOT_INITIAL_INVENTORY_DIRECTORY_RECEIPT_MISSING` | 1 | Ops accepted Directory convergence but returned no terminal root receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the sealed inventory receipt | recent failure |
| `ROOT_INITIAL_INVENTORY_RUNTIME_BEFORE_DIRECTORIES` | 1 | Root runtime activation is requested before terminal initial Directory convergence | self; existing exact identity | Complete and retain Directory convergence first | public |
| `ROOT_INITIAL_INVENTORY_RUNTIME_COMMIT_MISSING` | 1 | Ops accepted root-runtime activation but returned no terminal activation receipt | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the durable initial-inventory receipt | recent failure |
| `COMPONENT_MEMBER_CALLER_UNREGISTERED` | 1 | Transport caller has no Component principal-index identity on this root | self | Invoke from an exact registered Component-tree member | public |
| `COMPONENT_REGISTRY_PARTITION_MISSING` | 1 | Principal index resolves to a Component whose protected partition is absent | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile principal/partition indexes | recent failure |
| `COMPONENT_MEMBER_INDEX_MISSING` | 1 | Principal index resolves to a Component but no registered member row | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile normalized membership indexes | recent failure |
| `COMPONENT_MEMBER_CALLER_INACTIVE` | 1 | Exact registered caller or its owning partition is not `Active` | self | Retry only after exact membership returns to `Active` | public |
| `ROOT_INITIAL_INVENTORY_MEMBERSHIP_UNAVAILABLE` | 1 | Initial Component has no active Registry membership receipt | self; existing exact identity | Finish exact membership activation before convergence | public |
| `ROOT_INITIAL_INVENTORY_TERMINAL_EVIDENCE_UNAVAILABLE` | 1 | Initial Component lacks its terminal current-Directory acknowledgement | self; existing exact identity | Complete exact Directory synchronization | public |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 1 | Initial Component has no current Registry partition at observation time | self; existing exact identity | Restore/reload the exact current partition | public |
| `ROOT_INITIAL_INVENTORY_COMPONENT_INACTIVE` | 1 | Current initial-Component partition is not `Active` | self | Complete membership activation before convergence | public |
| `ROOT_INITIAL_INVENTORY_MEMBERSHIP_DIRECTORY_HASH_INVALID` | 1 | Retained active-membership receipt differs from reconstructed Directory authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the receipt and fail closed | recent failure |
| `ROOT_INITIAL_INVENTORY_RUNTIME_DIRECTORY_UNCONVERGED` | 1 | Live initial Component has not converged on its active current Directory | self | Resume bounded Directory synchronization and re-observe | public |
| `COMPONENT_ALLOCATION_OPERATION_UNRESERVED` | 1 | Initial Component allocation operation is absent | self; existing exact identity | Resolve/query the exact allocation operation first | public |

The 15 rows sum to all 15 selected sites. Nine exact identities are new and
six reuse persistence or earlier workflow identities. No projection is added.
Across all six workflow slices, 205 sites qualify 113 new exact candidates and
reuse 34 existing exact identities.

The active-member authorization failures are not root-activation readiness:
an unregistered or inactive caller must receive its exact authorization code.
Conversely, a sealed initial Component that is not yet Active or Directory-
converged is an operator-retryable activation boundary. A membership hash
contradiction remains a masked protected-state failure.

## Component Directory Paging And Protected-Member Boundary

This seventh slice accounts for all 20 direct constructors in:

- `registry_partition`, `directory_head` and `directory_page` (lines
  3616–3749);
- `component_deployment_limits`, `decode_component_directory_cursor` and
  `encode_component_directory_cursor` (lines 6926–7011); and
- `validate_partition` and `validate_directory_member`, excluding the already
  classified draining validator between them (lines 8367–8399 and
  8459–8485).

The Directory head/view constructors and current-authority adapter contain no
direct error constructor. Typed topology binding failures remain transparent
and must lose their current formatted wrapper without losing their exact
nested diagnostic.

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 3 | Requested Component has no committed Registry partition | self; existing exact identity | Commit/recover the exact partition before reading | public |
| `COMPONENT_DIRECTORY_PAGE_LIMIT_INVALID` | 1 | Requested page limit is zero or exceeds 100 | self | Supply a positive limit at most the protocol bound | public |
| `COMPONENT_DIRECTORY_MEMBER_REQUIRED` | 1 | Transport caller is not registered in the requested Component tree | self | Invoke as an exact registered member of that Component | public |
| `COMPONENT_DIRECTORY_MEMBER_INACTIVE` | 1 | Registered caller is not in a live Directory-readable lifecycle state | self | Retry only after exact membership becomes live | public |
| `COMPONENT_DIRECTORY_HEAD_STALE` | 1 | Requested Directory revision/hash is not the exact current head | self | Reload the current head and restart paging | public |
| `COMPONENT_DIRECTORY_PARENT_FILTER_INVALID` | 1 | Parent filter is not a registered member of the Component | self | Remove it or use an exact registered parent | public |
| `COMPONENT_DIRECTORY_SPEC_MISSING` | 1 | Protected partition Spec is absent while validating a role filter | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile topology/Registry authority | recent failure |
| `COMPONENT_DIRECTORY_ROLE_FILTER_INVALID` | 1 | Requested role filter is absent from the Component Spec | self | Use an admitted child role or omit the filter | public |
| `COMPONENT_DEPLOYMENT_LIMITS_SPEC_MISSING` | 1 | Protected ungrouped deployment references a Spec absent from topology | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile deployment/topology authority | recent failure |
| `COMPONENT_DIRECTORY_CURSOR_SIZE_INVALID` | 1 | Cursor is empty or exceeds the 2 KiB bound | self | Restart without it or supply a bounded cursor | public |
| `COMPONENT_DIRECTORY_CURSOR_MALFORMED` | 1 | Cursor does not decode as the canonical current payload | self | Restart paging from the exact Directory request | public |
| `COMPONENT_DIRECTORY_CURSOR_QUERY_CONFLICT` | 1 | Cursor is bound to another head or filter tuple | self | Never reuse across queries; restart this exact query | public |
| `COMPONENT_DIRECTORY_CURSOR_ENCODING_FAILED` | 1 | Canonical next-cursor payload cannot be encoded | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; emit no fabricated cursor | recent failure |
| `COMPONENT_DIRECTORY_CURSOR_BOUND_EXCEEDED` | 1 | Internally encoded cursor exceeds the protocol bound | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and correct bounded cursor construction | recent failure |
| transparent protected Component-binding cause | 1 | Topology rejects the committed top-level binding | preserve exact nested topology diagnostic | Remove formatted wrapper and retain typed cause | recent failure owner of nested cause |
| `COMPONENT_REGISTRY_PARTITION_AUTHORITY_INVALID` | 1 | Committed partition release/root/lifecycle/principal index is inconsistent | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile protected partition authority | recent failure |
| transparent protected child-binding cause | 1 | Topology rejects the registered Directory caller child binding | preserve exact nested topology diagnostic | Remove formatted wrapper and retain typed cause | recent failure owner of nested cause |
| `COMPONENT_DIRECTORY_MEMBER_AUTHORITY_INVALID` | 1 | Registered caller binding belongs to another protected partition | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile member/principal indexes | recent failure |

The 18 rows sum to all 20 selected sites. Fifteen exact identities enter the
qualified set, including the `COMPONENT_DIRECTORY_MEMBER_REQUIRED` name already
reserved by the dynamic-value ledger; `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE`
is reused and two sites preserve typed topology causes. No projection is added.

The page limit constrains examined rows in ops; this workflow identity only
owns invalid boundary input. A stale or cross-query cursor is a correctable
conflict, whereas failure to construct a bounded canonical cursor is a masked
implementation/state invariant.

## Directory Convergence And Runtime-Status Boundary

This eighth slice accounts for 51 direct constructors in:

- `active_component_direct_children_for_authority` and
  `active_component_direct_children` (lines 5093–5152);
- the four runtime query/activation/Directory call adapters and their two
  convergence orchestrators (lines 5153–5351);
- `converge_subtree_directory_recipients`,
  `converge_active_member_directory`, `active_member_directory_is_converged`
  and `validate_active_member_protected_status` (lines 5352–5594);
- `exact_active_member_directory_receipt` (lines 5612–5644); and
- the shared runtime-status validators from
  `validate_target_directory_status_for_deployment` through
  `active_membership_target_status_for_deployment` (lines 5684–6009).

The runtime adapters currently reconstruct generic public errors around call,
decode and remote-result boundaries. Those twelve sites receive no workflow
identity: B4 must replace the string transport/decode wrappers with exact typed
IC diagnostics and keep the remote result conversion transparent.

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 1 | Direct-child projection authority names a Component whose partition is unavailable | self; existing exact identity | Recover the exact committed partition before synchronization | public |
| `COMPONENT_DIRECT_CHILD_PROJECTION_AUTHORITY_STALE` | 1 | Requested runtime Directory authority is not the current Component head | self | Reload the current head and restart bounded synchronization | public |
| `COMPONENT_DIRECT_CHILD_PROJECTION_BOUND_EXCEEDED` | 1 | Bounded direct-child scan requires a continuation beyond committed descendant accounting | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile row/count indexes | recent failure |
| `COMPONENT_DIRECT_CHILD_PROJECTION_DUPLICATE_PRINCIPAL` | 1 | Direct-child projection contains one principal more than once | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile normalized membership indexes | recent failure |
| transparent runtime-status query adapter | 3 | Call, Candid decode and remote result retain their exact typed diagnostics | replace string call/decode wrappers; keep remote result transparent | Follow the typed call or remote action | owning IC call or remote runtime status |
| transparent runtime-activation adapter | 3 | Activation call, decode and remote result retain their exact typed diagnostics | replace string call/decode wrappers; keep remote result transparent | Recover through the exact activation operation | activation operation and remote runtime status |
| `COMPONENT_RUNTIME_DIRECTORY_AUTHORITY_UNREADY` | 2 | Runtime has not retained the complete prepared Directory authority | self; existing exact identity | Resume exact Directory preparation and re-observe | public |
| `COMPONENT_MEMBERSHIP_DIRECTORY_UNCONVERGED` | 2 | Active runtime has not retained a Directory covering active membership | self | Resume bounded synchronization and independently re-observe | public |
| transparent runtime-Directory synchronization adapter | 3 | Synchronization call, decode and remote result retain exact typed diagnostics | replace string call/decode wrappers; keep remote result transparent | Recover through exact Directory synchronization | synchronization operation and remote runtime status |
| transparent runtime-Directory preparation adapter | 3 | Preparation call, decode and remote result retain exact typed diagnostics | replace string call/decode wrappers; keep remote result transparent | Recover through exact Directory preparation | preparation operation and remote runtime status |
| `COMPONENT_DRAINING_AUTHORITY_MISSING` | 1 | A `Draining` partition has no durable draining authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile partition/draining persistence | recent failure |
| `COMPONENT_DRAINING_QUIESCENCE_INCOMPLETE` | 1 | Draining owner is not terminally quiescent before subtree Directory convergence | self | Complete the exact quiescence journey first | public |
| `COMPONENT_SUBTREE_DIRECTORY_OWNER_LIFECYCLE_INVALID` | 1 | Subtree convergence sees an owning Component outside `Active` or `Draining` | self | Resume only from an admitted lifecycle phase | public |
| `COMPONENT_SUBTREE_PARENT_MISSING` | 1 | Removed subtree leaf has no retained immediate parent membership | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile post-order membership history | recent failure |
| `COMPONENT_SUBTREE_PARENT_INACTIVE` | 1 | Distinct immediate parent is not `Active` when its Directory must converge | self | Restore or finish the parent's exact lifecycle before retry | public |
| `COMPONENT_DIRECTORY_CONVERGENCE_UNAVAILABLE` | 2 | Independently observed active member has not retained the committed Directory | self | Resume the exact bounded synchronization and re-observe | public |
| `COMPONENT_RUNTIME_CURRENT_DIRECTORY_MISSING` | 1 | Active runtime status has no current Directory authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and repair/reinstall protected runtime state | recent failure |
| `COMPONENT_RUNTIME_CURRENT_DIRECTORY_HASH_MISSING` | 1 | Active runtime status has no current Directory hash | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and repair/reinstall protected runtime state | recent failure |
| `COMPONENT_RUNTIME_DIRECT_CHILDREN_HASH_MISSING` | 1 | Active runtime status has no direct-child projection hash | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and repair/reinstall protected runtime state | recent failure |
| `COMPONENT_RUNTIME_STATUS_ACTIVATION_MISSING` | 5 | Active, converged or refresh-target runtime status has lost immutable activation evidence | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed; preserve root receipt and repair/reinstall runtime state | recent failure |
| `COMPONENT_DIRECTORY_COMPONENT_AUTHORITY_MISMATCH` | 1 | Runtime Directory belongs to another Component authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reject cross-Component substitution | recent failure |
| `COMPONENT_DIRECTORY_BINDING_AUTHORITY_MISMATCH` | 1 | Runtime Directory ownership differs from its protected managed binding | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and repair/reinstall exact binding | recent failure |
| `COMPONENT_DIRECTORY_SAME_REVISION_CONFLICT` | 1 | Runtime retained different authority at the committed revision | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve both observations and fail closed | recent failure |
| `COMPONENT_DIRECTORY_FORWARD_PROGRESSION_INVALID` | 1 | Proposed later Directory cannot safely advance current runtime authority | self | Refresh exact current authority; do not overwrite it | public |
| `COMPONENT_DIRECTORY_LATER_AUTHORITY_CONFLICT` | 1 | Runtime reports a later revision reached through conflicting authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the later evidence and fail closed | recent failure |
| `COMPONENT_RUNTIME_PROTECTED_BINDING_MISMATCH` | 1 | Runtime status reports a binding different from its protected Registry binding | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and repair/reinstall exact binding | recent failure |
| `COMPONENT_RUNTIME_PHASE_NOT_ACTIVE` | 1 | Directory convergence target is not in runtime phase `Active` | self | Complete exact activation before synchronization | public |
| `COMPONENT_RUNTIME_ACTIVATION_DIRECTORY_HASH_MISSING` | 1 | Immutable activation evidence contains a zero Directory hash | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and repair/reinstall protected runtime state | recent failure |
| `COMPONENT_RUNTIME_ACTIVATION_TIME_MISSING` | 1 | Immutable activation evidence contains no activation time | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and repair/reinstall protected runtime state | recent failure |
| `COMPONENT_RUNTIME_CURRENT_DIRECTORY_HASH_INVALID` | 1 | Current Directory bytes do not hash to the retained runtime hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the contradiction and fail closed | recent failure |
| `COMPONENT_RUNTIME_DIRECTORY_OPERATION_MISMATCH` | 1 | Runtime Directory status belongs to another installation operation | self | Query/replay only the exact operation | public |
| `COMPONENT_RUNTIME_DIRECTORY_BINDING_MISMATCH` | 1 | Runtime Directory status names another protected binding | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reject binding substitution | recent failure |
| `COMPONENT_RUNTIME_DIRECTORY_DEPLOYMENT_MISMATCH` | 1 | Runtime Directory status names another protected deployment context | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reject deployment substitution | recent failure |
| `COMPONENT_RUNTIME_DIRECTORY_STATUS_INVALID` | 1 | Runtime phase, Directory identity and activation evidence form an inadmissible combination | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the exact status and fail closed | recent failure |
| `COMPONENT_DIRECTORY_REFRESH_TARGET_IDENTITY_CHANGED` | 1 | Active refresh target changed operation, binding, deployment, phase or complete Directory identity | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the original target authority and fail closed | recent failure |
| `COMPONENT_RUNTIME_ACTIVATION_UNCONVERGED` | 1 | Independently observed runtime has not completed exact Directory-bound activation | self | Resume exact activation/re-observation | public |

The 36 rows sum to all 51 selected sites. Thirty exact identities are
new, two reuse earlier exact identities and twelve adapter sites preserve typed
IC or remote causes while their generic string wrappers are deleted. No safe
projection is added. Across all eight workflow slices, 276 sites now qualify
158 new exact candidates, reuse 37 exact identities and preserve fifteen
transparent typed-cause or adapter-sediment sites.

The current and immutable activation receipts remain separate authorities. A
later valid Directory may cover a required head, but same-revision conflict or
nonmonotonic later authority fails closed. Only the affected top-level owner
and distinct immediate parent converge; the slice introduces no descendant
fanout or unbounded Registry scan.

## Peer Revalidation And Protected-Allocation Boundary

This ninth slice accounts for 23 direct constructors in:

- `validate_allocation_caller`, `require_active_peer_allocation_caller`,
  `revalidate_peer_provisioning_origin`, `revalidate_retained_peer_origin`,
  `revalidate_same_root_peer_origin`,
  `revalidate_fleet_service_peer_origin` and
  `validate_retained_peer_grant` (lines 7647–7842);
- top-level `validate_creation_effect` and `validate_install_effect` (lines
  7843–7868), excluding the already-classified child-install validator; and
- `validate_allocation_record` plus `validate_provisioning_origin` (lines
  7914–8029).

Typed topology/Fleet-service/group-origin validators remain transparent. The
current compound active-requester predicate must split into exact binding/index
and lifecycle decisions; it cannot keep one opaque forbidden string.

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_ALLOCATION_CALLER_MISMATCH` | 1 | Lifecycle caller differs from the administrator retained in the allocation origin | self | Invoke as the exact reserving administrator | public |
| `COMPONENT_GROUP_AGGREGATE_LIFECYCLE_REQUIRED` | 1 | A grouped member is advanced outside its aggregate provisioning workflow | self | Resume through the exact owning aggregate operation | public |
| `COMPONENT_ALLOCATION_OPERATION_UNRESERVED` | 1 | Peer lifecycle operation has no retained top-level allocation | self; existing exact identity | Reserve/query the exact operation first | public |
| `PEER_COMPONENT_REQUESTER_PROOF_CONFLICT` | 1 | Lifecycle request proof differs from the requester proof retained by the allocation | self | Replay only the original same-root or Fleet-service proof | public |
| `PEER_COMPONENT_ORIGIN_REQUIRED` | 1 | Allocation was not created through a peer provisioning origin | self | Use its owning administrator/group lifecycle instead | public |
| transparent protected peer-binding cause | 1 | Topology rejects the retained same-root requester binding | preserve exact typed topology diagnostic | Remove the formatted forbidden wrapper and retain the typed cause | nested policy diagnostic owner |
| `PEER_COMPONENT_REQUESTER_INACTIVE` | 1 | Retained same-root requester is no longer registered | self; existing exact identity | Restore exact Active requester membership before retry | public |
| `PEER_COMPONENT_REQUESTER_BINDING_INVALID` / `PEER_COMPONENT_REQUESTER_INACTIVE` | 1 | Compound check merges caller, principal index, retained/current binding and lifecycle evidence | first projects to `PEER_COMPONENT_REQUESTER_UNAUTHORIZED`; second remains self; existing exact identities | Split predicates; repair authority mismatch or restore Active membership as indicated | recent failure for binding contradiction; public for inactive state |
| `PEER_COMPONENT_REGISTRY_PROOF_STALE` | 1 | Current Fleet Registry no longer covers the retained cross-root requester proof | self | Refresh/re-reserve against exact current Registry authority | public |
| `PEER_COMPONENT_REQUESTER_BINDING_INVALID` | 1 | Resolved Fleet-service requester differs from its retained identity | `PEER_COMPONENT_REQUESTER_UNAUTHORIZED`; existing exact identity | Reauthenticate/re-reserve exact requester authority | recent failure |
| `PEER_COMPONENT_RETAINED_GRANT_INVALID` | 2 | Retained same-root peer grant differs from protected topology | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve allocation evidence and fail closed | recent failure |
| `COMPONENT_CREATION_INTENT_MISMATCH` | 1 | Durable creation effect differs from verified Store/root creation authority | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve first intent and reject contradictory effect evidence | recent failure |
| `COMPONENT_INSTALL_INTENT_MISMATCH` | 1 | Durable top-level install effect differs from verified module/binding authority | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Preserve first intent and reject substitution | recent failure |
| `COMPONENT_ALLOCATION_RECORD_OPERATION_INVALID` | 1 | Stored operation ID is zero or differs from the requested operation | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Fail closed and inspect protected allocation persistence | recent failure |
| `COMPONENT_ALLOCATION_RECORD_IDENTITY_INVALID` | 1 | Stored Component ID or sequence differs from exact root-local derivation | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Fail closed; never derive or substitute another identity | recent failure |
| `COMPONENT_ALLOCATION_RELEASE_SET_INVALID` | 1 | Stored allocation release set differs from protected root authority | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Fail closed and reinstall/reconcile root authority | recent failure |
| `COMPONENT_ALLOCATION_ADMISSION_MISSING` | 1 | Stored allocation Spec is absent from protected root admissions | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Fail closed and reinstall canonical topology/admissions | recent failure |
| `COMPONENT_ALLOCATION_ADMITTED_SPEC_MISSING` | 1 | Admitted stored Spec is absent from compiled topology | `COMPONENT_ALLOCATION_AUTHORITY_INVALID`; existing exact identity | Fail closed and reinstall canonical topology | recent failure |
| `COMPONENT_ALLOCATION_SPEC_HASH_MISMATCH` | 2 | Stored Spec hash differs from protected admission or compiled Spec | `COMPONENT_ALLOCATION_AUTHORITY_INVALID`; existing exact identity | Fail closed and reinstall one canonical topology | recent failure |
| `COMPONENT_ALLOCATION_ROLE_MISMATCH` | 1 | Stored top-level role differs from its protected Component Spec | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Fail closed and reinstall exact Spec authority | recent failure |
| transparent protected provisioning-origin binding cause | 1 | Topology rejects the stored peer requester binding | preserve exact typed topology diagnostic | Remove formatted invariant wrapper and retain typed cause | nested policy diagnostic owner |

The 21 rows sum to all 23 selected sites and contain 20 exact-label
occurrences. Twelve exact identities are new, six existing identities are
reused and two sites preserve typed topology causes. No safe projection is
added. Across all nine workflow slices, 299 sites now qualify 170 new exact
candidates, reuse 43 exact identities and preserve seventeen transparent
typed-cause or adapter-sediment sites.

Peer origin is causal evidence, not parentage. Revalidation proves the exact
requester, current Registry coverage and retained grant without changing the
target Component's direct-root placement. Caller/controller status alone never
substitutes for Registry and topology authority.

## Registry Preparation, Allocation And Top-Level Create/Install Closure

This tenth slice accounts for the final 55 direct constructors in the file:

- Registry status, preparation/current-Mirror validation and response helpers
  (lines 548–647 and 6179–6289);
- administrator, peer and direct-child reservation/status entry paths (lines
  582–1054);
- ordinary/grouped top-level creation, installation and Registry-commit entry
  paths (lines 2439–2580);
- top-level pool claim/reconciliation, install planning, installation,
  verification and managed-binding observation (lines 3750–4796); and
- the top-level creation-plan Store authority check (lines 7012–7033).

Typed allocation/peer/child policy conversions receive no workflow code.
Managed-binding call/decode/result adapters follow the same typed IC boundary
as runtime Directory calls. The free-form root-runtime unavailable message is
hard-cut to the one existing `FLEET_SUBNET_ROOT_RUNTIME_INACTIVE` meaning.

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `COMPONENT_REGISTRY_AUTHORITY_UNPREPARED` | 2 | Root Component Registry meta authority is absent | self; existing exact identity | Complete exact Registry preparation before retry | public |
| `COMPONENT_REGISTRY_STATUS_AUTHORITY_CONFLICT` | 1 | Status request differs from the durable Registry preparation authority | self | Reload the retained preparation inputs and retry exact status | public |
| `COMPONENT_REGISTRY_PREPARATION_AUTHORITY_INVALID` | 2 | Durable Registry root/release authority differs from protected root authority | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile/reinstall protected state | recent failure |
| `COMPONENT_REGISTRY_PREPARATION_REGISTRY_MISMATCH` / `COMPONENT_REGISTRY_PREPARATION_ROOT_INACTIVE` | 1 | First preparation combines wrong expected Registry with a root not yet `Active` | self | Split predicates; refresh exact Registry or wait for Active root as indicated | public |
| `COMPONENT_REGISTRY_PREPARATION_NOT_COVERED` | 1 | Current mirror does not cover the Registry authority used for preparation | self | Refresh/re-prepare from exact current mirror authority | public |
| `FLEET_SUBNET_ROOT_RUNTIME_INACTIVE` | 1 | A root-local lifecycle operation requires an `Active` root runtime | self; existing exact identity | Wait for exact root activation; retry only after state change | public |
| `COMPONENT_ALLOCATION_RECORD_IDENTITY_INVALID` | 1 | Response allocation retains a zero sequence | `COMPONENT_ALLOCATION_AUTHORITY_INVALID`; existing exact identity | Fail closed and inspect protected allocation persistence | recent failure |
| `COMPONENT_ALLOCATION_OPERATION_CONFLICT` | 2 | Administrator or peer retry changes retained Spec/origin intent | self; existing exact identity | Replay only the exact original reservation | public |
| transparent top-level allocation-policy cause | 1 | Typed top-level reservation policy rejects the request | preserve exact typed policy diagnostic | Follow its exact admission/capacity action | policy diagnostic owner |
| `PEER_COMPONENT_REGISTRY_PROOF_STALE` | 1 | Cross-root peer request does not name the target root's exact current Registry | self; existing exact identity | Refresh requester proof and re-reserve | public |
| `ACCESS_ACTIVE_COMPONENT_REQUIRED` | 1 | Same-root peer caller is not a registered Component | self; existing exact identity | Invoke as an exact Active registered Component | public |
| `COMPONENT_REGISTRY_PARTITION_MISSING` | 1 | Principal index names a peer requester whose partition is absent | `COMPONENT_REGISTRY_STATE_INVALID`; existing exact identity | Fail closed and reconcile normalized indexes | recent failure |
| `PEER_COMPONENT_TOP_LEVEL_REQUESTER_REQUIRED` | 1 | Registered caller is a Component Child rather than a top-level Component | self; adopts the DPC candidate for this exact denial | Invoke from an admitted top-level Component | public |
| transparent peer-allocation policy cause | 1 | Typed peer readiness/grant/capacity policy rejects reservation | preserve exact typed policy diagnostic | Follow the exact peer policy action | policy diagnostic owner |
| transparent top-level allocation-decision cause | 1 | Typed top-level allocation policy rejects peer reservation | preserve exact typed policy diagnostic | Follow the exact admission/capacity action | policy diagnostic owner |
| `COMPONENT_ALLOCATION_OPERATION_UNRESERVED` | 3 | Status, create or install has no retained top-level allocation | self; existing exact identity | Reserve/query the exact operation first | public |
| `COMPONENT_CHILD_CALLER_NOT_PARENT` | 2 | Direct-child reserve/status caller is not its registered immediate parent | self; existing exact identity | Invoke from the exact registered parent | public |
| `COMPONENT_REGISTRY_PARTITION_UNAVAILABLE` | 1 | Direct-child reservation has no committed owning partition | self; existing exact identity | Recover the exact partition before reservation | public |
| `COMPONENT_DESCENDANT_COUNT_OVERFLOW` | 1 | Checked reserved-plus-committed descendant count overflows | `COMPONENT_DESCENDANT_CAPACITY_EXHAUSTED`; existing exact identity | Stop allocation and reconcile descendant accounting | recent failure |
| transparent child-allocation policy cause | 1 | Typed child authority/grant/capacity policy rejects reservation | preserve exact typed policy diagnostic | Follow the exact child policy action | policy diagnostic owner |
| `COMPONENT_CHILD_OPERATION_UNRESERVED` | 1 | Child status has no retained allocation operation | self; existing exact identity | Reserve/query the exact child operation first | public |
| `COMPONENT_GROUP_INSTALL_RECEIPT_MISSING` | 1 | Grouped installation completed but its allocation disappeared before reload | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect operation persistence | recent failure |
| `COMPONENT_GROUP_REGISTRY_ROOT_INACTIVE` / `COMPONENT_GROUP_REGISTRY_AUTHORITY_CHANGED` | 1 | One branch merges a root no longer `Active` with changed Fleet Directory authority before grouped commitment | self | Split predicates; wait for Active root or reload aggregate Directory authority as indicated | public |
| `CANISTER_POOL_READY_ASSET_UNAVAILABLE` | 1 | No root-local `Ready` prepaid Canister can satisfy top-level creation | self; existing exact identity | Let bounded pool maintenance/import replenish an asset | public |
| `COMPONENT_POOL_CLAIM_INTENT_INVALID` | 1 | Durable creation-intent commit returned another allocation phase | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and reconcile claim/operation state | recent failure |
| `COMPONENT_POOL_CLAIM_PHASE_INVALID` | 1 | Claimed asset reconciliation reached a phase with no retained Canister | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect claim plus allocation journal | recent failure |
| `COMPONENT_POOL_CLAIM_PRINCIPAL_MISMATCH` | 1 | Durable allocation principal differs from exact claimed pool asset | self | Preserve both records; never substitute a principal | public |
| `COMPONENT_STORE_AUTHORITY_STALE` | 1 | Verified Store root/release evidence differs from reserved Component authority | self | Refresh exact Store/bootstrap evidence and retry unchanged operation | public |
| `COMPONENT_MODULE_SOURCE_STORE_MISMATCH` | 1 | Resolved module source differs from the verified sibling Store | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Fail closed and re-resolve exact Store authority | recent failure |
| `COMPONENT_MODULE_SOURCE_ARTIFACT_MISMATCH` | 1 | Resolved module hash/size differs from verified Store catalog evidence | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Fail closed and re-resolve exact artifact evidence | recent failure |
| transparent protected Component-binding cause | 1 | Topology rejects the derived top-level install binding | preserve exact typed topology diagnostic | Remove formatted wrapper and retain typed cause | nested policy diagnostic owner |
| `COMPONENT_INSTALL_SPEC_MISSING` | 1 | Installed Component Spec is absent from protected topology | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Fail closed and reinstall canonical topology | recent failure |
| `COMPONENT_REMOVED_ALLOCATION_RESPONSE_INVALID` | 1 | Removed-response reconstruction lacks removed allocation authority | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect terminal allocation history | recent failure |
| `COMPONENT_ALLOCATION_INSTALL_TRANSITION_INVALID` | 1 | Installation begins before the allocation is created | self; existing exact identity | Resume only through the phase's admitted next edge | public |
| `COMPONENT_UNJOURNALED_INSTALL_DETECTED` | 1 | Created Component already has intended code without durable install intent | self | Stop and inspect unknown/foreign installation before retry | public |
| `COMPONENT_INSTALL_VERIFICATION_PHASE_INVALID` | 1 | Internal verification helper is reached from neither `Verified` nor `Committed` | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the durable operation | recent failure |
| `COMPONENT_INSTALL_VERIFICATION_RECEIPT_INVALID` | 1 | Verification commit returned a phase other than `Verified` | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect the durable operation | recent failure |
| `COMPONENT_CONTROLLER_MISMATCH` | 1 | Installed Component controllers differ from sole Fleet Subnet Root authority | self | Restore exact root-only controllers | public |
| `COMPONENT_MODULE_HASH_MISMATCH` | 1 | Observed module differs from frozen install intent | self | Preserve evidence and resolve contradictory installation | public |
| `COMPONENT_MODULE_UNAVAILABLE` | 1 | Independently observed Component has no module after installation | self | Re-observe; retry only through durable install intent | public |
| `COMPONENT_RETAINED_BINDING_MISMATCH` | 1 | Installed Component reports a binding different from root install authority | self | Fail closed and reinstall with exact protected binding | public |
| `COMPONENT_INSTALL_PREPARED_FENCE_INVALID` | 1 | Installed runtime did not remain empty behind `AwaitingDirectory` | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Preserve observation and fail closed before Registry commitment | recent failure |
| `COMPONENT_INSTALL_OPERATION_MISMATCH` | 1 | Runtime status belongs to another installation operation | self | Query/replay only the exact operation | public |
| `COMPONENT_INSTALL_RUNTIME_BINDING_MISMATCH` | 1 | Runtime status retains another protected binding | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Fail closed and reject binding substitution | recent failure |
| `COMPONENT_INSTALL_DEPLOYMENT_MISMATCH` | 1 | Runtime status retains another protected deployment context | `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | Fail closed and reject deployment substitution | recent failure |
| transparent managed-binding query adapter | 3 | Call, Candid decode and remote result retain their exact typed diagnostics | replace string call/decode wrappers; keep remote result transparent | Recover through exact binding observation | owning IC call or remote runtime status |
| `COMPONENT_INSTALL_INTENT_RECEIPT_INVALID` | 1 | Begin/renew-install commit returned a phase other than durable install intent | `COMPONENT_REGISTRY_STATE_INVALID` | Fail closed and inspect operation/cost-guard journals | recent failure |

The 47 rows sum to all 55 selected sites and contain 43 exact-label
occurrences. Twenty-eight exact identities are new, fifteen existing identities
are reused, one of the new identities adopts the matching DPC name and eight
sites preserve typed policy, topology, IC or remote causes. No safe projection
is added. Across all ten workflow slices, all 354 sites qualify 198 new exact
candidates, reuse 58 exact identities and preserve twenty-five transparent
typed-cause or adapter-sediment sites.

The ordinary top-level and child paths now share only genuinely identical
admission and retry meanings. Pool claims, Store authority, installation
effects, runtime observation and Registry commitment remain independently
diagnosable interruption boundaries. Creation still claims an already durable
pool asset; this evidence adds no raw autonomous creation path.

## Transparent Effect and Recovery Boundaries

- `perform_child_install` preserves the typed
  `ModuleInstallWorkflow::install_with_payload_with_permit` failure through the
  existing cost-guard recovery path. It receives no generic “child install
  failed” identity.
- `reconcile_component_child_pool_claim` and
  `reconcile_existing_child_creation` preserve the exact pool, Registry and
  cost-guard causes. A successful response is never treated as the authority
  for commitment.
- The pool path claims only an already durable `Ready` principal. This slice
  introduces no autonomous raw `create_canister` recovery claim.
- Management status/stop failures and pool-recycling failures remain typed
  effect causes. The draining workflow does not overwrite them with one
  aggregate quiescence or deletion code.
- Root activation re-observation preserves runtime transport and grouped-
  authority causes. Only inventory-wide change and missing post-commit receipt
  receive workflow-owned identities.
- Directory paging keeps page limit, stale head, filter, cursor shape and
  cross-query binding as distinct caller actions. Protected partition/member
  contradictions project to Registry state and never return partial pages.

## Required Tests

- a constructor-site manifest proving all 61 selected sites remain accounted
  after source movement;
- entry authorization and exact parent revalidation after every await;
- no-ready-asset, claim-intent loss, claim-phase contradiction and principal
  mismatch as separate cases;
- unknown installed module after lost/absent intent versus ordinary exact
  retry;
- Store principal, role, module hash/size and duplicate-catalog substitution;
- controller, module, retained binding and active partition substitution;
- each ops terminal receipt independently missing or contradictory; and
- typed management-install and cost-guard failures surviving without an
  aggregate workflow code;
- grouped Components remaining non-Active until the complete publication
  barrier and retaining immutable activation authority during refresh;
- each top-level Directory/runtime/membership receipt independently absent or
  contradictory; and
- exact terminal replay for ordinary, grouped and peer Component wrappers.
- a constructor-site manifest proving all 51 selected draining/deletion sites
  remain accounted after source movement;
- draining request mismatch versus corrupt retained receipt coverage;
- stop-before/status-after interruption, `Stopping` without a second stop and
  Running after a successful response;
- controller, module, Store root, release-set and deletion-binding
  substitution; and
- physical absence failing closed before pool recycling and membership removal.
- a constructor-site manifest proving all 26 subtree orchestration and 27
  subtree physical/validation sites remain accounted after source movement;
- response loss around stop and recycle, including `Stopping`, Running after a
  successful response and typed non-absence transport failure;
- stop/deletion controller, Store root, artifact and registered-target
  substitution; and
- transparent preservation of the exact typed target-binding rejection.
- a constructor-site manifest proving all 15 root-activation and initial-
  inventory convergence sites remain accounted after source movement;
- inventory change during re-observation, post-commit receipt loss and exact
  terminal replay; and
- unregistered/inactive callers versus retryably incomplete initial members
  and contradictory membership Directory hashes.
- a 20-site Directory/cursor/member manifest;
- zero, oversized, malformed, stale and cross-query cursor/page adversarial
  cases plus a sparse page with a continuation; and
- foreign Component member, invalid parent/role filter, protected binding and
  principal-index substitution without partial output.
- a 51-site Directory convergence/runtime-status manifest, including twelve
  adapter sites that preserve typed IC/remote causes without workflow codes;
- missing current authority/hash/direct-child hash/activation evidence and
  wrong operation/binding/deployment/phase as independent adversarial cases;
- same-revision conflict, unsafe forward progression and conflicting later
  authority without overwriting protected runtime evidence; and
- owner plus distinct immediate-parent convergence only, including Draining
  quiescence and retained-parent contradictions.
- a 23-site peer/protected-allocation manifest;
- same-root and Fleet-service requester substitution, de-registration,
  inactivity, stale Registry proof and changed grant as independent cases;
- explicit splitting of the compound caller/index/binding/lifecycle predicate;
  and
- zero operation, derived-identity, release-set, admission, Spec hash, role and
  provisioning-origin substitution against protected allocation state.
- a complete 354-site workflow manifest, with this final slice accounting for
  55 sites and eight transparent typed-cause/adapter sites;
- Registry preparation wrong-version versus inactive-root splitting and
  current-Mirror noncoverage;
- administrator, peer and child reservation retry substitution plus policy-
  cause preservation;
- pool claim intent/phase/principal, Store source/artifact and protected install
  binding substitution; and
- lost responses around install intent, effect, verification and grouped
  Registry commitment with exact replay and independent live observation.

## Next Workflow Slice

The Component Registry workflow file is closed. Return to the 375 open sites
in Component Registry ops, then proceed to Component provisioning and Fleet
Coordinator owners followed by the root-local pool and remaining bootstrap/
Store workflows. Reuse exact meanings and preserve typed effect causes.
