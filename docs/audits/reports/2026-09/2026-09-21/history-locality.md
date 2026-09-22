# Public metric history locality qualification

The post-.35 batch implements groups of eight history series and corrects Toko
Miner's arbitrary 20M instruction assertion under explicit maintainer authority.
The Canic optimization is qualified locally. Real Toko producer savings require
adoption and rebuilt application artifacts; this report does not claim them.

## Measurement boundary

The control restores the original .35 history model and projection from
`a99a8fe821c0d54b8591acca8aa98fa51aa5c13e`. Control and candidates use the same
Cargo.lock, production sampler, synthetic provider, fixture, Fast profile and
PocketIC 16.0.0. Each run uses `make test-pocketic-case CASE=timer_authority`.
Source hashes before and after each run match. The
[structured evidence](history-locality.json) retains all measurements, source,
lockfile, artifact and raw-log hashes. Variant patches under
[artifacts/history-locality](artifacts/history-locality/) reconstruct each
alternative from the selected source. They are experiment inputs, not supported
runtime alternatives. The control patch also restores the old ops projection;
width-comparison patches change only the model.

The protected fixture measures `PublicStatusApi::sample_metrics`, including
provider construction, validation, selected family collection and publication.
It excludes endpoint encoding and the enclosing timer wrapper. Requested
application row counts are 1, 100, 211 and 256, with fixed long synthetic names.
Other selected families also consume the shared 256-series history capacity;
requested application rows are not an assertion that all are admitted.

Each fresh canister receives 300 samples, 301 seconds apart. Separate query
messages request the first application series at limits 1, 12 and 288, after
samples 0, 1, 2 and 299. The final retained series has 287 points because the
301-second spacing crosses a missing source slot. It is a full-window traversal,
not a claim of 288 consecutive observations. A separate sparse canister covers
initial allocation, a 200-slot gap, same-slot replacement and a 1000-slot expiry.
No arbitrary instruction threshold determines test success. Assertions cover
actual measurements, values, page bounds, retention and the storage cap.

## Sampling and queries

All costs below are Wasm instructions. The sample column is period 299; the
maximum covers all 300 regularly spaced samples, not the separate sparse run.

| Application rows requested | Control sample | Selected sample | Reduction | Control regular maximum | Selected regular maximum |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 6,728,100 | 4,729,007 | 29.7% | 7,216,932 | 6,257,827 |
| 100 | 12,167,194 | 8,844,655 | 27.3% | 12,768,940 | 10,865,260 |
| 211 | 18,234,217 | 12,625,610 | 30.8% | 18,571,927 | 15,197,628 |
| 256 | 19,211,119 | 13,288,289 | 30.8% | 19,478,032 | 15,713,322 |

The query buffer is now built directly in chronological order and consumed by
ops without another copy or sort. Request/response schemas, page offsets,
source timestamps, coverage, freshness, counter deltas and display usage remain
unchanged. Queries still perform no collection or cache mutation.

| Application rows requested | Page limit | Control query | Selected query | Change |
| --- | ---: | ---: | ---: | ---: |
| 1 | 1 | 666,309 | 448,026 | -32.8% |
| 1 | 12 | 672,287 | 454,377 | -32.4% |
| 1 | 288 | 978,429 | 599,950 | -38.7% |
| 100 | 1 | 744,976 | 447,854 | -39.9% |
| 100 | 12 | 751,013 | 454,023 | -39.5% |
| 100 | 288 | 897,173 | 599,950 | -33.1% |
| 211 | 1 | 504,580 | 528,083 | +4.7% |
| 211 | 12 | 511,014 | 534,089 | +4.5% |
| 211 | 288 | 737,100 | 679,929 | -7.8% |
| 256 | 1 | 585,893 | 368,455 | -37.1% |
| 256 | 12 | 672,278 | 374,642 | -44.3% |
| 256 | 288 | 737,865 | 680,469 | -7.8% |

Eight is the selected internal width, not a protocol requirement. Width four's
100-row mature sample was 12,083,419 instructions, only slightly below the
12,167,194 control. Width sixteen reduced that sample to 6,929,393 but raised
the 256-row full-chart query from 737,865 to 1,167,293. An intermediate width-eight
reader also raised that query to 927,169. The selected implementation places the
group index inline and emits the two chronological bitmap ranges directly,
reducing that query to 680,469. These comparisons justify the balanced choice;
they do not establish a globally optimal width or isolate every compiler effect.

## Explicit tradeoffs

The selected layout is not cheaper for every request. At 211 application rows,
1- and 12-point mature pages increase by 23,503 and 23,075 instructions (4.7% and
4.5%). All four measured mature full-chart reads improve. For the 100-row sparse
case, the two-point query following a long gap increases from 176,619 to 334,112.
This bounded startup/read cost is retained in the evidence, not hidden by the
mature-window result. IC page-meter and allocator placement effects make cost
changes discontinuous; no arbitrary percentage becomes a release guard.

Lazy row initialization moves work to the first write crossing a large gap.
After the 200-slot gap, sampling costs 16,564,568 versus 13,049,179 at 100 rows,
and 23,646,537 versus 19,249,687 at 256 rows. The next same-slot sample is cheaper
than the control. Complete-expiry sampling also improves in those cases.
This is a tradeoff for lower regular sampling cost, not a worst-case cost win.
The complete measurements include cold and sparse cases for every load.

The conservative reservation charges unused positions, all 288 rows, index
headers, validity maps and bounded labels. At period 299 it rises from 3,445,208
to 3,617,736 bytes for 100 requested application rows, and from 5,284,864 to
5,294,592 for 256. It remains below 8 MiB. Empty positions are reused; the last
expired member frees its group's allocation. Full expiry releases all retained
heap storage. Public `reserved_bytes` truthfully reflects the new representation.

## Behavior and validation

- Thirty focused native public-metrics tests pass, including a semantic reference
  comparison over 609 ticks, lagging neighbours, rollover, gaps, unit/counter
  resets, exact selectors, capacity, sparse expiry and position reuse, and
  pagination/source-time/read-only behavior.
- All ten timer-authority PocketIC cases pass for the control, width four,
  width sixteen, intermediate width eight and final width eight. The final
  suite takes 169.52 seconds; the governed runner takes 305 seconds including
  compilation. These wall times are not deployment or sampler performance claims.
- Scoped warning-denied Clippy passes for the changed core, fixture and test
  packages. Native and lint logs are retained beside the performance logs.
- Existing canister cases retain authorization, timer registration, cancellation,
  trap recovery, lifecycle reconstruction, history reset and rejected-provider
  recovery. No full workspace gate ran and no release receipt is inferred.

## Toko Miner policy correction

`report_sampler_cost` now requires a real positive measurement and reports the
historical 20M reference as advisory. The exact managed metrics/history case
passes with the unchanged 20,062,824-instruction maximum, using retained build
`5480e288e344a1f7c0e78590a0f58591cb515e2c5ab4096f3c5009acb69b2af8`.
Scoped User Hub test-target Clippy passes. The structured record retains local
log hashes. Other semantic assertions remain intact. This corrects the stale
blocking policy under Canic's maintained
[sampling cost policy](../2026-09-10/metrics-cost-policy.md); it does not establish
an application operating budget or suppress measurement failures.

That result uses retained pre-optimization payloads, not rebuilt Canic candidate
artifacts or a new staging qualification. Toko Miner's current checkout retains
the correction. No pin change, commit, publication or deployment was performed
by this task. Downstream real-producer qualification remains separate from this
complete Canic implementation batch and does not imply B1 acceptance.
