# 0.106 B1 Baseline And Current Pool Inventory

Date: 2026-08-20
State: accepted repository inventory for B1 on 2026-08-20; B2 observations remain open

## Immutable Baseline

| Fact | Frozen value |
| --- | --- |
| Repository | Canic |
| Direct predecessor | annotated tag `v0.105.0` |
| Tag object | `b9531084db9345c001e51d08d0e1cd6f3c331fa2` |
| Peeled commit | `b6c46ca1d307e0a3fed6f7bfddfba7d9f1922811` |
| Tree | `5568651d2e29f229a2ee4557de88b490a07cd34a` |
| Commit subject | `Release 0.105.0` |
| Commit/tag time | 2026-08-20 11:01:29/11:01:32 +0200 |
| Workspace package version | `0.105.0` |
| Cargo.lock SHA-256 | `ce8705c5eee0274525f2bb24b73d12faea51a6ce5945a1ef849ca3c90b38ee66` |
| Repository Rust toolchain | `1.97.1`, minimal profile with rustfmt and Clippy |
| Observed rustc | `rustc 1.97.1 (8bab26f4f 2026-07-14)`, LLVM 22.1.6 |
| Observed Cargo | `cargo 1.97.1 (c980f4866 2026-06-30)` |
| Observed ICP CLI | `icp 1.3.0` |
| Submodules | none |

The source baseline was clean at the tag. The active B1 tree initially changed
only changelog/documentation plus the focused host test described below. A
source diff over the current pool storage, ops, workflow, lifecycle,
async-job-recovery and allocation paths is empty from `v0.104.2` to
`v0.105.0`; therefore `v0.105.0` is the direct 0.106 predecessor while the
accepted 0.104 timer/state ownership remains the unchanged inherited boundary.

## Q1 Empty-Topology Result

An App with a Root role and no Component Specs is valid configuration:
`ConfigModel::validate` returns after role validation when the Component Spec
map is empty (`crates/canic-core/src/config/validation/mod.rs:43`). The host
cannot turn that valid configuration into an explicit root plan, however.
`finalize_planned_root` always calls
`ComponentTopology::project_for_admissions`, and that projection rejects an
empty admission vector as `EmptyRootAdmissions`
(`crates/canic-host/src/component_topology/mod.rs:274` and
`crates/canic-core/src/config/topology/mod.rs:294`).

The focused current-boundary test uses exactly one Root, one explicit Subnet,
zero application Component Specs and zero admissions and asserts that exact
typed failure (`crates/canic-host/src/component_topology/tests.rs:167`). This
is the earliest blocker, before immutable Fleet planning, root/Store creation
or PocketIC activation. It is not evidence that an empty topology activates.

Disposition: Q1 is blocked on the current production topology contract. Under
the evidence-only 0.106 design, the production correction belongs to 0.109.
B1 cannot treat this as its permitted alternate outcome until the blocker and
later owner are explicitly accepted.

## Q6 Stable Authority

The complete current pool storage owner is the Fleet Subnet Root control plane.
It uses three stable allocations declared by the canonical allocation catalog
(`crates/canic-core/src/role_contract/allocation.rs:43`):

| Memory ID | Container and current record | Current bound | Authority and disposition |
| ---: | --- | --- | --- |
| 24 | Stable B-tree keyed by Canister Principal, `CanisterPoolAssetRecord` | The largest finite current variant is an empty-error Recycling row at 451 bytes. The record is structurally unbounded because `Failed(String)` and failed recycle state retain unrestricted text; a 1,024-byte reason produces 1,297- and 1,477-byte rows. Configured `maximum_size` is only a `u32` range/value check. | Physical asset identity and lifecycle are retained domain authority; a future indexed estate must deliberately bound or replace the error detail and define its estate ceiling. |
| 25 | Stable cell, `CanisterPoolStateRecord` | Declared bound 1,024 bytes; exact structural maximum across current fields and progress variants is 651 bytes. | Retains one sequence/timestamp plus at most one creation and one handoff in ordinary ops; retained exact-effect authority. The 651-byte structural measurement deliberately includes both option fields and a cost-guard settlement even though workflow admission normally keeps the lanes exclusive. |
| 26 | Stable B-tree keyed by handed-off Canister Principal, `CanisterPoolHandoffReceiptRecord` | Declared bound 64 bytes. A receipt containing the current 10-byte Canister-Principal shape is 47 bytes, but the Rust type admits a 29-byte Principal and then encodes to 67 bytes. No receipt-count retention ceiling exists. | Terminal exact handoff replay authority. The declared bound is not structurally valid for every value accepted by the field type, and future retention/index policy remains open. |

