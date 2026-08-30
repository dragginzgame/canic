# Current Status

Last updated: 2026-08-30

## Purpose

This is the compact handoff for the active Canic source and roadmap. Read this
file first, then follow only the linked design, audit or implementation owner
needed for the task.

Historical handoffs:

- [through 2026-06-30](archive/2026-06-30-precompact.md);
- [through 0.90.2](archive/2026-07-13-precompact.md);
- [through 0.101.52 Q4](archive/2026-08-12-precompact.md);
- [through published 0.109.12](archive/2026-08-26-pre-root-repair-hard-cut.md);
  and
- [pre-reorientation 0.109.24 handoff](archive/2026-08-30-pre-roadmap-reorientation.md).

## Published Release Truth

`v0.109.25` is the immutable maintained release. The annotated tag object
`3d5b9dde565fae333724ee8cd82f0278b40a57b5` peels to release commit
`da62ae03f7fb782914936f6124f7aeeeac8b77cc`; workspace packages, `main` and
tracked `origin/main` agree. Its complete validation marker binds source
`90329bde38fbafe72589359f9bdf4d1e43f5cb46`.

The former handoff incorrectly called 0.109.24 an unversioned draft and 0.109.23
the maintained release. That recurring release-evidence defect is downstream
`CANIC-014`. This handoff now treats structured version, tag, package and
validation records as release authority; narrative is a summary only.

`0.109.26` is an open changelog draft for the current source batch. It is not a
versioned workspace, tag, published package or deployment.

## Current 0.109 Correctness Batch

Published 0.109.25 closes `CANIC-091`: a retained schema-1 state with cycle
evidence but no Principal/topology maps can review and apply one exact
management-bound Root Start before protected child observation.

Downstream review then exposed `CANIC-092`. The retained stopped Root runs
module A while the current desired release binds successor module B. The
0.109.25 prerequisite required A to equal B, and its regression accidentally
hid the defect by changing desired state to A.

The current `canic-host` batch establishes this corrected invariant:

1. management-observe A with exact Root Principal, Subnet, controllers and
   stopped state;
2. atomically retain one typed generator authority binding A to the exact
   current Fleet ID, release build and desired successor B;
3. embed that authority into a content-addressed plan scoped only to same-ID
   Root Start, with zero install, replacement, creation, funding, transfer,
   fee or operator-debit authority;
4. reject missing authority, changed authority and arbitrary module C before a
   plan or effect;
5. apply the Start once and make lost-response/terminal replay effect-free;
   and
6. leave desired B unchanged, then require generation and ordinary protected
   convergence to be reviewed again after the Root runs.

The production-shaped regression now keeps distinct predecessor and successor
Wasm bytes. It proves deterministic authority retention, one zero-debit Start,
unchanged state ownership and effect-free replay; Principal, Subnet,
controller, runtime, missing-authority and arbitrary-module drift reject.
Targeted validation evidence is recorded below; no broad workspace or
PocketIC gate is run during coding.

## Safety State

The retained downstream evidence reports:

- default ICP identity restored to anonymous;
- desired Fleet authority byte-identical;
- finalized 0.109.25 release artifacts retained;
- no prerequisite plan digest or apply authority produced;
- no downstream state/archive edit or synthetic topology authority;
- no canister, controller, cycle, database, catalogue or frontend mutation;
  and
- no checksum, optimizer, size or authority bypass.

Canic does not authorize or perform downstream effects from this repository.
The Canic regression creates the new digest-bound Root-start authority only in
its disposable fixture directory.

## 0.109 Closeout State

0.109 remains open. Functional Fleet-wide admission is retained, but the
[binding post-implementation complexity audit](../audits/release-lines/0.109-post-implementation-complexity-audit.md)
still has `closeout_verdict: fail` and no accepted immutable superseding pass.

Required order:

1. finish and publish the current Canic-owned correctness batch through the
   maintainer-selected release flow;
2. adopt that exact release downstream and complete CI, release preparation,
   reviewed no-effect planning and terminal/effect-free replay evidence;
