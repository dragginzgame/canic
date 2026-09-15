# B1 row 8 build qualification

Date: 2026-09-14
Verdict: ready for matched measurement; B1 remains open.

The endpoint-declaration counterfactual passed all fourteen selected artifacts:
eleven canonical roles, runtime probe, payload-limit probe and blob-storage
probe. Each final Wasm validates, its gzip round-trip matches, its Candid
validates, and its structured metrics include an independently counted number
of defined functions. The runner exited successfully and restored its isolated
product worktree and lockfile.

## Exact inputs

- Product: immutable `v0.110.5`, commit
  `50f40171d6177c3d1e490b1fdb5f6163323b2cd5`, tree
  `5a66988735c707b188d9d1fe03a3ed3b4ff7a273`.
- Product Cargo.lock SHA-256 before and after:
  `0fd6c7897d08e6a0f4e436caaf319ba24ebc32236434010b5c6ae3507f663147`.
- Exact row-8 patch SHA-256:
  `e89c5a1488d7f87bc6bb1ea96913ea72307ed2769f9c59b47973fd12c91af39f`.
- Method checkout base: `v0.110.16`, commit
  `a875c6498721bd89ca98549e38389b8280910d68`, with the frozen-source
  preflight repair described below. Exact executed source hashes, separate
  harness lock hash and tool versions are in the metadata.
- Rust/Cargo 1.97.1, ic-wasm 0.11.1, didc 0.5.4 and WABT 1.0.34.

Evidence copied verbatim from the successful qualification:

- [Run metadata](b1-row8-qualification/run-metadata.tsv)
- [Per-artifact metrics and hashes](b1-row8-qualification/artifact-metrics.tsv)
- [Executed experiment catalog](b1-row8-qualification/experiments.tsv)
- [Artifact roster](b1-row8-qualification/artifacts.tsv)

The executed catalog truthfully records row 8 as `specified`; the maintained
catalog was promoted to `ready` only after this run passed. The source patch
is unchanged. Original logs, method snapshots and binaries remain local under
`.tmp/b1-08-qualification-20260914/` and are not required by release gates.

## Preflight correction

The runner previously checked historical patches against the active method
checkout. Row 8 therefore failed preflight after the host evolved beyond the
frozen baseline. Preflight now checks source owners, configuration paths and
patch applicability against the frozen Git tree through a temporary index.
Execution also rejects any different product commit. Neither the active index
nor product source is modified during that check.

Focused regression coverage accepts a method copy without product source paths
and rejects both an incorrect patch digest and a self-consistent digest for a
patch that cannot apply to the frozen tree. The full row-8 qualification then
used the existing release builder in a clean linked worktree. The existing
cache probe selected direct compilation; no cache behavior was changed.

## Limits and next work

This is a single variant build qualification, explicitly ineligible for
retained size/determinism evidence. There is no matched baseline run here and
no measured saving, deployment recommendation or production `.16` comparison.
The experiment deliberately changes generated Candid and protocol metadata;
its bytes cannot be interpreted as pure runtime serialization savings. Named
function attribution is not supplied by this run.

No PocketIC, watchdog/recovery or authorization-persistence behavior was
executed. A separate read-only source review found the existing auth stores
and restore paths remain feature-selected; it does not refresh runtime parity
or optimized-artifact evidence. Broader role-specific initialization remains
parked under ideas.

Next: qualify rows 10 and 12, obtain the required matched measurements for
ready rows including 6 and 8, and finish the accepted B1 ledger before wider
B2/B3 changes. This report does not accept B1 or close the minor.
