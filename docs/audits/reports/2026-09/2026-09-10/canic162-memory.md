# CANIC-162: protected physical allocation observations

Date: 2026-09-10. Open 0.110.14 candidate; packages still 0.110.13.

## Current disposition: published dependencies aligned

The maintainer requested newly published IcyDB 0.257.4, which requires
ic-memory 0.13.2. All six IcyDB packages advance together and the lockfile has
exactly one ic-memory identity. The reviewed CANIC-162 source is restored from
the verified deferral patch; the stale mismatched lockfile is not restored.
The single-runtime guard remains enforced. No sibling repository changed.

The crates.io sparse-index record gives IcyDB 0.257.4 checksum
`fdf31d03df0d31e06cfb437c61d665534c95bc51f529a6bc27e0e95a26cc10e8` and
publication time 2026-09-10T13:29:46Z. The verified package records upstream
commit `ddfb5823351d288e31313fef7105ee1d386a5ec6`. Canic retains
`default-features = false`. No additional Canic lifecycle adaptation is needed.

Fresh qualification on the aligned graph passes:

- All eight memory unit cases, three ABI guards and 15 timer-inventory guards.
- Exact Canic/IcyDB lifecycle PocketIC proof: 53.28s case, 80s runner. Startup,
  prepared/active restoration, persisted data, participant trap/retry,
  callback ordering and timer custody pass together.
- Exact protected allocation PocketIC proof: 19.79s case, 26s runner. Root and
  managed controller rejection/acceptance, relayed authority, conservation,
  identical repeated reports and unchanged stable-memory SHA-256 pass.
- The governed catalogue regression preserves registered membership/order.
- Warning-denied all-feature library/test Clippy for core, control plane,
  facade, host, internal fixtures, IcyDB probe/schema and both reset roles;
  separately scoped native-agent integration Clippy also passes.

Existing artifact reuse contributes to these timings; no full-gate speedup or
new VS1 measurement is claimed. Source content and membership are unchanged
through qualification. The lockfile changes only the seven intended external
package versions and their required dependency edges; unrelated external
package records are identical. The editor resolved the lockfile after the
manifest update; agent qualification waited for its check to finish and used
locked commands. Both owned PocketIC servers/scratch are cleaned up, with
Cargo artifacts retained. The complete scoped .14 batch and changelog are
ready for release review; no broad agent gate or publication ran.

The [qualification bundle](canic162-evidence/qualification.tar.gz) retains
current source/dependency evidence and logs alongside the original deferral
bundle. The temporary 0.12.3 / IcyDB 0.257.2 composition passed five memory
cases, three ABI guards, 23 host cases, four timing/catalogue cases, scoped
Clippy and its 53.99s PocketIC lifecycle proof before adoption resumed.
Those results and VS1's 0.13.1 timings remain historical checkpoints.

## Finding and implementation

Toko Miner's CANIC-162 reports a small Game Hub at 232 MiB physical stable
extent. At the retained default of 128 pages per bucket, 29 buckets plus the
manager page occupy 232.0625 MiB. That arithmetic does not identify owners,
measure payload occupancy, or establish a leak. No live call or deployment ran.

Canic adopts published ic-memory 0.13.2, including its exact
`ic_stable_structures` re-export. Core and control-plane stores and the two
application reset fixtures use `RuntimeMemory` directly. Canic-owned manifests
no longer depend separately on ic-stable-structures. The published package
records upstream commit `0dcd24d555d9260d24bea9b3a407f62dbbcb8583`.

The 0.13.2 patch forwards raw reads to upstream virtual memory and uses its
shared-backing implementation, avoiding wrapper-level destination zeroing.
Canic requires no Rust source adaptation for this patch. Bucket defaults,
allocation-report shape and durable format remain unchanged. Instruction and
cycle savings have not been measured.

`MemoryQuery::allocations()` projects the substrate's bounded owned report.
It preserves actual persisted bucket size, manager capacity, physical and
virtual extents, every usable ID's bucket allocation and slack, current bindings,
the ledger binding, range policy, unknown-binding bytes and unmanaged extent.
All 255 usable IDs remain ordered, including zero-page memories. Payload bytes
remain unavailable; range authority does not invent a stable-key owner.

Collection reads 34,848 bytes of validated manager metadata plus bounded current
declarations. It does not decode history, initialize stores, grow or write memory,
construct a second manager, or advance the generation. Canic requires completed
bootstrap and preserves typed runtime failures. Physical extent is the actual
backing extent: IC stable memory on Wasm, supplied backing on native.

The report preserves these conservation checks:

- Physical extent = manager metadata + allocated bucket bytes + unmanaged bytes.
- Allocated bucket bytes = the sum of per-ID allocated bytes.
- Allocated bucket bytes = known-binding bytes + unknown-binding bytes.
- Allocated bucket bytes = virtual extent + bucket slack.

