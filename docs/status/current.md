# Current Status

Last updated: 2026-08-18

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the active design, source, validation or changelog material needed for the
current task.

Historical handoffs: [through 2026-06-30](archive/2026-06-30-precompact.md),
[through 0.90.2](archive/2026-07-13-precompact.md) and
[through 0.101.52 Q4](archive/2026-08-12-precompact.md).

## Current Release

- Workspace package-version authority is the root `Cargo.toml`. It remains
  `0.102.2`, the latest published release, at
  `8cf4723cecd7579cbe3304b980c63b1bc3969d68`. The single current changelog
  target is `0.103.0`; it is not published or package-versioned yet.
- Release-truth warning: neither local Git nor `origin` has a `v0.103.*` tag,
  so the former stray-tag collision is closed. The governed minor-version flow
  advances the current `0.102.2` package to the exact `0.103.0` target and owns
  the complete validation gate. Do not publish before that flow completes.
- The published release owns the completed 161-reason register, typed mappings,
  compact public wire and flat `code + name` release baseline. B1 census/count
  coupling is absent from permanent tests. The open closeout-correction batch
  makes that released baseline tamper-evident, preserves the four genuine
  caller-continuation values and routes ordinary host failures through the
  prose catalogue. Active checklist:
  [0.102 compact diagnostic codes](../design/0.102-compact-diagnostic-codes/status.md).
