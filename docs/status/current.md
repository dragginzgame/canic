# Current Status

Last updated: 2026-08-31

## Purpose

This is the compact handoff for active Canic source and roadmap work. Read it
first, then open only the linked design, audit or implementation owner needed
for the task.

Historical handoffs:

- [through 2026-06-30](archive/2026-06-30-precompact.md);
- [through 0.90.2](archive/2026-07-13-precompact.md);
- [through 0.101.52 Q4](archive/2026-08-12-precompact.md);
- [through published 0.109.12](archive/2026-08-26-pre-root-repair-hard-cut.md);
  and
- [pre-reorientation 0.109.24](archive/2026-08-30-pre-roadmap-reorientation.md).

## Release Evidence Contract

Release truth comes from workspace package versions, dated root and detailed
changelogs, the annotated Git tag and release commit, the complete published
package set, and the governed validation marker at the end of this file. The
version transaction owns that marker. This handoff does not maintain a second
mutable latest-release claim.

Current development begins from published `v0.109.31`. Its immutable details
are in [the 0.109 changelog](../changelog/0.109.md). Source-development truth
comes from Git and the working tree; explanatory prose is not a release guard.

## Maintained 0.109 Contract

Fleet-wide ingress admission retains one Coordinator-owned canonical policy,
one Root-owned distribution operation per Root, and one exact local projection
on each enrolled non-Root target. `caller::is_fleet_admitted()` and
`canic::fleet_admission::require_caller()` read that same local projection and
the observed transport caller. Admission never replaces application
membership, resource ownership, service or infrastructure authority.

The hard-cut Fleet Ensure path remains desired-state and live-observation
driven. Paid or identity-changing effects require exact reviewed authority,
durable intent, bounded debit and lost-response reconciliation. Terminal
convergence proves cycle conservation and immediate replay is effect-free.
Historical install/recovery compatibility is not restored.

Published correctness history relevant to current operators:

- `0.109.27` added the IcyDB `0.249.1` test-only lifecycle graph and exact
  management-canister version observation;
- `0.109.28` retained fresh-process projected estate state and prevented
  repeated applied reinstalls under version-less ordinary status;
- `0.109.29` separated controller preparation from successor Store adoption
  and closed the retained predecessor-E132 replan boundary; and
- `0.109.30` removed generic stable-state migration authority and reset the
  maintained Canic-owned product schemas to `v1`; and `0.109.31` published the
  fresh-estate, Principal-canonicalization, pool-recovery and first B9 splits.

Published `0.109.31` closes `CANIC-099` by restricting the unobserved Root
exception to one explicitly fresh, unallocated bootstrap Root, and closes
`CANIC-101` by rejecting anonymous/duplicate identities before sorting the
authored set once with `Principal::Ord`. It also permits the temporary pool
Ledger terminal slot to rotate for one distinct later operation while exact
replay and same-operation retargeting stay fenced. That helper remains until
both live recoveries and terminal Fleet replay are immutably proven.

The open `0.109.32` draft addresses `CANIC-102` through `CANIC-110`: exact create balances, symbolic Store controllers, typed unavailable Registry status, bounded terminal proof, reinstall-evidence fencing, post-bootstrap helper staging, manifest-bound no-follow Candid, exact activation revision and exact Idle-cycle reconciliation.
These are current 0.109 correctness repairs, not new capability or downstream mutation.
A focused governed PocketIC journey now closes the `CANIC-107` proof gap: a
real Store publishes the application set, a real Root adopts and bootstraps
it, the temporary helper stages only afterward, and immediate Store/Root
replay has no nonterminal action. The helper-bearing five-Component journey
also crosses Registry synchronization, runtime provisioning and activation,
then reaches terminal Fleet state and immediate effect-free replay.

The detailed changelog owns the complete tests and negative cases; this handoff owns no downstream repository or live IC state.

## Safety State

The retained downstream evidence for the last deployment unblock reports:

- the reviewed Root prerequisite applied once and replayed without a second
  effect;
- zero funding, transfer, fee or operator debit authority;
- measured execution burn retained in the conservation equation;
- exact Root, Store, controller, module and operation identities retained;
- successor Store adoption delayed until the successor Root existed; and
- no later database, catalogue or frontend publication effect.

Canic repository work must not mutate that downstream estate. Same-release
interruption recovery, retry, idempotency, exact authority and cycle-safe
source deletion remain mandatory even where cross-release compatibility is
absent.

## Open 0.109 B9 Complexity Contraction

