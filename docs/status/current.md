# Current Status

Last updated: 2026-08-16

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
- The open `0.102.2` changelog draft contains the approved B1 allocation,
  complete B2 runtime/host catalogue and operator lookup. Unrelated timer and
  Saltz work is outside this diagnostic release decision;
  package/version mutation remains maintainer-owned.
- The published checkpoint contains the completed operator-performance/CLI-
  diagnostics outcomes and evidence-only B1 snapshot; the `.2` draft advances
  the approved register into prose-free B2 runtime identity and host lookup
  without changing the public error wire.
- Active design and checklist: [0.102 compact diagnostic codes](../design/0.102-compact-diagnostic-codes/status.md).
- Scheduled reserve-Fleet path: [0.103 `ic-timers` consumer hard cut](../design/0.103-ic-timers-consumer-hard-cut/status.md), [0.104 platform qualification](../design/0.104-fleet-estate-platform-qualification/status.md), [0.105 Coordinator-backed root funding](../design/0.105-coordinator-backed-root-funding/status.md), [0.106 reusable Fleet Subnet Canister estates](../design/0.106-fleet-subnet-canister-estates/status.md) and [0.107 Skynet T2 Fleet observatory](../design/0.107-skynet-fleet-observatory/status.md). Other future concepts are [unnumbered ideas](../design/ideas/README.md).
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

The independent timer/query candidate resolves exact `ic-timers 0.6.1`, removes
raw production `ic-cdk-timers` access and projects one cross-framework atomic
inventory through neutral `CanisterTimerStatus` records, metrics and explicit
availability. Its native and PocketIC evidence proves useful claim custody,
inventory and same-operation interruption behavior. The maintainer has
explicitly kept this candidate outside the 0.102.2 diagnostic closeout.

The ownership hard cut is not complete. Canic still duplicates provider
directive/result vocabulary, exposes an application timer facade and stores
generic `Ensure`/`Reconcile` commands, watchdog-owned deadlines, schedule-
ownership state and copied retry/terminal state in the private memory-ID-60
record. Scheduled 0.103 now owns removal of those timer mechanics while
retaining replay-safe business operation identity, attempt fencing, leases,
domain demand, one native watchdog and shared-inventory projection.

The host Subnet catalogue adapter uses published `ic-query 0.40.1` with only
`subnet-catalog-host`; portable Governance analytics remain unused.

The approved allocation uses dense monotonic numbers with no semantic bands,
compact unpadded `E<decimal>` rendering and nine host-only broad classes. The
complete guarded register expands 2,895 provisional labels into
3,898 exact producer-qualified entries plus 31 public projections. They map
exactly once onto 960 composable exact-condition contracts and 31 safe public
projections, yielding the dense `1..=991` allocation with full host metadata
and 503 singleton rationales.

Maintainer review first approved the architectural direction and required four
P1 corrections before mutation: permanent current/retired allocation history,
a Fleet-atomic activation boundary, mechanically enforced registered producer
identities distinct from raw decoded numbers, and a complete ownership audit of
dynamic values currently embedded in public messages. B1 closed all four, and
the maintainer approved the complete register and authorized B2 on 2026-08-16.

B2 now derives 991 canonical registered runtime constants, 991 current and zero
retired permanent ledger rows, a typed host catalogue, a byte-identical current
JSON registry and `canic diagnostic` lookup. Raw decoded values remain lossless
and cannot acquire producer authority. The public `ErrorCode + message`,
internal string propagation, Candid and diagnostic-owned stable state remain
unchanged. The timer record at ID 60 is an independent private runtime
contract, not a diagnostic schema generation.

The scheduled 0.103 line owns the `ic-timers` consumer and domain async-job
recovery hard cut. No implementation batch is promoted by the planning cut.
Repository-only 0.104 B1 evidence may continue, but its final timer/state
inventory must reconcile against accepted 0.103 before B2.

The scheduled 0.104 B1 is approved to freeze current pool/platform provenance,
measurement/reset protocol, horizon-qualified standby semantics and production
reachability for a 1,000-Canister reserve Fleet. B2 execution is held pending
accepted B1 and separate exact authorization for every external effect.

