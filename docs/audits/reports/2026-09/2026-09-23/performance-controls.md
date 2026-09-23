# Local performance controls

Date: 2026-09-23. These measurements continue the
[Toko Miner feedback investigation](toko-feedback-followup.md). They are local
controls, not a reproduction of the application release build or live replay.

## Inspection contract parsing

The opt-in `canister_protocol::inspection::tests::retained_contract_parsing_measurement`
reads the exact retained Root Candid and runs the existing inspection-selector
validator 33 times per sample, matching the replay's inspection count. It
includes file reads, parsing, service/type resolution and selector validation.
Five samples took 96,395, 97,066, 95,754, 95,531 and 95,473 microseconds; the
median is 95,754 microseconds. The contract is 101,033 bytes and its hash is
retained in the [machine evidence](performance-controls.json).

The measurement uses the native test profile (`opt-level = 1`), warm filesystem
pages and a separately retained .38 artifact. It excludes network requests,
reserve queries, protected inspections and production CLI startup. It is not a
timing attribution from the 97.872-second live receipt. There is no timing
threshold or release gate in this opt-in measurement.

Decision: leave contract parsing unchanged. This local cost does not justify
adding a cache owner and invalidation state. Keep fresh reserve checks and exact
contract validation. The final authority/asset observation spans added in the
preceding slice will expose the material remote verification costs on the next
qualified invocation.

Reproduce with `CANIC_BENCH_INSPECTION_CANDID` set to the exact retained artifact:

```sh
cargo test --locked -p canic-host --lib \
  canister_protocol::inspection::tests::retained_contract_parsing_measurement \
  -- --ignored --exact --nocapture
```

## Build reuse controls

The existing opt-in
`canister_build::reuse::tests::complete::environment::frozen_relocated_and_changed_tree_measurement`
uses a small real-Cargo Wasm fixture and synthetic release sealing. It separates
input lookup, the forced compiler probe and sealing time. Even its warm case
deliberately runs the compiler probe; its total is not the production complete-hit
path's latency. The fixture uses the Fast profile, private targets and local
stub dependencies, so it cannot qualify release-profile application performance.

| Input change | Expected and observed reuse |
| --- | --- |
| Original cold | Miss |
| Identical warm | Hit, same inputs and Wasm |
| Relocated frozen checkout | Miss, changed input identity |
| Identical warm in relocated checkout | Hit |
| Qualification document only | Miss, unchanged Wasm |
| Runtime source | Miss, changed Wasm |
| Dependency source | Miss, changed Wasm |

Whole package trees are intentional build inputs because build scripts may read
their files. The qualification-document miss therefore does not establish an
invalidation bug. Relocation retains its current authority boundary. No cache
key, output validation, compiler mode or release requirement changes here.

The governed reproduction uses repository-local scratch:

```sh
RUSTC_WRAPPER="" bash scripts/ci/run-with-test-scratch.sh \
  cargo test --locked -p canic-host --lib \
  canister_build::reuse::tests::complete::environment::frozen_relocated_and_changed_tree_measurement \
  -- --ignored --exact --nocapture
```

## Remaining work

These controls do not close CANIC-176's application-scale release-profile
measurement or CANIC-160's live performance qualification. Neither a production
speed-up nor a faulty cache miss has been demonstrated. The separate completed
B1 footprint review remains the evidence for structural runtime contraction.
The maintainer accepted complete B1 and authorized B2 after these measurements.

Both controls pass, including the build matrix under governed repository-local
scratch. Scoped `canic-host` Clippy with all targets/features and warnings denied
also passes. The [parsing log](inspection-parsing.log) and
[governed build-control log](build-reuse-controls.log) retain raw output.