The state contract names all three as current version-1 domains with no
migrations (`crates/canic-control-plane/src/state_contract.rs:263`). This is
same-release authority, not permission to add a cross-release decoder.

### Exact Encoding Measurements

Focused test-only measurements use the production stable serializer and
`Storable` implementations. They freeze the current repository shape rather
than proposing future estate limits:

| Asset status | Encoded bytes with maximum finite fields and retained previous claim |
| --- | ---: |
| Store | 268 |
| Store deletion pending | 364 |
| Pending reset | 275 |
| Ready | 268 |
| Claimed | 427 |
| Workload | 428 |
| Recycling, reset pending | 450 |
| Recycling, reset ready | 448 |
| Recycling, empty failed reason | 451 |
| Handing off | 316 |
| Failed, empty reason | 271 |

The finite table is not an asset-record maximum. A 1,024-byte failure reason
encodes to 1,297 bytes in top-level Failed state and 1,477 bytes in Recycling
failed state. The unrestricted `String` makes memory ID 24 structurally
unbounded, independent of the configured pool count.

The pool singleton encodes to 73 bytes when empty. With maximum numeric and
Principal values, both optional lanes populated and the cost-guard settlement
retained, its Intent, Created and Blocked progress forms encode to 604, 651 and
624 bytes. Created is the exact current structural maximum and remains 373
bytes below the declared 1,024-byte cell bound.

One 10-byte Canister Principal key encodes to ten bytes. A handoff receipt with
a 10-byte recipient encodes to 47 bytes. A structurally maximal 29-byte
recipient encodes to 67 bytes and exceeds the declared 64-byte bound. Current
workflow provenance expects both the handed-off asset key and recipient to be
Canister Principals, but that narrower invariant is not represented by the
stored field type. B1 therefore records the 64-byte declaration as an existing
0.109 blocker rather than treating it as a complete structural ceiling.

### Physical Asset State

`CanisterPoolAssetRecord` retains observed cycles, origin, lifecycle status,
last recycle claim and added/updated timestamps
(`crates/canic-control-plane/src/storage/stable/canister_pool/mod.rs:188`). Its
closed origins are infrastructure Store, created, imported and recycled. Its
current status union is:

- Store and Store-deletion-pending;
- pending reset, Ready and Failed;
- Claimed and Workload with exact Component instance plus operation ID;
- Recycling with the same claim and pending/ready/failed reset state; and
- HandingOff with the exact recipient.

The durable pool singleton retains `next_creation_sequence`,
`last_creation_timestamp_ns`, one optional creation and one optional handoff
(`crates/canic-control-plane/src/storage/stable/canister_pool/mod.rs:98`).

### Singleton Creation Lane

The one optional creation record freezes:

- Canic operation ID;
- exact Cycles Ledger Principal;
- placement Subnet;
- owning Root;
- Ledger amount and `created_at_time`;
- preparation time and optional replay-cost settlement; and
- Intent/Created/Blocked progress, including uncertain-result state and the
  returned block/Canister identity.

`begin_creation` accepts byte-equivalent current authority but rejects a
different request while the singleton is occupied
(`crates/canic-control-plane/src/ops/canister_pool/mod.rs:422`). Automatic
refill writes that intent before calling the Cycles Ledger and retries the same
stored request (`crates/canic-control-plane/src/workflow/canister_pool/refill.rs:37`).
The current repository therefore has exactly one paid creation lane per Root.

The lane touches the shared receipt-backed intent/cost-guard authority only
through one optional `ReplayCostGuardSettlement`, which holds exactly two
`IntentId` values: one quota intent and one reservation intent. The global
current intent store admits at most 1,000 receipt-backed records and 1,000
resource-total rows. Pool creation uses the exact static command kind
`cycles_ledger.control_plane.canister_pool_create.v1`, a 60-second window and
at most 64 control-plane deployment reservations per window. The pool
singleton still permits only one creation request at a time, so those generic
limits do not create parallel paid pool lanes.

