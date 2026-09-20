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

ic-timers 0.7.1 owns both measurement phases, but its public `TimerSnapshot`
does not expose registration continuity. `TimerEpoch` identifies the runtime,
whereas unregister/re-register can replace a timer's counters within that same
epoch. The callback `generation()` changes during ordinary scheduling and is
not a registration token. Even nondecreasing sampled counters cannot exclude a
reset followed by additional activity. Canic therefore retains gauges and
explicitly declines callback-rate and interval-instruction calculations.

The timer-owner follow-up is a source-owned registration identity available on
each snapshot, changing on every registration/reset and distinct from callback
generation, together with saturation and epoch semantics. Qualify cancellation,
unregister/re-register with the same identity, reset followed by regrowth beyond
the previous sample, interrupted callbacks and runtime restarts. This is a
dependency API request, not authority to edit a sibling repository or add a
parallel tracking owner in Canic.

Funding grant counters do not cover every balance-changing transfer. Safe
consumption attribution additionally needs interval-aligned balances, complete
incoming/outgoing transfer evidence, source continuity, and an explicitly
bounded measured estate. Parent counters alone cannot account for external
deposits, all attached-cycle messages or source-to-new-canister transfers.
The private report keeps `transfer_coverage_incomplete` and
`unattributed_execution_message_storage` visible instead of claiming burn.

The source-evidence implementation can be reviewed independently. The entire
CANIC-002 cycle-attribution extension remains open for timer-owner continuity,
complete transfer evidence and downstream actual-balance verification. The
local comparison admits aligned source windows; it cannot manufacture alignment
when independently cached measurements cover different intervals.
An exact timer optimisation claim requires a controlled workload comparison.
No deployment, sibling mutation, release transaction or Git publication is
part of this work. The existing unrelated Cargo.lock update is preserved.

The .110 line already exceeds the twelve-release cadence guideline. Retain this
bounded observability follow-up to the published Fleet surface in its current
open patch instead of opening a new minor without the human closeout gate.
This decision does not sequence an attribution redesign or a new minor.

## Qualification

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
