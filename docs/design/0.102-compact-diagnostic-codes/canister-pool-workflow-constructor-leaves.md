# Canic 0.102 Canister Pool Workflow Constructor Leaves

Date: 2026-08-13

## Status

This B1 evidence ledger classifies all 11 direct constructors in
`crates/canic-control-plane/src/workflow/canister_pool/mod.rs` and all six in
its `refill.rs` child. It assigns no number and changes no runtime behavior.

## Maintenance, Import And Handoff

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_STATUS_LIMIT_INVALID` | 1 | Status page limit is zero or exceeds 256 | self | Request `1..=256` entries | public |
| `CANISTER_POOL_REFILL_RETRY_DRAINING` | 1 | Explicit refill retry is attempted after the root's admission fence | self | Reconcile/cancel existing creation; do not start another | public |
| `CANISTER_POOL_MAINTENANCE_ROOT_PHASE_INVALID` | 1 | Maintenance runs before Prepared or after the root leaves Active/Prepared operation | self | Wait for Prepared/Active or inspect root lifecycle | public |
| `CANISTER_POOL_IMPORT_DRAINING` | 1 | Import is attempted after root draining fences pool growth | self | Import before draining or hand off existing assets | public |
| `CANISTER_POOL_HANDOFF_ROOT_NOT_DRAINING` | 1 | Handoff is attempted before the root enters draining | self | Fence the root before transferring asset authority | public |
| `CANISTER_POOL_HANDOFF_RECIPIENT_INVALID` | 1 | Recipient is anonymous, management, the source root or the asset itself | self | Select a distinct non-reserved replacement authority | public |
| `CANISTER_POOL_HANDOFF_RECIPIENT_CONFLICT` | 1 | Terminal handoff receipt names another replacement authority | self | Preserve the first receipt and use its exact recipient | public |
| `CANISTER_POOL_IMPORT_INFRASTRUCTURE_FORBIDDEN` | 1 | Import target is the root, Coordinator or a sibling Wasm Store | self | Import only an empty non-infrastructure Canister | public |
| `CANISTER_POOL_IMPORT_COMPONENT_MEMBER_FORBIDDEN` | 1 | Import target is a registered Component-tree member | self | Recycle through Component removal rather than import | public |
| `CANISTER_POOL_IMPORT_SUBNET_ROUTE_MISSING` | 1 | Mainnet Registry has no Subnet route for the import target | self | Verify the Canister exists and Registry evidence is current | public |
| `CANISTER_POOL_IMPORT_SUBNET_MISMATCH` | 1 | Mainnet import target is not on the root's exact physical Subnet | self | Select a Canister on the root Subnet | public plus guarded routing status |

All 11 sites add 11 exact meanings and no safe projection.

The Subnet-mismatch message currently contains the requested Canister, observed
Subnet and protected expected Subnet. The Canister is caller-derivable. Expected
and observed Subnets are required to remediate placement but have no existing
retrievable response owner; B3 must add a guarded bounded
`CanisterPoolImportRoutingStatusResponse` keyed by the target Canister and
carrying expected/observed Subnet plus the NNS Registry version used. The
diagnostic itself remains code-only.

## Recoverable Autonomous Refill

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `CANISTER_POOL_MAXIMUM_EXHAUSTED` | 1 | Automatic refill is below minimum but cannot add another standby asset | self; reuses the ops capacity identity | Claim/handoff an asset or change immutable policy | public |
| `CANISTER_POOL_CREATION_NOT_PENDING` | 2 | Normal or draining reconciliation has no durable creation | self; both sites reuse the ops absence identity | Inspect status and begin/recover the exact creation | public |
| `CANISTER_POOL_CREATION_COST_AUTHORITY_PENDING` | 1 | Draining reconciliation sees known-unapplied Intent but unsettled paid-attempt authority | self; reuses the ops cost-authority identity | Recover the retained cost settlement before cancellation | public |
| `CANISTER_POOL_CREATION_RECOVERY_DISAPPEARED` | 1 | Durable creation vanishes between cost reconciliation and the next attempt | `COMPONENT_REGISTRY_STATE_INVALID` | Stop and inspect the single-writer creation state | recent failure |
| `CANISTER_POOL_CREATION_CYCLES_LEDGER_MISMATCH` / `CANISTER_POOL_CREATION_PLACEMENT_SUBNET_MISMATCH` / `CANISTER_POOL_CREATION_ROOT_MISMATCH` | 1 | Retained creation differs from current protected Ledger, Subnet or root authority | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Preserve creation and identify the exact authority mismatch | recent failure |

The six sites produce seven label occurrences: three reuse qualified pool ops
identities and four exact meanings are new. No dynamic value is interpolated by
these constructors.

## Reconciliation

The source and table counts agree at 11 plus six references. The two workflow
owners add 15 exact meanings and no projection after reuse is deducted.

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