The exact Cycles Ledger request identity is one operation ID, Ledger Principal,
placement Subnet, owning Root, amount and `created_at_time`. Its only retained
success receipt is one block index plus one returned Canister Principal. The
same singleton record retains uncertainty and the two cost-guard intent IDs
across interruption; no second pool-specific replay store exists.

### Reset Selection And Full Scans

Reset processing materializes the complete stable asset map, filters pending
reset/recycling rows and takes the first Principal-ordered result. One
maintenance pass resets at most one asset
(`crates/canic-control-plane/src/ops/canister_pool/mod.rs:218` and
`crates/canic-control-plane/src/workflow/canister_pool/mod.rs:179`). It is not
a durable reset queue or a parallel lane set.

The following current paths materialize or traverse the complete asset map:

| Consumer | Current behavior |
| --- | --- |
| Stable export | Collects every asset row plus the singleton state. |
| Pending reset selection | Full export/filter before selecting one candidate. |
| Oldest Ready claim | Full export, duplicate-claim scan, then minimum by added time and Principal. |
| Existing claim lookup | Full export/filter to prove zero or one matching claim. |
| Status | Full export and complete aggregate count before applying a page of at most 256 rows. |
| Ready/pool/workload/Store/summary counts | Separate full exports and filters for each derived count. |
| Standby capacity | Calls the full-scan pooled count, then adds the singleton pending-creation bit. |

These are derived views, not durable counters or indexes. The public status
page is bounded after collection; it does not make the underlying count and
projection work bounded by page size.

### Configuration And Capacity

Current validation requires `minimum_size > 0`, `maximum_size >= minimum_size`
and positive per-Canister cycles. It does not impose a maintained upper bound
below the `u32` type limit (`crates/canic-control-plane/src/ops/canister_pool/mod.rs:1080`).
Imports and new standby capacity are checked against `maximum_size`, while
proactive maintenance stops when the full-scan Ready count reaches
`minimum_size`. Setting 1,000 is therefore syntactically possible but remains
an unqualified serialized/full-scan configuration, not a reserve-estate
contract.

## Timer, Recovery And Lifecycle Inventory

The Root directly owns two volatile native registration capabilities in heap
state:

| Owner label | Native registration | Cadence | Domain work |
| --- | --- | ---: | --- |
| `canic:canister_pool:maintain` | after-completion | 30 seconds | One non-overlapping maintenance pass: reconcile creation, reset one asset, or start one refill. |
| accepted Root recovery watchdog | watchdog | 30 seconds | Bounded takeover for expired core jobs plus `CanisterPoolMaintenance`; it is not an ordinary second scheduler. |

The maintenance attempt lease is five minutes. Its attempt fence remains in
the accepted memory-ID-60 async-job-recovery domain rather than the pool
records. Both timer capabilities use native `ic-timers` reconciliation and
cancellation (`crates/canic-control-plane/src/workflow/canister_pool/mod.rs:43`,
`:260` and `:283`). No provider handle, deadline, cadence, callback generation
or timer inventory is stable pool state.

Root lifecycle declares native custody before activation, starts it only after
the Fleet reaches the admitted active path, stops it when draining fences new
allocations, and cancels/reconstructs it around authority snapshots
(`crates/canic-control-plane/src/api/lifecycle.rs:88`, `:157`, `:800`,
`crates/canic-control-plane/src/workflow/runtime/fleet_activation/mod.rs:55`
and `crates/canic-control-plane/src/workflow/fleet_subnet_root.rs:285`).

## Restart, Export And Snapshot Ownership

Same-release restart reconstructs all three stable allocations in place. The
Root owns memory IDs 24-26, while volatile maintenance and watchdog custody is
redeclared and reconciled by the accepted lifecycle path. No pool record stores
a provider deadline, registration handle, callback generation or timer
inventory.

