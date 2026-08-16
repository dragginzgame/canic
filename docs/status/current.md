# Current Status

Last updated: 2026-08-17

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
- The open `0.102.2` draft owns the completed 161-reason register, typed
  mappings, compact public wire and flat `code + name` release baseline. B1
  census/count coupling is removed from permanent tests. All assignments remain
  unreleased and freely changeable until `.2` ships; package/version mutation
  remains maintainer-owned. Active checklist:
  [0.102 compact diagnostic codes](../design/0.102-compact-diagnostic-codes/status.md).
- Scheduled reserve-Fleet path: [0.103 role-owned Candid surface](../design/0.103-role-owned-candid-surface/status.md), [0.104 `ic-timers` consumer hard cut](../design/0.104-ic-timers-consumer-hard-cut/status.md), [0.105 platform qualification](../design/0.105-fleet-estate-platform-qualification/status.md), [0.106 Coordinator-backed root funding](../design/0.106-coordinator-backed-root-funding/status.md), [0.107 reusable Fleet Subnet Canister estates](../design/0.107-fleet-subnet-canister-estates/status.md) and [0.108 Skynet T2 Fleet observatory](../design/0.108-skynet-fleet-observatory/status.md). The scheduled post-path line is [0.109 framework-neutral local application authorization](../design/0.109-framework-neutral-local-application-authorization/status.md); implementation remains held pending accepted 0.108 closeout and explicit promotion. Other future concepts are [unnumbered ideas](../design/ideas/README.md).
- Release boundary: 0.102 is reinstall-only and is not rolling-compatible with
  pre-0.102. Every Canic-owned canister in a Fleet must come from one admitted
  release set before activation. Same-release interruption recovery, exact
  retry, backup and restore remain required.

## Current Progress

B1's 2,895 labels exposed the rejected unreleased 991-row tuple taxonomy. The
accepted frontier is 161 registered causes and ten local typed families. All
four phases—Register, Map, Cut, Clean and measure—are complete: lossless raw
codes, producer-only registered codes, `Error { code: u16 }`, code-first
`InternalError`, and host-only prose. The flat release guard freezes only
released `code + name`; it neither reads B1 evidence nor freezes row counts,
and the ledger rejects fields beyond its six maintained fields.

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

Diagnostic registration is active on untagged 0.102.2. Codes identify semantic
causes; typed callers retain handling. Only public, retrievable operator,
durable-evidence or machine-decision boundaries qualify. Projection is explicit;
masked reasons without an independent exact owner stay local, required dynamic
data stays typed, and nonessential context creates no infrastructure. The 991
candidate was replaced rather than retired, and B1 evidence has no allocation
authority. Activation requires one admitted release set; do not add compatibility
decoding, message fallback, diagnostic version or observability machinery.

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

The retained `v0.101.53` `CANIC-WASM-001/v3` baseline and focused `.2` checks
cover ledger generation, released identities, raw/registered separation,
projection, host lookup, compact Candid, typed mappings and generic
current/retired catalogue invariants. Core, control-plane, host, CLI,
Coordinator and Wasm Store targets compile. Representative `app`, Root,
Coordinator and Store release builds pass integrity and bounded absence scans;
all data sections shrink, but only Root shrinks overall, so no causal size claim
is made. The complete 2026-08-16 `make validate` remains historical push
evidence and is not rerun during focused development.

Separate timer-recovery checks remain useful pre-0.104 evidence but do not
establish completion of the 0.104 ownership hard cut.

The standalone cycle-burn waveform idea completed its bounded B0b pulse and
B0c plateau mainnet calibrations on 2026-08-16. Canister
`w47na-gaaaa-aaaad-qmclq-cai` burned exactly `4 Tcycles` on the frozen public
13-node `verified_application` Subnet and retained more than `1 Tcycle`.
Direct-burn visibility passed, but the public Subnet peak rose only
approximately `0.883 Bcycles/second` and stayed elevated across the observed
tail, rejecting the assumed independent approximately 100-second buckets by
approximately a `45.3x` flattening factor. The later exact `3.6 Tcycle`
host-driven plateau proved that repeated bounded input accumulates cleanly,
raising the observed series approximately `0.990 Bcycles/second`. The
maintainer then explicitly promoted a standalone, non-Fleet executor and inert
mainnet install. The complete decay kernel remains open, so its embedded
`4,531`-second rectangular controller is a dated bounded proposal rather than
a Dashboard-fidelity claim. Exact calibration evidence is retained in the
[B0b calibration report](../audits/working/saltz-b0b-calibration/mainnet-calibration.md).

