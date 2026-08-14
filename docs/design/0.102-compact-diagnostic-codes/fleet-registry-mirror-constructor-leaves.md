# Canic 0.102 Fleet Registry Mirror Constructor Leaves

Date: 2026-08-14

## Status

This B1 evidence ledger classifies all 32 production `InternalError`
constructor sites owned by the root-local Fleet Registry Mirror operations and
workflow. It assigns no number and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `ops/fleet_registry_mirror/mod.rs` | 5 |
| `workflow/fleet_registry_mirror/mod.rs` | 27 |
| **Total** | **32** |

The workflow's inline test tail is excluded. Typed Fleet Registry validation,
topology compilation, root Store status and Component Registry draining errors
remain owned by their existing ledgers rather than receiving Mirror wrappers.

## Active Mirror Storage Authority

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_MIRROR_ROOT_AUTHORITY_MISMATCH` | 1 | Protected root authority names a Canister other than the receiver | self | Reinstall or invoke the exact protected Fleet Subnet Root | authenticated root authority |
| `FLEET_MIRROR_ACTIVE_MISSING` | 1 | No active root-local Fleet Registry Mirror is retained | self | Complete or recover initial synchronization and activation | root-local Mirror status |
| `FLEET_MIRROR_STORED_MANIFEST_MISMATCH` / `FLEET_MIRROR_STORED_VERSION_MISMATCH` / `FLEET_MIRROR_STORED_DIRECTORY_MISMATCH` / `FLEET_MIRROR_PREVIOUS_AUTHORITY_MISMATCH` / `FLEET_MIRROR_REVISION_NOT_ADVANCED` / `FLEET_MIRROR_PREVIOUS_HASH_INVALID` / `FLEET_MIRROR_CURRENT_HASH_INVALID` | 1 | One storage predicate merges three canonical snapshot projections with authority, monotonic-revision and two nonzero-hash requirements | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Preserve the retained Mirror and reconcile or reinstall its exact authority | recent failure plus guarded Mirror status |
| `FLEET_MIRROR_ROOT_ROW_MISSING` | 1 | Active Mirror Registry omits the protected local root | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the Mirror and fail closed | recent failure plus guarded Mirror status |
| `FLEET_MIRROR_ROOT_PLACEMENT_MISMATCH` / `FLEET_MIRROR_ROOT_ADMISSIONS_MISMATCH` / `FLEET_MIRROR_ROOT_TOPOLOGY_MISMATCH` / `FLEET_MIRROR_ROOT_RELEASE_SET_MISMATCH` / `FLEET_MIRROR_ROOT_LIMITS_MISMATCH` / `FLEET_MIRROR_ROOT_STATUS_NOT_CURRENT` | 1 | One protected-row predicate merges five immutable root fields with the `Active`-or-`Draining` lifecycle fence | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Preserve protected authority and Mirror row; identify the exact contradiction | recent failure plus guarded Mirror status |

The five sites add 16 exact meanings. Canonical Registry validation remains
typed and transparent before these root-local evidence comparisons. B4 must
replace both aggregate predicates with named field predicates or typed
authority validation; a single `Mirror invalid` code would conceal the
specific corrupt authority.

## Joining Synchronization And Acknowledgement

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_MIRROR_SYNC_EXPECTED_REGISTRY_MISMATCH` | 2 | Fetched or stored Joining snapshot differs from the host-frozen Registry version | self | Reload the exact install plan and current snapshot; retry only matching authority | host plan and root sync status |
| `FLEET_MIRROR_ACTIVE_ALREADY_PRESENT` | 1 | Initial Joining synchronization is attempted after active Mirror commitment | self | Use active status or the admitted monotonic advance path | root-local Mirror status |
| `FLEET_MIRROR_CANDIDATE_CONFLICT` | 1 | Retained Joining candidate differs from the freshly fetched snapshot | self | Preserve both observations and reconcile; never overwrite the candidate | root-local Mirror status |
| `FLEET_MIRROR_ACK_ROOT_MISMATCH` / `FLEET_MIRROR_ACK_REGISTRY_MISMATCH` | 1 | Coordinator acknowledgement names another root or Registry version | self for each exact leaf | Reject the response and preserve the staged candidate | synchronization response and candidate status |
| `FLEET_MIRROR_CANDIDATE_MISSING` | 1 | Synchronization status has no staged Joining snapshot | self | Start or recover exact initial synchronization | root-local Mirror status |
| `FLEET_MIRROR_CANDIDATE_ACK_MISSING` | 2 | Staged candidate lacks its Coordinator acknowledgement at status or activation | self | Recover the exact acknowledgement call; do not infer success from candidate presence | root-local Mirror status |
| `FLEET_MIRROR_JOINING_CANDIDATE_MISSING` | 1 | Initial activation has neither an active Mirror nor its acknowledged Joining candidate | self | Complete exact synchronization before activation | root-local Mirror status |
| `FLEET_MIRROR_JOINING_SOURCE_MISMATCH` | 1 | Joining candidate differs from the requested previous Registry authority | self | Reload the retained candidate and rebuild the exact activation request | activation request and Mirror status |
| `FLEET_MIRROR_STORED_ACK_ROOT_MISMATCH` / `FLEET_MIRROR_STORED_ACK_REGISTRY_MISMATCH` | 2 | Durable acknowledgement differs from its candidate root or Registry version | `COMPONENT_REGISTRY_STATE_INVALID` for either exact leaf | Preserve candidate and acknowledgement and fail closed | recent failure plus guarded Mirror status |

