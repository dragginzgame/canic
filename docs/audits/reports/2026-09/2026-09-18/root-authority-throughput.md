# Root/Store authority-read throughput — 2026-09-18

The post-.24 CANIC-160 planning slice overlaps independent authority pairs using
the existing collector, with at most four queries in flight. Each Root's response
still selects its Store, whose returned authority must match exactly. Results and
errors retain configured order. An issued batch drains before failure returns;
later batches are not started. Every retry performs fresh queries. No payment,
deployment effect, identity binding or runtime state changed.

## Upstream feedback

The latest Toko scan records adoption of published .24 library/CLI pins at
`933a35b66403f6a94f99803252cc52c0aec32958`. Read-only sources were
`../toko-miner/docs/upstream/canic.md` and `docs/upstream/scan-log.md` in that
checkout, including its “Published Canic 0.110.24 adoption” entry. CANIC-172's
retained staging recovery remains unverified. CANIC-160/176's actual deployment
timing and warm reuse remain qualification work. No new confirmed Canic defect
was identified; no sibling state or source was modified.

## Matched scheduling measurement

One executable runs serial pairs and the actual bounded collector against the
same typed responses and CLI fixture. Each query has 20 ms synthetic latency.
Five pairs alternate order for each topology. The serial schedule invokes the
same reader with one Root at a time; it is a scheduling counterfactual, not the
old published binary. Both schedules assert identical ordered authorities and
exactly two queries per Root. No timing threshold gates correctness.

| Roots | Serial samples (µs) | Bounded samples (µs) | Serial median | Bounded median |
| --- | --- | --- | --- | --- |
| 1 | 55115, 55163, 55230, 55326, 55241 | 57576, 54635, 55172, 53915, 55379 | 55.230 ms | 55.172 ms |
| 4 | 221086, 220864, 218983, 212681, 211999 | 56265, 54462, 54234, 53558, 53346 | 218.983 ms | 54.234 ms |
| 9 | 477000, 476991, 478187, 474412, 478293 | 160439, 159523, 159990, 159507, 159509 | 477.000 ms | 159.523 ms |

Median reduction is 75.23% for four Roots and 66.56% for nine; the single-Root
difference is negligible. This does not measure IC instructions/cycles, live
network latency, artifact compilation, total deployment or release validation.
Unrelated machine activity was not controlled. No other Canic build/test owned
the shared target at the start of these runs. The complete .24 test runner's
4,187 seconds remains the measured release baseline; this slice cannot explain
or promise a large reduction in that total.

## Source and validation

- Base: `933a35b66403f6a94f99803252cc52c0aec32958`, tag `v0.110.24`.
- Cargo.lock SHA-256: `e2d8e186b747bb35ec191f597b8e5dd90f5f172749196e8bdfbfbc1054496cc9`.
- Changed `current_protocol/mod.rs` SHA-256:
  `b86255386cca33c17f987e7c039bbcecde4f53086255df0dbb2318f6a959609c`.
- Measured `current_protocol/tests.rs` SHA-256:
  `b5ac4705b7752e74d9df0192b7f14533fb4c3349abee946fe52ad9f1f2c897ab`.
  Final import-only cleanup produces
  `5aef470c29ffbe9cb7bfeb554ab4f7bf05521ecac4be2cd3b4c1e1014eaf9883`.
- Rust 1.98.1, commit `48a229ceaefd4985c50990b14116b6d856af0985`, LLVM 22.1.8,
  x86_64-unknown-linux-gnu, repository optimized test profile. Both schedules
  share the executable, dependency lock, features and fixture.
- Twenty focused current-protocol tests pass; the timing test is ordinarily
  ignored and passes when selected explicitly. New transport fixtures require
  overlap without timing assertions and cover empty/single/partial batches,
  missing authority, disagreement, drained failure, no later scheduling and
  corrected retry. These are host query-adapter tests, not simulated IC lifecycle
  qualification; no canister behavior changed.
- Warning-denied host library/test Clippy, changed-file formatting and whitespace
  checks pass. No broad gate or PocketIC suite ran.

Local logs: `/tmp/canic-authority-protocol-tests.log`,
`/tmp/canic-root-authority-measurement.log`, and
`/tmp/canic-root-authority-clippy.log`.

Reproduce the phase measurement with:

```sh
bash scripts/ci/run-with-test-scratch.sh cargo test --locked -p canic-host --lib \
  fleet_ensure::ops::current_protocol::tests::root_authority_reads_matched_latency_measurement \
  -- --exact --ignored --nocapture --test-threads=1
```

Keep the bounded read change. Continue the larger accepted speed work in the
same .25 draft, concentrating on expensive artifact recipes and repeated setup
in the long PocketIC journeys. Preserve capacity, interruption, conservation
and replay coverage. No version bump, commit, push or live recovery occurred.
