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

- Workspace package version: `0.102.1`. Latest published release: `v0.102.1`
  at `86763c5f16478e2e548e2059e5efaa963bf9a966`.
- The open `0.102.2` changelog draft contains accepted B1 evidence, the
  completed 161-reason register and typed mappings, and the compact public
  diagnostic hard cut. Package/version mutation remains maintainer-owned.
  Active design and checklist:
  [0.102 compact diagnostic codes](../design/0.102-compact-diagnostic-codes/status.md).
- The published checkpoint contains the completed operator-performance/CLI-
  diagnostics outcomes and evidence-only B1 snapshot. The `.2` draft changes
  the public wire to `Error { code: u16 }`; its 161 assignments remain
  unreleased and may be changed without retirement history until `.2` ships.
- Scheduled reserve-Fleet path: [0.103 role-owned Candid surface](../design/0.103-role-owned-candid-surface/status.md), [0.104 `ic-timers` consumer hard cut](../design/0.104-ic-timers-consumer-hard-cut/status.md), [0.105 platform qualification](../design/0.105-fleet-estate-platform-qualification/status.md), [0.106 Coordinator-backed root funding](../design/0.106-coordinator-backed-root-funding/status.md), [0.107 reusable Fleet Subnet Canister estates](../design/0.107-fleet-subnet-canister-estates/status.md) and [0.108 Skynet T2 Fleet observatory](../design/0.108-skynet-fleet-observatory/status.md). The scheduled post-path line is [0.109 framework-neutral local application authorization](../design/0.109-framework-neutral-local-application-authorization/status.md); implementation remains held pending accepted 0.108 closeout and explicit promotion. Other future concepts are [unnumbered ideas](../design/ideas/README.md).
- Release boundary: 0.102 is reinstall-only and is not rolling-compatible with
  pre-0.102. Every Canic-owned canister in a Fleet must come from one admitted
  release set before activation. Same-release interruption recovery, exact
  retry, backup and restore remain required.

## Current Progress

B1's 2,895 labels expand to 3,898 producer-qualified entries plus 31 public
projections. Combining cause with handling/exposure/context produced the
rejected 991-row candidate and 503 singletons; it was never released. Review
split four of 167 mechanical buckets and exactly partitions the frontier into
161 registered causes and ten local typed families across 171 rows.

All four simplified phases—Register, Map, Cut, Clean and measure—are complete.
The maintained model is lossless `DiagnosticCode`, producer-only
`RegisteredDiagnosticCode`, public `Error { code: u16 }`, code-first
`InternalError` and a host catalogue generated with runtime constants from
`reasons.toml`. There is no JSON or generic handling framework. Once released,
`code + name` is immutable; the current assignments remain freely changeable.

The scheduled 0.103 line owns the role-owned Candid and autonomous-operation
hard cut. Root, Coordinator, Store and managed applications own methods;
capabilities add variants, each command/status variant owns its authority and
workflows own phases. Only async/durable commands get operation identities;
atomic commands return typed responses. B1 classifies the 118-Root/20-Burner
working signal through six dispositions with `private/delete` as the default.
Root/Coordinator emit two methods and managed roles at most two. Store starts
with two; each later method needs independent evidence and six is an emergency
ceiling. B2/B3 stay unreleased until exact pruning. B1 repository evidence is
approved; no product implementation batch or mutation is promoted.

The scheduled 0.104 line owns the `ic-timers` consumer and domain async-job
recovery hard cut after 0.103 establishes the final autonomous-operation
consumers. Repository-only 0.105 B1 evidence may continue, but its final
Candid, timer and state inventory must reconcile against accepted 0.103 and
0.104 before B2.

The scheduled 0.105 B1 is approved to freeze current pool/platform provenance,
measurement/reset protocol, horizon-qualified standby semantics and production
reachability for a 1,000-Canister reserve Fleet. B2 execution is held pending
accepted B1 and separate exact authorization for every external effect.

The scheduled 0.106 line closes replay-safe Coordinator-backed root operating
funding separately from the estate budget. Its proof and mutation require
completed 0.103 and 0.104 plus accepted 0.105 B1 ownership/cost evidence, not
0.105 B2, plus its own proof. Its public work adds Root/Coordinator command and
status variants rather than funding methods.

Scheduled 0.107 owns indexed estates, parallel creation/reset, transfer and
the 10/100/1,000 proof; 0.108 then serves an evidence-labelled T2 topology and
Fleet overview from every installed Skynet Fleet Canister. Empty estate assets
remain visible module-free inventory. Scheduled 0.109 owns framework-neutral
local application authorization after that path. Framework composition,
transport, Workers, authentication profiles, blob/archive storage and Motoko
stay ideas.

