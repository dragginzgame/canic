# Toko feedback follow-up — 2026-09-18

The selected Canic work for CANIC-176, CANIC-172 and CANIC-160 is complete in the
open .24 batch. Packages remain .23. This is working-tree qualification, not a
published validation receipt or a claim that Toko staging recovered.

## Build ownership and early readiness

Complete-build waiting reports bounded advisory owner PID, workspace, profile
and acquisition time on the same locked inode. Kernel exclusion remains the
only locking authority. Focused proofs cover a killed holder, stale/malformed
metadata, exclusion after reacquisition, normal cleanup, and a waiter reusing
an exactly verified completed release. Tampered output still rejects.

`canic --environment staging fleet readiness staging --operator <principal>`
checks signer, enrolled network identity, operator Ledger balance and retained
local work before compilation. `--estimated-cycles` is explicitly a caller
estimate; no estimate means an unknown requirement. Blockers produce unsuccessful
exit status after the report. The command does not create state, take the
operation lock, generate plans or authorize payments. Exact admission remains
at apply. Native tests cover retained completion states, malformed evidence,
unsafe labels, signer/network mismatch, unknown requirements and shortfalls.

Toko's deployment wrapper still needs to call this command before its build
after adopting the published Canic version. Artifact-only builds remain usable
without a Fleet or online signer. No Toko files changed.

## Combined real-Ledger recovery proof

The new exact host PocketIC case uses production ICP Ledger, CMC and Cycles Ledger
canisters and the production authenticated mint transport. A minimal test
EnsurePlatform adapter reads real management state and submits real withdrawals
to one empty installed canister designated as the host plan's treasury.

The proof reproduces a retained original Fund intent with zero starting operator
cycles, reviews it without changing its plan or initial balance, authenticates
one conversion credit, and resumes the original withdrawal. The first successful
withdrawal reply is deliberately lost. Retry receives the Ledger's Duplicate
receipt, reaches host terminal conservation and leaves the actual debit equal
to the reviewed amount plus fee. Repeating conversion and Fleet apply makes no
further withdrawal and leaves the journal and Ledger balance unchanged.

This qualifies the combined monetary path and same-operation host persistence.
It does not run a managed Coordinator/Root/Store lifecycle, the native ICP CLI
withdrawal adapter, production mainnet traffic or Toko's exact retained estate.
The older four-boundary mint proof was rerun after sharing its production-Ledger
setup; normal approval, lost transfer and notification replies, and interruption
before credit persistence remain covered. The new proof passed in 3.73 seconds
and the existing proof in 5.30 seconds; these are test diagnostics, not throughput
or deployment benchmarks.

## Controlled readiness phase comparison

One test executable runs both schedules against the same configured owners,
status JSON, artifact bytes, host code, dependency lock and tools. The serial
counterfactual calls the existing status reader sequentially; the candidate
calls the actual bounded protocol-owner readiness collector (four in flight).
The fixture inserts a 20 ms delay per read. Three pairs alternate execution
order for each topology. Both schedules check Running status and exact module
hash and issue exactly one read per owner. No paid effect is involved.

| Owners | Serial samples (microseconds) | Bounded samples (microseconds) | Median serial | Median bounded | Median reduction |
| --- | --- | --- | --- | --- | --- |
| 3 | 74144, 71543, 70899 | 24835, 23999, 23908 | 71.543 ms | 23.999 ms | 66.46% |
| 9 | 217870, 218131, 213244 | 73459, 72796, 71514 | 217.870 ms | 72.796 ms | 66.59% |

The measurement ran alone among Canic tests. Unrelated machine activity was not
controlled. This is a synthetic host scheduling comparison, not two published
release binaries, a live IC latency sample, instruction/cycle savings or a
whole Toko deployment comparison. It does not quantify compilation, payment,
provisioning or full-suite speed. Three pairs are local evidence, not a latency
distribution. The timing test has no speed threshold and is ignored in ordinary
release testing; existing correctness tests own concurrency and failure bounds.

Reproduce this phase only with:

```sh
RUSTC_WRAPPER= cargo test --locked -p canic-host --lib \
  fleet_ensure::ops::platform::tests::protocol_owner_readiness_matched_latency_measurement \
  -- --exact --ignored --nocapture --test-threads=1
```

Both schedules were compiled together in the repository's optimized test profile
on x86_64-unknown-linux-gnu, Rust 1.98.1 (`48a229ceaefd4985c50990b14116b6d856af0985`),
LLVM 22.1.8. Base HEAD was `e4e04ea166acebb91a5ee8bef154daf747ede4ea` plus the dirty
.24 work. The selected source and dependency identities are:

| Input | SHA-256 |
| --- | --- |
| `Cargo.lock` | `0d589fecdcc43038fd3c4285f02b68dce61f28f84f2f17954ed501f17753e80c` |
| `Cargo.toml` | `9697988acf273bd28e24ce49fc7a0cd1dd56384b3c8a180566da99e6c1c676e0` |
| `crates/canic-host/src/fleet_ensure/ops/platform.rs` | `5a3a19c7150d6c56fe4eed99daa1c345386c3cc65300e11aea4017b5bb408dd8` |
| `crates/canic-host/src/fleet_ensure/workflow/funding_tests/real_mint_funding/mod.rs` | `930c373b27c1d3ce217dd34b2eed7602343cb9bdfae5449d085369061c356713` |

## Focused validation and release disposition

- 30 selected host/CLI native regressions pass, including exact release reuse,
  lock ownership, early readiness and existing CLI Fleet behavior.
- Both recursive CLI help/order tests pass.
- Both exact production-Ledger PocketIC cases pass; no broad lifecycle suite ran.
- The isolated readiness comparison passes with unchanged call counts.
- Host and CLI warning-denied Clippy passes with all targets and all features.
- Changed-file formatting, whitespace, layering, shell syntax, scoped ShellCheck
  and exact runner selection checks pass.

Local logs: `/tmp/canic-feedback-native-final.log`,
`/tmp/canic-feedback-help.log`, `/tmp/canic-feedback-mint-native.log`,
`/tmp/canic-feedback-mint-interruptions.log`,
`/tmp/canic-feedback-readiness-comparison.log` and
`/tmp/canic-feedback-clippy.log`. Sample timings and source identities above are
retained here because temporary logs are not durable release receipts.

The complete selected .24 batch, including the earlier donation, funding,
dependency and speed changes, is ready for the maintainer-selected release flow.
Both changelog surfaces are updated. Publication/adoption, exact Toko staging
recovery and matched end-to-end downstream timings remain separate qualification;
none is represented as completed here. No version bump, commit, push, live
funding/deployment or sibling mutation occurred.
