# Current Status

Last updated: 2026-09-02

## Purpose

This is the compact handoff for active Canic source and roadmap work. Read it
first, then open only the linked design, changelog or audit owner needed for the
task.

Historical handoffs:

- [through 2026-06-30](archive/2026-06-30-precompact.md);
- [through 0.90.2](archive/2026-07-13-precompact.md);
- [through 0.101.52 Q4](archive/2026-08-12-precompact.md);
- [through published 0.109.12](archive/2026-08-26-pre-root-repair-hard-cut.md);
  and
- [pre-reorientation 0.109.24](archive/2026-08-30-pre-roadmap-reorientation.md).

## Release Evidence Contract

Release truth comes from workspace package versions, the root and detailed
changelogs, the annotated Git tag and release commit, the complete published
package set, and the governed validation marker at the end of this file. The
version transaction owns that marker; explanatory prose is not a second release
guard.

Current development begins from published `v0.110.2` at
`f9009d5ae7be78d4f9dd746431584368770e8364`. Its governed marker records the
validated source below; immutable details are in
[the 0.110 changelog](../changelog/0.110.md). `0.110.3` is the single open
patch draft. Source-development truth comes from Git and the working tree.

## Maintained 0.109 Contract

Fleet admission retains one Coordinator-owned canonical policy, one Root-owned
distribution operation per Root, and one exact local projection on each
enrolled non-Root target. `caller::is_fleet_admitted()` and
`canic::fleet_admission::require_caller()` read that same projection and the
observed transport caller. Admission never replaces application membership,
resource ownership, service authority or infrastructure authority.

Fleet Ensure remains the sole desired-state reconciliation owner. Paid or
identity-changing effects require exact reviewed authority, durable intent,
bounded debit and lost-response reconciliation. Terminal convergence proves
cycle conservation and immediate replay is effect-free. Historical install,
repair, migration and recovery compatibility is not restored.

Published `v0.109.34` owns the complete `CANIC-102` and `CANIC-112` through
`CANIC-117` corrections: terminal Create balances, exact available/required/
shortfall guidance, bounded first-observation burn, active Registry-operation
binding, retained terminal Component inventory, issued withdrawal evidence,
bootstrap import capacity, role-specific command entrypoints and the managed
cross-release runtime fence. Exact tests and negative cases are retained in the
detailed changelog.

## Accepted 0.109 Closeout

Published `v0.109.35` closes `CANIC-118`. ICP CLI 1.3.0 returns
only public identity/controller/module fields when the operator is not a newly
created Root-owned pool's controller; those fields cannot prove its live cycle
balance.

The accepted correction keeps one executable authority sequence:

- a new fresh-estate plan creates each Root-only pool with the exact operator
  as a temporary direct controller, observes its real balance, installs the
  Root, then removes the operator before protocol convergence;
- an immutable 0.109.34 plan is not rewritten: its issued Create retains the
  exact Ledger receipt and Principal, defers only the unavailable balance
  observation, advances its reviewed infrastructure prerequisites, then uses
  the installed Root's protected inspection;
- the inspecting Root must match the retained Principal, exact desired
  controllers, successor module and running state;
- the target must retain the exact Root-only controller set, module-free pool
  shape and live native balance; and
- public status never supplies inferred cycles, controllers or runtime state.

The deferral is restricted to an issued fresh Root-only pool Create with exact
retained identity and a later Root install in an infrastructure-only plan. It
cannot authorize protocol work, duplicate creation, funding or a different
effect. The global stalled-observation budget resets only on genuine progress.

Primary owners:

- `crates/canic-host/src/icp/model.rs` — typed full/public ICP status shapes;
- `crates/canic-host/src/fleet_ensure/policy/mod.rs` — temporary-controller plan;
- `crates/canic-host/src/fleet_ensure/ops/platform.rs` — exact observations;
- `crates/canic-host/src/fleet_ensure/workflow/mod.rs` — bounded deferral; and
- `crates/canic-host/src/fleet_ensure/generate/tests.rs` — production-shaped
  fresh and immutable-plan replay evidence.