0.109 remains open. Functional admission and the deployment corrections are
published, but the
[binding post-implementation complexity audit](../audits/release-lines/0.109-post-implementation-complexity-audit.md)
still has `closeout_verdict: fail`. B9 must produce an immutable superseding
pass before human closeout.

The first fresh v2 measurement correctly stopped on the post-baseline
`diagnostics/` owner. The governed
[v3 correction and rerun](../audits/reports/2026-08/2026-08-31/0.109-b9-complexity-method-v3.md)
keeps v2 byte-identical, adds exactly that subsystem, retains the five-slice
change-friction population and reruns both the original 0.92 baseline and
`v0.109.30` twice with deterministic output.

| Mechanical v3 measure | 0.92 baseline | `v0.109.30` |
| --- | ---: | ---: |
| non-test files | 516 | 553 |
| non-test logical LOC | 64,216 | 103,414 |
| non-test files at least 600 logical LOC | 14 | 32 |

The [working contraction evidence](../audits/working/0.109-fleet-wide-ingress-admission/b9-complexity-contraction.md)
uses the corrected measurement to select the first seams:

| Owner | Direct lines | Disposition |
| --- | ---: | --- |
| `ops/component_registry/mod.rs` | 17,452 | 6,303-line parent; focused allocation, Directory and retirement owners |
| `workflow/component_registry/mod.rs` | 8,751 | 5,838-line parent; focused lifecycle, install, authority and response owners |
| `ops/fleet_coordinator/mod.rs` | 8,997 | 2,688-line parent; focused admission, lifecycle and provisioning owners |

The production simplification keeps the Coordinator record as the sole durable owner while separating focused responsibilities:

| Coordinator owner | Lines | Exact responsibility |
| --- | ---: | --- |
| parent `mod.rs` | 2,688 | single-step Coordinator state operations and remaining provisioning logic |
| `admission.rs` | 304 | admission publication, capacity, canonical mutation and replay |
| `component_provisioning_projection` | 245 | current status and terminal receipt projection |
| `component_provisioning_progress` | 665 | read-only Directory/runtime progress reconstruction and advance classification |
| `component_provisioning_reconciliation` | 338 | observed Root acceptance/provisioning response validation |
| `component_provisioning_root_progress` | 572 | read-only Root progress reconstruction and replay/advance classification |
| `component_provisioning_directory` | 482 | Directory call/receipt authority and response validation |
| `component_provisioning_retry` | 67 | active-intent retry authority and pending-failure projection |
| `component_provisioning_validation` | 1,954 | retained Root acceptance/provisioning plus Directory/runtime response and receipt validation |
| `service_publication` | 326 | canonical Fleet services plus atomic Registry/receipt evidence |
| `registry_history.rs` | 924 | ordered canonical Registry reconstruction and validation |
| `root_lifecycle.rs` | 598 | Root join/snapshot, grouped fences, draining/removal reservation and publication authority |
No DTO, Candid, record, endpoint or effect-ordering changes. The parent is now 2,688 lines, down from 8,997.

The Component Registry now has a 1,647-line Root/Store retirement owner with
complete durable operations, validation and hashes; Directory refresh (326),
top-level allocation/activation (397/477),
direct-child allocation/activation (518/379) and top-level Component retirement
(618). A 1,143-line `subtree_retirement` owner now retains bounded traversal,
stop/delete, Directory synchronization and membership-removal evidence for
descendants. A 311-line `initial_inventory` owner seals active membership and
monotonically retains Directory and Root-runtime convergence. Its 5,628-line
unit-test corpus now has a dedicated test owner. All use the same Registry ops
authority; workflow, transport, Candid, schemas and effect ordering are unchanged. The production
parent is now 6,303 lines, down from 17,452 total lines. The workflow
gravity well now owns scheduling (651), installation (986) and authority
validation (709), Registry/allocation response projection (341) and retirement
response projection (309) separately; its parent is 5,838 lines, down from
8,751. These passive response owners read no store and make no lifecycle or
effect decision.
Remaining B9 work:

1. freeze one immutable candidate and rerun the canonical v3 complexity,
   change-friction, structure, duplication and module-surface methods;
2. run the maintainer-owned complete gate and record actual PocketIC elapsed,
   RSS, thread and case-count evidence against the provisional envelope; and
3. obtain the human-owned accepted superseding verdict.
## B10 And Closeout

After B9 passes, B10 reconciles the already-published managed-App
qualification support. It must replace downstream private init/activation and
admission-fixture machinery without adding runtime authority. The final 0.109
closeout remains human-owned.

