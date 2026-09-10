# Metrics history index qualification — 2026-09-09

The uninstrumented Canic candidate passes the frozen Toko Miner metrics ceiling
at **19,963,567 instructions**, against the unchanged **20,000,000** threshold.
Headroom is **36,433 instructions (0.18%)**. This is a measured fixture result,
not a guarantee for different application activity or concurrent source changes.
Downstream publication/adoption and actual staging recovery remain open.

## Change and scope

Profiling the real 64-entity/194-row producer identified history lookup as the
largest phase: roughly 9.6M instructions in steady samples and 11.28–11.44M
on initial allocation. Repeated ordered comparisons of shared-prefix names
cost more than the source collector. The private history owner now uses a
hash index with full equality on family, metric name and optional canister.
Hash collisions cannot merge distinct keys. There is one index, with no parallel
lookup owner or protocol change.

Admission still follows validated input order. Index iteration only expires
old entries and sums reservations; it never chooses public output order.
The 256-series cap, 288-slot rings, 8 MiB bound, name limits, source timestamps,
counter resets and public ordering remain unchanged. Shrinking after mutation
releases sparse index capacity, including the final empty allocation. Existing
conservative per-series index accounting is retained.

The prior optional-producer timer correction is also present. The downstream
producer is unchanged; the earlier name-allocation experiment and all diagnostic
instrumentation are absent from this candidate. Qualification entities and
setup endpoints exist only in the disposable source copy.

## Exact candidate and evidence

- Canic base: `v0.110.13`, `5250d69186047bc373db7cdc7b7680c3aad031d0`, with the
  open 0.110.14 candidate changes described in the four-item report.
- Frozen Toko source: `f077590a4d2ab4fb85a901a5afab416c2e51dc36` plus the retained
  working-tree fixture; IcyDB 0.257.3. This does not change Canic's dependency pin.
- Candidate release: `35a269ebe02694e9c247b1476333ce092be67d8eb3e943e4fe4bb8fe60337857`.
- Build profile: `fast`; complete eight-artifact build, 177.42 seconds.
- All eight Candid sidecars match the preceding timer-corrected candidate.
- Three-period ceiling/recovery test: 25.02 seconds, peak 19,660,290.
- Twelve-period ceiling/recovery test: 28.11 seconds, peak 19,719,081.
- Full-window test: 82.94 seconds, 300 initial periods followed by recovery;
  peak 19,963,567. Every complete-callback maximum stays below 20M.

All nine nodes across five roles are checked. Each Game Shard publishes 194
application gauges from 64 actual IcyDB entities, including normal observer
entities and maximum-length fixture names. All three Game Shards then move
64 → 65 → 64: rejection preserves exact cached rows and source timestamp,
retained history gains no observations (natural expiry is allowed), independent
Cycles/Operations/Performance remain fresh, and corrected data resumes with a
new timestamp and gauge history. The instruction assertion runs again after
recovery. Other checks cover protected observer denial and memory/timer status.
Translation initialization completes before the clock jumps; startup-load cost
is outside this acceptance claim.

The [evidence manifest](toko-feedback-evidence/manifest.json) binds retained logs,
phase measurements, exact test body, unchanged producer, compiled core sources
and release manifests. Compiled runtime source matches the working tree apart
from comments and native tests. Diagnostic profiling uses a separate release,
`3b54a188afa0848db846a740f6a21325798dad72fdf50a3bbb5941e931348de7`;
its instrumented timings are explanatory, not acceptance measurements.

## Reproduction and checks

Use the existing `metrics-ceiling-fixture.patch` only in a disposable Toko copy.
Use retained `metrics-index-retention-test.rs.txt` for the exact test, the retained
producer unchanged, and a local override of `canic-core` to the retained source.
Build with the candidate CLI and profile above, then select the finalized
artifact directory explicitly:

```sh
RUSTC_WRAPPER= CARGO_NET_OFFLINE=true \
TOKO_MINER_QUALIFICATION_RELEASE_BUILD_ID=<candidate-release-id> \
TOKO_MINER_QUALIFICATION_ARTIFACTS_DIR=<candidate-artifacts-directory> \
cargo test --locked -p canister_toko_miner_user_hub --lib \
  qualification::metrics::managed_public_metrics_qualifies_failure_isolation \
  -- --ignored --exact --nocapture
```

Native public-metrics regressions cover exact key separation, full-to-sparse
index expiration, complete allocation release, bounded admission, timestamps,
retention and counter resets. The new expiry fixture initially used a time one
nanosecond before its source sample and was correctly rejected; correcting its
input preserves production validation. Targeted results are recorded below.

- `cargo test --locked -p canic-core --lib public_metrics`: 18 passed.
- `cargo clippy --locked -p canic-core --all-targets --all-features -- -D warnings`:
  passed (26.43 seconds).
- Targeted Rust formatting, diff whitespace, current-document semantics,
  edited local links, retained evidence hashes and 0.110.14 draft preflight:
  passed.
- `make test-pocketic-case CASE=timer_authority`: all eight cases passed after
  the index change (122.92 seconds including Wasm fixture builds; runner 192
  seconds including native compilation). The governed runner cleared its
  invocation-owned scratch and PocketIC resources.

The Canic implementation batch and open 0.110.14 changelog are ready for release
review. The full upstream acceptance set still needs downstream runtime adoption
and separately approved staging recovery. No broad validation, version bump,
Git publication or live deployment was performed.