## CANIC-119-CANIC-123 First 0.110 Release Corrections

Read-only Toko Miner qualification after the accepted closeout exposed one
fresh-estate ordering defect in published `v0.109.35`. Five exact Creates and
the Coordinator, Store and Root installs were Applied, but the first temporary
pool-controller finalization could not observe its pre-effect cycles through
the installed Root. The applied identities remained under
`pending_principals`, while exact cycles and topology were published only after
every action, including both finalizations, had completed.

The targeted-complete source correction keeps pending identities nonterminal.
After a Create is durably journaled Applied, workflow separately retains its
exact observed cycles and sealed desired topology. Resume reconstructs that
authority from the unchanged plan, journal and state before later actions. The
production Root observer accepts a pending Principal only when the pending and
terminal identity slots do not conflict and the retained child kind/parent
topology is exact. Issued Creates still cannot cross the boundary.

`CANIC-120` additionally separates a pool's configured readiness floor from
its creation funding. Fresh generation adds the exact 1T observation and 100B
controller-effect ceilings, so the downstream-shaped 1.9T floor produces a 3T
Create. Planning rejects an insufficient contract before effects with typed
requested/floor/burn/required/shortfall fields. Resume rejects the preserved
1,899,998,056,000-cycle applied result before controller or protocol work and
reports its exact 1,944,000 readiness shortfall.

The first `CANIC-121` journey created the Coordinator, Root, Store and two pool
assets but bypassed the installed Root's Prepared endpoint and observed
controllers directly through PocketIC. Read-only downstream qualification of
published `v0.110.1` therefore exposed `CANIC-122`: production correctly keeps
the Root `Prepared` at that boundary, while the Root endpoint incorrectly
rejected its controller-authenticated, read-only `InspectCanister` command with
`LIFECYCLE_INACTIVE`.

The `0.110.2` correction admits only `InspectCanister` to the Prepared Root
command set. Its host Fleet Ensure proof starts with no canisters, runs the
reviewed plan and durable journal through real PocketIC management effects,
loses the first Create response, reconstructs the host adapter, resumes the
same operation and proves terminal conservation plus immediate zero-effect
replay. The open `0.110.3` CANIC-121 completion closes the remaining seam in
the control-plane journey: the concrete `IcpEnsurePlatform` and an isolated
real ICP CLI identity now finalise both pool controller sets through the
Prepared Root, lose one successful update response, reconstruct the adapter,
adopt the exact live result and prove replay does not repeat either effect.
That production path also proved that the prior 100B update-burn ceiling could
leave a freshly created pool asset below its 1.9T Ready floor after controller
finalisation and Root reset. The generated current contract now reviews 1T for
first observation plus 1T for updates, producing a bounded 3.9T Create. The
same PocketIC estate then imports/resets both assets, provisions one Component
and finishes with exactly one Workload plus one Ready asset.

Published `v0.110.2` included `CANIC-123` for the then-supported temporary
pool-Ledger recovery canister. The open `0.110.3` current contract hard-deletes
that obsolete feature rather than retaining a generated helper graph: its
artifact/build role, Store publication, Root command/status/stable state, Fleet
Ensure action, fixtures and dedicated CI owner are gone together. Canic funds
pool assets through native canister creation or top-up and does not generate a
plain Ledger-account transfer to a pool Principal.

Published `v0.110.2` contains the fresh-estate, Prepared-Root and deterministic-
lock corrections. The open `0.110.3` batch supplies the final concrete-adapter
CANIC-121 proof and the temporary-helper hard cut rather than reopening the
accepted 0.109 line. No downstream or live-network effect is part of this
source correction.

## B1-B10 State

| Batch | State | Current evidence owner |
| --- | --- | --- |
| B1 | Accepted | 0.109 design baseline |
| B2-B7 | Complete | design/status tracker and governed admission suites |
| B8 | Complete | published CANIC-118 correction and downstream evidence |
| B9 | Complete | accepted immutable superseding audit |
| B10 | Complete | published host-only facade and downstream adoption report |

