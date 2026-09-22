# Public metric history locality

Status: implemented and locally qualified after published .35, on 2026-09-21.
The maintainer explicitly activated this formerly parked proposal and separately
authorised Toko Miner's advisory-threshold correction. The
[qualification report](../../audits/reports/2026-09/2026-09-21/history-locality.md)
owns measurements, alternatives, tradeoffs and downstream limits.

## Storage owner and addressing

`model::public_metrics::history` retains the sole heap history. An exact
`(family, name, canister_id)` index resolves a series to one position in a group
of eight. Each group reserves one contiguous allocation containing 288 rows of
eight inline observations, ordered by source slot and then series position.
The bounded 32-entry group index lives inline. Labels, unit, latest source time
and a per-series validity bitmap remain separate from the observation cells.

Groups keep stable positions while populated. Admission follows validated input
order, fills vacancies before creating a group, and frees an empty group without
relocating unrelated series. Full reserved capacity, unused positions, headers,
validity metadata and bounded labels are conservatively charged against 8 MiB.
The existing 256-series and 288-slot limits remain unchanged.

Source slots map into rows using a fixed group origin and modulo arithmetic.
Rows initialise lazily as the vector grows; all reads use initialized safe Rust
storage. Each cell retains its absolute source slot, timestamp, value and kind.
A write changes only its selected position, preserving lagging neighbours.
Changing a unit or reusing a position clears that series' bitmap, preventing
previous cells from becoming observations for another identity or unit.

Older source observations remain rejected. Same-slot writes coalesce to the
latest source observation. Missing samples remain gaps; counter resets retain
their existing reset identity. Expiry releases series metadata and frees the
allocation when its last member expires. Full expiry releases retained heap
storage; no stable-memory migration or compatibility path exists.

## Queries and display

`PublicHistoryRequest`, `PublicHistorySnapshot`, Candid and frontend usage are
unchanged. A request selects one exact series and receives chronological, paged
observations with existing units, coverage, freshness and truncation fields.
The model reads only that position's valid cells, traversing the two physical
ranges in chronological order. The owned view lives in `view::public_metrics`.
Ops consumes its vector without another copy or sort, filters the query-time
window and computes DTOs and counter deltas with the correct page predecessor.

A query performs one index lookup and at most 288 observation reads. It never
collects, expires, compacts or refills history. No additional client request,
client-side grouping or chart rendering logic is required. Disabled families,
source-age filtering and lifecycle resets retain their existing behavior.

## Layout choice and limits

Matched real-Wasm comparisons cover widths four, eight and sixteen, regular
sampling beyond rollover, independent 1/12/288-point queries, cold allocation,
long gaps and expired-series reuse. Eight supplies useful regular sampler
savings with improved mature full-chart reads. Sixteen saves more sampling work
but materially worsens measured full-chart reads; four provides little benefit
at the 100-row workload. A single timestamp-batch layout would scatter one
chart across many buffers, while duplicate stores would retain the write cost.

The selected implementation does not make every operation cheaper. Some small
pages and sparse startup queries cost more, and lazy initialization increases
the first sample after a long gap. Those measured costs and the modest reserved
storage increase are explicit in the report. They are accepted tradeoffs for
regular sampling and mature chart improvements, not hidden by a new absolute
instruction gate. Eight is internal tuning, not a public protocol constant.
Changing stable-memory bucket sizes cannot fix this heap-history write layout.

Native semantic-reference tests and real canister regressions cover identity,
lagged/equal source time, rollover, missing observations, unit/counter resets,
saturation, pagination deltas, expiry, capacity, reuse, query immutability,
rejection recovery and lifecycle reset. Toko Miner's real producer still needs
qualification with adopted candidate artifacts before claiming its savings.