These 12 sites add 11 exact meanings. Two status/activation sites share the
same missing-acknowledgement prerequisite, and both durable-response checks
reuse the same two stored acknowledgement identities.

## Active Transition And Publication Commit

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_MIRROR_TARGET_NOT_REACHED` | 1 | Active status remains at the admitted transition source rather than the requested target | self | Resume or recover the exact Mirror advance | active Mirror status |
| `FLEET_MIRROR_UNCHANGED_PUBLICATION_MISMATCH` | 1 | A no-change Component publication names a Registry other than the active Mirror | self | Reload current publication and Mirror authority | publication operation and Mirror status |
| `FLEET_MIRROR_DIRECTORY_SOURCE_MISMATCH` | 1 | Prepared Component Directory synchronization starts from a different active Registry | self | Rebuild the synchronization transition from the active Mirror | prepared publication operation |
| `FLEET_MIRROR_PREPARED_COMMIT_SOURCE_CHANGED` | 1 | Active Mirror changed after transition preparation and before its no-await commit | self | Discard the prepared value and prepare again from current authority | prepared transition plus active Mirror status |
| `FLEET_MIRROR_TARGET_PREVIOUS_REGISTRY_MISMATCH` / `FLEET_MIRROR_TARGET_DIRECTORY_MISMATCH` | 1 | Current Registry already equals the requested target but its predecessor or Directory binds another transition | self for each exact leaf | Preserve current authority and reject the conflicting retry payload | activation request and active Mirror status |
| `FLEET_MIRROR_CURRENT_AUTHORITY_MISMATCH` | 1 | Current Mirror and requested target belong to different Fleet Registry authorities | self | Use the exact Fleet/root authority; never cross an authority boundary | activation request and active Mirror status |
| `FLEET_MIRROR_TRANSITION_SOURCE_MISMATCH` | 1 | Current Mirror is neither the exact source, exact target nor an admitted later revision | self | Reload current authority and rebuild the monotonic transition | activation request and active Mirror status |

The seven sites add eight exact meanings. A later valid Mirror may satisfy a
stale request only through the existing explicit revision classifier; these
conflicts must not become a generic exact-retry success.

## Snapshot, Target And Transition Validation

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_MIRROR_SNAPSHOT_ROOT_STATUS_MISMATCH` | 1 | Snapshot root lifecycle differs from the required Joining/target state | self | Fetch the Registry state admitted by the current lifecycle phase | supplied snapshot and Registry status |
| `FLEET_MIRROR_SNAPSHOT_ROOT_MISSING` | 1 | Coordinator snapshot omits the protected local root | self | Reject the snapshot and reconcile Coordinator Registry state | supplied snapshot |
| `FLEET_MIRROR_SNAPSHOT_MANIFEST_MISMATCH` / `FLEET_MIRROR_SNAPSHOT_VERSION_MISMATCH` / `FLEET_MIRROR_SNAPSHOT_ROOT_PLACEMENT_MISMATCH` / `FLEET_MIRROR_SNAPSHOT_ROOT_ADMISSIONS_MISMATCH` / `FLEET_MIRROR_SNAPSHOT_ROOT_TOPOLOGY_MISMATCH` / `FLEET_MIRROR_SNAPSHOT_ROOT_RELEASE_SET_MISMATCH` / `FLEET_MIRROR_SNAPSHOT_ROOT_LIMITS_MISMATCH` | 1 | One supplied-snapshot predicate merges canonical manifest/version with five immutable protected root fields | self for every exact leaf | Reject the untrusted snapshot and identify the exact changed authority field | supplied snapshot plus protected authority |
| `FLEET_MIRROR_TARGET_REGISTRY_MISMATCH` | 1 | Coordinator target snapshot differs from the controller-expected Registry version | self | Reconcile controller and Coordinator observations before commit | activation request and fetched snapshot |
| `FLEET_MIRROR_TARGET_DIRECTORY_MISMATCH` | 1 | Canonically derived root Directory differs from controller-expected authority | self | Rebuild the request from the canonical target snapshot | activation request and derived Directory |
| `FLEET_MIRROR_PREVIOUS_AUTHORITY_REQUEST_MISMATCH` / `FLEET_MIRROR_TARGET_AUTHORITY_REQUEST_MISMATCH` / `FLEET_MIRROR_REQUEST_REVISION_NOT_ADVANCED` / `FLEET_MIRROR_REQUEST_PREVIOUS_HASH_INVALID` / `FLEET_MIRROR_REQUEST_TARGET_HASH_INVALID` | 1 | One request predicate merges source/target Fleet authority, monotonic revision and two nonzero Registry hashes | self for every exact leaf | Correct the independently named transition field before any fetch or commit | activation request |

