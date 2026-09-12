# CANIC-160 bounded pool inspection concurrency

2026-09-10. Development from published v0.110.14 (`c9c91f19c`), in the same
open 0.110.15 draft as the [initial reuse correction](canic160-fresh-observation.md).

## Outcome and boundaries

The host now overlaps independent configured pending-pool inspections after
preparing their Root/operator authority. It uses the existing limit of four
in-flight observations. Four is a scheduling budget, not a Fleet capacity or
admission limit; larger inventories continue in bounded batches. This change
does not claim that four is optimal for every network.

The production owner is `crates/canic-host/src/fleet_ensure/ops/platform.rs`.
Root status, pool-page queries and cursor traversal remain ordered. Prepared
inspection requests are deduplicated by exact Root/target and skip successful
responses already retained in the current observation. Workers only perform
the independent protected inspection calls; the existing snapshot stays on
the calling thread. A local batch holds completed transport outcomes, then
consumers validate and project them in configuration order.

All issued reads finish before success or failure returns. Transport arrival
order does not change which preparation, transport or consumer error wins.
A failed batch prevents the next batch from starting. Responses enter the
existing snapshot only after successful consumer validation; fresh creation
and funding retain their distinct controller/module rules. Errors and completed
observations expire the snapshot before effects. Pool assets encountered only
in balance review still read serially. Mutation scheduling, terminal inventory,
polling, product contracts and Candid endpoints are unchanged.

`InspectCanister` remains a protected Root command that awaits management
`canister_status`. Its transport is an update and can advance Root's canister
version; this is not a reinstall. Existing lost-response and terminal replay
proofs retain that distinction.

## Controlled native measurements

The same test binary compares the serial single-canister observation path with
the bounded configured-observation path. The native CLI fixture injects 20ms
per inspection, returns identical Candid responses and checks exact resulting
observations, remote-call counts and zero additional balance-review calls.

| Assets | Configured calls, both | Serial ms | Bounded ms | Extra balance calls, both |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 3 | 40 | 41 | 0 |
| 14 | 16 | 440 | 164 | 0 |
| 27 | 29 | 836 | 290 | 0 |

These are single controlled native measurements, including local process
startup. They demonstrate overlap and output parity, not mainnet latency or
whole-deployment savings. No timing threshold is enforced. Barrier/event-based
cases establish the concurrency bound, partial-batch draining, deterministic
error precedence despite reversed completion order, refusal to issue an
unauthorized inspection, and a fresh successful retry after failure. Test
batch sizes derive from the maintained concurrency bound.

All 33 focused platform/bounded-observation cases pass, including the earlier
reuse, expiry and authority regressions. Host library/tests Clippy passes with
all features and warnings denied.

## Exact runtime qualification

The governed exact PocketIC case
`pic::fleet_registry::baseline::tests::generated_reinstall_recovers_lost_install_and_reaches_working_fleet`
passes with the production host adapter. Its 27-canister estate contains three
infrastructure canisters and 24 pool assets, including 19 workloads and five
ready reserves. It proves reviewed authority rejection, retained-asset
inspection, lost-install-response recovery, bounded additional funding
reviews, working Fleet convergence, terminal conservation and effect-free
replay. This is the existing recovery journey; it does not substitute for a
live Toko Miner benchmark or the separate application-row wipe journey.

| Qualification phase | Seconds |
| --- | ---: |
| Initial artifacts | 217.997 |
| Initial working Fleet | 98.449 |
| Replacement artifacts and generation | 38.263 |
| Root reinstall review and authority rejection | 1.271 |
| Lost-response recovery | 15.797 |
| Successor reviews and convergence | 229.628 |
| Retained state and replay | 37.057 |
| Whole test, including other setup/cleanup | 659.46 |
| Governed runner, including native compilation | 759 |

Initial artifacts were rebuilt at package version 0.110.14. The runner compiled
the selected native test target in about 97 seconds. Earlier release timings
have different package/cache states, so no whole-journey speedup is inferred.
PocketIC 16.0.0 and native ICP 1.4.0 were selected by the governed runner.

Across 24 observations, balance review makes zero additional remote calls
and takes 7.826 seconds in total; local authority work still has a cost. Six
pool-heavy configured stages each make 28 calls and take 4.821–5.183 seconds.
Those absolute runtime observations confirm that the exercised larger estate
uses the changed path; the controlled native comparison above isolates overlap.

## Reproduction and evidence

Run these targeted commands with `ICP_ENVIRONMENT=local`, `RUSTC_WRAPPER=`
and `CARGO_NET_OFFLINE=true`, after checking that the shared target is idle:

```sh
bash scripts/ci/run-with-test-scratch.sh cargo test --locked -p canic-host --lib --all-features -- fleet_ensure::ops::platform::tests fleet_ensure::ops::bounded_observations::tests --nocapture
cargo clippy --locked -p canic-host --lib --tests --all-features -- -D warnings
make test-pocketic-case CASE=pic::fleet_registry::baseline::tests::generated_reinstall_recovers_lost_install_and_reaches_working_fleet
```

The [structured receipt](canic160-pool-concurrency.json) binds source, measured
records and results. The [evidence archive](canic160-pool-concurrency-evidence.tar.gz)
contains final native, Clippy and PocketIC logs. The tracked source aggregate
was checked unchanged before/after PocketIC over 3,898 regular tracked files:
sorted Git paths, each followed by NUL and the binary SHA-256 of its bytes,
then SHA-256 over that stream. Documentation changed only after the proof.

## Remaining acceptance

The complete scoped Canic implementation, adversarial cases, recovery evidence
and .15 changelog are ready for release review. Package versions remain .14;
no publication, live deployment, broad gate or sibling edit ran.

CANIC-160 remains open for equivalent-estate downstream IC timings. Record the
same topology, release inputs, network, cache state and observation-stage call
counts when comparing before/after. Provisioning waits still need attribution;
unconfigured balance-only inspections are a separate possible optimization.
Neither uncertainty warrants weakening authority, extending cached evidence
across effects or changing the application's initial/reserve estate policy.