The immutable
[B9 superseding audit](../audits/reports/2026-08/2026-08-31/0.109-b9-superseding-complexity-audit.md)
reports `closeout_verdict: pass` on `v0.109.32`; the human maintainer accepted
it on 2026-08-31. The three control-plane parents remain 6,303, 5,838 and 2,688
lines. Canonical complexity and change-friction remain 8/10 and 7/10 pressure,
routed to blocked 0.110 rather than a second 0.109 authority. This active
handoff remains below the accepted 250-physical-line ceiling.

Published `v0.109.33` completed the host-only `canic::testing` facade, isolated
packaged consumer and managed plus standalone-local lifecycle proof. Read-only
downstream evidence confirms adoption of that facade, removal of the private
payload adapter and direct `canic-core`/`ic-testkit` test dependencies, and a
passing exact managed-Wasm lifecycle. The
[B10 reconciliation](../audits/reports/2026-08/2026-08-31/0.109-b10-managed-app-qualification-reconciliation.md)
records the boundary.

The human-requested closeout audit against `v0.109.34` is retained at
[the canonical report](../audits/release-lines/0.109-closeout-audit.md). Its
CANIC-118, active-handoff and documentation blockers are correction inputs, not
an accepted verdict for that older candidate. Published `v0.109.35` corrected
those blockers, and the human maintainer accepted the 0.109 closeout on
2026-09-01 before explicitly promoting 0.110 B1.

## Roadmap Boundary

Toko Miner remains a read-only steering source. Canic gains no downstream
runtime or repository dependency.

| Line | Active owner | State |
| --- | --- | --- |
| [0.109](../design/0.109-fleet-wide-ingress-admission/status.md) | admission, Ensure and managed-App support | accepted and closed at `v0.109.35` |
| [0.110](../design/0.110-fleet-runtime-contraction/status.md) | zero-capability runtime contraction | `v0.110.2` published; `0.110.3` closes the CANIC-121 qualification seam before B1 resumes |
| [0.111](../design/0.111-bounded-multi-fleet-estates/status.md) | bounded cycle-safe multi-Fleet estates | blocked on 0.110 and Q0 capsule proof |

The cancelled stateful-adoption proposal remains archived. Pre-1.0 release
transitions are reinstall-only; cycle conservation is the sole cross-release
compatibility invariant. Same-release interruption recovery, idempotency,
backup, restore, authority and cycle-safe retirement remain mandatory.

## Active 0.110 B1

The accepted first batch freezes the post-0.109 artifact, tool and capability
baseline before any runtime contraction. Initial work:

- freezes `v0.109.35` (`3185dc45b`) as the Canic predecessor;
- confirms dated IC limits of 10 MiB code section, 100 MiB total module and
  50,000 declared functions from the authoritative IC documentation;
- promotes `CANIC-WASM-001/v5` so path-confined staged release artifacts are
  measured from one role-local build log;
- retains the corrected deterministic nine-role baseline, whose largest role
  has 3,776,168 code-section bytes and 40,322 declared functions of absolute
  headroom; and
- keeps the exact downstream canary read-only and separate from Canic source.

No broad workspace or full PocketIC gate is run during coding. The maintainer's
release flow owns that boundary. The independent `0.110.3` CANIC-121 proof must
land before this measurement work resumes; it does not promote B2.

## Next Authorized Action

Complete and publish the bounded `0.110.3` CANIC-121 production-adapter proof,
then resume B1 from immutable `v0.110.2`: complete the frozen downstream canary,
controlled ablations, generated-surface inventory and generic-instantiation
cohort. Retain the active B1 evidence and do not begin B2 until the maintainer
accepts the complete B1 baseline.





<!-- canic-release-validation: version=0.110.2 source=6fc23b9a1d83a37af4c23ae6b3606365e7d2db76 date=2026-09-02 gate=complete -->