The integer compiler embeds 45 pre-roll plus 864 waveform amounts at exact
100-second deadlines. After two bounded 1× attempts proved timer/receipt
execution but remained too close to public noise, the maintainer authorized an
exact 10× controlled-signal trial. Authorization digest
`e5977055cf691d29353c6649bd464a821475efd66432ff56ea93d76de419ff8d`
can burn at most `958,052,944,723,000 cycles`, but the staged trial binds only
the first 42 pulses and requires `54,924,680,000,000 cycles` at `Arm`. Exact
schedule/unit checks, warning-denied Clippy, strict extracted Candid and
targeted PocketIC funding, authority, 42-pulse exhaustion, timer-burn and abort
evidence pass. Release Wasm
`2388f3f4e38274999682da7a3525d6fbc41724c073c61d16b7c9b253ebecbfc9`
is installed on `w47na-gaaaa-aaaad-qmclq-cai`. Converting exactly
`49.1 Tcycles` cost `30.15060661 ICP`; the exact top-up produced
`55,080,036,485,836 cycles` before Arm. The 10× run began at
`2026-08-17T00:15:00+02:00`. Sixteen exact receipts burned
`20,504,640,000,000 cycles` with no terminal reason. The owning Subnet's exact
600-second public series rose from `0.389` and `0.508` before the controlled
rise to `1.496`, `3.378` and `5.157 Bcycles/second`. The successive controlled
increases of approximately `0.988`, `1.881` and `1.779 Bcycles/second` passed
the multi-bin proportional-response continuation gate.

The exact remaining top-up was `904,073,274,118,831 cycles`. Its discrete
mint deposited `904,073,274,124,352 cycles` and cost `556.42132824 ICP`; the
canister received only the exact requested top-up. After receipt 17 it held
`937,366,738,334,627 cycles` against `936,266,764,723,000` remaining
intentional cycles, preserving `1,099,973,611,627 cycles` for reserve and
execution. The controller identity retained `4,635.83518221 ICP`.

At `2026-08-17T00:57+02:00`, the maintainer clarified that the intended canvas
was the global Dashboard homepage rather than the owning Subnet graph. Its
approximately `45.9 Bcycles/second` global value exposes a material scope
mismatch: the installed controller was calibrated only against the Subnet's
background and kernel. The standing loss-of-confidence rule therefore caused
an immediate controller Abort before chart start. Terminal status is
`Aborted` / `ControllerAbort`, with exactly 26 receipts,
`33,320,040,000,000 cycles` burned and `925,797,679,907,302 cycles` remaining.
No waveform step executed and no later intentional burn is authorized.

Read-only B0e global qualification is active. The exact seven-day pre-run
global series contains 1,009 ten-minute samples with `37.576 Bcycles/second`
median, `37.983` mean, `53.915` maximum and `4.255 Bcycles/second` 95th-
percentile absolute adjacent change. Replaying a provisional
`100..=150 Bcycles/second` trace over every historical day yields
`0.923..=0.965` correlations. The 26-pulse controlled rise fits an effective
`4,200.842`-second rectangular window with `R² = 0.999475`; the complete
post-Abort tail remains the acceptance gate.

The local candidate hard-cuts the obsolete 10× scale into a direct global
contract: `30 Bcycles/second` conservative background credit,
`100 Bcycles/second` floor, `50 Bcycles/second` relief, `4,201`-second candidate
window and `500 Bcycles/second` hard rate ceiling. Its exact integer plan is 42
pre-roll plus 864 waveform pulses totaling `8,319,493,755,865,700 cycles` with
digest `87694d17f2b57f03b6345bce51db4be97e45eeb78b2dd5cedaf891b703579590`.
Simulator/unit checks, warning-denied Clippy, Wasm-target compilation, strict
Candid equivalence and targeted PocketIC funding, exhaustion, receipt and
Abort evidence pass. This code remains inert local evidence; it is not
reinstall, funding or Arm authority.

## Next Action

The focused `.2` identity-guard cleanup is complete. Do not rerun the broad
census/full suite or add JSON, generic handling metadata, observability
infrastructure, compatibility decoding, B1 test coupling or retired 991 rows.

For the standalone waveform idea, retain the terminal canister and its
remaining cycles without further effects. Observe the complete B0d trailing
edge, accept or revise the candidate `4,201`-second kernel, then reconcile the
integer plan, historical global replay and financial ceiling. Any reinstall,
mint, top-up or Arm still requires a new exact mainnet authorization; passing
local checks cannot supply it.

Design roots retain authority; supporting evidence lives under `docs/audits/`.
