# Current Status

Last updated: 2026-09-01

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

Current development begins from published `v0.109.34`. Its immutable details
are in [the 0.109 changelog](../changelog/0.109.md). Source-development truth
comes from Git and the working tree.

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

## Open 0.109.35 Correction

`CANIC-118` is the only open Canic-owned 0.109 blocker. ICP CLI 1.3.0 returns
only public identity/controller/module fields when the operator is not a newly
created Root-owned pool's controller; those fields cannot prove its live cycle
balance.

The correction keeps one executable authority sequence:

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

## B1-B10 State

| Batch | State | Current evidence owner |
| --- | --- | --- |
| B1 | Accepted | 0.109 design baseline |
| B2-B7 | Complete | design/status tracker and governed admission suites |
| B8 | Active | CANIC-118 correction and downstream fresh-Fleet proof |
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
an accepted closeout verdict.

## Roadmap Boundary

Toko Miner remains a read-only steering source. Canic gains no downstream
runtime or repository dependency.

| Line | Active owner | State |
| --- | --- | --- |
| [0.109](../design/0.109-fleet-wide-ingress-admission/status.md) | admission, Ensure and managed-App support | CANIC-118 correction before a fresh closeout audit |
| [0.110](../design/0.110-fleet-runtime-contraction/status.md) | zero-capability runtime contraction | blocked on accepted 0.109 closeout |
| [0.111](../design/0.111-bounded-multi-fleet-estates/status.md) | bounded cycle-safe multi-Fleet estates | blocked on 0.110 and Q0 capsule proof |

The cancelled stateful-adoption proposal remains archived. Pre-1.0 release
transitions are reinstall-only; cycle conservation is the sole cross-release
compatibility invariant. Same-release interruption recovery, idempotency,
backup, restore, authority and cycle-safe retirement remain mandatory.

## Targeted Validation State

Development checks for the open correction currently pass:

- public ICP 1.3.0 non-controller status decodes as typed unavailable evidence;
- fresh plans add temporary authority only to exact Root-only pools and order
  final removal after Root installation;
- the immutable Root-only plan retains Create identity across lost response,
  performs one mutation per pool, converges and immediately replays with zero
  effects;
- exact funding-inspection controller and module predicates pass;
- `canic-host` locked all-target checking and warning-denied Clippy pass;
- backup lifecycle mapping, `canic-cli` locked all-target checking and
  warning-denied Clippy pass; and
- formatting and diff hygiene pass after the final source review.

No broad workspace or full PocketIC gate is run during coding. The maintainer's
release flow owns that boundary.

## Next Authorized Action

Run the maintainer-owned release validation and publication workflow for the
targeted-complete CANIC-118 batch, then qualify the downstream fresh Fleet
against that immutable tag. After that, request a fresh human-owned 0.109
closeout audit. Do not begin 0.110 before the maintainer accepts that verdict.

<!-- canic-release-validation: version=0.109.34 source=2e48628b1127568a47430e02e15791006ea5d2a0 date=2026-09-01 gate=complete -->