3. close B8 with the maintained public operator/adoption loop;
4. execute B9 pure simplification: localize decisions, decompose the three
   gravity wells, retain this handoff below 250 lines and freeze a bounded
   PocketIC time/RSS/process/case envelope;
5. reconcile and complete B10's already-published managed-App qualification
   surface without adding runtime authority;
6. rerun the canonical methods on one immutable candidate; and
7. obtain the maintainer-requested and accepted 0.109 closeout verdict.

No 0.110 implementation begins before that human-owned closeout.

## Reoriented Roadmap

Toko Miner is the primary read-only real-application steering source. Canic
retains repository-owned fixtures and never gains a Toko or IcyDB production
dependency.

| Line | Accepted purpose | State |
| --- | --- | --- |
| [0.110](../design/0.110-fleet-runtime-contraction-and-stateful-safety/status.md) | Contract release builds, endpoint/runtime code, control-plane/operator paths and validation; then add stateful-retirement safety | Accepted reorientation; blocked on 0.109 closeout and promotion |
| [0.111](../design/0.111-stateful-fleet-release-adoption/status.md) | One exact whole-Fleet stop-the-world predecessor/successor transition under inherited budgets | Accepted; blocked on 0.110 closeout and promotion |
| [0.112](../design/0.112-bounded-multi-fleet-estates/status.md) | Indexed estates, an ordinary reserve Fleet and one same-Subnet single-asset cross-Fleet transfer | Accepted reorientation; blocked on 0.111 closeout and promotion |
| [Fleet Observatory](../design/ideas/fleet-observatory/status.md) | Host/downstream-first passive observation without an assumed every-role runtime protocol | Deferred unnumbered idea |

### 0.110 Steering Facts

- `CANIC-014`: release truth must be structured; handoff prose is not an
  independent publication authority.
- `CANIC-087`: eliminate release-LTO declaration links and serial compatible
  runtime links while preserving canonical artifact/determinism gates.
- `CANIC-090`/`CANIC-091`/`CANIC-092`: a prerequisite effect may short-circuit
  unavailable protected observation only under exact management and retained
  module authority plus mandatory post-effect revalidation.
- Endpoint-heavy Toko evidence: Binaryen has converged; shared non-generic
  wrappers and role pruning must supply at least 350 KiB useful current-profile
  code-section headroom, with 500 KiB preferred.

0.110 does not inherit unresolved 0.109 work. It makes the accepted reductions
durable budgets and adds only stateful retirement as a new safety capability.

### Deferred Scope

Adaptive creation/reset lanes, transfer batches, broad automatic estate
funding and 1,000-canister qualification are unscheduled. The former 0.112
runtime Observatory is deferred because a new cross-role projection plus
HTML/JSON adapters on every role conflicts with current size and complexity
evidence.

## Validation State

Targeted `canic-host` library checking and warning-denied all-target Clippy,
`canic-cli` all-target checking, the production-shaped retained-estate
generator/ensure journey, current-plan JSON/content-addressing tests, layering,
formatting, diff hygiene, changelog governance, release-draft preflight and the
current-document semantics guard pass. The evidence intentionally excludes a
broad workspace or PocketIC gate during coding.

`CANIC-034` is already closed by the maintained fresh-estate creation graph:
each Root pool asset is funded directly by its reviewed Cycles Ledger creation
action with exact creation and Ledger fees, so no Root-ledger bootstrap or
parallel funding authority is needed. `CANIC-087` remains sequenced to 0.110
B2 and is not pulled across the human closeout gate.

## Next Authorized Action

Finish targeted review of the current `CANIC-092` batch without modifying Toko
Miner. When the maintainer selects a release flow, publish and adopt the exact
successor, then repeat downstream no-effect generation through the reviewed
Start, post-Start protected verification, terminal plan and effect-free
replay. Only then begin 0.109 B9 simplification and its superseding audit.


<!-- canic-release-validation: version=0.109.25 source=90329bde38fbafe72589359f9bdf4d1e43f5cb46 date=2026-08-30 gate=complete -->
