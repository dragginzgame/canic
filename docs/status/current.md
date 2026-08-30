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

`v0.109.24` is the immutable maintained release. The annotated tag peels to
release commit `129a5c710778a5284b2163eacf93c255eaddb055`; workspace packages,
`main` and tracked `origin/main` agreed at publication. Its complete validation
marker binds source `8aaeef70b91d289628d11c641858344928e69efe`.

The former handoff incorrectly called 0.109.24 an unversioned draft and 0.109.23
the maintained release. That recurring release-evidence defect is downstream
`CANIC-014`. This handoff now treats structured version, tag, package and
validation records as release authority; narrative is a summary only.

No later version, release, tag, package publication or deployment is described
or authorized here.

## Current 0.109 Correctness Batch

Toko Miner adopted immutable 0.109.24, passed complete CI and finalized all
nine IC release artifacts. Its public no-effect generator correctly
management-verifies the deliberately stopped retained Root, performs no
protected Root query or desired replacement and returns one deterministic
same-ID Start prerequisite.

The following public Fleet Ensure plan then fails before producing a digest.
Toko's retained schema-1 state predates topology retention: it retains the
cycle map but has empty `principals` and `topology`. Ordinary planning tries to
observe Root-owned children before reaching the Root Start and rejects the
missing topology authority. Toko records this as blocking `CANIC-091`.

`0.109.25` is the open changelog draft for that correction. The current
`canic-host` implementation establishes this invariant:

1. management-observe the configured Root and require exact Principal, Subnet,
   controller, installed module and stopped state;
2. compile one content-addressed plan scoped only to the same-ID Root Start;
3. assign zero creation, replacement, reinstall, funding, transfer, fee or
   operator-debit authority;
4. perform no protected Root query, child observation or topology backfill;
5. apply only the exact reviewed digest and make replay effect-free; and
6. rerun generation after Start, requiring complete protected Fleet and pool
   authority before any ordinary plan.

The production-shaped retained-estate regression starts with exact retained
cycle balances and empty Principal/topology maps. It passes from management
observation through one Root Start, bounded measured burn, zero operator debit,
unchanged authority maps and effect-free terminal replay, then performs the
complete protected generator observation only after the Root is running.
Principal, Subnet, controller, module and stopping-state drift reject before a
plan or effect. Locked host and CLI compilation, that focused journey,
current-plan JSON round trips, changelog governance, formatting, diff hygiene,
the current-document semantics guard and warning-denied all-target host/CLI
Clippy pass. No broad workspace or PocketIC gate was run during coding.

## Safety State

The retained downstream evidence reports:

- default ICP identity restored to anonymous;
- desired Fleet authority byte-identical;
- finalized 0.109.24 release artifacts retained;
- no prerequisite plan digest or apply authority produced;
- no state/archive edit or synthetic topology authority;
- no canister, controller, cycle, database, catalogue or frontend mutation;
  and
- no checksum, optimizer, size or authority bypass.

Canic does not authorize or perform downstream effects from this repository.

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
- `CANIC-090`/`CANIC-091`: a prerequisite effect may short-circuit unavailable
  protected observation only under exact management authority and mandatory
  post-effect revalidation.
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

This roadmap amendment changes documentation, status paths and the
document-semantics guard's 0.110 path only. It does not qualify the concurrent
`CANIC-091` source changes. Targeted documentation checks are the appropriate
amendment boundary; code, workspace and PocketIC gates remain outside it.

The current-document semantics guard, Bash syntax, warning-level ShellCheck,
diff whitespace and stale-roadmap-path scan pass for the amendment. The
detailed [`CANIC-*` disposition](../design/0.110-fleet-runtime-contraction-and-stateful-safety/0.110-design.md#upstream-feedback-disposition)
records the retained, sequenced and deferred Toko requirements through
`CANIC-091`.

## Next Authorized Action

Review the current `CANIC-091` implementation batch without modifying Toko
Miner, then use the maintainer-selected release flow when explicitly directed.
After an immutable successor is published and adopted, repeat downstream
no-effect preparation through the exact Start, post-Start protected
verification, terminal plan and effect-free replay. Only then begin 0.109 B9
simplification and its superseding audit.


<!-- canic-release-validation: version=0.109.25 source=90329bde38fbafe72589359f9bdf4d1e43f5cb46 date=2026-08-30 gate=complete -->