`CanisterPoolStore::export()` materializes memory ID 24 plus the memory-ID-25
singleton as `CanisterPoolData`; there is no corresponding pool import path.
The handoff-receipt map is not part of that export. Its state-contract snapshot
name resolves only to the zero-field `CanisterPoolHandoffReceiptData` marker;
no canonical receipt-map `export()` or `import()` exists. Authority-snapshot
preparation suspends and resumes native pool timers but does not serialize any
of these pool allocations.

This means current same-release upgrade recovery remains truthful because the
stable maps stay resident, while a canonical external pool snapshot/restore
contract is incomplete—especially for terminal handoff receipts. B1 records
that as current ownership evidence and a 0.109 blocker; 0.106 does not add a
snapshot payload or migration lane.

## Current Release-Tree Reachability Classification

| Changed path class | Classification | Production reachability |
| --- | --- | --- |
| Host component-topology test | test evidence | none |
| Host qualification-package terminal guard | semantic test evidence | none; rejects a moved, published, depended-on or role-configured qualification canister |
| Pool stable-storage `#[cfg(test)]` module | encoding evidence | excluded from non-test builds; production records and storage methods are unchanged |
| Cycles Ledger boundary canister | test-only qualification fixture | unpublished dependency leaf under `canisters/test` |
| `payload_limit_probe` | test-only qualification fixture | unpublished dependency leaf under `canisters/test`; exact workload artifact only |
| Fleet Registry qualification helpers | PocketIC test evidence | `#[cfg(test)]` helpers in unpublished `canic-testing-internal` |
| Direct pinned `pocket-ic` dependency | dev dependency of unpublished test support | absent from every shipped role graph; enables exact management-call routing only |
| Payload-limit fixture identity test | PocketIC test evidence | integration-test target only |
| Native-agent proof-provisioning correction | predecessor qualification test evidence | integration-test target only; no production retry or authorization behavior changes |
| `root_funding_probe` and its recovery journey | scheduled 0.108 B1 evidence | unpublished dependency leaf and PocketIC integration-test target only; outside 0.106 B2 and absent from shipped role configuration |
| Workspace member and test-inventory entries for `root_funding_probe` | test catalogue metadata | admits only the unpublished 0.108 fixture and its classified integration target |
| 0.106 working evidence | audit evidence | none |
| 0.106 design/status and current handoff | documentation | none |
| 0.105 changelog reconciliation | documentation | none |
| 0.107-0.111 renumbering and linked historical/current guidance | documentation | none; preserves the settled successor sequence without implementing it |
| 0.108 B1 working evidence and root `Unreleased` note | successor test evidence and documentation | none in shipped runtime; deliberately remains outside the 0.106 patch summary |

One production-owned source file gains only code gated by `#[cfg(test)]`.
Non-test source, stable records, Candid, CLI, production dependencies, package
version and timer ownership are unchanged. Every package already present in
the `v0.105.0` lockfile retains its exact predecessor version. The dependency
additions are the dev-only direct edge from unpublished
`canic-testing-internal` to the already locked, exactly pinned PocketIC
provider and the unpublished `root_funding_probe` workspace leaf. The terminal
guard proves the two 0.106 qualification canisters plus that separately
classified 0.108 probe remain unpublished dependency leaves beneath
`canisters/test` and absent from shipped role configuration.

## Q6 Disposition And B1 Acceptance

Q6's repository inventory is complete. It records rather than repairs four
current constraints for 0.109 acceptance: unbounded failure text, the
structurally narrow 64-byte handoff-receipt bound, unbounded terminal receipt
count and the absence of a canonical handoff-receipt snapshot payload.

The maintainer accepted repository-local B1 on 2026-08-20. That acceptance:

- freezes Q3/Q4 protocol `canic-0.106-q3q4-v1`;
- assigns the exact `EmptyRootAdmissions` Q1 blocker to 0.109; and
- assigns the four Q6 constraints above to 0.109 without treating them as
  corrected by 0.106.

No repository-local B1 work remains. The separate Q2 provenance matrix is
complete, while every empirical contract cell remains pending B2. B1
acceptance does not bind a network or identity and does not authorize any
remote call, cycle spend, asset creation, controller mutation or terminal
asset disposition. The local 1/8/16/32 creation and empty/installed reset
lanes, exact unresolved retry, first excess, controller/routing contradictions
and terminal reachability guard remain the accepted repository-local evidence.
