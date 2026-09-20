# CANIC-002 cost evidence — 2026-09-20

The maintainer accepts the cycle-attribution follow-up in Toko Miner's
`docs/upstream/canic.md`, read without mutating that repository. The live audit
attributes most observed backend balance consumption to three application
shards; it does not establish that timer work instructions account for those
cycles or that Canic chose the application's watchdog cadence.

## Upstream decision and implemented scope

Extend existing bounded source measurements and the host's optional private
snapshot. Keep timer scheduling, sampling cadence, funding ownership, publication
configuration and public renderer unchanged. Split timer scheduler/work
instructions and completed measurement counts, expose scheduler starts beside
work starts, and collect existing balance/grant/timer evidence with source
timestamps and a final heap/version anchor. Ordinary snapshots add no calls;
`--costs` adds at most four reads per retained role under the existing deadline.

Preserve stale, disabled, unavailable, truncated, saturated and reset evidence.
A restart during collection receives a structured limitation. Values remain
exact decimal integers in private JSON; public reports cannot leak the new
private evidence. There is no instruction-to-cycle conversion, bill, savings
percentage or synthetic zero for missing measurements.

The continuation adds `canic observatory compare before.json after.json --out
comparison.json`. It reads bounded saved files locally, without workspace
discovery or remote calls. Exact recorded Fleet/environment/network, retained
authority and role bindings must agree. The output retains recorded authority
and parent identity; saved JSON is evidence, not an authenticated receipt.

Each role reports independently qualified balance movement, grants received
from its recorded parent and grants sent by that role. A known-grant-adjusted
decrease is available only when all three actual source intervals match.
Arithmetic uses signed decimal magnitudes across the full u128 cycle range;
opposing terms cancel before checked addition. Missing rows never become zero.
Stale, truncated, malformed, saturated, reset and nonadvancing evidence retains
typed unavailable results without hiding independently usable measurements.
The comparison does not calculate callback rates or extrapolate daily burn.

## Owner gaps and return handoff

Canic now pins published ic-timers 0.8.0. Its `TimerSnapshot::registration_id()`
provides the requested source-owned runtime epoch and checked registration
sequence. Cancellation preserves it; unregister/re-register changes it even
within the same runtime epoch. The callback `generation()` remains unrelated
to counter continuity. The upstream contract also defines saturation and
completed-measurement limits; interrupted callbacks do not fabricate samples.

Canic now carries that identity alongside each cached timer performance total
and completed-sample count. The same inventory observation supplies values and
identity; four optional reads per role and the existing sampling cadence remain.
Private snapshot projection preserves the fields, and offline comparison
qualifies independent instruction/sample deltas by exact registration, source
time, freshness, completeness, type and saturation. Reset followed by regrowth
is rejected. Aggregate callback events remain gauges; completed samples do not
account for interrupted callbacks or establish callback rates. No parallel
tracking owner is introduced, and public history does not derive timer rates.
IcyDB 0.259.6 still resolves ic-timers 0.7.1 in external composition fixtures;
their shared-inventory qualification awaits a matching published IcyDB dependency.
The maintainer explicitly accepts this temporary test-only mismatch and leaves
the IcyDB pin unchanged; it is not a blocker for Canic's dependency update.

Funding grant counters do not cover every balance-changing transfer. Safe
consumption attribution additionally needs interval-aligned balances, complete
incoming/outgoing transfer evidence, source continuity, and an explicitly
bounded measured estate. Parent counters alone cannot account for external
deposits, all attached-cycle messages or source-to-new-canister transfers.
The private report keeps `transfer_coverage_incomplete` and
`unattributed_execution_message_storage` visible instead of claiming burn.

The source-evidence implementation can be reviewed independently. The entire
CANIC-002 cycle-attribution extension remains open for complete transfer
evidence and downstream actual-balance verification. The
local comparison admits aligned source windows; it cannot manufacture alignment
when independently cached measurements cover different intervals.
An exact timer optimisation claim requires a controlled workload comparison.
No deployment, sibling mutation, release transaction or Git publication is
part of this work. The existing unrelated Cargo.lock update is preserved.

The .110 line already exceeds the twelve-release cadence guideline. Retain this
bounded observability follow-up to the published Fleet surface in its current
open patch instead of opening a new minor without the human closeout gate.
This decision does not sequence an attribution redesign or a new minor.

## .32 source and comparison qualification

All 81 focused native tests pass: 13 host observatory cases, the CLI private
collection contract and 67 core metrics/public-cache cases. They cover private
projection, bounded collection, independent failures, restart limitations,
full-width cycle values, saturation, source-page limits and safe counter deltas.

All nine existing `timer_authority` PocketIC cases pass on the final test source
in 28.40 seconds (32-second runner with cached fixtures). The initial run takes
152.85 seconds, with a 250-second runner including native compilation and Wasm
preparation; these are different cache conditions, not an optimisation comparison.
The existing runtime fixture
now exercises one retained watchdog on explicit request; both scheduler and
work instruction totals and completed measurement counts are positive, and
public timer rows remain gauges. The same target preserves authorization,
same-release restoration, native timer custody/capacity and bounded publication
with rejected-sample recovery. No additional Fleet setup or test target is added.

The scheduled sampling maximum is 18,925,601 instructions; the retained-history
case reaches 287 points with 4,097,496 reserved bytes within its 8 MiB limit.
These are fixture observations, not a cycle price, a before/after improvement
or downstream workload measurement.

