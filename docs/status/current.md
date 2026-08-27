# Current Status

Last updated: 2026-08-27

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the active source, validation, design, or changelog material needed for the
current task.

Historical handoffs: [through 2026-06-30](archive/2026-06-30-precompact.md),
[through 0.90.2](archive/2026-07-13-precompact.md),
[through 0.101.52 Q4](archive/2026-08-12-precompact.md), and
[through published 0.109.12](archive/2026-08-26-pre-root-repair-hard-cut.md).

Published `v0.109.12` at
`513d628ff4fb3ba6882ae3db32be8bcf84dbe1b8` is the immutable predecessor of
the open `0.109.13` hard cut.

## Current Decision

CANIC-059 is now the maintained host architecture. `canic fleet ensure
<fleet>` is the sole Fleet installation/convergence workflow. Its plan-only
form observes the current configured estate and retains one immutable reviewed
plan. `--apply <plan_sha256>` advances only that plan. Fresh install,
deployment-plan, historical-plan, retained-recovery, retained-Root repair,
recovery-bundle, installed-Fleet cache, adoption, and autonomous Root-deletion
owners are removed rather than adapted.

The current contract is schema `v1` and reads no historical operator evidence.
Its only local authorities are the current desired Fleet document and
`.canic/fleet-ensure/<environment>/<fleet>/` current-generation plan, journal,
identity map, and operation lock. Unknown schemas, changed desired/artifact
digests, changed authority, and unreviewed balance drift fail closed.

Every mutating action has a retained intent before the platform call. Lost
responses are resolved by exact live-state reconciliation or an idempotent
Ledger/drain operation identity before retry. The stall count is consecutive
and resets only on durable progress. A terminal invocation completes in the
same call; its immediate successor plan has no mutation actions when the live
estate already equals desired state.

## Cycle-Safety Boundary

The plan reports the complete observed controlled balance, retained balance,
scheduled transfers, maximum fees, bounded observation/update burn, maximum
new funding, maximum operator debit, every canister disposition, and the
reviewed post-operation conservation equation. Apply refuses a changed plan,
insufficient operator balance, or a debit/burn above that bound. Terminal
evidence records:

```text
observed starting cycles
+ received new funding
- measured execution/observation burn
= final controlled cycles
```

Controllers cannot pull cycles out of an arbitrary IC canister. A canister
with a material balance may therefore be replaced or deleted only when its
current desired entry supplies an exact treasury-bound, idempotent drain
method and Candid contract. Otherwise planning returns `NoSafeDrain` and leaves
the canister untouched. A stopped-state and residual-balance check immediately
precedes deletion. Creation charges, Ledger fees, requested initial funding,
and execution/observation burn remain separate quantities.

An accepted drain response is not conservation proof. The journal retains
source and treasury balances from before the call and requires a fresh bounded
source debit plus the exact controlled-treasury credit before stop or delete.
Similarly, a successful control-plane update marks only `issued`; later work
remains fenced until the exact status query proves terminal application.

The current host journal prevents duplicate effects across interrupted or
repeated invocations sharing the operator-state root. Ledger create/withdraw
and configured drain effects additionally use exact replay identities. A
globally distributed lock across independent operator-state roots is not yet
provided; do not run concurrent apply commands from different Canic state
roots.

## Current Completion State

The current candidate converges canister existence, code, controllers, running
state and cycles. It now compiles one ordered current protocol graph from exact
role authority: Store release-set and artifact staging, Root Store adoption and
bootstrap, deterministic Coordinator Registry joins, Root synchronization,
Registry activation, Root-mirror activation, exact local Component Registry
preparation, then Component provisioning and Fleet-catalog publication. Every
response remains issued until its exact protected status proves terminal state.
Store adoption retains the protected operator plus owning Root as its one
direct controller set; the obsolete temporary/final controller transition is
removed.

The governed production five-Component PocketIC journey now begins from a
fresh estate and traverses that complete typed graph through terminal catalog
publication. It then recompiles the exact live successor Registry against the
retained Component operation receipt and proves an immediate second run has no
nonterminal action or update effect. The control-plane convergence evidence gap
is closed without restoring a deleted install or recovery owner.

The composed-framework lifecycle fixture resolves the exact published IcyDB
0.245.1 runtime and model family without a compatibility path. Its dependency
declarations are confined to the two unpublished fixture packages, while
published Canic package graphs remain IcyDB-free.

`app config`, admission, auth-renewal status, backup, blob-storage, cycles,
funding status, `info endpoints`, `info env`, `info list`, `info metrics`,
`info subnets`, inspection, Medic, restore, status and token operations are
exposed against terminal current ensure inventory. Protocol-bound operations
resolve exact Registry-retained Candid bindings and fail closed when the current
inventory does not retain them. Subnet reporting requires the retained
Coordinator/Root bindings and a complete agreeing live Registry/Root snapshot.
Medic reports current desired/plan drift, exact topology, Registry authority
and reviewed conservation bounds without reading or recommending deleted
install or recovery state. The old funding-policy rotation flags are not
restored because their mutation authority came from the deleted install plan;
`cycles funding` is current protected status only.

