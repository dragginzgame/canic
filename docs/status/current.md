# Current Status

Last updated: 2026-08-19

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the active design, source, validation or changelog material needed for the
current task.

Historical handoffs: [through 2026-06-30](archive/2026-06-30-precompact.md),
[through 0.90.2](archive/2026-07-13-precompact.md) and
[through 0.101.52 Q4](archive/2026-08-12-precompact.md).

## Current Release

- The root `Cargo.toml` is the sole live workspace package-version authority;
  this handoff deliberately does not mirror its value. Immutable local and
  remote tag refs own publication truth, and tag presence determines whether a
  changelog section is an open draft or a published patch.
- The completed role-owned Candid release owns the 161-reason register, typed mappings,
  compact public wire and flat `code + name` release baseline. B1 census/count
  coupling is absent from permanent tests. The open closeout-correction batch
  makes that released baseline tamper-evident, preserves the four genuine
  caller-continuation values and routes ordinary host failures through the
  prose catalogue. Active checklist:
  [0.102 compact diagnostic codes](../design/0.102-compact-diagnostic-codes/status.md).
- Scheduled application-safety and estate path: [0.103 role-owned Candid surface](../design/0.103-role-owned-candid-surface/status.md), [0.104 timer ownership plus synchronous lifecycle composition](../design/0.104-ic-timers-consumer-hard-cut/status.md), [0.105 framework-neutral local application authorization](../design/0.105-framework-neutral-local-application-authorization/status.md), [0.106 platform qualification](../design/0.106-fleet-estate-platform-qualification/status.md), [0.107 Coordinator-backed root funding](../design/0.107-coordinator-backed-root-funding/status.md), [0.108 reusable estates plus application retirement](../design/0.108-fleet-subnet-canister-estates/status.md), [0.109 stateful Fleet release adoption](../design/0.109-stateful-fleet-release-adoption/status.md) and [0.110 generic Fleet observatory](../design/0.110-fleet-observatory/status.md). External [Prequel Wars](https://github.com/dragginzgame/prequel-wars) replaces the checked-in Skynet App as the flagship demonstration. Other future concepts are [unnumbered ideas](../design/ideas/README.md).
- Release boundary: every pre-1.0 transition is reinstall-only. Every
  Canic-owned canister in a Fleet must come from one admitted release set
  before activation. Same-release interruption recovery, exact retry, backup
  and restore remain required. Scheduled 0.109 is the first explicit
  one-predecessor-to-one-successor exception; no current release is adoptable
  until that line is implemented and published.

## Current Progress

The compact diagnostic and role-owned Candid lines are published. Their
read-only closeout audit confirmed the compact wire, typed runtime, host-only
catalogue and Wasm boundary, then closed one dynamic data-loss defect plus
released-baseline, ordinary-rendering and release-reliability defects. One
typed cycles-funding preflight response owns the four genuine caller-
continuation values; the other 64 provisional B1 owner proposals are
reconciled as local typed state, caller-derived data, existing authority or
deliberately dropped operator convenience. The release guard reads the exact
released reason-ledger Git object, including retirement state, and the central
host decoder renders known diagnostics automatically.

B1's 2,895 labels exposed the rejected unreleased 991-row
tuple taxonomy. The accepted frontier is 161 registered causes and ten local
typed families. All
four phases—Register, Map, Cut, Clean and measure—are complete: lossless raw
codes, producer-only registered codes, `Error { code: u16 }`, code-first
`InternalError`, and host-only prose. The flat release guard freezes only
released `code + name`; it neither reads B1 evidence nor freezes row counts,
and the ledger rejects fields beyond its six maintained fields.

The completed 0.103 line owns role methods and autonomous operations. Its B1 evidence
was accepted on 2026-08-17: the immutable `v0.102.2` baseline freezes 207 methods across
representative Root/managed profiles and canonical Coordinator/Store
interfaces, separated into 188 Canic-owned, three external-standard and 16
fixture-owned methods. B2-B7 are complete in the published source: the closed
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
tests and four-role artifact scans pass. The human-owned package bump, complete
release gate, annotated tag and atomic push are complete.

Scheduled 0.104 owns the `ic-timers` consumer/domain async-job recovery hard
cut, a maintained native-timer adoption guide/fixture and the synchronous
framework-neutral lifecycle participant required to compose Canic with IcyDB.
Its exact 0.103.0 source/provider baseline, consumer and native-claim census,
memory-ID-60 disposition and downstream propagation contract are complete as
B1 evidence and were accepted on 2026-08-18. B2 was accepted the same day: the
public application timer macros, facade, handle and transient claims are
deleted without aliases, callbacks consume native provider vocabulary, and
the runtime probe owns its application registrations directly. B3 is accepted.
Memory ID 60 now contains only checked domain
attempt leases and cycle-top-up's exact retry generation; pending timer
commands, schedule ownership, generic recovery deadlines, copied retry streaks
and terminal provider state are deleted. The exact worst-case record encoding
is 589 bytes. B4 is implementation-complete: auth renewal, automatic cycle
top-up and placement acknowledgement now own lazy native registrations and
exact domain recovery. The representative provider inventory falls from seven
rows to five and its measured interval path uses about 5.1% fewer instructions.
Same-builder product Wasm is 18,030 raw and 5,457 gzip bytes larger than B3,
so no B4 size win is claimed. B5 is accepted: the remaining central claim and
participant registries are deleted, pool/lifecycle/snapshot owners retain
exact native custody, the representative interval path falls another 1.9514%
and the four-role product total falls to 19,123,973 raw and 4,959,574 gzip
bytes. B6 is accepted on locked `ic-testkit 0.8.8`: the maintained native
guide and direct-provider fixture replace the removed facade examples, and
ordinary managed, Root and local start macros accept one safe paired
synchronous lifecycle participant after Canic restoration and before deferred
work. Exact product raw bytes remain unchanged from B5; deterministic gzip
falls one byte and the measured interval differences remain noise-scale.
Repository-only 0.106 B1 evidence may continue, but its final
Candid/timer/state inventory must reconcile before its next source batch.

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
closeout and human-owned release flow are complete. The maintainer accepted
0.104 B1 and B2, then continuation accepted B3-B8. B3 hard-cuts
generic timer recovery into four closed
domain async-job fences, retains a generated exact retry identity only for
cycle top-up and removes every stable provider mirror. Its bounded-state,
property, interruption and exact Wasm/provider evidence is accepted. B4 moves
auth, cycles and placement out of central fixed-job selection while preserving
snapshot custody and exact recovery; its current-graph evidence passes on
the then-locked `ic-testkit` 0.8.7. B5 deletes the remaining pool/lifecycle/
snapshot registries, restores role-pruned authority linkage and passes on
`ic-testkit` 0.8.8. B6 publishes direct native adoption and adds the paired
synchronous lifecycle participant with exact restore-before-participant-before-
defer ordering, Prepared/inactive execution and trap rollback. B7 composes that
participant with exact published IcyDB 0.230.2 and proves one timer provider,
one lifecycle export pair, separate reconstructed rows, rollback and corrected
retry. B8 replaces lexical counts with semantic ownership classification
across applications, crates and executable fixtures, then closes the graph,
inventory, documents and measurements. The 0.104 implementation batch is
complete.
0.105 local authorization
follows it without an observatory dependency. 0.106 remains evidence-only
until accepted B1, 0.103/0.104 reconciliation and an approved external run
plan. 0.107 requires
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

The release-reliability foundation removes the contradictory tag-only green
signal, uses Cargo as the sole live package-version authority, adds a post-bump
release candidate guard, gates expensive jobs behind preflight/security and
reports ordinary versus serial PocketIC timing separately. Local validation
and CI's preflight, security and Rust-check jobs now collect every independent
failure inside their barriers, retain complete logs and do not admit expensive
work after a cheap failure.
Workspace tests continue across selected binaries and suites, while the serial
PocketIC group preserves one warm Wasm build state instead of clearing it
between suites. The internal harness now consumes `ic-testkit` 0.8.8's
collect-all Wasm batch report and compatible-spec input snapshot reuse, so one
bad package no longer prevents later independent Wasm acquisitions. Configured
deployment builds collect invalid roles before compilation and ask Cargo to
continue across independent package failures.
Exact release publication
also disables implicit followed-tag pushes; the historical-tag deletion helper
now verifies the remote boundary before removing local refs. A disposable Git
fixture covers remote rejection, exact local/remote deletion and non-
resurrection from the cleaned clone; workflow permissions, fixed runners and
CI ownership are guarded. Targeted actionlint, ShellCheck, release/current-
document guards, release-flow tests and plan-only test-lane checks pass; the
complete suite has not been rerun.

Focused `ic-testkit` adoption checks compile the internal harness,
integration package and Saltz test target under the locked dependency graph.
The payload-limit PocketIC suite passes with structured standalone-pool
outcomes and exact cache paths. A warm three-spec lifecycle acquisition reused
one compatible input snapshot: only the first spec resolved Cargo inputs, the
two later specs reported zero input-resolution time, and the focused lifecycle
test passed.

The current follow-up locks `ic-testkit` 0.8.8, hard-cuts the removed anonymous
diagnostic printer, passes exact controllers into labeled collect-all bounded
status/log reports and aggregates same-tick readiness query failures across
deployment targets. The compact failure report now exposes each diagnostic
target's elapsed time and the total sequential batch time. Failed Wasm entries
provide their caller label, index, typed error, primary phase, partial phase
timings and elapsed time directly to Canic's aggregate error. Wasm progress,
success and failure reporting no longer
depend on a parallel package-label slice, and selected-graph semantic identity
permits reuse across unrelated host-only workspace dependency changes. Version
0.8.8 makes the optional cross-call session's source-immutability assertion
explicit; Canic deliberately stays on ordinary per-call validation because it
does not hold a genuine repository write-exclusion guard. Every
repository PocketIC builder now uses bounded instance construction against one
runner-owned server shared across the serial lane. The runner verifies the
exact binary checksum, bounds port readiness, retains startup output in its
private scratch and owns cleanup; a cheap source guard rejects direct unbounded
startup.
The underlying 0.8.4 migration passed isolated affected-package compile,
warning-denied Clippy, three artifact tests and the two-test payload-limit
PocketIC suite; that cold run finished in 38.99 seconds. After advancing the
lock to 0.8.5, all three direct consumers pass live all-target compilation and
warning-denied Clippy, and the focused artifact tests pass. An isolated
payload-limit rerun compiled and completed its 34.98-second cold Wasm build,
but a concurrently owned PocketIC process prevented a new server from binding;
the run was stopped without claiming runtime evidence. After advancing to
0.8.6, all three direct consumers pass all-target compilation in 36.03 seconds;
warning-denied Clippy passes for the complete internal/Saltz targets and the
three directly affected Canic integration targets, and all three artifact
tests pass. The wider Canic integration Clippy command reaches a separately
owned 0.104 timer test that is 103 lines against its 100-line lint limit, which
this dependency batch does not alter. The payload-limit suite built its new
semantic-key artifact in 28.52 seconds, and the immediate rerun reused it in
436.6 milliseconds. An intermediate use of 0.8.6's managed spawn path then
failed in about 21 milliseconds because the upstream helper
pre-created its `--port-file`; the exact PocketIC 15 binary exits zero and
silently when that path already exists, while a genuinely absent path proceeds
to bind. `ic-testkit` 0.8.7 repairs that upstream spawn boundary with a private
startup directory and absent server-owned port path. The final Canic flow
starts one runner-owned server with a new port path and uses bounded testkit
`connect` calls. All three direct consumers compile on the locked graph, the
four timer-authority journeys pass in 6.89 seconds with one shared server, and
the runner retains and terminates that exact child on every handled exit before
the scratch cleanup fallback runs. Its numeric direct-child port path also
keeps forced cleanup invocation-scoped after an abrupt runner failure. Exact
and deliberately mismatched server binaries are
accepted and rejected respectively, and PocketIC alignment, locked offline
metadata, the workspace inventory/startup guard, release-integrity contract,
cheap current-document guard, focused ShellCheck, formatting and diff hygiene
pass. The complete suite was not rerun.

Focused 0.104 B2 validation removes the public timer facade and compiles the
runtime probe as a direct `ic-timers` consumer. Warning-denied affected-package
Clippy, 1,122 core unit tests, timer inventory guards, pool ownership tests,
facade/endpoint tests and the four-test timer-authority PocketIC journey pass.
The 24-hour simulated window records zero callbacks for idle capability-pruned
cycle top-up, intent cleanup and log retention. The native application
interval reports two work samples, 25,145 latest/maximum and 50,248 total
instructions with zero Wasm or stable-memory page growth; the retained B1
evidence has no numeric pre-cut sample, so this is not presented as a causal
performance delta. The same canonical fast builder produces a four-role total
of 19,416,477 raw and 5,030,322 gzip bytes, increases of 2.7225% and 3.4326%
from `v0.103.0`. The isolated direct-native runtime probe instead shrinks by
9,782 raw and 736 deterministic-gzip bytes. The complete suite was not rerun.

Focused 0.104 B3 validation passes 1,118 core library tests, exact state and
role contract checks, timer workflow and pool tests, warning-denied affected-
package Clippy, both source-inventory guards and all four timer-authority
PocketIC journeys. The expired-business-lease fixture commits one trapped
continuation, admits one takeover after expiry and clears only the successor's
exact attempt. The interval remains at two work samples with 50,179 total
instructions and zero memory-page growth; its sub-percent differences from B2
are not a causal performance claim. The watchdog takeover records one 21,503-
instruction scheduler sample and one 51,476-instruction work sample with zero
memory-page growth, for which no B2 numeric baseline exists. The canonical
four-role product total is 19,398,431 raw and 5,021,151 gzip bytes: 0.0929% and
0.1823% smaller than B2, but 2.6270% and 3.2440% larger than `v0.103.0`. All
four product builds and the final instrumentation-free interruption journey
pass. The complete suite was not rerun.

Focused 0.104 B4 validation passes 1,118 of 1,119 core library tests with one
ignored, both source-inventory guards, warning-denied Clippy for core, control
plane, facade, runtime probe and the timer-authority target, the real restored-
Root snapshot/resume journey and all four canonical product builds. All four
timer-authority PocketIC journeys passed on the B4 source. The representative
inventory falls from seven rows to five because an absent `AutomaticTopup`
capability and empty receipt index reserve no native declarations. Its two-
callback interval path falls from 50,179 to 47,607 instructions. The watchdog
records 21,515 scheduler and 51,221 work instructions, both noise-scale against
B3. The four-role total is 19,416,461 raw and 5,026,608 gzip bytes, 0.0929% and
0.1087% larger than B3 but still 16 raw and 3,714 gzip bytes smaller than B2.
The current `ic-testkit` 0.8.7 graph reruns all four journeys successfully in
6.89 seconds through one runner-owned shared server. The complete suite was
not rerun.

Focused 0.104 B5 validation deletes the remaining central native-claim union
and snapshot/recovery participant registries. Intent cleanup, log retention
and Root Canister-pool maintenance retain exact native custody; one Root
watchdog dispatches expired core and pool attempts. Lifecycle deferrals are
direct remove-when-stopped claims, while Root and Coordinator authority
snapshots use distinct exact paths so Coordinator does not link Root owners.
The three timer-authority journeys pass in 6.04 seconds on locked
`ic-testkit 0.8.8`; exact Coordinator snapshot/restore passes in 11.49 seconds,
and the real restored-Root journey proves the pool/watchdog rows are scheduled
live, unregistered while sealed, reconstructed after live resume and absent
from a restored sealed snapshot. Its two-sample application interval falls
from 47,607 to 46,678 total instructions with zero memory-page growth. The
canonical four-role total falls 292,488 raw and 67,034 gzip bytes from B4 to
19,123,973 raw and 4,959,574 gzip bytes. Coordinator falls about 6.0% from B4
after a first measurement exposed and the role split removed accidental Root
workflow linkage. The total remains 1.1750% raw and 1.9779% gzip above
published `v0.103.0`. Targeted compilation, warning-denied Clippy, source
guards, pool/protocol tests and all nine canonical fast artifacts pass. The
complete suite was not rerun.

Focused 0.104 B6 validation passes both public compile-fail examples, the six
lifecycle-boundary guards, the managed endpoint and protocol-surface guards,
and warning-denied Clippy for every touched facade, fixture, harness and
integration target. Managed Prepared/repeated-upgrade behavior, participant
trap rollback with an unchanged committed module hash, corrected retry and the
real Root participant path pass focused PocketIC checks. The final four-test
timer-authority journey passes in 7.86 seconds on locked `ic-testkit 0.8.8` and
PocketIC 15. The managed, Root and runtime-probe artifacts retain exact B5
Candid and normalized canister-export sets with one `canister_init` and one
`canister_post_upgrade` each. The canonical product total is 19,123,973 raw
and 4,959,573 gzip bytes: raw is exactly unchanged from B5 and gzip is one byte
smaller. Its two-sample interval records 46,593 total instructions with zero
memory-page growth; the sub-percent B5 differences are noise-scale. The
complete suite was not rerun.

Focused 0.104 B7 validation resolves one `ic-timers 0.6.1` package for Canic
and exact published `icydb = "=0.230.2"`. Normal and trapping composition
artifacts retain identical Candid and exactly one lifecycle export pair. The
focused PocketIC journey passes in 19.74 seconds, proving Prepared and Active
same-release reconstruction, distinct Canic/IcyDB rows, participant rollback
with an unchanged committed module hash and corrected retry. The final
test-only probe is 5,959,481 raw and 1,519,923 deterministic-gzip bytes; it
changes no shipped product role. Locked affected-package compilation,
warning-denied Clippy, the lifecycle payload unit check and the workspace-test
inventory guard also pass. The complete suite was not rerun.

Focused 0.104 B8 validation replaces the lexical call-count guard with a
45-file semantic ownership contract and closes the exact one-provider lock and
nine-manifest declaration set. All six semantic provider, ownership,
documentation, raw-access, wait and snapshot guards pass. The four
timer-authority journeys pass in 14.01 seconds, and the exact isolated
measurement journey passes warm in 3.24 seconds. It reports four managed rows,
one scheduled, no top-up row and the exact B6 interval result: two work
samples, 46,593 total instructions and zero memory-page growth. The focused
Root restore journey passes in 47.50 seconds and reports four declared rows,
two scheduled while active and zero while sealed, with no top-up row. The
fresh-target four-role product total remains 19,123,973 raw bytes exactly and
moves by seven compression-noise bytes to 4,959,566 gzip. Targeted
warning-denied Clippy passes for the semantic guard, timer target and internal
Root harness. The six lifecycle-boundary guards, changelog governance and
current-document semantics also pass. The complete suite was not rerun.

The follow-up test-harness correction retains standalone package/profile Wasm
bytes once per test process and returns typed, named role-overview observations
from pooled readiness. The formerly failing restored Component Registry proof
now passes with an 835 ms warm baseline restore. With the then-locked
`ic-testkit` 0.8.2 graph warm, the five-test blob-storage suite completes in
12.45 seconds;
its two Wasm acquisitions take 824 ms in total and report zero builds plus two
reuses. This is an observed 3.50-second, 21.9% reduction from the retained
15.95-second warm run, not an isolated causal benchmark. Its separate cold run
completes in 45.42 seconds, including 32.258 seconds of artifact acquisition.
The four-test timer-authority suite completes warm in 7.18 seconds with 414 ms
of artifact acquisition, zero builds and three reuses; its cold artifact build
is retained separately from that runtime signal.

The exact previously failing active-Registry deployment proof passes in 30.81
seconds warm. Store, Root and issuer Wasm reuse take 956, 731 and 475 ms,
respectively. Its 268.25-second cold run includes approximately 165.995 seconds
of one-time artifact builds. This directly exercises controller-bound,
multi-canister readiness diagnostics without conflating the warm deployment
path with compilation.

No production Canic canister source changed in this follow-up, so there is no
product-role Wasm size delta to claim. The touched `runtime_probe` test fixture
remains exactly 3,673,361 raw bytes; deterministic gzip moves from 910,593 to
910,615 bytes, a 22-byte or approximately 0.0024% increase, while the artifact
hash changes with the build-input fingerprint. That is test-fixture compression
noise rather than a shipped-role size regression.

The completed B2-B8 timer checks establish the public surface, stable domain
recovery, fixed-owner custody, pool/lifecycle/snapshot propagation, downstream
adoption, synchronous lifecycle composition and exact IcyDB qualification.
Semantic ownership and measured closeout are complete.

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

The bounded CI/test-harness feedback batch is implementation-complete with
targeted compilation, warning-denied Clippy, artifact checks, deployment and
timer PocketIC evidence, changelog governance, shell lint and document checks.
The maintainer-owned complete validation and release flow remain separate. No
further 0.104 implementation batch remains. Keep the open 0.104.0 changelog
entry and workspace version under that human-owned flow. Scheduled 0.105 B1
still requires explicit maintainer promotion after accepted 0.104 closeout and
is not authorized by ordinary continuation.
Do not reopen compact diagnostics with JSON, generic handling metadata,
observability infrastructure, compatibility decoding, B1 test coupling or
retired 991 rows.

For the standalone waveform idea, retain the terminal canister and its
approximately `2.590 Pcycles` without further effects. The completed attempt
proved the controlled trace was visible but also proved the global public
canvas can be spoiled by unrelated Subnet burn. Any transfer, reinstall,
deletion, mint, top-up or new Arm requires an explicit disposition or run plan
and separate mainnet authorization; passing local checks cannot supply it.

Design roots retain authority; supporting evidence lives under `docs/audits/`.