Required order:

1. complete B9 simplification and immutable evidence;
2. complete B10 public-package consumer and managed/standalone qualification;
3. request the human closeout audit against the complete candidate; and
4. begin 0.110 only after the maintainer accepts that verdict.

## Roadmap Boundary

Toko Miner remains a read-only application steering source. Canic retains its
own fixtures and gains no downstream production dependency.

| Line | Active owner | State |
| --- | --- | --- |
| [0.109](../design/0.109-fleet-wide-ingress-admission/status.md) | Fleet-wide admission, B9 contraction and B10 qualification | open |
| [0.110](../design/0.110-fleet-runtime-contraction/status.md) | runtime contraction | blocked on accepted 0.109 closeout |
| [0.111](../design/0.111-bounded-multi-fleet-estates/status.md) | bounded multi-Fleet estates | blocked on 0.110, then Q0 capsule proof |

0.110 is a zero-capability contraction line. It owns controlled ablations,
direct role storage wiring, capability-owned records, conditional codecs and
whole generated-surface pruning across lifecycle, endpoint, Candid, provider,
timer and recovery roots. B1 includes a `1..=N` generic-instantiation cohort;
downstream slopes remain non-forecast routing evidence. The temporary pool
Ledger recovery family is also measured, but hard deletion waits for both live
recoveries, conservation, official Root restoration and terminal zero-effect
replay. Completion requires at least 350 KiB code-section and 5% frozen replica-
validator function headroom. Build acceleration remains parallel support.

The former stateful-retirement and cross-release-adoption proposal is
cancelled and archived. Active 0.111 preserves no application data, stable
memory or Principal. Its B1 is held behind Q0 proof that a finalized one-shot
capsule can attach exact source cycles, obtain an atomic destination receipt,
recover response loss, retain execution slack and account for the final
discarded residual before source deletion.

Deferred scope remains adaptive pool/reset lanes, broad funding and transfer
batches, 1,000-canister qualification and a universal runtime observatory.

## Validation State

For the current B9 slice:

- frozen v2 remains unchanged; governed v3 definitions, scripts, catalog and
  fingerprints pass, and deterministic original/current mechanical reruns are
  retained in the linked report;
- `cargo fmt --all -- --check` and `git diff --check` pass;
- all 21 focused admission tests, 45 Registry-family tests, 10 draining tests
  and 5 removal tests pass; the top-level draining-fence, deletion-replay and
  atomic membership-removal regressions pass after extraction, as does the
  terminal Root/Store retirement restart-and-replay test; bounded deterministic
  subtree restart and terminal fence-release regressions also pass;
- exact Registry preparation and the initial-inventory seal, Directory flag,
  Root-runtime flag and idempotent activation receipt regressions pass;
- the Component Directory refresh selection, prepare, commit and intent-replay
  regression passes before and after the exact stable transition, including
  duplicate selection, stale timestamp and conflicting-intent rejection;
- the top-level and direct-child allocation/activation regressions pass
  reservation, creation, installation, Registry commit, runtime activation,
  membership and exact-retry boundaries after extraction;
- warning-denied control-plane Clippy and layering pass; focused scale-out restore preserves cross-document authority;
  retained Root acceptance, provisioning and coalesced Directory/runtime tests pass through the completed validation owner;
- the focused pool Ledger regression passes sequential terminal recovery and
  exact replay for two distinct assets while same-operation retargeting fails;
- the relocated Component Registry test owner recompiles; focused preparation/idempotency, Root/Store retirement,
  grouped lifecycle/install and peer-caller regressions pass through extracted owners; changed-package Clippy passes;
- focused Component deletion, subtree terminal fence-release and grouped-allocation regressions pass through the
  extracted retirement and Registry-response projection owners;
- dependency-light conservation and public generated-estate regressions pass no-effect planning, Principal rejection,
  Root observation, typed ordering and all permutations, including duplicate and anonymous cases;
  retained-input and pre-retention digest regressions also pass without reissuing an effect; and
- no broad workspace or PocketIC gate was run during coding.

Published-batch validation remains in the detailed changelog and immutable
release evidence; it is not duplicated here.

## Next Authorized Action

Freeze the completed B9 contraction as one immutable candidate, run the
canonical audit methods and maintainer gate, and obtain the human verdict. Do
not begin 0.110 before accepted 0.109 closeout.


<!-- canic-release-validation: version=0.109.32 source=641f843ac5bc1ddb823bef6b3c32427a5cca70dc date=2026-08-31 gate=complete -->