The scheduled 0.105 line closes replay-safe Coordinator-backed root operating
funding separately from the estate budget. Its proof and mutation require
completed 0.103 plus accepted 0.104 B1 ownership/cost evidence, not 0.104 B2,
plus its own proof.

Scheduled 0.106 owns indexed estates, parallel creation/reset, transfer and
the 10/100/1,000 proof; 0.107 then serves an evidence-labelled T2 topology and
Fleet overview from every installed Skynet Fleet Canister. Empty estate assets
remain visible module-free inventory. Framework composition, transport,
Workers, authentication profiles, blob/archive storage and Motoko stay ideas.

## Current Decision

Diagnostic B2 is implemented as the 0.102.2 release outcome. The maintainer has
explicitly separated unrelated timer and Saltz work from this diagnostic
decision; neither supplies evidence for nor blocks B2 push-readiness.
The exhaustive host-only diagnostic frontier maps many-to-one onto approved
canonical conditions and handling contracts. The same condition and contract
share a code across roles, modules and wrappers; orthogonal operation context
does not create compound codes. Every registered code has one canonical path
and no producer-local alias. The complete allocation stays at 991, below the
four-digit rejection gate.

Do not fold B3-B6 into this B2 handoff. The coverage frontier and mapping remain
repository evidence and must never enter release Wasm.
The public cut must install all Canic-owned Fleet canisters from one admitted
release set before activation, with matching host/CLI callers and regenerated
external bindings. Do not introduce a temporary dual protocol, generation
name, compatibility decoder, diagnostic protocol version or message fallback.

The 0.103 design is scheduled, but B1 source mutation awaits explicit
promotion. The 0.104 B1 may freeze experiment semantics, inventory current
repository state/reachability and build local harnesses. No external effect is
authorized. B2 needs accepted B1, accepted 0.103 reconciliation and an exact
approved run plan; absent mainnet approval, 0.104 is blocked rather than
failed.

The 0.105 B1 PocketIC proof follows completed 0.103 and accepted 0.104 B1 root
ownership/current-cost evidence. Its mutation waits for those outputs and its
own passing proof, not 0.104 B2.

The 0.106 estate B1 waits for completed 0.103, accepted 0.104, completed 0.105
and explicit promotion. 0.107 implementation waits for accepted 0.106 closeout
and its own B1 promotion. Deferred ideas do not gate this five-line path.

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

Fresh B2 validation passes all 64 targeted diagnostic-ledger tests, the complete
991-way runtime/ledger/catalogue/JSON bijection, current/retired/unknown lookup,
strict CLI parsing and recursive help ordering. The symbolic public-error
protocol guard continues to pin the unchanged pre-cut wire. Canonical
role-scoped release builds for a representative Component, Wasm Store and Fleet
Subnet Root contain none of the bounded host-ledger, JSON, catalogue-prose or
review-map markers. Warning-denied Clippy passes for the touched core, host,
CLI and asset-generator targets. A fresh complete `make validate` passed on
2026-08-16, including release guards, full workspace tests and every serial
PocketIC suite. The repository-wide result does not make unrelated maintainer
work part of the scoped 0.102.2 diagnostic claim.

Separate timer-recovery validation passes ten durable-state tests, eleven timer
workflow tests, role/state contracts, pool outcomes, cycle-request replay and
warning-denied Clippy for core, control-plane and the runtime probe. The
four-test PocketIC timer suite passes; its adversarial probe traps after one
self-call, preserves the lease and proves one same-identity watchdog takeover
clears only its exact attempt. These checks are unrelated to 0.102.2 and do not
establish completion of the 0.103 ownership hard cut.

## Next Action

Diagnostic B2 remains implemented and its register deterministically derives
the canonical runtime declarations, permanent ledger and current JSON. Its
scoped patch and release notes are ready for the maintainer's human-owned
version/stage/commit/push flow. The public Candid surface, internal string
propagation and diagnostic stable schema remain intentionally unchanged for
later diagnostic batches.

Design roots retain authority; supporting evidence lives under `docs/audits/`.
