# Deployment and test performance follow-through

Date: 2026-09-28. Published base: 0.110.46. Open batch: 0.110.47.

## Measured starting point

The retained successful full validation at
`target/validation-runs/20260928T111240Z-42322.xEaV9P/0.log`
took 3368 seconds. Ordinary tests took 133 seconds; the internal PocketIC suite
took 2760 seconds. Its two workers finished in 2393.37 and 2046.06 seconds.
The source-bound recovery barrier precedes both workers.

The current compiler-cache snapshot (`/tmp/canic-cache-health-before.json`)
reports 125 requests, 3 hits, 37 misses, 85 uncacheable calls and zero cache
errors. Of the uncacheable calls, 67 report `crate-type`, 11 report `-` and
7 report `missing input`. Low hit counts alone do not establish a broken cache.
These are shared-server counters, not isolated per-command measurements.

The existing [Toko build measurements](toko-feedback-followthrough.md) record
1027.38 seconds with an empty private build target and 2.16 seconds for unchanged
reuse. Comment changes in application/dependency sources took 456.48/483.64
seconds; their unchanged repeats took 2.22/2.19 seconds. These are older frozen
source measurements, not a before/after measurement of this patch.

## Implemented

1. **Persistent CI compiler cache.** Rust checks, ordinary tests and PocketIC
   each restore a separate cache bounded to 2 GiB. An installer verifies the
   repository-pinned SHA-256 and exact sccache version. Failed test runs also save
   useful compiler results; cancellation skips saving. Cargo target caching
   remains disabled. Cache restoration confers no test or artifact authority.
   The pinned binary checksum was checked against both the upstream release API
   and its checksum asset before testing the installer in `/tmp`.
2. **Useful cache accounting.** The test runner distinguishes uncacheable calls
   and cache errors from hits and misses. It rejects incomplete statistics and
   reports counter resets instead of negative or fabricated deltas. Output stays
   at one start line and one summary line per runner.
3. **Balance existing workers.** The independent managed-App and Component Group
   cases move to the shorter worker, before its Fleet journeys. Their retained
   times total about 201 seconds. Holding all other times fixed projects a
   critical-path reduction of about 146 seconds. This is a scheduling estimate;
   compilation, resource contention and cache state can change the actual gain.
   The source-bound recovery barrier, complete case inventory, catalogue order
   within each worker, two-process bound and cancellation behavior remain.
4. **Separate build assertions from artifact inputs.** Artifact-builder and
   compiler-cache unit tests now have standalone test modules. The existing
   producer classifier excludes those tests from cache identity, while retaining
   their production modules. The complete source snapshot still rejects edits
   during an active cache transaction. This incurs a one-time cache-key change
   for the module move; later assertion-only changes can reuse fixture Wasm.

5. **Bind identities after compilation.** Generated entrypoints read a fixed-width
   data slot. Host Cargo commands remove the release identity from compiler inputs;
   finalization fills one unbound slot in a private template copy before shrink,
   Binaryen, qualification, hashes and publication. Missing, duplicate, overlapping
   and already-bound slots fail without mutation. Installed lifecycle identity
   checks and retained final bytes on retry remain unchanged. This avoids rebuilds
   caused solely by a new release identity; it adds no inter-canister call.

## Further work and limits

- **Full producer separation remains architectural work.** Fixture recipes
  conservatively include the production `canic-host` dependency graph. Unrelated
  production operator changes can still invalidate fixture Wasm. Excluding files
  by directory name would lose assurance that a changed helper cannot affect the
  artifacts. A smaller independently compiled producer and its exact source and
  dependency closure are needed before removing those inputs.
- **Finalization still runs for a new release.** Unchanged roles can retain Cargo
  freshness, but each new identity needs bound final artifacts and qualification.
  Source, configuration, feature and compiler changes still invalidate their
  affected targets. Full application-scale timing after this change remains
  unmeasured; earlier deployment numbers are not a forecast of the new gain.
- **Recovery estates are already small.** Funding pause uses one workload.
  Retained-estate recovery uses two workloads and one Ready spare, deliberately
  forcing successor funding. The mixed-topology case covers five workloads with
  distinct topology obligations. Reducing those counts indiscriminately removes
  tested behavior. Further setup reuse must preserve independent replica state,
  real paid effects and each case's fault boundary.

This patch does not establish a new full-suite duration or faster changed-input
deployment. A future maintainer-selected full gate supplies those measurements;
the coding pass does not pre-run it.

## Targeted qualification

- 19 Host artifact/cache tests pass, including the real declaration/runtime Cargo
  probe, role feature identity and build-lock exclusion.
- Five producer-cache tests pass: assertion-only reuse, production invalidation,
  embedded/uncertain inputs, actual Host module classification and concurrent
  edit rejection. Three worker/inventory tests pass; one ran in both selections.
- Warning-denied Host/internal-testing Clippy passes (32.26 seconds).
- Runner mocks cover healthy, unavailable, malformed and reset cache statistics,
  success/failure ordering and retained diagnostics. Worker launcher mocks cover
  success, cancellation, interruption and owned-resource cleanup.
- Actionlint, scoped ShellCheck, release-integrity guard, scoped Rust formatting
  and diff whitespace checks pass. The real pinned installer ran in `/tmp`.
- The initial module moves preserved both production implementations. The later
  release-binding change adds identity finalization to the artifact builder.
- 13 native release-binding/build-context checks pass. The optimized PocketIC
  probe passes under Fast and Release: real Cargo reuse across two identities,
  invalidation on source edit, unchanged template bytes, matching runtime identity,
  rejection of mismatched/unbound installation, and valid reinstall. Its body took
  1.65 seconds; the governed runner took 169 seconds including host compilation.
  Logs: `/tmp/canic-047-binding-{native,pocketic}.log`.

Logs: `/tmp/canic-047-{build-cache-tests,worker-partition,performance-clippy,runner-tests,worker-launcher,ci-integrity}.log`.
The first sandboxed Host test run could not access the compiler-cache socket;
the unchanged tests passed when rerun with normal socket access. No full suite
or live deployment ran. Runtime timing acceptance remains unmeasured.

## Release-binding closeout

The exact `activation_reset::source_bound_activation_reset_recovers_and_replays`
PocketIC case passes through generated Coordinator, Root, Store and workload
artifacts. Its existing lost-response, retained evidence, selected replacement,
reset and effect-free terminal replay assertions remain intact. Body: 347.10s;
runner including harness compilation: 513s. In this run, initial artifacts took
4m16s and the replacement identity's artifacts took 23.87s. The latter reused
verified Candid declarations and completed Root/workload Cargo processing in
9.13s. These observations are within one fixture, not a controlled comparison
against the previous implementation or an application-scale speed guarantee.

The final 13 native binding/context checks pass, including touching/disjoint
segment boundaries. Final warning-denied Clippy passes for `canic-core`, `canic`
and `canic-host` library/test targets with all features (24.38s). Logs:
`/tmp/canic-047-binding-{native,clippy,recovery}.log`. Scoped formatting,
ShellCheck with the repository exclusions, runner mocks and diff checks pass;
runner log: `/tmp/canic-047-binding-runner.log`.

The accepted .47 batch is ready for review and the maintainer-selected release
flow. No broad validation, version transaction, Git publication, live deployment
or sibling mutation ran. Full-suite timing remains for the selected release gate.