- Scheduled application-safety and estate path: [0.103 role-owned Candid surface](../design/0.103-role-owned-candid-surface/status.md), [0.104 timer ownership plus synchronous lifecycle composition](../design/0.104-ic-timers-consumer-hard-cut/status.md), [0.105 framework-neutral local application authorization](../design/0.105-framework-neutral-local-application-authorization/status.md), [0.106 platform qualification](../design/0.106-fleet-estate-platform-qualification/status.md), [0.107 Coordinator-backed root funding](../design/0.107-coordinator-backed-root-funding/status.md), [0.108 reusable estates plus application retirement](../design/0.108-fleet-subnet-canister-estates/status.md), [0.109 stateful Fleet release adoption](../design/0.109-stateful-fleet-release-adoption/status.md) and [0.110 generic Fleet observatory](../design/0.110-fleet-observatory/status.md). External [Prequel Wars](https://github.com/dragginzgame/prequel-wars) replaces the checked-in Skynet App as the flagship demonstration. Other future concepts are [unnumbered ideas](../design/ideas/README.md).
- Release boundary: 0.102 is reinstall-only and is not rolling-compatible with
  pre-0.102. Every Canic-owned canister in a Fleet must come from one admitted
  release set before activation. Same-release interruption recovery, exact
  retry, backup and restore remain required. Scheduled 0.109 is the first
  explicit one-predecessor-to-one-successor exception; no current release is
  adoptable until that line is implemented and published.

## Current Progress

0.102.2 is published. Its read-only closeout audit confirmed the compact wire,
typed runtime, host-only catalogue and Wasm boundary, then found one dynamic
data-loss defect plus released-baseline, ordinary-rendering and release-truth
closeout defects. The bounded correction is implemented before any 0.103 B2
mutation: one typed cycles-funding preflight response owns the four genuine
caller-continuation values; the other 64 provisional B1 owner proposals are
reconciled as local typed state, caller-derived data, existing authority or
deliberately dropped operator convenience. The release guard reads the exact
0.102.2 reason ledger Git object, including retirement state, and the central
host decoder now renders known diagnostics automatically.

B1's 2,895 labels exposed the rejected unreleased 991-row
tuple taxonomy. The accepted frontier is 161 registered causes and ten local
typed families. All
four phases—Register, Map, Cut, Clean and measure—are complete: lossless raw
codes, producer-only registered codes, `Error { code: u16 }`, code-first
`InternalError`, and host-only prose. The flat release guard freezes only
released `code + name`; it neither reads B1 evidence nor freezes row counts,
and the ledger rejects fields beyond its six maintained fields.

Scheduled 0.103 owns role methods and autonomous operations. Its B1 evidence
was accepted on 2026-08-17: the immutable `v0.102.2` baseline freezes 207 methods across
representative Root/managed profiles and canonical Coordinator/Store
interfaces, separated into 188 Canic-owned, three external-standard and 16
fixture-owned methods. B2-B7 are complete in the unreleased worktree: the closed
capability boundary, immutable Overview and bounded shared-request DTOs now
drive the exact managed, Root, Coordinator and Store status dispatchers. Exact
Candid/profile identity survives release metadata, verified Root Store
catalogues, install/Registry replay validation and Component Directory
projection. Root, Coordinator, managed and Store operations resolve their exact
durable IDs through their owning commands rather than a universal operation
store. Representative Runtime-only,
Sharding/AutomaticTopup, Root signer/non-signer and Store builds prove the
corrected B1 protocol is actually pruned:
external artifact metadata selects the exact profile-specific binding before
the first call, `Overview` only verifies it, and
config-derived `AutomaticTopup` rather than mandatory `Runtime` owns top-up
history and public handler reachability. Private automatic-top-up timer and
callback pruning remains the explicit 0.104 owner. Endpoint
source, authorization/payload attributes, immediate delegates and replay
policy are frozen; existing typed config derivation remains the capability
authority. The corrected six-way review disposes all 188 Canic-owned appearances: 49
role-command variants, 78 role-status variants, two Store byte lanes and 59
private deletions. Every retained row names its exact role-specific variant,
released Rust signature and reviewed executable-caller subset. The normalized
manifest also freezes request/response correlation, sync/async and variant
counts, selector nesting and the atomic B4/B5 caller cuts. Distinct status
concepts stay flat rather than becoming nested family enums; remote composite
phase observations collapse into local operation status and update-only
canister inspection becomes an atomic Root command. DTOs, the exact pruning
matrix, operation ownership and the no-new-timer 0.104 handoff are frozen.
Host/CLI fixtures and then-current application presentation use the role
surface. The obsolete checked-in Skynet demo is removed in favor of external
Prequel Wars; frozen B1 source evidence remains historical. Legacy
shared/non-root/cycle/topology emitters are
deleted, `start_local!` emits one local status method, and the representative
Canic count falls from 188 method appearances to ten. Current fast-profile
Wasm identities are recorded without claiming causal size savings. Raw B1
pre-cut Candid snapshots are no longer retained in the current worktree; the
capture tool derives the immutable normalized register and manifest hashes
from scratch-generated interfaces so historical contracts cannot be mistaken
for live ones. The same cleanup removes five unconsumed legacy build cfgs and
their dead environment/manifest probes; the generated-role cfg catalog is once
again exact and singly owned. The ceremonial default-on `metrics` Cargo
feature is also removed from the facade catalog, all 38 live manifests and
generated/package-contract fixtures; metrics profiles remain config-derived.
The dated 0.103 closeout audit initially found two P0 implementation gaps plus
Store-pruning and active-document/count drift. The correction now authenticates
every Root/Coordinator variant before protected work, requires complete
immutable host/CLI protocol identity before transport, compiles Store lanes
only for Store and its exact Root caller, and reconciles active names/counts.
Focused all-target compilation, warning-denied Clippy, binding/auth/replay
tests and four-role artifact scans pass. Only the maintainer-owned package bump
and complete release gate remain.

Scheduled 0.104 owns the `ic-timers` consumer/domain async-job recovery hard
cut, a maintained native-timer adoption guide/fixture and the synchronous
framework-neutral lifecycle participant required to compose Canic with IcyDB.
Repository-only 0.106 B1 evidence may continue, but its final
Candid/timer/state inventory must reconcile before B2.

Scheduled 0.105 now owns framework-neutral caller-bound scoped local
application sessions directly after 0.104; presentation supplies no semantic
dependency. No evidence or implementation batch is promoted by the
resequencing cut.

The scheduled 0.106 B1 is approved to freeze current pool/platform provenance,
measurement/reset protocol, horizon-qualified standby semantics and production
reachability for a 1,000-Canister reserve Fleet. B2 execution is held pending
accepted B1 and separate exact authorization for every external effect.

The scheduled 0.107 line closes replay-safe Coordinator-backed root operating
funding separately from the estate budget. Its proof and mutation require
completed 0.103 and 0.104 plus accepted 0.106 B1 ownership/cost evidence, not
0.106 B2, plus its own proof. Its public work adds Root/Coordinator command and
status variants rather than funding methods.

Scheduled 0.108 owns indexed estates, parallel creation/reset, transfer and
the 10/100/1,000 proof. Opted-in stateful roles must produce an immutable,
bounded application retirement acknowledgement before ordinary uninstall;
forced removal is separately authorized and permanently marked unqualified.
Scheduled 0.109 then owns one stop-the-world exact predecessor/successor
adoption before stateful production claims. Scheduled 0.110 publishes generic
supported observatory views/rendering for downstream Prequel Wars without a
game dependency. Estate-budget replenishment, a product-frontend delivery
handoff, transport, Workers, authentication profiles, blob/archive storage and
Motoko remain ideas.

## Current Decision

Diagnostic registration shipped in `v0.102.2`. Codes identify semantic causes;
typed callers retain handling. Only public, retrievable operator,
durable-evidence or machine-decision boundaries qualify. Projection is explicit;
masked reasons without an independent exact owner stay local, required dynamic
data stays typed, and nonessential context creates no infrastructure. The 991
candidate was replaced rather than retired, and B1 evidence has no allocation
authority. Activation requires one admitted release set; do not add compatibility
decoding, message fallback, diagnostic version or observability machinery.

0.103 B2-B7 source mutation is complete. The maintainer explicitly authorized
B4 mutation on 2026-08-17 and then accepted the bounded 32-Root/nine-Coordinator
variant correction exposed by implementation. Component provisioning now accepts one intent,
self-advances privately and is observed through operation status; Root and
Coordinator command ingress also enforces the selected variant's exact payload
limit. The correction retains scale-out synchronization, Registry
acknowledgement and the two external Root-deletion evidence outcomes, while
Root snapshot reads reuse Registry status, Coordinator removal polls Root
operation status and pre-adoption bytes move to the B5 Store lanes. The atomic
Root/Coordinator and managed/Store caller/fixture cuts, generated role Candid
and exact operation authority now pass focused evidence. Managed profiles
expose only their cfg-selected command/status variants; Store exposes
command/status plus its two admitted byte lanes. Cross-cutting presentation,
legacy-emitter deletion, current-surface residue guards and the count/Wasm
closeout are complete. The maintainer-owned release/version flow is the
remaining 0.103 boundary. 0.104 remains sequenced behind the published 0.103
boundary. 0.104 now also owns native downstream timer adoption and synchronous
lifecycle composition. 0.105 local authorization follows it without an
observatory dependency. 0.106 remains evidence-only until accepted B1,
0.103/0.104 reconciliation and an approved external run plan. 0.107 requires
its accepted inputs and own proof; 0.108 requires completed 0.107 plus
application-retirement evidence; 0.109 requires accepted 0.108 and an exact
released predecessor; and 0.110 requires accepted 0.109 closeout. Deferred
ideas authorize none of these mutations.

## Validation

The retained `v0.101.53` `CANIC-WASM-001/v3` baseline, exact final-tag
`v0.102.2` artifact identities and focused correction checks
cover ledger generation, released identities, raw/registered separation,
projection, host lookup, compact Candid, typed mappings and generic
current/retired catalogue invariants. Core, control-plane, host, CLI,
Coordinator and Wasm Store targets compile. Representative `app`, Root,
Coordinator and Store release builds pass integrity and bounded absence scans;
all data sections shrink, but only Root shrinks overall, so no causal size claim
is made. Correction-specific unit, Candid, host/CLI rendering, canonical
Coordinator/Store build and warning-denied package checks pass. The complete
2026-08-16 `make validate` remains historical push
evidence and is not rerun during focused development.

The open CI-reliability batch removes the contradictory tag-only green signal,
uses Cargo as the sole live package-version authority, adds a post-bump release
candidate guard, gates expensive jobs behind preflight/security and reports
ordinary versus serial PocketIC timing separately. Local validation and CI's
preflight, security and Rust-check jobs now collect every independent failure
inside their barriers, retain complete logs and do not admit expensive work
after a cheap failure.
Workspace tests continue across selected binaries and suites, while the serial
PocketIC group preserves one warm Wasm build state instead of clearing it
between suites. Configured deployment builds collect invalid roles before
compilation and ask Cargo to continue across independent package failures.
Exact release publication
also disables implicit followed-tag pushes; the historical-tag deletion helper
now verifies the remote boundary before removing local refs. A disposable Git
fixture covers remote rejection, exact local/remote deletion and non-
resurrection from the cleaned clone; workflow permissions, fixed runners and
CI ownership are guarded. Targeted actionlint, ShellCheck, release/current-
document guards, release-flow tests and plan-only test-lane checks pass; the
complete suite has not been rerun.

Separate timer-recovery checks remain useful pre-0.104 evidence but do not
establish completion of the 0.104 ownership hard cut.

Focused 0.103 B3 validation builds Runtime-only, Sharding/AutomaticTopup,
Root signer/non-signer and Store profiles through the canonical artifact
builder. Exact positive/negative Candid selectors and referenced types pass,
as do the native reserved-`canic_status` collision, incompatible-feature
rejection, cfg ownership and thin-`start!` guards. The complete suite was not
rerun.

Authorized 0.103 B4 work emits only `canic_command` and `canic_status` for Root
and Coordinator, retains exact domain-owned operation identities, enforces
variant-specific payload bounds and advances Component provisioning through a
private reconciler. The Component advance/publication/activation phase variants
and their replay rows are removed. The corrected register admits 32 Root and
nine Coordinator command variants. The three temporary Root Store-staging
variants and five Coordinator snapshot/removal phase variants are deleted with
their callers; pre-adoption Store staging, participant-specific operation
status and the autonomous Component-provisioning and Root-removal PocketIC
journeys pass focused checks.

Focused 0.103 B5-B7 validation covers exact managed/Store caller and Candid
cuts, Store bootstrap/reverification, host/CLI decoding fixtures, the
then-current application presentation, local-only status emission, the current
endpoint allowlist and all role replay manifests. Seven representative
generated services expose two Canic methods per ordinary role and four for
Store. The four-profile Canic
total is ten instead of 188; representative fast-profile Wasm hashes and sizes
are retained only as current artifact identity because no isolated same-source
pre-cut pair exists. Post-closeout cleanup removes raw pre-cut DID snapshots,
regenerates the three still-configured stale local role artifacts and verifies
all 19 remaining repository/build-local DIDs parse with zero retired Canic
method declarations. The focused 35-test protocol-surface suite passes. The
empty `metrics` feature cleanup also passes locked metadata resolution, core
catalog parity, host package-contract and generated Store-wrapper tests,
facade manifest/documentation guards and CLI medic role-contract fixtures.
The complete suite was not rerun.

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
`100..=150 Bcycles/second` trace over every historical day yields strong
correlations. The 26-pulse controlled rise fits a `4,200.842`-second
display-gain denominator with `R² = 0.999475`. The observed tail begins at
timestamp `1786922200`, one 100-second sample after the first pulse reaches
3,600 seconds of age. Its successive approximately `0.3 Bcycles/second` losses
separate the measured gain from 3,600-second visible support. Ten-second onset
evidence shows attribution beginning about 10 seconds after execution and
becoming complete by about 60 seconds; the controller conservatively leads the
label by one 100-second control step. Remaining pulse expirations are still the
acceptance gate.

The local candidate hard-cuts the obsolete 10× scale into a direct global
contract: `30 Bcycles/second` conservative background credit,
`100 Bcycles/second` floor, `50 Bcycles/second` relief, a `4,201`-second gain
denominator, `3,600`-second support, `100`-second control-grid phase lead and
`500 Bcycles/second` hard rate ceiling. Its exact integer plan is 35 pre-roll
plus 864 waveform pulses totaling `9,481,510,455,119,000 cycles` with digest
`dc1cc6ba53470e0f4abf8045224c8a9bb92516b86e458e9238d4428def3e13d9`.
Simulator/unit checks, warning-denied Clippy, Wasm-target compilation, strict
Candid equivalence and targeted PocketIC funding, exhaustion, exact first
waveform receipt, complete 899-pulse execution and Abort evidence pass. This
code remains inert local evidence; it is not reinstall, funding or Arm
authority.

`Arm` is now an immutable staged authority: it can burn only the 35-pulse
pre-roll. Surplus balance cannot enter the drawing. The first waveform pulse
requires a separate exact-digest `AuthorizeWaveform` command whose current
balance covers the remaining pre-roll, first waveform pulse and retained
reserve. Every later pulse separately preserves that reserve. Focused PocketIC
proves an absent or minimally underfunded continuation rejects, a partially
funded continuation stops before its first unaffordable pulse, Abort prevents
later pulses and a fully funded continuation completes all 899 receipts.

The separately authorized global attempt used that exact
`dc1cc6ba53470e0f4abf8045224c8a9bb92516b86e458e9238d4428def3e13d9`
plan. It began pre-roll at `2026-08-17T02:30:00+02:00`, began the chart at
`03:30`, reached receipt 535 and burned exactly
`5,859,496,546,135,400 cycles`. At `16:36:50`, an unrelated burn on Subnet
`brlsh-zidhj-3yy3e-6vqbz-7xnih-xeq2l-as5oc-g32c4-i5pdn-2wwof-oae`
added approximately `180 Bcycles/second` to the global canvas while the owning
Subnet remained on the expected trace. The maintainer classified the 24-hour
image as spoiled and explicitly ordered Abort. Independent controller status
confirmed terminal `Aborted` / `ControllerAbort`, 535 receipts and no later
intentional burn authority.

**Preserved mainnet asset:** canister `w47na-gaaaa-aaaad-qmclq-cai` retained
`2,589,936,553,122,558 cycles` at final verification, approximately
`2.590 Pcycles`. The controller identity retained only `0.00010000 ICP`.
Those cycles cannot be converted back into ICP, but they remain useful funding
for later authorized canister work. Do not reinstall, delete or otherwise
dispose of this canister without an explicit plan for that balance.

## Next Action

Run the maintainer-owned minor release flow to advance the root package
authority from `0.102.2` to `0.103.0`. The open `0.103.0` changelog records the
completed B2-B7 hard cut, but no agent-owned version, tag or push is authorized.
Begin 0.104 only after that release boundary is published. Do not rerun the
broad census/full suite or reopen 0.102 with JSON, generic handling metadata,
observability infrastructure, compatibility decoding, B1 test coupling or
retired 991 rows.

For the standalone waveform idea, retain the terminal canister and its
approximately `2.590 Pcycles` without further effects. The completed attempt
proved the controlled trace was visible but also proved the global public
canvas can be spoiled by unrelated Subnet burn. Any transfer, reinstall,
deletion, mint, top-up or new Arm requires an explicit disposition or run plan
and separate mainnet authorization; passing local checks cannot supply it.

Design roots retain authority; supporting evidence lives under `docs/audits/`.
