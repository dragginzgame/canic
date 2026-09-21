# Public metric history locality

Status: design proposal requested and subsequently parked by the maintainer on
2026-09-21; no implementation or measured candidate yet. Keep it as an idea while
addressing the other Toko Miner feedback. It has no release position.

## Recommendation

Use small groups of series sharing one contiguous history ring. Start with eight
series per group and compare four and sixteen before fixing the internal width.
Within a group, lay out observations by time slot, then by series position:

```text
                         series positions
                       A    B    C   ... H
physical slot 0        [a0   b0   c0  ... h0]
physical slot 1        [a1   b1   c1  ... h1]
...
physical slot 287      [a287 b287 c287 ... h287]
```

These are inline cells in one allocation, not separately allocated row vectors.
The same sampler tick generally updates neighboring cells in each group. A
query selects one series position and reads at most 288 cells directly. It does
not scan other metrics, decode a batch, or assemble data in the frontend.

This is the recommended balance between sampling and chart reads. The
[investigation](../../../audits/reports/2026-09/2026-09-21/sampler-cost-investigation.md)
found that scattered ring writes dominate the measured history cost. Grouping
all series in timestamp-only buffers would favor writes but spread a single
metric's chart across up to 288 buffers. Small groups bound that read footprint.
Eight is a starting candidate, not a protocol constant or demonstrated optimum.

## Ownership and addressing

Keep one history owner in `model::public_metrics::history`. Its existing exact
`(family, name, canister_id)` index resolves to a group and series position.
Keep labels, unit, latest source time and occupancy metadata outside the sample
cells. There is no second history copy or new cache owner.

Groups have stable internal positions while populated. Fill vacant positions
before allocating another group; reclaim an empty group without relocating
unrelated series. Admission still follows validated input order. Index iteration
must not select which series survive a capacity limit.

Each group has 288 physical rows. Map source slots into those rows using a fixed
group origin and modulo arithmetic, retaining each cell's full absolute source
slot, timestamp, value and kind. Different source times within one callback are
normal: address each observation by its source slot, never by collection time.

A compact per-series validity bitmap distinguishes live cells from holes and
old occupants. Clear that bitmap when reusing a position or changing units;
old cell bytes must never become observable through a new identity or unit.
Ordinary counter resets remain observations carrying their existing reset
identity. They do not clear history or manufacture a delta.

Do not clear a whole shared row on rollover: another series can still have a
valid, older source observation there. Write only the selected cell and its
validity bit. Query-time source-window filtering rejects old ring contents.
Existing latest-source checks continue to reject older reappearing observations.

## Allocation and bounded work

Reserve each group's full ring capacity once. Use safe initialized storage;
the first candidate should initialize rows only as the contiguous vector's
length grows, rather than eagerly touching a whole day for the first sample.
Choose the first admitted source slot as the fixed origin so ordinary initial
sampling begins at row zero. A late source or large time jump can require a
larger initialized prefix; measure that case explicitly. No uninitialized reads,
unsafe custom allocator, forced 64KiB alignment or background transpose job is
needed for this design.

Charge full reserved capacity, unused series positions, actual cell sizes,
validity metadata, indexes and bounded labels against the existing 8MiB ceiling.
Do not assume the native and Wasm layouts have identical sizes. Preserve the
256-series and 288-slot limits and keep maximum supported occupancy qualified.
Free the allocation and its reservation when its final series expires.
`reserved_bytes` remains truthful and can change with the storage layout.

Lazy initialization reduces the expected startup write footprint; it does not
remove the full reservation or establish a worst-case cost claim. Measure group
creation, large gaps, unit changes, full capacity and churn as well as steady
sampling. Increasing group width trades fewer update regions for more regions
read by an individual chart. Do not choose width solely from sampler timings.

## Query and display contract

Keep `PublicHistoryRequest`, `PublicHistorySnapshot`, Candid and frontend usage
unchanged. A request still selects one exact metric and receives chronological,
paged observations with the same units, coverage, freshness and truncation
fields. Missing samples remain gaps. No interpolation or synthetic zero appears.

The model reconstructs only the requested series, bounded to 288 observations.
The current ops projection remains responsible for DTO conversion and counter
deltas, including the predecessor needed at a page boundary. Queries remain
read-only and never collect, expire, compact or refill history. Existing disabled
family filtering, source-age checks and restart/reset behavior remain intact.

The algorithmic read bound stays one index lookup plus at most 288 cell reads.
Those reads will span more memory than the current dedicated ring, so unchanged
API and complexity do not prove unchanged instruction cost. Measure one-point,
partial-page and full-day queries before choosing the width. No additional client
request, client-side grouping or chart rendering logic is required.

## Alternatives considered

| Layout | Sampling | One-metric history | Decision |
| --- | --- | --- | --- |
| Dedicated ring per series | Scattered writes | Compact | Measured baseline |
| One full batch per timestamp | Compact writes | Up to 288 separate buffers | Poor balance for current per-series API |
| Small grouped rings | Fewer write regions | Direct reads within one bounded allocation | Recommended candidate |
| Append buffer plus later transpose | Cheap immediate append | Requires merging or another representation | Adds flush spikes and ownership complexity |
| Duplicate row and column stores | Must maintain both copies | Compact | Retains expensive writes and doubles sample storage |

Changing stable-memory bucket sizes does not alter this heap history. Shrinking
retention or weakening downstream qualification is outside the proposed fix.

## Complete implementation batch and evidence

One bounded batch owns storage replacement, model/projection regressions, real
Wasm sampling and query comparisons, accounting, documentation and probe cleanup.
It changes no public schema, timer owner, sampler cadence or application provider.
The old internal layout is removed on adoption; no compatibility implementation
or migration is needed for this heap-only cache.

Compare the existing layout and grouped widths with identical frozen source
inputs apart from the layout, profiles, providers, names, fixtures and probes.
Record complete callback costs and query costs separately. Include sparse and
dense loads, the 100-row workload shape, maximum admission, at least one full
288-slot rollover, cold allocation, long gaps and expired-series reuse.

Behavior evidence must cover exact selectors, interleaved families, lagging and
equal source times, missing observations, unit changes, counter/registration
resets, saturation, pagination deltas, expiry, truncation, zero retained bytes
after complete expiration, rejection followed by recovery, and lifecycle reset.
Verify that query requests cannot change stored state. Compare observable
responses while checking allocation accounting against the new layout rather
than requiring the old `reserved_bytes` number.

Choose the smallest group width that supplies useful measured sampler savings
without a material chart-query cost regression. If grouped rings cannot meet
both goals, revisit the layout rather than quietly shifting cost to queries.
There is no claimed savings percentage or new arbitrary absolute release gate.
The real Toko producer must subsequently qualify under its existing contract;
synthetic Canic evidence cannot close that downstream result.