## Scope Removed

- Historical install contracts, version-specific plan loaders and patch-pair
  recovery allowlists.
- Retained Root repair, provisional successor authority, repair receipts and
  content-addressed recovery-bundle import/verification.
- Fresh install/deploy/adoption/installed-catalog host owners and their public
  CLI modes, aliases, diagnostics and compatibility-only tests.
- The dedicated retained-repair fixture and Root-deletion examples.

Historical release notes and archived audits remain truthful history; they are
not active contracts.

## Validation State

Release governance: source development state; no validated release candidate is staged.
Current operator-surface rebinding, focused governed runtime
qualification and active sediment/documentation reconciliation are complete.

Targeted evidence on the dirty source candidate:

- Focused `canic-host` Fleet-ensure tests pass, including lost responses at all
  seven mutation kinds,
  conservation, unsafe-retirement rejection, plan-tamper rejection, and a
  Toko-shaped PocketIC estate that converges then immediately replans/applies
  with zero mutation actions. Separate tests prove authority validation and
  treasury reuse, live Ledger-fee drift,
  rejects before intent/effect, a short paid result closes safely into a new
  reviewed plan without duplicating the retained creation, two-sided treasury
  receipt proof gates retirement, update issuance remains fenced until status
  proves terminal application, and consecutive stalls reach the configured
  bound before later genuine progress resets it.
- The same focused host suite now includes typed Root-placement compilation and
  exact one-command issuance/terminal-status replay for current Component
  provisioning. Warning-denied `canic-host` and `canic-testing-internal`
  package Clippy pass. The governed targeted production five-Component
  PocketIC case passed in 77 seconds with the shared compiler, complete
  Store/Registry/Component Registry convergence, terminal runtime activation,
  Fleet-catalog publication and an effect-free immediate replay. Peak reported
  RSS was 414,212 kB with 19 threads.
- Six focused current-protocol compiler tests now also prove deterministic
  Registry-chain construction, exact Root/Store authority rejection,
  path-confined qualified Store bytes, content-bound chunk publication and
  deterministic replay identities. The sixth binds a post-publication Registry
  successor to the exact retained Component operation authority and rejects
  Registry or plan drift. Focused control-plane adoption/stable-state
  tests prove the exact retained Root/operator Store controller set. The
  canonical Wasm Store Candid was regenerated and its five focused surface
  checks pass.
- Current operator rebinding has 38 focused Medic tests, 48 focused cycles
  tests, the recursive CLI ordering/help check, three focused top-level
  dispatch/global-option checks, and warning-denied `canic-cli --all-targets`
  Clippy passing. The terminal ensure inventory regression also passes with its
  exact retained plan/journal authority assertion.
- Terminal current inventory now derives its complete authority from the exact
  active Coordinator Registry, Root provisioning receipt, Component Registry
  partitions, pool rows and bounded sharding-child pages. Two focused observer
  tests prove current module/profile binding, and the effect-free-successor
  regression proves the active Registry, protocol-created topology and its
  independently observed cycles remain retained across the successor plan.
- `info subnets` is restored as a current-only leaf. Five focused tests cover
  its terminal-authority binding and complete live aggregation, and the
  recursive CLI ordering/help test passes with the restored surface.
- The final targeted governed five-Component PocketIC rerun passed in 73
  seconds. It reached terminal runtime activation and Fleet-catalog publication,
  then proved an immediate replay issued no update. Reported shared-server
  high-water RSS was 421,668 kB with 19 threads.
- The IcyDB 0.245.1 lifecycle fixture passes targeted compilation plus its
  direct-ingress and same-release transition/recovery PocketIC proofs. The
  cold fixture proof passed in 49 seconds; its cached recovery proof passed in
  5 seconds.
- Earlier warning-denied package Clippy and maintained layering, timer,
  current-status, release-matrix, release-integrity and local v1 readiness
  checks passed. They likewise require maintainer-owned final validation after
  the protocol blocker closes.

The broad workspace and complete `make validate` gate remain human-owned and
have not been run by the automated implementation pass.

No version, commit, tag, package publication, push, deployment, identity
switch, Ledger call, live canister call, or sibling-repository mutation was
performed.

## Next Action

The complete planned `0.109.13` implementation batch is ready for the human
maintainer's broad validation, versioning and publication workflow. The
automated pass intentionally did not run the broad workspace or complete
`make validate` gate. Do not begin 0.110. Do not begin 0.111 from this batch.
