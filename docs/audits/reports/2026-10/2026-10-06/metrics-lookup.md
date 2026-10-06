# Performance lookup and attribution evidence

The uncommitted `crates/canic-core/src/perf.rs` change borrows compound endpoint
and checkpoint keys for existing counters, owning their strings only on a miss.
A private standard `Borrow` view retains the public owned key, its original
derived ordering and the single authoritative map. `entries()` directly collects
ordered map iteration instead of sorting it again. Public APIs, report order,
zero, saturation, reset and endpoint-attribution functions are unchanged.
The compatible pending 0.110.53 notes cover
[#456](https://github.com/dragginzgame/canic/issues/456) and
[#451](https://github.com/dragginzgame/canic/issues/451).

Baseline is `e1a211a00f01568ccc99bedc494c62a7141444dd`. Final `perf.rs` SHA-256:
`01d5d77aca2a2fbec1f0ee244310b0e349599fd4a2f2005875bb0615f1632a77`.
Preserved root lock SHA-256:
`f0d49d1b6fc8c370322cb0661fa88ec677351098fc24e7dc61c668660dc7a052`.
It selects ic-metrics 0.1.5. Package versions remain 0.110.52.

## Actual IC observations

A separate unpublished fixture links the actual primary Core library, selecting
only dependency versions, sources and checksums present in the root lock.
Rust 1.99 builds size-optimized (`z`), fat-LTO Wasm with one codegen unit,
stripped symbols and aborting panics. PocketIC 16.0.0 executes on one application
subnet; server SHA-256:
`69e324bdb68d32d878b7a9504b1379f08f8d1921272bacb065b0fabb3d0f3792`.

Each scenario runs three fresh canisters after seeding two zero-valued keys:
10,000 repeated recordings, 1,000 new checkpoints, or 1,000 two-key reports.
Cardinality, counts and totals are checked. Each artifact/scenario has identical
observations across its three runs. Instructions bracket the loop. Actual cycle
charges are whole-update balance differences, excluding installation; they are
not conversions from instructions or native timing measurements.

| Scenario | Before instructions | After instructions | Before cycles | After cycles |
| --- | ---: | ---: | ---: | ---: |
| Repeated endpoint | 11,080,236 | 5,220,236 | 18,098,830 | 12,238,856 |
| Repeated checkpoint | 20,030,236 | 8,110,236 | 27,048,830 | 15,128,856 |
| New checkpoints | 17,182,532 | 31,710,300 | 26,657,645 | 40,507,669 |
| Two-key reports | 2,578,236 | 2,464,236 | 9,596,830 | 9,482,856 |

Repeated recording uses about 53% fewer endpoint instructions and 60% fewer
checkpoint instructions. New checkpoints cost about 85% more instructions:
a borrowed miss requires a second owned insertion lookup. This favors stable,
repeated keys, not unrestricted dynamic-label workloads. Raw fixture Wasm falls
from 332,503 to 328,805 bytes, saving 3,698 bytes. These are isolated local IC
fixture observations, not whole-managed-canister or mainnet savings.

Baseline Wasm SHA-256:
`c18f9001a9d6e3be251774cfc7cbe80bd2bc1ed8401aa389f0b05e5029c8bf19`.
Final Wasm SHA-256:
`35768521217146fa6fe478ca6d6c2eaecf005faf6f1349929de8248bafcc20ee`.
Fixture lock SHA-256:
`402b4854693071d4288d25a071018e8569b9cdc56b437f378377bb15be8226a6`.
Fixture inputs, before/final Wasm, build and check logs remain in
`target/evidence/metrics-improvements/`; host source and execution logs remain
in `/home/adam/projects/ic-metrics/target/evidence/consumer-improvements/`.
Final measurement log SHA-256:
`9651672861e35781c0662c7bace7bf77948c4969b10ab8a012b4bc04d10fb81e`.
The first candidate changed owned comparison implementation; retaining the
original derive improved its cold behavior. Its separate source, Wasm and logs
are retained. An initial alternate dependency graph was rejected before
measurement and its artifact retained as `alternate-graph.wasm`; it is not the
qualified baseline. The root lockfile was never changed.

## Async attribution confirmation

A separate source-copy fixture exposes audit trampolines around the actual
private enter/exit functions, whose bodies and counter reader match committed
source byte for byte. Two IC updates are queued before completion; both await a
self-call. Actual surrounding call-context reads bound each endpoint's interval:

| Endpoint | Valid instruction interval | Recorded total |
| --- | ---: | ---: |
| a | 976,726–979,030 | 1,473,794 |
| b | 1,216,697–1,218,747 | 1,463,693 |

Both totals are outside their own intervals. This confirms
[#99](https://github.com/dragginzgame/canic/issues/99#issuecomment-6012668977);
the allocation changes do not repair it. This probe exercises actual async IC
counter contexts and the private recorder, not complete managed deployment or
checkpoint `PERF_LAST` interleaving.

Scope probe Wasm SHA-256:
`15af1bf5b1d3660182e4c425f1d165746300969a1dcf87ce476d19387532184a`.
Execution log SHA-256:
`002876626256248c264e57288daf0b335327e15176bb382aad635ecdd3b1a192`.
The owned `scope-canister` directory and root harness evidence retain source,
lock, build and execution inputs. No native counter fake was used.

## Focused checks

`make fmt` passed before these locked offline checks using this repository's
`target/`:

- `cargo clippy -p canic-core --lib --tests --locked --offline -- -D warnings`
- `cargo clippy -p canic-core --lib --target wasm32-unknown-unknown --locked --offline -- -D warnings`
- `bash scripts/ci/run-with-test-scratch.sh cargo test -p canic-core --lib --locked --offline perf::tests::`
- `cargo +1.91.0 check -p canic-core --lib --target wasm32-unknown-unknown --locked --offline`

All five selected perf tests pass, covering mixed owned/borrowed keys, transport
kind, Unicode/prefix labels, ordering, zero, saturation and reset. The guarded
runner removes only its invocation-owned scratch. The Rust 1.91 Wasm check
passes after explicitly installing its missing target; the initial missing-target
failure log is retained. These are focused Linux results, not macOS or complete
release qualification. No broad tests/CI, release, commit, push, package-version
mutation or retained-artifact cleanup ran.