The six sites contain 16 exact meanings. `FLEET_MIRROR_TARGET_DIRECTORY_MISMATCH`
reuses the same exact target-authority meaning already selected when an active
target is bound to another expected Directory, so this section adds 15 new
identities. Snapshot validation continues to call the typed Fleet Registry
compiler first; its exact validation causes are not renumbered here.

## Transparent Coordinator Calls

| Disposition | Sites | Current meaning | Required hard cut |
| --- | ---: | --- | --- |
| transparent: Coordinator snapshot public diagnostic | 1 | Successfully decoded Coordinator result already carries the exact public Registry diagnostic | Propagate the registered remote diagnostic unchanged |
| transparent: Coordinator acknowledgement public diagnostic | 1 | Successfully decoded Coordinator result already carries the exact acknowledgement diagnostic | Propagate the registered remote diagnostic unchanged |

Transport, request-encoding and response-decoding failures are already typed
by `CallOps` before these two result adapters. Neither adapter receives a
Mirror wrapper code.

## Dynamic Public Context

All messages in this owner group are static. Registry versions, hashes,
principals, root fields and Directory contents remain in their authenticated
request, response, protected authority or guarded Mirror status. This slice
therefore adds no dynamic-public-context row and no new retrieval owner.

## Reconciliation

All 32 direct sites have dispositions. Two are transparent. The remaining
sites add 50 new exact meanings, reuse one target-Directory identity within
this owner group and add no safe projection. The effective whole-program
constructor frontier therefore moves from 2,075 to 2,107 classified sites and
from 424 to 392 open sites.

The qualified semantic ledgers move from 2,349 to 2,399 provisional exact
candidates. Their 31 additional safe projections remain unchanged, producing
2,430 current symbolic identities before final whole-program reuse and
allocation review.

## Required Tests

- independently reject all seven active canonical/monotonic evidence leaves
  and all six protected root-row leaves;
- distinguish absent, conflicting, unacknowledged and already-active Joining
  state, including exact status/activation retry;
- distinguish remote acknowledgement root/version rejection from corrupt
  durable acknowledgement root/version state;
- prove active source, target, predecessor and Directory conflicts do not
  overwrite current authority;
- independently reject canonical snapshot manifest/version and every protected
  root field before commitment;
- independently reject both transition authorities, nonadvancing revision and
  each absent hash; and
- prove both decoded Coordinator result adapters preserve the exact remote
  diagnostic without wrapper text or a second numeric identity.

## Next Slice

Continue the effective frontier with Component Directory synchronization and
Fleet-service peer owners, preserving Component-scoped Directory revisions and
local Registry-authenticated peer authority.
