# Fleet infrastructure reuse and retirement contraction — 2026-10-01

The initial blanket-removal recommendation is withdrawn after the maintainer
identified possible Root return-to-pool and recommissioning. Retirement is
unnecessary for release updates, but safely ending a Root's ownership is a real
requirement for reuse. Preserve that responsibility while assessing which
deletion-specific machinery can be removed. The maintainer subsequently clarified
that Coordinator and Wasm Store must also be reusable when removing a Fleet.
The outcome is role-independent physical reuse; no runtime cut is implemented.

The maintainer accepted [FR1 in the 0.110 plan](../../../design/0.110-fleet-runtime-contraction/0.110-design.md#fr1-fleet-release-to-reusable-capacity--accepted-2026-10-01)
on 2026-10-01. That design and its tracker now own contract and sequencing; this
note retains the bounded source investigation, not separate implementation gates.

## All infrastructure roles: retained requirement and contraction opportunity

Coordinator, Root and Store are physical canisters with temporary Fleet roles.
Removing an entire Fleet should permit retaining all selected IDs and their
controlled cycles as reusable capacity. Current explicit infrastructure
bootstrap already accepts supplied IDs; current pool reset knows how to stop,
normalize controllers, uninstall code, clear observed snapshots and verify an
empty asset. Those building blocks do not yet constitute a qualified Fleet
release-to-capacity journey. Active infrastructure cannot simply be supplied to
ordinary capacity import; that route correctly rejects infrastructure/ownership
conflicts. It must first cease being owned by the former Fleet.

| Role | Current retirement endpoint outcome | Necessary role-specific work before reuse |
| --- | --- | --- |
| Root | Child handoff, Store deletion, Ledger/native evacuation, physical-deletion readiness | Close provisioning and unfinished paid work; release child/Store ownership; settle or retain exact Ledger authority |
| Coordinator | Ledger transfer after every Root has a physical deletion receipt | Preserve complete inventory outside the retiring Fleet; close membership/funding operations; settle or retain exact Ledger authority |
| Store | Prepared/clearing/complete GC, binding finalization, native drain and physical deletion | Close consumers/grants and release controller ownership; clear content/snapshots under exact reset authority |

For whole-Fleet reuse, the likely simplification is one externally durable
reviewed operation using existing Host operation locks, effect journalling and
observation/reconciliation. Capture the complete inventory before clearing any
canister that owns it. Quiesce and reconcile unfinished effects, retain custody
of every child, bind exact post-release owners and destinations, and reset each
selected asset only after those facts hold. Keep IDs and native balances where
reuse selects retention: native evacuation is then unnecessary. Ledger balances
must have explicit retained controlled-account evidence or exact settlement
receipts; clearing local code/state does not settle a Ledger obligation.

Return pool-ready capacity only after exact controller, empty-module/snapshot,
subnet and balance observations. Root-owned pools require a surviving independent
Root on that subnet; other IDs can remain explicitly operator-held for later
infrastructure bootstrap. A Coordinator on another subnet cannot be inserted
into a workload Root's pool. No new reserve-Fleet/custody runtime, predecessor
decoder or second generic reconciler is proposed.

Partial Root removal while keeping its Fleet active remains a different scope:
old Registry/service references must be released correctly. Whole-Fleet teardown
can discard completed application/framework state after external inventory and
unfinished-effect authority are secured. It need not replay every internal
publication merely to preserve a Registry that will itself be reset.

Concrete contraction candidates are deletion-execution attestations/tombstones,
the requirement to physically delete Roots before releasing Coordinator assets,
native-balance evacuation selected solely for deletion, and Store content-clearing
orchestration selected solely before destructive reset. Keep bounded accounting,
authority checks and effect recovery. Each removed phase needs proof that the
common reviewed reset owns the same safety obligation. A generic per-canister
helper alone cannot replace Fleet ownership release.

Eight inspected main owner files total **6,418 Rust lines**, including mixed
responsibilities and tests. This is a footprint, not promised deletion or Wasm
savings. It comprises the preceding five-file 3,988-line sample plus Coordinator
Root-deletion ops (745), Root-owned Store GC/deletion orchestration (1,471) and
Store deletion-cycle workflow (214). DTOs, stable schemas, role dispatch, receipts,
projections, resumers and other tests retain additional complexity not counted.

Implementation starts by freezing the bounded current-contract Fleet release
that preserves reusable IDs, limiting role-specific work to ownership and
external balances under accepted FR1.
Its proof must include all three infrastructure roles, children, native/Ledger
conservation, interruptions and immediate effect-free replay. The current
deployment batch and CANIC-188 repair continue independently; this investigation
does not authorize effects. FR1 implementation is now accepted and sequenced by
the plan above; its safety dependencies and acceptance own the next steps.

## Maintainer clarification: reusable Root capacity

Returning a Root to a pool would retain its physical canister ID and controlled
cycles while releasing its former Fleet/Root role. A bounded operation must
close workload and paid-effect ownership, settle or hand off children and Store,
account for native and Ledger balances, close the old Registry membership, and
transfer exact controller authority to a reviewed destination before reset and
pool admission. A canister cannot be returned to its own Root-owned pool; the
destination must retain an independent controller and be on the same subnet.

The current removal driver hands off child pool assets but ends at Root deletion
readiness. It neither enrolls Root itself into a destination pool nor qualifies
recommissioning. Existing pool reset/import is useful lower-level machinery,
not proof of a safe complete Root-to-pool transition. Coordinator joining also
retains immutable join receipts and rejects new joins after activation; there
is no qualified removed-Root rejoin in the same active Registry. Future Root
bootstrap on a reused ID must use explicit current authority, not historical
join replay. The accepted FR1 design now owns this outcome and its sequencing;
it authorizes no live or downstream effects.

## What it actually enables

`CoordinatorCommand::RemoveRoot` accepts a draining reservation, submits one
Root removal intent and polls it. Root retires its Component partitions, hands
off spare pool canisters, freezes final inventory, clears fixture/template
content, deletes the Store, transfers its Ledger/native balances and publishes
deletion readiness. An external management controller must still delete Root;
Coordinator then accepts an exact completion receipt. Coordinator `Retire`
requires every registered Root to have a deletion receipt and transfers its
Ledger balance. This family permanently removes infrastructure.

Its remaining product reason is deliberate same-release decommissioning of an
unused standalone Root/subnet or Fleet. Keeping that capability would justify
the current recovery/conservation code. Updating a retained Root does not.

## Evidence against retaining it for current deployment

- The [current design](../../../design/0.110-fleet-runtime-contraction/0.110-design.md#non-goals)
  expressly separates reinstall from evacuation: selected identities and cycle
  accounts can remain; reinstall does not require old retirement endpoints.
  Complete whole-Fleet deletion is optional follow-up, not a deployment gate.
- Current Host Root reset compiles `InstallMode::Reinstall` on the reviewed
  Principal, with exact controller checks, retained journals and terminal cycle
  conservation. The current pre-1.0 release policy is reinstall-only; a
  state-preserving cross-release upgrade is a separate, unsupported contract.
- No invocation of `RemoveRoot`, `PrepareRootDeletionExecution`,
  `CompleteRootDeletion` or Coordinator `Retire` was found in current Host/CLI,
  applications or scripts. Generated command declarations are reachable, but
  are not evidence of an operator need.
- `require_grouped_root_lifecycle_open` rejects Roots referenced by provisioning
  journals, placement records or services. The journal predicate does not filter
  completed operations. This cannot retire a normally provisioned grouped Root;
  no maintained grouped drain/removal route was found.
- The [fixture guide](../../../features/build-and-evidence/fixture-artifacts.md#source-retirement)
  documents that restriction. Its retirement journey and the autonomous Root
  removal journey qualify specially constructed standalone Registries.
- Git commit `bb221fd50` (`0.109.13`, 2026-08-27) removed the 1,027-line Host
  deletion executor and both retirement/deletion examples. The surviving
  control-plane implementation therefore outlived its Host product route.

## Conditional deletion-only cut

The following is the earlier deletion-only candidate boundary, not the current
blanket-removal recommendation. Resolve Root reuse before choosing which parts
can disappear; conservation and ownership release may need existing helpers.

Remove the Root/Coordinator deletion commands and statuses, retirement-specific
DTOs/Candid and replay registrations, draining/deletion journals and resumers,
Root Ledger evacuation, Coordinator retirement, Store final-inventory/deletion
orchestration and their exclusive tests/documentation. Coordinator retirement
belongs in the same scope review because it depends on completed Root deletion.

Keep live Store adoption and Root inventory from the mixed
`workflow/fleet_subnet_root.rs` file. Preserve funding, pool refill/import/reset,
component/subtree removal and recycling, fixture delivery/readiness, backup and
snapshot recovery, generic Ledger/management operations and cost/replay guards
where independently consumed. Component removal has its own generated command;
pool refill independently uses the Ledger. Some funding/admin validators are
currently housed in `ops/component_registry/root_retirement.rs`; removing the
module requires updating those current owners, not deleting their admission
checks or leaving empty compatibility helpers.

The earlier 3,988-line owner footprint is not a deletion estimate. Additional
wire/storage/Store code may fall out; shared owners and tests remain. Measure
actual source and optimized raw Wasm changes after a bounded feature cut.

## Recovery boundary and qualification

No live Root/Coordinator journals or controllers were queried. Local generated
Candid/cache artifacts are declarations, not proof of issued operations. Any
genuinely unfinished paid retirement must keep its original reconciliation
authority; do not install a hard cut over it and erase that evidence. Completed
historical records need no compatibility decoder. The exact CANIC-188 import
repair and retained original/candidate artifacts are separate and remain intact.

Qualification for the proposed cut should cover current generated role/Candid
surfaces, fresh Fleet activation, Store adoption, Root funding/refill, component
removal/recycling and exact clean-reinstall recovery/replay. Use focused tests;
do not add tests preserving removed command spellings. No runtime source change,
Cargo compile, PocketIC run or live effect was made by this investigation.

## Reproduction and snapshot

Source checkpoint: `02af7277664c1664a58bfaa5e6fde997474c9802`, dirty worktree;
concurrent deployment changes excluded. The preceding
[manifest](../../reports/2026-10/2026-10-01/surface-cleanup-and-subsystem-usefulness.json)
pins the sampled retirement owners. Additional inspected source SHA-256:

- `workflow/runtime/template/publication/lifecycle/gc.rs` (Control Plane):
  `148c71fe071c7b211506ce952896ddbf067d143d225d2b4686f3bcb02a5cc9a1`.
- `fleet_ensure/policy/root_reinstall/mod.rs` (Host):
  `8bce0ef46345de9c95bbdd3a4f5a38acd5455f4163337432cc2721ab997d5cf8`.

Reproduce the caller screen with `rg -n
'RemoveRoot|PrepareRootDeletionExecution|CompleteRootDeletion|Retire\('
crates/canic-host/src crates/canic-cli/src apps scripts --glob '*.rs' --glob
'*.sh' --glob '*.toml'`; inspect Root workflow, grouped lifecycle validation,
Coordinator retirement authority and Host Root reinstall. Reproduce the removed
Host route with `git show --stat bb221fd50 --
crates/canic-host/src/fleet_subnet_root_deletion/mod.rs
crates/canic-host/examples/empty_fleet_subnet_root_retirement.rs
crates/canic-host/examples/fleet_subnet_root_deletion.rs`.
