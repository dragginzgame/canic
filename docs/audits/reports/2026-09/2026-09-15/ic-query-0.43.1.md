# ic-query 0.43.1 integration

## Dependency and upstream review

Canic selects registry package `ic-query 0.43.1` with checksum
`b30024aee5d574295e2870f82a67aa50dedad5c14fee11f73ce13514d1d83d89`.
The targeted offline Cargo update changed only that package's version and
checksum. The existing host-only feature selection is retained. Review used
published registry source; the upstream checkout was read-only. A direct
crates.io API verification attempt returned HTTP 403; the local Cargo registry
index and package supplied resolution evidence.

The release addresses the previous [acquisition feedback](ic-query-0.43.md):

- Agreement collection polls up to two endpoints concurrently and validates
  results in deterministic endpoint order. Each endpoint retains its own pin;
  exact version and canonical-content agreement remain mandatory.
- `LiveSubnetCatalogSource` retains validated routing-history prefixes in
  bounded, endpoint-local memory. Reusing the source allows later attempts to
  resume that history. A new process still scans from version zero.
- Structured progress includes endpoint start, version pin, history watermark,
  record reads and retry attempts. It is transient diagnostic data.
- Catalog query retries cover connection/timeouts and HTTP 502/504, with at most
  three explicit attempts and cancellable 250/500 ms backoffs. They preserve
  request inputs and final typed failures. The policy avoids multiplying
  ic-agent's existing HTTP 429/503 retry behavior.

## Integration and validation

Canic retains its one-hour generation freshness,
exact agreement floor, cache-only recovery and shared ten-minute deadline.
No certification, independent-provider assurance or cross-process history reuse
is implied by this update.

`MainnetCatalogClient` replaces the one-shot live loader. Host callers may retain
the client across attempts; generation creates one client for its acquisition.
Exclusive mutable access prevents mixing progress between simultaneous loads.
The client keeps upstream history while clearing transient progress before each
load. It retains only the latest event per endpoint for bounded heartbeat and
typed timeout snapshots, alongside the exact active endpoint set. CLI output
shows history watermarks, record reads, retry details and query counts without
printing every upstream event. Desired-state authority remains separate.

## Live sample

The isolated production-client probe completed at Registry version **64,109**:

| Acquisition | Elapsed | New explicit Registry queries |
| --- | ---: | ---: |
| Cold two-endpoint agreement | 63,253 ms | 316 |
| Immediate cache hit | 3 ms | 0 |
| Simulated expiry, retained client | 13,851 ms | 162 |

The last row advances the caller's freshness-observation time by 3,601 seconds
to exercise real live stale-cache refresh without waiting an hour. It does not
measure an hour's Registry changes. Prefix retention saves 154 history queries;
the maintained live records are still read. The cache-hit log's `queries=316`
is retained snapshot provenance, not new traffic: no endpoint collection starts
on that hit and its exact authority equals the preceding cold acquisition.

Cold/cache-hit digest:
`c2f79683ce0db568c8561cb0fc5e000dba0dbacf1247cfb3abdde6d2cf963dd4`.
The new refresh has its own snapshot digest:
`3d17eda41e47231fa547eb0e8b570c552679b9f2b06392114fbdd7c6eb6449c6`.
The snapshot digest includes acquisition provenance, including collection time
and query count; it is distinct from upstream's agreement-content comparison.
Every acquired result has `MultiEndpointAgreement` from `https://ic0.app` and
`https://icp-api.io`. The earlier 0.43.0 cold sample took 148,245 ms at Registry
version 64,108. These are individual network samples, not controlled benchmarks
or a latency guarantee. Log: `/tmp/canic-0431-live.log`.

The temporary probe source, executable and cache were removed after measurement.
The cached `.crate` archive's SHA-256 independently matches the lockfile checksum.

## Feedback

The release addresses the observed sequential-collection delay while preserving
exact agreement and typed failures. Retained-source refresh also demonstrates
that history reuse removes work rather than merely hiding latency. Canic keeps
its own deadline because bounded per-query retries alone do not bound an entire
collection.

The largest remaining opportunity is cold history discovery for a new process.
Persistent validated checkpoints would need an explicit upstream integrity,
endpoint-binding, resource-bound and invalidation design; Canic does not add an
ad hoc cache or weaken snapshot verification. No additional upstream blocker
was identified in this focused review. No live transport-fault injection or
certification qualification is claimed.

## Focused checks

- Ten host catalog tests and three CLI progress/summary tests pass, including
  both-endpoint cancellation, retained history diagnostics, cache preservation,
  lock release and immediate retry: `/tmp/canic-0431-tests.log`.
- The generated retained-estate planning/apply/effect-free-replay regression
  passes: `/tmp/canic-0431-generation.log`.
- The exact synthetic growth-catalog agreement case and governed inventory
  regression pass in the freshly built internal consumer binary:
  `/tmp/canic-0431-fixture.log` and `/tmp/canic-0431-inventory.log`.
- Scoped formatting, document semantics and whitespace checks pass.
- All-target/all-feature warning-denied Clippy passes for `canic-host`,
  `canic-cli` and `canic-testing-internal`: `/tmp/canic-0431-clippy.log`.

This extends the existing 0.110.17 draft. Package versions remain 0.110.16.
The complete accepted release batch remains not push-ready because independent
funding/recovery work in the current handoff is unfinished. No broad gate,
version action, Git publication, deployment or upstream mutation was performed.
