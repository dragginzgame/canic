# ic-query 0.43 integration review

## Update

Canic now selects published `ic-query 0.43.0`, verified against the crates.io
registry on 2026-09-15. Its checksum is
`cfab999d1882c7adb6d57a0ab4cc022ed5939b4ee1b66471a715af46410e1c17`.
The existing `subnet-catalog-host` feature selection is retained.
Cache-only reads forward the upstream boxed detailed failure directly. Generation
retains that failure inside a typed Canic acquisition error, alongside runtime
startup, worker failure and deadline variants.

The upstream 0.43 release adds Governance probe execution and retained smoke
receipts. Those tools and the `canister` feature are separate from Canic's
selected host dependency. No Governance probe is deployed by this update.
The sibling ic-query checkout was inspected read-only; Canic consumes the
published registry package.

## Accepted Canic improvements

The maintainer requested implementation of all three recommendations.

1. New mainnet generation uses `RefreshMissingInvalidOrOlderThan` with a
   Canic-owned 3,600-second bound. Exactly one hour remains acceptable;
   older content requires successful refresh. Invalid or insufficient-assurance
   caches also require refresh. Recovery's Root-management and reinstall
   observations now use cache-only reads, with no expiry-triggered network or
   cache mutation. Their existing exact placement comparisons remain in force.
2. `GeneratedDesiredFleet::subnet_catalog` and the CLI generation summary retain
   collection/observation times, age/bound, cache disposition/path, Registry
   version/digest, assurance and actual source endpoints. Local generation has
   no catalog observation. These are read-only projections outside `DesiredFleet`
   and its review digest. Generation shares the same validated snapshot between
   placement checks and reporting. Detailed catalog failures retain their typed
   upstream value through generation and recovery errors.
3. Refresh requires agreement from `https://ic0.app` and `https://icp-api.io`.
   Both must return the same Registry version and canonical content; there is
   no weaker fallback. The validated cache must meet at least
   `MultiEndpointAgreement`. Existing weaker cache content is refreshable during
   generation but rejected during cache-only recovery reads.

Upstream deliberately does not classify `InsufficientAssurance` as a generic
recoverable cache error. Canic's generation adapter handles that exact typed
failure by explicitly requesting one atomic `ForceRefresh` at the stronger
assurance floor. It neither deletes the old cache first nor accepts its weak
evidence after a failed replacement. This path reports `forced_refresh` on
success, preserving upstream acquisition semantics.

Agreement compares distinct hostnames, not independent providers, and does not
establish certification. Cached evidence may come from other contributing
endpoints if it meets the assurance floor; the report preserves those endpoints.
The certified catalog feature remains unselected: adopting authenticated archive
collection and its trust/freshness policy is a separate design, not necessary to
implement the selected agreement policy. On refresh this policy collects twice
the single-endpoint snapshots, sequentially through the upstream adapter;
cache hits issue no Registry calls. Both selected `/api/v2/status` endpoints
returned HTTP 200 during the read-only connectivity check; that alone is not a
complete live Registry-agreement qualification.

An isolated production-loader mainnet probe then reached its external 90-second
deadline before producing a catalog (exit 124). It was terminated and its
temporary cache/executable removed. Both endpoints also returned HTTP 400 to
deliberately invalid query bodies, establishing transport responsiveness only.
That short probe prompted the acquisition follow-up below; the native policy
tests do not substitute for live latency qualification.

## Acquisition deadline and progress follow-up

Read-only tracing of the published 0.43.0 collector identified sequential
endpoint acquisition and reconstruction of the routing-key family from Registry
history starting at version zero. Individual latest-version queries took
283–397 ms. The first five history pages reached version 4,270 in 3,906 ms;
the pinned current version was 64,108.

A complete first endpoint (`ic0.app`) took 88,119 ms and 158 query calls,
producing 80 Registry records, 69 subnets and 93 routes. The second endpoint
failed after a further 96,627 ms with a typed `get_changes_since` transport
failure. This explains why the original 90-second whole-collection probe was
insufficient. Logs: `/tmp/canic-catalog-query-probe.log`,
`/tmp/canic-catalog-pages-probe.log`, `/tmp/canic-catalog-collection-probe.log`.

Canic now gives new-generation acquisition a shared 600-second deadline,
including any explicit stronger-assurance repair. Progress reports cache lookup,
the active endpoint, endpoint completion with Registry version/query count, and
final validated cache disposition. Heartbeats occur every ten seconds; the CLI
writes progress to stderr. Endpoint completion alone never claims agreement.
These diagnostics remain outside desired-state authority.

