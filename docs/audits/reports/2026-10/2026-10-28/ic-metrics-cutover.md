# IC Metrics cutover checkpoint — 2026-10-28

The working tree on `b9ed7f298bdaf8c9cceac142edf7e56b15d33208` now carries the
Metrics source/client hard cut adapted from PR #599. This is uncommitted work;
package versions remain 0.112.0 with pending 0.113.0 notes. Release/push is held.
The existing NNS, signed-request, ICRC-21 and provisional snapshot repairs were
preserved. No complete validation, commit, version bump, publication or deployment
was run for this checkpoint.

## Implemented boundary

IC Metrics 0.16.3 owns scalar and measurement arithmetic. Canic retains tables,
keys, bounded checkpoint/call admission, invocation attribution, reset/lifecycle,
units, authorization and history. Operational balances, budgets, quotas and
persisted funding records remain with their existing owners.

Protected `Count` and `U128` carry wide values and explicit exactness;
`Measurement` carries a sample count, wide total and independent actual-overflow
flags. Exact numeric caps remain exact. Latest timer delay is a scalar, never an
average. Current tracked Timer counters retain source quality; deliberately
bounded status/probe contracts use checked narrowing. Retained refill-derived
counts lack overflow flags and conservatively mark their cap inexact.

Public performance and operation projections consume typed owners directly.
Child funding and target refill amounts remain protected. Refill metric identity
normalization precedes bounded selection without changing persisted records or
spending authority. CLI native Wire/JSON and human rendering preserve quality.
No old payload alias, generic registry or persistence adapter was added.

Root and both private consumer locks select the same registry Metrics 0.16.3
checksum. The only dependency-edge change is the CLI's use of that existing
workspace dependency. Active docs describe the new contract; historical
changelogs remain historical.

## Evidence

Local receipts are under `target/metrics-cutover-20261028/`. `before.patch`
preserves the incoming dirty batch; `tested.patch` and `tested-files.sha256`
capture the final working files, including untracked additions. `tools.txt`
records Rust/Cargo 1.99.1, managed sccache 0.19.0, the Canic wrapper,
`target/cargo`, isolated runtime directory and incremental builds disabled.
Commands use Make's governed defaults and offline Cargo. Supplementary focused
Make targets are retained in `focused-checks.mk`; they do not define a new runner.

| Command / selection | Tested state | Result | Limit / receipt |
| --- | --- | --- | --- |
| `cargo test --locked -p canic-core --lib metrics --all-features` | Metrics source cutover | 111 pass | `core-2.log`; native projections/history, not installed behavior |
| `cargo test --locked -p canic-core --lib perf:: --all-features` | Same runtime source | 15 pass | `native-1.log`; exclusive nesting, async ownership, cancellation, zero, bounds |
| `cargo test --locked -p canic-cli --lib metrics::` | Current Wire client | 27 pass | `native-1.log`; real Wire/JSON, quality, rendering, pagination |
| `cargo test --locked -p canic-host --lib observatory::ops::comparison` | Current public projection consumer | 15 pass | `native-1.log`; source identity, quality, history continuity |
| `cargo test --locked -p canic --test metrics_facade` | Current facade | 3 pass | `native-1.log` |
| `cargo test --locked -p canic-core --lib ops::storage::icp_refill::tests --all-features` | Normalized derived index | 14 pass | `refill.log`; bounded identity/rebuild and operational invariants |
| Strict all-target/all-feature Clippy for Core, Contracts, CLI, Host, facade, internal testing and public tests | Adapted source and consumers | Pass | `callers-2.log`; seven selected packages, not workspace qualification |
| Existing `refresh_embedded_root` owner | Changed runtime source | Pass | `peer-refresh.log`; bytes and structured provenance regenerated through owner |
| `CARGO_NET_OFFLINE=true make test-pocketic-case CASE=pic::lifecycle::tests::interleaved_endpoint_and_checkpoint_metrics_preserve_call_contexts` | Changed installed runtime and probes | 1 pass | `interleaved.log`; proof 74.95s, runner 146s inclusive |
| `CARGO_NET_OFFLINE=true make test-pocketic-case CASE=timer_authority` | Same installed Metrics source | 11 pass | `timer-authority.log`; tests 49.27s, runner 166s inclusive |
| `CARGO_NET_OFFLINE=true make test-pocketic-case CASE=pic_ingress_payload_limits` | Wide/exact denial-counter consumer | 11 pass | `payload-limits.log`; tests 11.48s, runner 27s inclusive |
| Strict all-target/all-feature Clippy for `runtime_probe` and `payload_limit_probe` | Changed probe source | Pass | `probe-lint.log` |
| `cargo test --locked -p canic-testing-internal --lib embedded_root::tests` and existing `verify_embedded_root` example | Final fixture helper guard | 4 pass, 1 intentionally ignored; provenance pass | `peer-verify-2.log`; path/version reproducibility experiment not selected |
| `cargo test --locked -p runtime_probe --lib perf_context::tests --all-features` | Final source | 1 pass | `final-focused.log`; bounded probe zero/cap/wide projection |
| Strict all-target/all-feature Clippy for `canic-testing-internal` | Final helper guard | Pass | `final-focused.log` |

