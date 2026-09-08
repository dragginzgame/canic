# CANIC-148 Public Sampling Correction

Date: 2026-09-08. Baseline: maintainer-pushed `v0.110.10`,
`50b76e0d10b698f89aa3b1479adc302ecfa17dcd`. Evidence qualifies the working-tree
correction recorded in the open 0.110.11 draft; package versions remain 0.110.10.

## Result

The CANIC-148 batch is complete and ready for release approval. Optional public
sampling now reads deterministic bounded prefixes directly from metric owners.
A rejected family retains its prior values and timestamp while independent
selected families continue sampling. Cycle tracking retains its existing owner
and completes through optional-family rejection.

The changes introduce no scheduler, history store, public permission change or
compatibility path. CANIC-147 scheduled chart history is a separate capability
request awaiting contract acceptance. CANIC-141 remains deferred.

## Collection Contract

- Operations visit at most 257 entries from each of four counter owners and,
  on Root, each of three ICP-refill aggregate indexes. ICP target aggregates
  project two rows each. Intermediate projections are bounded before public
  row selection.
- Performance visits at most 129 recorded counters plus the upstream timer
  inventory, which ic-timers 0.7.0 bounds to 64 registrations with bounded
  identities. It avoids unrelated runtime, intent and timer diagnostic rows.
- Occupancy visits at most 129 bounded shard records and reads assigned counts
  and capacities without enumerating assignment keys.
- Each family retains at most 256 rows and one overflow sentinel determines
  truncation. The cache sorts the bounded retained selection. Ordered owner
  maps provide deterministic selection independent of recording order.
- Public names, including projection prefixes/suffixes, must fit 128 bytes.
  Borrowed checkpoint and role text is checked before cloning; complete names
  are checked before formatting. Invalid selected series reject their family.
- The application publisher retains a bounded prefix of caller-supplied rows.
  Application code owns input construction and disposal. Sampling does not
  impose a registration cap on internal performance instrumentation or bound
  unrelated protected diagnostic queries.

All selected families are attempted. The sampling API returns the first typed
error after completing the loop. A failed family's original timestamp remains
unchanged, so its existing stale/unavailable state remains accurate. No stale
snapshot is restamped as fresh.

## Targeted Evidence

| Check | Result |
| --- | --- |
| Selected core library tests, all features | 112 passed |
| Public sampling/performance tests, default features | 10 passed |
| Stable-memory ABI and timer inventory guards | 17 passed |
| Clippy: core, canic-tests and runtime_probe, all targets/all features, warnings denied | Passed |
| Final fixture-only Clippy after dependency correction | Passed |
| `make test-pocketic-case CASE=timer_authority` | 7 passed, 92.85s tests / 96s runner |
| Audit method catalog and current document semantics | Passed |

Core tests used the governed scratch wrapper and selected public metrics,
publication, performance, runtime metrics, ICP-refill, sharding registry and
RPC handler tests. No full workspace or broad PocketIC gate was run.

The PocketIC regression samples the production collector with 256 and 4,096
synthetic recorded checkpoints. Its measurement excludes checkpoint population:

| Recorded checkpoints | Sampling instructions |
| --- | ---: |
| 256 | 4,449,546 |
| 4,096 | 4,647,992 |

This is approximately 4.5% more instructions for 16 times as many source entries,
below the regression's 20-million-instruction budget and 2-times growth bound.
It is a local fixture measurement, not a whole-deployment or live App benchmark.

The same case accepts a 128-byte checkpoint scope into internal instrumentation
and then rejects its oversized public identity with `REQUEST_INVALID`. The prior
Performance sample and timestamp remain unchanged and become stale; occupancy
refreshes. The existing cycle owner returns success and records a later cycle
history observation. Native tests additionally verify real shard assignment
counts, bounded registry output and recording-order-independent truncation.
Existing PocketIC cases retain public/observer denial and lifecycle/timer checks.

The runtime probe uses the published performance API. Its internal fixture-only
cycle qualification calls the existing cycle owner through the Canic subtree;
it adds no application runtime API or direct protected-package dependency.
An initial direct canic-core fixture dependency was rejected by the role-contract
guard and removed before the passing run. The final run covers that correction.
Subsequent edits are documentation and a test-section comment only.

Local logs: `/tmp/canic148-unit.log`, `/tmp/canic148-default.log`,
`/tmp/canic148-guards.log`, `/tmp/canic148-clippy.log`,
`/tmp/canic148-fixture-clippy.log`, `/tmp/canic148-pocketic.log`,
`/tmp/canic148-catalog.log`, `/tmp/canic148-docs.log`.

## Handoff

No in-scope implementation blocker remains. Root and detailed changelogs cover
the complete correction batch. The maintainer-selected release gate and
publication remain; these focused checks are not a published validation receipt.

After adopting the released correction, Toko Miner must verify selected families
against actual instrumentation cardinality and series names, check truncation
and stale-data rendering, and qualify sampling cost with its application work.
Its User-count and IcyDB measurements remain downstream-owned. Publication stays
opt-in; this correction does not provide periodic sampling or historical charts.
Sibling repositories were inspected read-only. No version, Git publication or
deployment action was performed.