Timeout returns a typed progress snapshot and drops the pending collection.
Tests verify preservation of old cache bytes, removal of the upstream refresh
lock and immediate successful retry. Acquisition uses a joined worker/runtime
so synchronous host callers may also call it inside an existing Tokio runtime.
The deadline bounds cooperative asynchronous acquisition; it cannot preempt
synchronous filesystem operations or a blocking caller-supplied progress callback.

History-scan cost and finer Registry-record progress remain upstream improvement
opportunities. No upstream repository was modified.

The final read-only production-loader retry succeeded. Cold acquisition took
**148,245 ms** (`refreshed_missing`), with 158 queries per endpoint. The first
endpoint finished at 72 seconds and the second at 148 seconds. Immediate reuse
took **3 ms** (`cache_hit`) and started no endpoint collection. Both outcomes
retained exactly the same `MultiEndpointAgreement` snapshot: Registry version
**64,108**, endpoints `https://ic0.app` and `https://icp-api.io`, catalog digest
`5105443e72d78451011c415b3f7b946b882d879f7f38c7fcfaf04362450692c3`.
Log: `/tmp/canic-catalog-live-final.log`. This is one successful live sample,
not a latency guarantee or an upstream collection-speed improvement. The isolated
probe executable, source and cache were removed after measurement.

The focused acquisition suite passes: nine host catalog tests and two CLI
rendering tests (`/tmp/canic-catalog-deadline-tests.log`). The retained-estate
generation/apply/replay regression also passes
(`/tmp/canic-catalog-deadline-generation.log`). Scoped formatting and document
semantics pass. The three-package all-target Clippy attempt encountered four
private-access compiler errors in the concurrently added
`baseline/tests/activation_reset` fixture; these are outside catalog acquisition
(`/tmp/canic-catalog-deadline-clippy.log`).
The final all-target/all-feature warning-denied host/CLI Clippy run passes
(`/tmp/canic-catalog-deadline-host-cli-clippy.log`).

These improvements adopt existing upstream capabilities. Governance probe
testing remains outside Canic's selected dependency surface.

## Dependency-update validation (before the policy improvements)

- Four catalog regressions pass: source/assurance selection, cache-only policy,
  complete failure projection and missing-cache pre-effect evidence. Executed
  `target/debug/deps/canic_host-55f0a4ad7db96c64 subnet_catalog` against the host
  test binary freshly compiled with 0.43.0 by the concurrent inspection check.
  Log: `/tmp/canic-ic-query-043-tests.log`.
- `cargo clippy --locked --offline -p canic-host -p canic-testing-internal
  --all-targets --all-features -- -D warnings` passes, including the direct
  consumer fixture targets. Log: `/tmp/canic-ic-query-043-clippy.log`.
- Scoped whitespace checks pass. No full workspace or new PocketIC run was
  performed for this dependency update.

## Canic policy implementation validation

- Seven focused catalog tests pass, including the exact age boundary,
  matching snapshot cache reuse, version/payload disagreement, endpoint failure,
  unchanged old bytes after failed refresh, stronger acquisition after a weak
  cache, same-request recovery and complete typed failure projection.
  Log: `/tmp/canic-catalog-focused-tests.log`.
- The CLI catalog summary regression passes in
  `/tmp/canic-catalog-cli-tests.log`. The existing generated retained-estate
  planning/apply/effect-free-replay case passes with local catalog absence
  asserted: `/tmp/canic-catalog-generation-tests.log`.
- The synthetic modern-routing fixture passes the same upstream agreement and
  cache validation used by the host. It is registered in the governed inventory;
  the exact fixture and inventory tests pass in
  `/tmp/canic-catalog-fixture-tests.log` and
  `/tmp/canic-catalog-inventory-tests.log`.
- All-target/all-feature warning-denied Clippy passes for `canic-host`,
  `canic-cli` and `canic-testing-internal` in
  `/tmp/canic-catalog-policy-clippy.log`; the fixture's final import-order cleanup
  also passes in `/tmp/canic-catalog-fixture-final-clippy.log`.
- Scoped formatting and whitespace checks pass.

The grouped Cargo invocation compiled the affected packages with
`canic-testing-internal/governed-pocketic-tests`. Its `catalog` substring filter
also matched three unrelated canister journeys. That run was stopped during
fixture build setup and is not a completed PocketIC qualification. Final focused
runs use the freshly compiled host `canic_host-120f9fd3bccf9aa7`, CLI
`canic_cli-7fcfca7bd3d8c341` and internal-testing
`canic_testing_internal-ee0035f2a5f7f751` binaries with the `subnet_catalog`
filter or the exact recorded test names. No broad release gate ran.

## Scope

This update extends the existing 0.110.17 draft. The complete accepted release
batch remains not push-ready because the independent funding/recovery work in
the current handoff is unfinished. Package versions remain 0.110.16; no broad
gate, publication or deployment is part of this update.