Initial merge compilation/CLI test failures were corrected before these passes;
`core-1.log` and `callers-1.log` retain them. The first embedded-peer native test
attempt found provisional snapshot helpers whose sole callers were excluded from
that test configuration. Both helpers now use the same configuration guard as
those callers. `peer-verify.log` retains the failure; the rerun passes without
lint suppression. This final change excludes unused functions in native tests;
it leaves the governed installed-test configuration unchanged. No production
Metrics or probe source changed after its passing installed checks.

The public sampling test reports actual recorder instructions: **3,418 first
insertion, 1,511 warm** on this graph. Complete sampling/history measurements are
retained in its log. These are not a controlled before/after comparison, cycle
costs, or a release speedup. Nested proof and runner timings must not be added.

## Remaining acceptance and release limits

The arithmetic/client cutover and selected installed behavior are qualified
locally. This is **not unconditional 100% acceptance**:

- [Canic #535](https://github.com/dragginzgame/canic/issues/535) still needs
  heap-cost acceptance. Earlier PR value-layout evidence (32 bytes per counter)
  is historical, not current whole-table allocation measurement. Current
  recording-cost evidence does not discharge that separate obligation.
- Fresh hosted/macOS evidence remains unresolved. No complete release gate was
  rerun. Optional histograms and stable Metrics adapters are not requirements
  of this arithmetic cutover.
- [IC Tooling #263](https://github.com/dragginzgame/ic-tooling/issues/263) records
  the independently reproduced cached-snapshot/global-timer mismatch. The
  maintainer owns upstream follow-up. Do not weaken timer ownership checks or
  infer its resolution from the passing Metrics cases.
- Earlier NNS/signed-request/Root-baseline runtime repair acceptance remains
  separate. The historical October 27 activation conflict remains unexplained;
  isolated recovery passes are not proof against that historical failure.

The source batch is reviewable, but release readiness is not established. Retain
these results and the supplier reproducer rather than starting another release
loop. Source release, package publication, CLI installation, App build, Fleet
deployment and live verification remain separate stages.

## Additional local qualification — 2026-10-29

The recorded frozen locked graph passes 51 focused recorder cases, 15 performance cases
and all 11 installed timer/metrics cases. A native allocation observer now measures
the production Auth pilot table through its four authored recording paths, including
application-session events. On x86_64 Linux, cold recording requests 620 bytes,
releases 212 and retains 408; 512 further records allocate nothing. The complete
internal table snapshot requests 600 temporary bytes in two allocations and
releases them all; public DTO/label conversion is outside that measurement.
Dropping the table releases its remaining 408 bytes. These are allocator requests,
including HashMap capacity and control bytes, rather than counter value sizes.
Allocator headers, fragmentation and Wasm-specific Auth table heap remain unmeasured.
This supplies native whole-pilot evidence for #535 without claiming all-family or
deployment-target heap acceptance.

The current installed Wasm-store scalar recorder measures 3,411 instructions for
first insertion and 1,532 warm. This is a different recorder from the native Auth
table measurement. The original four-case same-instance timer/snapshot sequence
and all five affected NNS/funding/signed-handoff cases also pass on Testkit 0.69.13.
Those direct sequence results supersede the local failure status above for the
reproduced cases; they do not establish hosted/macOS or live-provider acceptance.
Commands, source hashes, observations and retained setup failures are under
`target/adoption-qualification-20261029/`.

The root changed concurrently after the qualification freeze (Platform/Call source,
Build/Testkit 0.69.14 and Metrics 0.17.4 among the later selections). These results
do not qualify that moving source; its owner must rerun affected checks.