## Current Decision

Diagnostic registration is active on untagged 0.102.2.

Codes identify semantic causes; typed callers retain handling decisions. Only
public, retrievable operator, durable-evidence or machine-decision boundaries
qualify global codes. Projection is explicit, never text-derived or table-led;
masked reasons without an independent exact owner stay local. Required dynamic
data remains typed, while nonessential context may be dropped rather than
creating infrastructure.

The unreleased 991 candidate was replaced, not retired. The reviewed 161-row
register, raw/registered split, host catalogue and typed mappings own `.2`.
B1 tooling/allocation authority is removed while its evidence remains archived.
Activation requires one admitted release set and matching callers. Do not add a
dual protocol, compatibility decoder, message fallback, diagnostic version or
observability subsystem.

The 0.103 design is scheduled, but B1 inventory awaits explicit promotion and
no endpoint mutation is authorized. 0.104 then consumes its completed
autonomous-operation surface; 0.104 B1 source mutation also awaits explicit
promotion. The 0.105 B1 may freeze experiment semantics, inventory current
repository state/reachability and build local harnesses. No external effect is
authorized. B2 needs accepted B1, accepted 0.103/0.104 reconciliation and an
exact approved run plan; absent mainnet approval, 0.105 is blocked rather than
failed.

The 0.106 B1 PocketIC proof follows completed 0.103 and 0.104 and accepted
0.105 B1 root ownership/current-cost evidence. Its mutation waits for those
outputs and its own passing proof, not 0.105 B2.

The 0.107 estate B1 waits for completed 0.103 and 0.104, accepted 0.105,
completed 0.106 and explicit promotion. 0.108 implementation waits for
accepted 0.107 closeout and its own B1 promotion. Scheduled 0.109 then waits
for accepted 0.108 closeout and a separate maintainer implementation decision.
Deferred ideas do not gate the six-line reserve-Fleet path or authorize 0.109
mutation.

## Validation

The clean `v0.101.53` baseline at `23c0328f78b215580d734ef01b52b35fa3e38ade`
passes `CANIC-WASM-001/v3` at risk `5/10` across all nine release/debug roles
with isolated offline state and pinned tools. V2 is non-comparable; earlier
0.101 validation remains historical evidence only.

The simplified candidate passes targeted reason-ledger/generation, compact
Candid wire, host lookup, typed projection and mapping checks. Core and
control-plane production and test targets compile; host diagnostics, CLI,
Fleet Coordinator and Wasm Store feature builds compile. Focused RPC, ICP
refill, delegated-authentication, cost-guard, metric, retry, publication,
template and directory-synchronization tests pass. Targeted release builds for
`app`, Root, Fleet Coordinator and Wasm Store pass through the canonical
builder with gzip integrity, `ic-wasm` structure evidence and bounded absence
scans. All four data sections are smaller than the retained baseline; only Root
is smaller overall across the non-isolated release-line comparison, so no
causal diagnostic-savings claim is made. A complete `make validate` recorded on
2026-08-16 is historical push evidence and must not be repeated during focused
development; automated agents run targeted checks only.

Separate timer-recovery checks remain useful pre-0.104 evidence but do not
establish completion of the 0.104 ownership hard cut.

The standalone cycle-burn waveform idea completed its sole authorized B0b
mainnet calibration on 2026-08-16. Canister
`w47na-gaaaa-aaaad-qmclq-cai` burned exactly `4 Tcycles` on the frozen public
13-node `verified_application` Subnet and retained more than `1 Tcycle`.
Direct-burn visibility passed, but the public Subnet peak rose only
approximately `0.883 Bcycles/second` and stayed elevated across the observed
tail, rejecting the assumed independent approximately 100-second buckets by
approximately a `45.3x` flattening factor. B1 and any further destructive
effect remain held; the current 860-point executor/economic model is invalid
until a complete dated aggregation kernel and a different bounded strategy
are accepted. Exact evidence is retained in the
[B0b calibration report](../audits/working/saltz-b0b-calibration/mainnet-calibration.md).

## Next Action

The diagnostic batch is ready for maintainer commit/push validation. Do not
rerun the broad census/full suite or add JSON, generic handling metadata,
observability infrastructure, compatibility decoding or retired 991 rows.

Design roots retain authority; supporting evidence lives under `docs/audits/`.
