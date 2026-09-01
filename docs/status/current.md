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

Current development begins from published `v0.109.32`. Its immutable details
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
  and closed the retained predecessor-E132 replan boundary;
- `0.109.30` removed generic migration authority and reset current Canic-owned
  product schemas to `v1`;
- `0.109.31` published fresh-estate, Principal-canonicalization, pool-recovery
  and first B9 splits; and
- `0.109.32` publishes the initial `CANIC-102` through `CANIC-111` corrections,
  including creation balances, Store controllers, Registry retry, bounded
  terminal proof, reinstall fencing, post-bootstrap helper staging, manifest-
  bound Candid, activation revision, cycle reconciliation and timer inventory.

`CANIC-102` is reopened in the current 0.109.33 draft. The published workflow
retained the exact requested balance from both successful and duplicate-with-
Principal Create responses, but its next observation replaced that evidence
with `None` because production cannot resolve `action_cycles(Create)`. The
current correction makes observation merging monotonic. One affected applied
0.109.32 record may replay only the same operation's exact idempotent Ledger
request; operation, plan/action hashes, state-bound Principal, retained receipt
and returned requested balance must all match. It adds no inferred value,
migration, compatibility path, fallback or schema generation.

The open 0.109.33 candidate also hard-cuts the two infrastructure command
exports to `canic_root_command(RootCommand)` and
`canic_coordinator_command(CoordinatorCommand)`. Managed application and Wasm
Store command traffic remains on `canic_command`; Root and Coordinator retain
no old-name alias, fallback or mixed endpoint contract.

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

## 0.109 B9 Complexity Contraction

The immutable
[superseding B9 audit](../audits/reports/2026-08/2026-08-31/0.109-b9-superseding-complexity-audit.md)
reports `closeout_verdict: pass` on the exact `v0.109.32` source. The human
maintainer accepted it on 2026-08-31. It supersedes the binding preliminary
`fail` without erasing that historical gate.

The three measured control-plane parents contract from 17,452 to 6,303 lines,
8,751 to 5,838 lines and 8,997 to 2,688 lines along existing authority and
lifecycle seams. One Coordinator record, one Component Registry store family,
one admission policy, one timer owner and the existing effect owners remain.
No extraction adds a DTO, Candid variant, endpoint, stable record, journal,
policy decision or effect path.

The canonical v3 runs are deterministic. Their 8/10 complexity and 7/10
change-friction scores remain truthful inherited pressure rather than product
defects; blocked 0.110 owns the broader runtime graph. Structure, duplication,
layering and Tier-2 module-surface reviews pass. The 63-case serial PocketIC
inventory retains a 2,100-second, 6-GiB and 300-thread ceiling at capacity one.

No broad gate was rerun for this audit. The governed release marker binds a
complete gate to the exact source. Its local resource transcript was not
retained separately, so the audit labels the nearest 1,718-second,
5,037,288-kB and 257-thread run as capacity evidence rather than as the exact
0.109.32 transcript.

## B10 And Closeout

B9 is accepted. The current candidate completes the Canic-owned B10 facade,
isolated packaged consumer and managed plus standalone-local lifecycle proof
without adding runtime authority. The exact read-only downstream still owns
its private init/activation adapter and direct `canic-core`/`ic-testkit` test
dependencies, so B10 is pending that separately owned adoption and removal.
The [reconciliation report](../audits/reports/2026-08/2026-08-31/0.109-b10-managed-app-qualification-reconciliation.md)
records the exact boundary. The final 0.109 closeout remains human-owned.

Required order:

1. publish the current Canic B10 candidate through the maintainer-owned release
   workflow;
2. let the downstream adopt that immutable package, remove its private adapter
   and direct test pins, and retain its exact managed/standalone qualification;
3. request the human closeout audit against the complete evidence; and
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
replay. The exact frozen Toko `project_instance` is now a binding read-only
B1/B5 canary with an explicit authentication/blob/IcyDB/lifecycle/status/
recovery capability matrix. Completion requires 5% code-section headroom
(512 KiB under the retained 10 MiB limit) and 5% frozen replica-validator
function headroom in both the canonical worst case and that canary; 1 MiB code
headroom is preferred. B4 cannot stop while known role-inapplicable generated
machinery remains. A canary deficit after safe Canic cuts produces an explicit
Toko/IcyDB residual handoff rather than downstream mutation. Build acceleration
remains parallel support.

The former stateful-retirement and cross-release-adoption proposal is
cancelled and archived. Active 0.111 preserves no application data, stable
memory or Principal. Its B1 is held behind Q0 proof that a finalized one-shot
capsule can attach exact source cycles, obtain an atomic destination receipt,
recover response loss, retain execution slack and account for the final
discarded residual before source deletion.

Deferred scope remains adaptive pool/reset lanes, broad funding and transfer
batches, 1,000-canister qualification and a universal runtime observatory.

## Validation State

For the current B9, B10 and reopened `CANIC-102` slices:

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
- the B10 isolated packaged consumer compiles through packaged `canic` alone,
  including managed construction and standalone same-release upgrade; generated
  and canonical Store probes report 2,988,974-byte and 2,986,771-byte code
  sections;
- the B10 governed lifecycle case passes managed composed-IcyDB fencing,
  activation, admission, same-release upgrade and successor fencing plus
  standalone-local install, public call, same-release upgrade and replay in
  136.27 seconds;
- warning-denied Clippy passes for `canic` with `testing`, `canic-host` and
  `canic-testing-internal`;
- the canonical Fleet Coordinator artifact build resolves its checked-in DID,
  derives the exact profile and completes the sidecar-only optimized Wasm
  without runtime Candid extraction; the governed serial runner now preflights
  this shared artifact once before its scenario loop; and
- focused reopened `CANIC-102` regressions pass successful and duplicate-with-
  Principal Create journalling, terminal and zero-effect replay; exact retained
  0.109.32 balance recovery; six authority-mismatch rejections; and the existing
  interruption-at-every-effect and logical-controller Create journeys; the
  production response mapper also retains the exact Principal, receipt and
  requested balance for both Ledger response forms;
- all 41 focused protocol-surface tests pass with distinct Root and Coordinator
  command exports; the replay-policy, sealed-authority and prepared-Root
  policy tests pass, and all directly affected packages compile across all
  targets;
- no broad workspace or full PocketIC gate was run during coding.

Published-batch validation remains in the detailed changelog and immutable
release evidence; it is not duplicated here.

## Next Authorized Action

Hand the targeted-ready 0.109.33 draft to the maintainer-owned validation and
publication workflow. Downstream adapter adoption follows the immutable
package; the final human-owned 0.109 closeout audit follows that evidence. Do
not begin 0.110 before the accepted closeout.


<!-- canic-release-validation: version=0.109.32 source=641f843ac5bc1ddb823bef6b3c32427a5cca70dc date=2026-08-31 gate=complete -->