Warning-denied all-target/all-feature Clippy passes for core, host and CLI;
scoped Clippy also passes for the changed integration target and runtime-probe
library. Changed-file formatting and diff checks pass. Logs are
`.tmp/canic002-{host-tests,cli-tests,core-tests,pocketic,pocketic-final,clippy,fixture-clippy,runtime-probe-clippy}.log`.
The local compiler-cache socket was unavailable, so these checks used the
ordinary compiler. PocketIC needed local loopback access. No broad validation
gate, version bump, publication or live deployment was run.

The offline-comparison continuation passes all 25 focused host/CLI tests:
23 host observatory cases and two CLI cases. New cases cover exact binding
changes, parent/child grants, independent failures, reset followed by regrowth,
source-time mismatch, repeated cached points, malformed rows, saturation and
full-width signed arithmetic. The CLI test uses a nonexistent ICP executable,
checks private output permissions and proves existing output is not overwritten.
The combined log is `.tmp/canic002-compare-final-tests.log`; initial separate
runs are `.tmp/canic002-compare-{host-tests,cli-tests}.log`. This continuation
changes host/CLI code only and does not require another PocketIC run.

Both recursive CLI-help tests pass, checking functional command ordering and
concise examples throughout the command tree. The log is
`.tmp/canic002-compare-help-tests.log`. Final warning-denied all-target/all-feature
Clippy passes for host and CLI (`.tmp/canic002-compare-clippy.log`); changed-file
formatting and diff checks pass. The unrelated lockfile change remains intact.

The bounded source-evidence and offline-comparison implementation and both .32
changelog surfaces are ready for review; packages remain .31. The complete
cycle-attribution batch is not yet push-ready: timer continuity, complete
transfer evidence and downstream verification remain open. An unrun broad gate
is not this blocker.

## Published timer dependency qualification — 2026-09-20

The maintainer confirms .32 publication and requests ic-timers 0.8.0. Both
Canic lockfiles now resolve the published package with checksum
`c1474fd7c9bcc237404173c2689c4175e61ef2c9670bccf539ca10ccd193d132`.
The standalone lock also synchronizes existing Canic path-package versions to
.32; no Canic package version is bumped. IcyDB's accepted test-only 0.7.1
dependency remains unchanged.

All 13 focused all-feature core timer regressions and all nine existing
PocketIC timer-authority cases pass. The PocketIC target takes 138.37 seconds
including rebuilt Wasm fixtures; the runner takes 239 seconds including native
compilation. It exercises application cancellation/recurrence, runtime timer
custody/restoration, identity/capacity rejection, scheduler/work measurements,
protected observability and public sampling/history bounds. Logs are
`.tmp/ic-timers08-core-tests.log` and `.tmp/ic-timers08-pocketic.log`.
Diff checks pass. No Rust source adaptation was required.

The complete dependency-update batch and both .33 changelog surfaces are ready
for the maintainer-selected release flow. This qualifies the dependency under
existing Canic behavior, not registration-aware comparison or full cycle
attribution. No agent-started broad gate, release transaction, publication,
deployment or sibling edit ran.

## Registration-evidence integration qualification — 2026-09-20

The maintainer accepts the source/host integration into the same .33 batch.
Core projects `TimerCounter` values and source registration metadata from one
inventory observation. Private schema-1 JSON retains this as `timer_counter`;
comparison adds `roles[].timer_measurements` with exact decimal amounts, units
and source intervals. Public history retains raw typed observations without
timer deltas. Aggregate callback events remain gauges and receive the explicit
`aggregate_timer_callbacks_unqualified` limitation. Scheduling and the four-read
optional collection budget are unchanged. Endpoint performance remains available
when timer inventory is unavailable; absent timer measurements remain unknown.

All 59 focused native/contract checks pass: 28 core public-metric cases,
28 host observatory cases, two CLI offline/private-output cases and canonical
public-metric Candid equality. They cover complete registration equality,
reset/regrowth, missing counters, repeated and mismatched source times,
freshness/truncation, canonical full-width numbers, explicit and sentinel
saturation, malformed rows, independent cycle results and JSON preservation.

All nine existing PocketIC timer-authority cases pass in 148.19 seconds,
including rebuilt Wasm fixtures (243-second runner including native compilation).
The retained watchdog proves cancellation preserves registration and measurements;
a real callback trap fabricates no completed work sample; unregister/re-register
changes the sequence within the same runtime epoch even after counters exceed
the prior sample. The target also retains same-release restoration, timer
custody/capacity, protected observation and bounded public sampling/history.

The final fixture reports a scheduled sampling maximum of 21,963,109 instructions,
above the unchanged 20-million advisory reference. The full-history sample uses
20,974,053 instructions; 287 points retain 5,277,144 reserved bytes under the
8,388,608-byte cap. Registration metadata enlarges history points. These are
current fixture observations, not a matched overhead benchmark, timer bill or
downstream savings claim; no threshold or memory budget was raised.

Warning-denied all-target/all-feature Clippy passes for core, host and CLI;
scoped Clippy passes for the runtime-probe library and timer-authority test target.
Changed-source formatting and diff checks pass. Logs are
`.tmp/canic002-registration-{core,host,cli,candid,clippy,probe-clippy,test-clippy,pocketic}.log`.

The complete expanded .33 batch and both changelog surfaces are ready for the
maintainer-selected release flow. Canic versions remain .32; the accepted
IcyDB test-only mismatch is unchanged. Complete transfer attribution and actual
downstream balance verification remain open. No broad gate, version bump,
Git publication, deployment or sibling edit ran.
