# CANIC-164 public memory allocation summaries

2026-09-10. Canic development from published v0.110.14 (`c9c91f19c`), in the
existing open 0.110.15 batch with CANIC-160. Sibling repositories were read only.

## Implemented contract

The existing `performance` selection now includes 23 fixed anonymous allocation
gauges. It reuses the public metrics sampler, cache, source timestamps, paging
and bounded history. No new configuration key, public endpoint, Candid shape,
application producer, polling coordinator or dependency version is introduced.
The [public contract](../../../../features/runtime/public-observability.md#public-allocation-summaries)
defines every name, unit and interpretation.

The implementation owner is `canic-core` public-metrics ops. Memory ops shares
the substrate's bounded current report with protected diagnostics and the public
projection. The public collector never initializes the runtime, opens stores,
decodes rows/history or invokes remote collection. Only successful complete
reports publish numeric allocations. It independently checks the physical,
capacity and binding partitions, usable-ID coverage and unavailable payload
occupancy before creating public rows.

`memory.allocations.state` has unit `state`: 1 means available, 2 unsupported,
3 failed/incomplete. An absent state means no allocation sample was published.
Disabled publication skips collection and hides rows. Store publishes the
unsupported state without allocation values. A first failure supplies only state;
subsequent failures retain successful allocation rows at their original times.
The state row records the new attempt time. Other Performance rows can remain
valid, so consumers must inspect allocation state and age rather than infer
availability from the containing family's state alone.

Bindings are summarized as current declarations, the substrate ledger, and
unknown bindings. Range claims and owner/key strings never become public labels.
Current bindings are not a crate-specific breakdown. Known-binding totals already
include the ledger. Ownership and capacity are independent partitions, not
quantities to add together. Payload bytes are unavailable; bucket slack does
not measure unused record capacity or prove a memory leak.

## Native and failure evidence

All 26 focused public-metrics/memory tests pass. New cases cover disabled and
unsupported collection without calling the source, first failure without zero
measurements, retained source time/history on failure and successful recovery,
unknown ownership despite a claimed range, and rejection of independently broken
conservation/coverage fields. A real native manager with one-page buckets allocates
every usable ID, bootstraps the ledger, and produces the expected measured bucket
size and nonzero unknown ownership. Its complete backing bytes remain unchanged
by report collection and projection. Existing unopened-memory and ledger-generation
checks remain intact.

Core library/tests, internal Fleet fixture/probe library/tests, and the timer
integration target pass warning-denied Clippy with all features. Final import
organization was nonsemantic; runtime source was unchanged afterward.

## Runtime sampling and history

All nine `timer_authority` cases pass in 133.49 seconds (203-second governed
runner, including compilation). This includes actual canister sampling, protected
observer refusal, optional-family rejection/cycle-tracking isolation, scheduled
sampling and restoration behavior.

The new case populates 4,096 performance checkpoints, beyond the producer's
bounded retained prefix, then samples across 300 periods and history rollover.
Each iteration checks an unchanged complete allocation report, including all
255 usable IDs, sizes and ledger generation. The native case above additionally
covers all IDs physically allocated. The collection path is local and synchronous;
the no-remote-collection boundary is established by its source call graph.

| Measured sampler scope | Instructions |
| --- | ---: |
| First sample, 256 checkpoints | 17,741,093 |
| Repeat sample, 4,096 checkpoints | 17,766,504 |
| Scheduled maximum | 18,728,667 |
| Maximum through full history and rollover | 18,065,610 |

The history case retains 287 points after rollover, using 4,097,514 conservatively
accounted bytes across retained series against the unchanged 8,388,608-byte ceiling.
Points are bounded to the existing 288-slot window; the 301-second test cadence
can skip a slot. Both the byte and series limits and truncation reporting remain
enforced. These are complete sampler measurements, not an isolated before/after
allocation-cost delta. The 20M historical reference remains advisory. No claim
is made about Toko's complete application producer or its live IC cost.

## Root, Hub, Shard and Store proof

The exact governed case
`pic::fleet_registry::baseline::tests::generated_mixed_topology_and_ready_reserve_recover_one_reviewed_operation`
passes in 1,110.58 seconds (1,138-second runner), including 369.31 seconds of
initial artifact work. The maintained mixed-topology fixture selects Performance
and retains its existing two deliberate wipes, lost-response recovery, real row
reset/preservation, conservation and effect-free replay assertions.

After recovery, anonymous queries receive cached samples with all 255 IDs covered
on Root, Scale Hub, User Hub, User Shard, App and Scale Replica. Protected
diagnostics remain denied to anonymous callers. Authorized measurements agree
with cached physical extent and bucket size; public queries preserve stable
backing bytes. Store returns state 2 and no allocation totals.

| Role | Sampled physical stable bytes |
| --- | ---: |
| Root | 469,827,584 |
| Scale Hub | 218,169,344 |
| App | 218,169,344 |
| Scale Replica | 218,169,344 |
| User Hub | 251,723,776 |
| User Shard | 234,946,560 |

These are disposable fixture allocations, not Toko attribution or memory savings.
The public-memory qualification phase takes 35.03 seconds, including test-side
full stable-memory copies for equality checks. Those copies are test verification,
not work done by the public sampler or query. Prior journey timings use different
source/config/cache states and are not a controlled speed comparison.

## Reproduction and retained evidence

With `ICP_ENVIRONMENT=local`, `RUSTC_WRAPPER=` and `CARGO_NET_OFFLINE=true`, after
checking that no other Canic command owns the shared target:

```sh
cargo test --locked -p canic-core --lib --all-features -- ops::runtime::public_metrics:: ops::runtime::memory::tests --nocapture
make test-pocketic-case CASE=timer_authority
make test-pocketic-case CASE=pic::fleet_registry::baseline::tests::generated_mixed_topology_and_ready_reserve_recover_one_reviewed_operation
```

The [receipt](canic164-public-memory.json) records source, tools, commands,
measurements and checksums. The [evidence archive](canic164-public-memory-evidence.tar.gz)
retains final native, lint and runtime logs. The aggregate includes regular tracked
files and the two new Rust files: sorted paths followed by NUL and each binary
SHA-256 digest, then SHA-256 over that stream. All 3,900 files remain unchanged
across the two PocketIC proofs. Final status/evidence documentation follows those
proofs. The extracted evidence passes a targeted built-in-rule secret scan.

## Delivery boundary

The scoped Canic correction is complete with the open .15 notes. Package versions
remain .14; no Git publication, deployment, broad validation or sibling mutation
ran. Downstream work is to adopt the released projection, map state/units/coverage
correctly, preserve selected-canister explicit refresh, and qualify displays and
charts. CANIC-162's live attribution and any memory-saving policy change remain
separate; this projection does not change bucket allocation policy.