Ledger allocation is already included. Unknown bindings remain managed
allocations; unmanaged extent is a separate residual. Virtual capacity and
bucket slack do not measure record payload occupancy.

`MemoryAllocations` uses Root and managed protected observations and the existing
controller-authenticated Root relay. Host routing supports it; Store returns a
typed unsupported result. Shared Store and Coordinator Candid do not reference
this DTO or the changed role macros. There is no new public endpoint or CLI
command. The 128-page default remains unchanged.

## Previous 0.13.2 qualification before IcyDB alignment

The 0.13.2 adoption passes all eight focused memory unit cases and the exact
protected PocketIC authority/conservation/read-only case. The real case takes
243.19s; its governed runner takes 344s including compilation and cleanup.
The target's stable-memory SHA-256 is unchanged after both relayed observations.
Warning-denied all-feature library/test Clippy passes for core, control plane,
facade, host and internal fixtures. Both managed-memory access guards pass;
the package-identity guard still fails for the exact 0.12.3/0.13.2 pair below.

The source inventory remains unchanged throughout qualification. The
[0.13.2 evidence](canic162-evidence/adoption-0132.log) retains commands, results,
source/package hashes and the freshly fetched IcyDB registry record. The
0.13.1 VS1 throughput timings remain separate historical checkpoints; no new
throughput percentage is inferred from this adoption case.

### Previous 0.13.1 checkpoint

All eight 0.13.1 memory unit cases pass. They cover
bootstrap requirements, ledger/generation stability, identical replay, zero-page
IDs, ordering, conservation and Candid roundtrip, plus actual nondefault buckets
with explicitly unknown allocation ownership.

The exact PocketIC case passes in 271.03s (381s for the governed runner,
including compilation). It checks Root/Workload controller rejection, Root
controller acceptance, relay authority, actual physical extent, conservation,
ledger inclusion, unavailable payload, identical reports and unchanged target
stable bytes by SHA-256. This is fixture qualification, not diagnostic latency
or live Game Hub attribution.

Warning-denied library/test Clippy passes for all five changed runtime, host,
facade and fixture packages, as does scoped Clippy for the changed native-agent
integration target. Both changed application roles build successfully through
Canic's validated role path. A direct Wasm Cargo check first hit the intentional
role-build guard; the supported builds replace that attempt. Formatting,
document semantics, diff whitespace and release-note preflight pass.

Both managed-memory access guards pass. The package-identity guard fails with
the two exact memory versions documented below; this remains a release blocker.
The [0.13.1 evidence](canic162-evidence/adoption-0131.log) retains these results,
including the failure and the refused direct build attempt.

Commands:

```text
cargo test --locked -p canic-core --lib --all-features ops::runtime::memory::tests
cargo test --locked -p canic-core --all-features --test stable_memory_abi_guard
RUSTC_WRAPPER= make test-pocketic-case CASE=pic::fleet_registry::baseline::tests::protected_memory_allocations_preserve_stable_state_and_root_authority
cargo clippy --locked -p canic-core -p canic-control-plane -p canic-host -p canic -p canic-testing-internal --lib --tests --all-features -- -D warnings
cargo clippy --locked -p canic-tests --test native_agent_delegation --all-features -- -D warnings
RUSTC_WRAPPER= cargo run --locked -p canic-cli --bin canic -- build test user_hub --profile fast
RUSTC_WRAPPER= cargo run --locked -p canic-cli --bin canic -- build test user_shard --profile fast
```

The initial 0.12.3 observation passed seven memory cases and the exact protected
PocketIC case, plus scoped Clippy. Historical logs remain
[unit cases](canic162-evidence/memory-tests.log),
[PocketIC](canic162-evidence/pocketic.log),
[Clippy](canic162-evidence/clippy.log),
[document check](canic162-evidence/documents.log), and
[release-note preflight](canic162-evidence/release-notes.log).
The intervening 0.13.0 projection passed eight unit cases and scoped Clippy;
earlier results do not substitute for 0.13.1 qualification.

## Remaining ownership

The published dependency mismatch is resolved by IcyDB 0.257.4. Current
composition and allocation-report proofs pass together; the maintainer-selected
release gate remains.
The [original ic-memory prompt](canic162-ic-memory-prompt.md) and temporary
deferral are handoff history. Actual small-Hub attribution, lifetime capacity
and IC cost measurements remain prerequisites to any smaller bucket policy.
No memory reduction or instruction/cycle saving is claimed. VS1 remains
qualified at its recorded checkpoint; this update does not measure the full
release gate or authorize versioning, publication or deployment.
