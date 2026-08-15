# Canic 0.102 Canister Pool Workflow Constructor Leaves

Date: 2026-08-15

## Status

This B1 evidence ledger classifies all 11 direct constructors in
`crates/canic-control-plane/src/workflow/canister_pool/mod.rs` and all six in
its `refill.rs` child. It assigns no number and changes no runtime behavior.

## Maintenance, Import And Handoff

| Exact candidate | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_STATUS_LIMIT_INVALID` | 1 | `status`; limit is zero or exceeds `MAX_STATUS_PAGE_ENTRIES` | self | Request `1..=256` entries | public |
| `CANISTER_POOL_REFILL_RETRY_DRAINING` | 1 | `admin` RetryRefill branch; `root_is_draining` is true | self | Reconcile/cancel existing creation; do not start another | public |
| `CANISTER_POOL_MAINTENANCE_ROOT_PHASE_INVALID` | 1 | `maintain_once`; activation phase is neither Prepared nor Active | self | Wait for Prepared/Active or inspect root lifecycle | public |
| `CANISTER_POOL_IMPORT_DRAINING` | 1 | `import`; `root_is_draining` is true | self | Import before draining or hand off existing assets | public |
| `CANISTER_POOL_HANDOFF_ROOT_NOT_DRAINING` | 1 | `handoff`; `root_is_draining` is false | self | Fence the root before transferring asset authority | public |
| `CANISTER_POOL_HANDOFF_RECIPIENT_INVALID` | 1 | `canister_pool::handoff`; recipient is anonymous, management, root or source asset | self | Select a distinct non-reserved replacement authority | public |
| `CANISTER_POOL_HANDOFF_RECIPIENT_CONFLICT` | 1 | `canister_pool::handoff`; terminal receipt names another recipient | self | Preserve the first receipt and use its exact recipient | public |
| `CANISTER_POOL_IMPORT_INFRASTRUCTURE_FORBIDDEN` | 1 | `require_import_candidate`; target is root, Coordinator or sibling Store | self | Import only an empty non-infrastructure Canister | public |
| `CANISTER_POOL_IMPORT_COMPONENT_MEMBER_FORBIDDEN` | 1 | `require_import_candidate`; target has Component Registry membership | self | Recycle through Component removal rather than import | public |
| `CANISTER_POOL_IMPORT_SUBNET_ROUTE_MISSING` | 1 | `validate_import_subnet`; NNS route is absent | self | Verify the Canister exists and Registry evidence is current | public |
| `CANISTER_POOL_IMPORT_SUBNET_MISMATCH` | 1 | `validate_import_subnet`; observed route differs from protected root Subnet | self | Select a Canister on the root Subnet | public plus guarded routing status |

All 11 sites add 11 exact meanings and no safe projection.

The Subnet-mismatch message currently contains the requested Canister, observed
Subnet and protected expected Subnet. The Canister is caller-derivable. Expected
and observed Subnets are required to remediate placement but have no existing
retrievable response owner; B3 must add a guarded bounded
`CanisterPoolImportRoutingStatusResponse` keyed by the target Canister and
carrying expected/observed Subnet plus the NNS Registry version used. The
diagnostic itself remains code-only.

## Recoverable Autonomous Refill

| Exact candidate or disposition | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_MAXIMUM_EXHAUSTED` | 1 | `refill::start`; standby capacity is exhausted below the minimum | self; reuses the ops capacity identity | Claim/handoff an asset or change immutable policy | public |
| `CANISTER_POOL_CREATION_NOT_PENDING` | 2 | `refill::reconcile` or `refill::reconcile_draining`; pending creation is absent | self; both sites reuse the ops absence identity | Inspect status and begin/recover the exact creation | public |
| `CANISTER_POOL_CREATION_COST_AUTHORITY_PENDING` | 1 | `refill::reconcile_draining`; known-unapplied Intent retains cost authority | self; reuses the ops cost-authority identity | Recover the retained cost settlement before cancellation | public |
| `CANISTER_POOL_CREATION_RECOVERY_DISAPPEARED` | 1 | `refill::retry_intent`; creation disappears after `reconcile_previous_cost_guard` | `COMPONENT_REGISTRY_STATE_INVALID` | Stop and inspect the single-writer creation state | recent failure |
| `CANISTER_POOL_CREATION_CYCLES_LEDGER_MISMATCH` / `CANISTER_POOL_CREATION_PLACEMENT_SUBNET_MISMATCH` / `CANISTER_POOL_CREATION_ROOT_MISMATCH` | 1 | `refill::validate_creation_authority`; retained Ledger, Subnet or root differs from protected authority | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Preserve creation and identify the exact authority mismatch | recent failure |

The six sites produce seven label occurrences: three reuse qualified pool ops
identities and four exact meanings are new. No dynamic value is interpolated by
these constructors.

## Reconciliation

The source and table counts agree at 11 plus six references. All eighteen
referenced exact identities name their functions/branches; after three ops
reuses are deducted, the two workflow owners add 15 meanings and no projection.

## Required Tests

- reject zero/oversized paging and every reserved handoff recipient;
- test import, refill retry and handoff on both sides of the draining fence;
- reject root, Coordinator, Store and Component members as import candidates;
- exercise missing, matching and foreign NNS routes and prove guarded routing
  status retains exact Registry-versioned expected/observed evidence;
- reconcile absent, pending-cost and vanished creation state independently; and
- corrupt each retained Cycles Ledger, physical Subnet and root authority field
  independently before any repeated paid effect.

## Next Slice

Classify root Store bootstrap ops/workflow, then remaining Wasm Store lifecycle
and Fleet Mirror synchronization owners.
