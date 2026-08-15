# Current Status

Last updated: 2026-08-15

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the active design, source, validation or changelog material needed for the
current task.

Historical handoffs are archived at:

- [status through 2026-06-30](archive/2026-06-30-precompact.md);
- [status through the 0.90.2 release](archive/2026-07-13-precompact.md); and
- [status through the 0.101.52 Q4 qualification](archive/2026-08-12-precompact.md).

## Current Release

- Workspace package version: `0.102.1`.
- Latest published release: `v0.102.1` at
  `86763c5f16478e2e548e2059e5efaa963bf9a966`.
- The open `0.102.2` changelog draft contains the complete B1 allocation-
  review register; package/version mutation remains maintainer-owned.
- The published checkpoint contains the completed operator-performance/CLI-
  diagnostics outcomes and evidence-only B1 snapshot; the `.2` draft advances
  B1 to explicit maintainer allocation review without runtime mutation.
- Active design and checklist: [0.102 compact diagnostic codes](../design/0.102-compact-diagnostic-codes/status.md).
- Evidence-only adjacent designs: [0.103 local application authorization](../design/0.103-framework-neutral-local-application-authorization/status.md), [0.104 synchronous lifecycle composition](../design/0.104-framework-neutral-synchronous-lifecycle-composition/status.md) and [0.105 Fleet-estate platform qualification](../design/0.105-fleet-estate-platform-qualification/status.md).
- Release boundary: 0.102 is reinstall-only and is not rolling-compatible with
  pre-0.102. Every Canic-owned canister in a Fleet must come from one admitted
  release set before activation. Same-release interruption recovery, exact
  retry, backup and restore remain required.

## Current Progress

0.101 is closed at `0.101.53`; its historical design, status, closeout and
changelog records remain intact. 0.102 B1 has been reread against the current
source. The public error still carries a Candid `ErrorCode` plus owned message
text, the maintained enum has 20 leaves, `InternalError` remains string-first,
canonical infrastructure Candid retains the old shape and the host still
matches typed variants. No diagnostic runtime, Candid or stable-state surface
has changed in B1.

The released operator-maintenance and performance outcomes retain exact Clap
diagnostics, pre-parse argv tracing, one shared application build, concurrent
artifact finalization, exact finalized-build reuse/recovery and additive install
timings. The latest recorded Demo App builds took 110.31 seconds cold and 22.97
seconds on a cache hit. Their targeted evidence is recorded in the active
tracker and published changelogs; they do not implement compact diagnostics.

An independent timer/query batch hard-cuts to exact `ic-timers 0.5.0`, removes
Canic's parallel timer authority and projects one complete cross-framework
inventory through neutral `CanisterTimerStatus` records, metrics and explicit
availability. Policy-specific callback capabilities and bounded instruction
and memory-page observations now remain typed through Canic's runtime status.
Role declarations, fallible/detachable handles, custody rollback,
truthful pool outcomes and atomic snapshot fencing have targeted native and
PocketIC evidence. Authority snapshots reject out-of-custody claims; combined-
framework snapshots still require the separately qualified lifecycle seam.

The batch is not production-complete. Recovery-critical asynchronous renewal,
top-up, acknowledgement and pool work still lack trap-safe serial re-kicking;
the synchronous fixed-cadence Watchdog cannot supply that contract. Exact
`0.5.0` includes the released transient-cancellation and stale-claim fixes; the
remaining blocker is Canic's asynchronous recovery protocol, not shared timer
claim correctness.

The host Subnet catalogue adapter uses published `ic-query 0.40.1` with only
`subnet-catalog-host`; portable Governance analytics remain unused.

The allocation-policy proposal uses dense monotonic numbers with no semantic
bands, compact unpadded `E<decimal>` rendering and nine host-only broad classes.
The complete guarded review register expands 2,895 provisional labels into
3,898 exact producer-qualified entries plus 31 public projections. They map
exactly once onto 960 composable exact-condition contracts and 31 safe public
projections, yielding a dense proposed `1..=991` allocation with full host
metadata and 503 singleton rationales. It is not permanent or runtime code
authority until the maintainer approves the register.

Maintainer review approved the architectural direction and required four P1
corrections before mutation: permanent current/retired allocation history, a
Fleet-atomic activation boundary, mechanically enforced registered producer
identities distinct from raw decoded numbers, and a complete ownership audit of
dynamic values currently embedded in public messages. The normative design and
B1 contracts now include all four. This is design/evidence progress only.

The approved 0.103 direction uses bounded local Fleet/role/scope authority and
one synchronous `caller + scope` decision. It separates an at-most-60-second
proof from a protected local session of up to 1,800 seconds and requires
native-agent acquisition evidence. Its mutation waits for accepted 0.102.

The approved 0.104 direction adds one paired synchronous application lifecycle
participant after Canic restoration. Root, Store and Coordinator remain
inventory-only; mutation waits for accepted 0.103 and explicit promotion.

The approved evidence-only 0.105 line extracts early Fleet-estate platform
qualification from the full implementation. It preserves 0.103/0.104 and moves
only the former provisional 0.105-0.113 designs to 0.106-0.114. Published
versions, historical changelogs, retained audits and archived handoffs keep
their historical identities.

