# CANIC-148 sampler investigation — 2026-09-21

Canic's per-series history buffers are a substantial sampler cost. Each series
owns a separately allocated 288-slot ring; updating many series touches many
memory regions. Real Wasm probes show history ring writes dominating index
lookup, with costs consistent with the pinned IC runtime's page metering.
Improving write locality is the next optimization to test.

This is an exploratory investigation, with `run_result: partial`,
`result_validity: valid` for the recorded diagnostics, and finding status `open`.
It is not a frozen-source audit, release qualification, production savings
measurement or an explanation of the exact difference between Toko artifacts.
Temporary probes were removed after recording the results. No production
sampler change or downstream edit is included.

## Trigger and scope

Toko's CANIC-148 entry reports 20,289,111 instructions for its complete scheduled
sampler with 100 application rows, Canic 0.110.33 and IcyDB 0.261.1. Its strict
20M assertion fails. The retained IcyDB 0.261.0 application artifact also fails
at 20,016,138. Those artifacts contain different application source, so their
difference does not isolate an IcyDB regression. These are read-only downstream
observations from `docs/upstream/canic.md` and
`docs/upstream/artifacts/icydb-0.261.1-adoption-2026-09-21.json` in Toko Miner.

Canic's maintained [cost policy](../2026-09-10/metrics-cost-policy.md) treats 20M
as advisory. This investigation changes neither policy nor Toko's assertion.

The inspected Canic owner is
`crates/canic-core/src/model/public_metrics/history/mod.rs`, reached through
`workflow::metrics::publication` and `ops::runtime::public_metrics`. The sampler
and history sources match Toko's Canic .33 base before instrumentation. Current
Canic HEAD is `c2e1c5bb5ef27d5e13b65b8ad9dd042ff2c158c7` (.34), with the already
requested IcyDB .261.1 dependency update present in the dirty worktree.

## Method and provenance

Three successive temporary probe revisions instrumented `performance_counter(0)`
around provider collection, validation, history recording and memory collection.
The second revision split history into lookup/admission, ring mutation and index
shrinking. The third repeated sampling in the same IC message. Each revision
ran the governed `make test-pocketic-case CASE=timer_authority` against real
Fast-profile Wasm and PocketIC 16.0.0. All 10 cases passed in each run; test times
were 170.25, 154.99 and 33.28 seconds respectively. No broad suite ran.

The controller-only diagnostic endpoint installed a synthetic application
provider with 0, 100 or 211 rows. Names were qualified entity-like strings;
values were gauges, source time was current and canister ID was absent. Each
load used a fresh probe canister. Calls advanced time by 301 seconds. The first
revision ran 300 periods per load and retained periods 0, 1, 2 and 299; later
revisions ran three periods. The existing full-window timer test remained in
all runs. At the largest load, application rows plus native rows can exceed the
shared 256-series history cap; this is a capacity stress case.

The [structured record](sampler-cost-investigation.json) retains all 30 printed
observations, exact source/lockfile hashes, patch and local log hashes, tool
identity and limitations. Reproduction patches are independent alternatives:
[baseline](sampler-cost-baseline.patch), [history breakdown](sampler-cost-history.patch),
and [same-message control](sampler-cost-control.patch). They are evidence, not
maintained product changes. Apply only one to the recorded source in a disposable
checkout with the recorded dependency inputs, then use the governed target above.

The experiment used the existing dirty workspace and shared target after checking
for other Canic builds, rather than a frozen isolated product snapshot. Exact
start/end timestamps and a product-tree measurement were not retained. Input
hashes and patches bind the probes, but these limitations prevent treating this
as a formal comparable audit or closeout receipt. Single runs and different
instrumentation revisions must not be presented as an optimized before/after.

## Measurements

At period 299 of the initial probe, history was the largest measured component
for both populated application loads:

| Application rows | Whole sample | All family history | History share | Memory collection |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 6,431,873 | 2,754,250 | 42.8% | 2,375,155 |
| 100 | 12,388,775 | 7,265,813 | 58.6% | 2,227,028 |
| 211 | 18,296,597 | 12,108,530 | 66.2% | 2,213,709 |

All numbers are instructions. Period 2 totals were 6,412,746, 12,817,001 and
18,228,488 respectively. These samples show no large increase from early to
full-window operation; they do not establish a universal bound.

The deeper probe at period 2 localized application history work:

| Application rows | Application history, inclusive | Lookup/admission | Ring writes | Index shrinking |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 4,499,395 | 478,200 | 3,971,400 | 1,607 |
| 211 | 10,415,890 | 1,090,079 | 9,228,454 | 1,607 |

Subphases are contained in the inclusive total; do not add them to it. Ring
writes include constructing/replacing/appending the point and updating latest
source time. Index lookup includes key construction and admission. Instrumentation
itself affects total instructions and allocation layout.

## Why page metering fits

The deep probe's application ring totals have exact decompositions:

- 100 rows: `100 × 514 + 49 × 80,000 = 3,971,400`.
- 211 rows: `211 × 514 + 114 × 80,000 = 9,228,454`.

[PocketIC 16.0.0](https://github.com/dfinity/pocketic/releases/tag/16.0.0)
identifies IC commit `fc21803c3c3a8dd452b3b58b959751c41fecb89c`.
Its [subnet configuration](https://github.com/dfinity/ic/blob/fc21803c3c3a8dd452b3b58b959751c41fecb89c/rs/config/src/subnet_config.rs)
sets page overhead to 5,000 instructions per OS page. Its
[deterministic memory tracker](https://github.com/dfinity/ic/blob/fc21803c3c3a8dd452b3b58b959751c41fecb89c/rs/memory_tracker/src/deterministic.rs)
maps and dirties 64KiB regions, charging all sixteen 4KiB pages on first access
and again on first dirtying. Each event therefore costs 80,000 instructions.
The arithmetic above identifies charge-sized increments, not distinct region
counts: one region can incur both charges.

The third probe provides a control within the same message, at period 2:

| Application rows | First application ring writes | Second application ring writes | First whole sample | Second whole sample |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 4,051,400 | 43,600 | 12,664,087 | 5,190,472 |
| 211 | 9,228,454 | 91,996 | 18,658,261 | 6,996,081 |

The second sample reuses already accessed/dirty regions and costs exactly 436
instructions per application row in the ring phase. It also replaces a sample
in the same slot instead of appending, and other cached state changes. Thus it
is supporting evidence for page-cost attribution, not a production optimization
or a promise of these savings on the next scheduled message.

Together, source inspection, exact charge-sized increments and the control
strongly identify scattered history-buffer writes as an optimization target.
Provider work, validation and memory collection still contribute. A synthetic
provider cannot account for all work in Toko's real IcyDB/gameplay provider.

## Follow-up and completion

Canic owns the next candidate: group samples written together into fewer heap
regions inside the existing history owner. Compare a slot-oriented or chunked
layout with the current per-series rings using identical source, fixture and
instrumentation. Avoid swapping the hash index again without measured benefit.

Preserve the 256-series, 288-slot and 8MiB limits; exact identity, units and timer
registration semantics; source timestamps, gaps and stale-source rejection;
truncation, expiry, same-slot replacement, restart reset and sampler recovery.
Measure cold, repeated and full-window work, as well as query and expiry costs,
before adopting a layout. Real Toko qualification still requires its unchanged
producer and artifact contract; no downstream result is inferred here.

All seven tracked probe files were restored byte-for-byte and the temporary
fixture module removed. Existing IcyDB dependency work was preserved. This
investigation is complete, while CANIC-148 remains open and no sampler fix is
ready to publish. The dependency batch retains its earlier targeted validation
and .35 changelog readiness. B1 remains the next accepted implementation batch;
these diagnostics do not accept B1 or open B2. No commit, version bump,
publication or deployment ran.
