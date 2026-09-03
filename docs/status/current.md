# Current Status

Last updated: 2026-09-03

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

Current development begins from published `v0.110.4` at
`baffacab7d5ae4d93ddb9e5fac279bc4c3ddd177`. Its governed marker records the
validated source below; immutable details are in
[the 0.110 changelog](../changelog/0.110.md). `0.110.5` is the single open
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

Published `v0.110.0` through `v0.110.3` close the fresh-estate corrections:
applied Creates retain exact nonterminal identities, cycles and topology;
pool readiness includes separately reviewed observation/update burn; and only
controller-authenticated `InspectCanister` is admitted while a Root is
Prepared. The concrete `IcpEnsurePlatform` proof crosses lost Create and
controller-update responses, reconstructs the adapter, finishes one Workload
plus one Ready pool asset and replays without repeating an effect.

Published `v0.110.3` also hard-deletes the obsolete temporary pool-Ledger
helper, contracts offline Medic/state-audit CI output and aligns the PocketIC
test stack. Exact sealed Root-module authority remains mandatory. Full
correction and test details are retained in the 0.110 changelog.

## CANIC-125 Bounded Component-Provisioning Observation

GitHub issue 23 reports a retained Fleet Ensure operation with 65 of 66 effects
Applied and the final `ProvisionComponents` action Issued. The Coordinator was
still progressing, but eight immediate identical status queries exhausted the
generic unchanged-observation limit before the distributed operation could
reach its next durable phase.

Published `v0.110.4` keeps the command/status boundary intact:

- the command is still issued once, and only exact typed retryable-failure
  evidence may replay its retained operation identity;
- passive `ProvisionComponents` status observations use bounded exponential
  pacing from 250 milliseconds to five seconds;
- the unchanged-progress limit is raised only for that exact protocol action,
  using a retained topology-derived floor capped at 64 while honoring an
  explicitly reviewed larger configured limit;
- any durable phase, Root-count, Component-count or failure change resets the
  consecutive-stall budget; and
- a true stall remains typed, resumable and reports the action plus compact
  durable progress evidence and its full status digest.

Retained current-schema plans and journals require no rewrite. This is a host
reconciliation correction and does not add runtime capability or alter the
0.110 contraction design.

## CANIC-124 Managed Component-Tree Qualification

The open `0.110.5` batch adds one public host-only fixture for downstream tests
that must qualify a managed Hub together with children created through Canic's
placement workflows. `install_managed_component_group` consumes one validated
Component Group deployment and exact Wasms for all selected roles; Canic alone
derives each top-level `Component` and descendant `ComponentChild` authority.

The governed proof covers configured sharding and scaling children, on-demand
index and scale-out allocation, exact parent and Component Group bindings,
Fleet-admitted and denied direct child ingress, same-release child upgrade,
timer restoration and successor projection fencing. The same Root allocation
journal and fixture settlement path serves sharding, scaling and index; the
downstream never constructs protected child payloads or directly pins private
`canic-core`/`ic-testkit` lifecycle machinery.

## CANIC-126-CANIC-127 Convergence Corrections

The open `0.110.5` batch also closes the two production ordering defects exposed
after the final Fleet protocol action:

- a Root-owned pool asset whose exact balance is not yet available remains a
  bounded passive observation, not a failed effect. Fleet Ensure re-observes the
  same operation and topology without issuing a protocol, install, controller,
  creation or funding command. Exact progress clears the stall count; exhaustion
  reports the target and last authoritative lifecycle while leaving the operation
  resumable; and
- a managed Hub keeps readiness closed while its configured initial children are
  unavailable. The exact registered Hub may request only its compiled initial-
  child allocation while both it and the Root are Prepared. One durable Root
  allocation owns creation, installation, Directory convergence and membership;
  a detached idempotent driver prevents a Root-to-Hub callback cycle, and the Hub
  retries only typed transient bootstrap failures within a finite bound. If that
  bound is exhausted, an exact Root-authenticated runtime-configuration replay
  may reclaim the transient init failure without rerunning application init;
  non-retryable failures and active retry owners remain unchanged.

Root membership activation now requires the target's exact readiness response.
Runtime activation remains bound to the Directory authority under which it
occurred even when an initial child legitimately advances the current Directory
before Root records the activation response. The Root therefore cannot publish
an Active zero-descendant Hub whose required initial-child bootstrap failed, and
lost activation responses adopt only the exact already-active runtime receipt.

Component retirement keeps the committed runtime operation resolvable during
the exact validated `Draining` interval before a quiescence intent exists. This
allows the Root to converge the final member Directory while the Component is
still runnable; quiescence intent, a stopped receipt and removal all close that
runtime-operation path again.

The governed Prepared-Root journey reaches three top-level Components plus
configured sharding and scaling children, terminal Component membership and an
effect-free replay. A second governed literal-zero-estate journey now drives the
real Fleet Ensure plan and journal through the concrete `IcpEnsurePlatform`, an
actual lost controller response, fresh-process adapter reconstruction and the
real Coordinator/Root/Store protocol. It reaches one Workload plus one Ready
pool asset, proves cycle conservation and immediately replans and applies with
zero effects. The public fixture independently covers configured and on-demand
sharding, scaling and index children, direct admission, same-release upgrade,
timer restoration and fencing. A downstream live reset remains downstream-owned
adoption evidence rather than a Canic release effect.

## 0.110.5 Fleet Ensure Operator Corrections

The open patch distinguishes a retained Component-provisioning source Registry
from its published active successor during terminal inventory validation. Root
top-level status is bound to the Coordinator's retained source Registry, plan
hash and configuration digest; Root and Coordinator publication remain bound
to the active Registry. Fleet Ensure JSON also keeps Store chunk bytes in the
existing content-addressed object store and reports only their local path,
SHA-256 and byte size. Text-mode cycle quantities use consistent three-decimal
`B`, `T` and `Q` units.

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
| [0.110](../design/0.110-fleet-runtime-contraction/status.md) | zero-capability runtime contraction | `v0.110.4` published; `0.110.5` closes managed Component-tree qualification and its two observed convergence blockers while B1 remains active |
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
release flow owns that boundary. Published `v0.110.4` closes the independent
CANIC-121 and CANIC-125 corrections, so B1 measurement may proceed; B2 remains
blocked on accepted complete B1 evidence.

## Next Authorized Action

Complete the bounded `0.110.5` CANIC-124, CANIC-126 and CANIC-127 corrections,
then continue B1 from immutable `v0.110.4`: complete the frozen downstream
canary, controlled ablations, generated-surface inventory and generic-
instantiation cohort. Retain the active B1 evidence and do not begin B2 until
the maintainer accepts the complete B1 baseline.


<!-- canic-release-validation: version=0.110.4 source=27eb97333a2fb73bedbfde1e566d13ab055b339c date=2026-09-02 gate=complete -->
