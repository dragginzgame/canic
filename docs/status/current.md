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

Current development begins from published `v0.109.30`. Its immutable details
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
  maintained Canic-owned product schemas to `v1`.

Current source corrects downstream `CANIC-099`: the initial plan for an
explicitly fresh estate may omit management observation only for an
unallocated Root named by the exact bootstrap authority and only when no Root
management target is under review. Retained desired Roots still require a
Principal, an explicitly targeted Root cannot use the exception, and a fresh
Root returns to ordinary management observation as soon as creation retains
its Principal.

Current source also corrects `CANIC-101`: human-authored Fleet input rejects
anonymous and duplicate decoded identities, then sorts once by `Principal::Ord`.
The strict runtime compiler gives every permutation one digest-bound authority.

Current source also lets the temporary pool Ledger recovery terminal slot
rotate for a distinct later operation while exact replay and same-operation
retargeting remain fenced. The feature stays present until the two-pool live
recovery and terminal Fleet replay are immutably proven.

The detailed changelog owns the complete tests and negative cases for those
batches. No downstream repository or live IC state is owned by this handoff.

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

The corrected current measurement confirms the direct source pressure used to
select the first seams:

| Owner | Direct lines | Disposition |
| --- | ---: | --- |
| `ops/component_registry/mod.rs` | 17,452 | pending allocation/directory/retirement decomposition |
| `workflow/component_registry/mod.rs` | 8,751 | pending lifecycle-driver decomposition |
| `ops/fleet_coordinator/mod.rs` | 8,997 | admission, Registry history and Root lifecycle authority extracted first |

The first production simplification keeps the Coordinator record as the sole
durable owner while separating three existing responsibilities:

| Coordinator owner | Lines | Exact responsibility |
| --- | ---: | --- |
| parent `mod.rs` | 7,493 | single-step Coordinator state operations and remaining provisioning logic |
| `admission.rs` | 304 | admission publication, capacity, canonical mutation and replay |
| `registry_history.rs` | 924 | ordered canonical Registry reconstruction and validation |
| `root_lifecycle.rs` | 361 | Root draining/removal reservation and publication authority |

No DTO, Candid, digest, stable record, endpoint or effect ordering changes.
The split removes 1,504 direct lines from the former 8,997-line parent, but the
remaining parent and the two Component Registry gravity wells still require
material decomposition before the finding can pass.

The Component Registry now has focused owners for Root/Store retirement (508
lines), Directory refresh (326), top-level allocation/activation (397/477),
direct-child allocation/activation (518/379) and top-level Component retirement
(618). A 1,143-line `subtree_retirement` owner now retains bounded traversal,
stop/delete, Directory synchronization and membership-removal evidence for
descendants. A 311-line `initial_inventory` owner seals active membership and
monotonically retains Directory and Root-runtime convergence. Every owner uses
the same Registry store and `ComponentRegistryOps`; workflow, transport,
Candid, stable schemas and effect ordering are unchanged. The parent is now
13,158 lines, down from 17,452. Remaining provisioning and validation-helper
families still require material separation.
Remaining B9 work:

1. materially narrow all three named gravity-well responsibilities;
2. localize remaining admission decisions and retain one authority owner;
3. separate dependency-light pure recovery-plan validation from IC/PocketIC drivers;
4. retain this handoff below 250 lines;
5. record bounded PocketIC elapsed-time, RSS, process/thread and case-count evidence;
6. preserve retained-decision diagnostics and retriage the 0.110 scope; and
7. complete the manual v3 complexity/change-friction attribution plus the
   structure and duplication methods on one immutable candidate, then obtain
   an accepted superseding verdict.
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
- warning-denied `canic-control-plane` library/test Clippy and the layering
  guard pass;
- the focused pool Ledger regression passes sequential terminal recovery and
  exact replay for two distinct assets while same-operation retargeting fails;
- the targeted public generated-estate host regression passes fresh no-effect
  planning, the retained/targeted missing-Principal rejection cases and
  post-allocation Root management observation; the same production-shaped
  journey accepts noncanonical display-text ordering and emits exact typed
  Principal order, while all six permutations, duplicate and anonymous cases
  pass their focused regression; and
- no broad workspace or PocketIC gate was run during coding.

Published-batch validation remains in the detailed changelog and immutable
release evidence; it is not duplicated here.

## Next Authorized Action

Continue B9 with the next measured control-plane ownership split. Keep the
changes behavior-preserving, run only targeted changed-package checks during
coding, and do not begin 0.110 before accepted 0.109 closeout.


<!-- canic-release-validation: version=0.109.31 source=3f4a047f2451ef76373350ce7215f3fd1ac96be2 date=2026-08-31 gate=complete -->