## Current Decision

Follow the six release batches in the active tracker. Evidence-only B1 may
continue. It must produce the exact current producer/consumer,
dynamic-public-context and durable-string inventories, a reproducible
representative-Wasm baseline, a permanent current/retired allocation ledger,
typed host disposition and explicit public projections with retrievable
operation correlation. It must map the exhaustive host-only coverage frontier
many-to-one onto canonical semantic conditions and handling contracts. The
same condition and contract share a code across roles, modules and wrappers;
orthogonal operation context does not create compound codes. Declaration code
may be split by semantic domain, but every code has one canonical path and no
producer-local alias. The design's numeric examples are not authority, and a
four-digit initial allocation fails the compression gate.

Do not begin mutating batches B2-B6 until the maintainer has reviewed the
complete B1 inventories, coverage-to-code map, compressed initial allocation,
host catalogue and projections. The coverage frontier and mapping are
repository evidence and must never enter release Wasm.
The public cut must install all Canic-owned Fleet canisters from one admitted
release set before activation, with matching host/CLI callers and regenerated
external bindings. Do not introduce a temporary dual protocol, generation
name, compatibility decoder, diagnostic protocol version or message fallback.

The 0.103 B1 must inventory and freeze complete signed-presenter propagation from the preparation caller with no legacy fallback; an authorized mutating batch performs it.
B1 must also resolve scope, separate proof/session lifetimes and capacities, canonical ownership, purity, the complete authority-generation transition table and native-client acquisition.
B2-B7 remain blocked until 0.102 is accepted and complete and the 0.103 promotion review explicitly authorizes mutation.

The 0.104 B1 may inventory lifecycle owners, macro expansions, symbols, ordering and costs. B2-B6 wait for accepted, complete 0.103, the frozen paired `fn() -> ()` grammar/exclusions and explicit promotion.

The 0.105 B1 may inventory current pool state and build repository/local
qualification harnesses, including the empty-topology proof. No remote or
IC-mainnet effect is authorized. Its B2 requires a separate exact network,
identity, count, concurrency, cycle-budget and asset-disposition approval. The
full 0.114 estate B1 also waits for accepted 0.105 and 0.110 predecessors plus
explicit promotion.

## Validation

Freshly observed baseline identity:

```text
branch: main
commit: 23c0328f78b215580d734ef01b52b35fa3e38ade
tag: v0.101.53
worktree: clean before 0.102 documentation work
```

The fresh retained `CANIC-WASM-001/v3` baseline passes at risk `5/10` over six
Components plus Fleet Subnet Root, Fleet Coordinator and Wasm Store in both
release and debug profiles. It uses immutable tag `v0.101.53`, a clean detached
worktree, isolated local/offline build state and the checksum-pinned toolchain.
Valid v2 evidence remains superseded and non-comparable. Previously recorded
0.101 test and release claims remain historical evidence only.

Fresh B1 validation passes all 60 targeted diagnostic-ledger tests, its
warning-denied target Clippy, changelog governance, workspace-test inventory,
current-document semantics and diff hygiene. The audit-method catalog also
passes after admitting the lifecycle method's current timer scans as
`CANIC-LIFECYCLE-001/v3` and preserving v2 as superseded. The concurrent
runtime batch remains release-blocked above.

## Next Action

The current-candidate scan remains pinned to
`0750c309104b111fa6f5a1b3355c04fcb38faf71`. Its closed host-only frontier is
2,864 exact provisional identities plus 31 projections, or 2,895 coverage
labels; this is not a proposed runtime code count. Qualifying the labels by
their producer anchors yields 3,898 exact entries and 3,929 total review
observations. The targeted guard derives that set only from explicit coverage
rows. All eighteen maintained consumers are source-addressed. The
producer-anchor pass closes all 2,864 exact identities and pins an empty debt
set. The final family guards cover 239
Component Registry workflow, 449 direct Registry-ops and 230 grouped-
provisioning labels; detailed family evidence remains in the active tracker.

All 656 dynamic public values and all projection observability owners are
classified. The durable-string census is closed, and no decision parses
retained failure text. The four current Cashier coverage conditions remain in
scope; any resulting compressed codes retire without reuse in the 0.109 hard
cut.

B1 is ready for maintainer allocation review. Its checked-in register is
derived byte-for-byte from the closed producer-function manifest and binds
every row's class, semantic origin, disposition, condition, qualified
coverage, provisional identities, producer set, projection, observability,
remediation and action. Equivalent conditions
and handling contracts share a code across roles and modules; wrapper and
operation context do not create compound codes. Eight exact identities reused
as projection targets and all 31 projection-only identities are explicit. The
frontier and map remain repository-only and must not enter Wasm. No proposed
number, runtime contract, Candid surface or stable schema has become runtime
authority. B2 remains blocked until the maintainer accepts or corrects the
complete register.

Design-document hygiene now applies across current and archived numbered
lines: design roots retain their design/status authority, while necessary
working or historical supporting evidence lives under `docs/audits/`. This
changes document placement only and preserves historical findings.
