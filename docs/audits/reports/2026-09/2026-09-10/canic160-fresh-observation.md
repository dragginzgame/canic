# CANIC-160 fresh-estate inspection reuse

2026-09-10. Development from published v0.110.14, release commit `c9c91f19c`.
The maintainer selected the fresh-estate duplication follow-up after publication.

This report retains the initial deduplication checkpoint. The subsequent
[bounded-concurrency qualification](canic160-pool-concurrency.md) records the
current source and PocketIC proof separately; the hashes and timings below
continue to identify the original checkpoint.

## Outcome and scope

The fresh pending path now retains a successfully validated inspection in the
existing Fleet observation snapshot. Later pool-balance review reads that same
response. Root-owned and funding callers share the private management projection
and read helper; no product protocol or Candid endpoint changes.

The owner is `canic-host/src/fleet_ensure/ops/platform.rs`. This batch contains
that correction, native transport regressions, measurement evidence and .15
release notes. It is a follow-up to the published .14 observation work, not a
new cache or a cross-observation financial-authority store.

Each consumer keeps its authority rules. Fresh creation checks the exact Root
artifact/controllers, active operator, empty target module and exact temporary
Root-plus-operator or Root-only controllers. Funding keeps its running-Root,
operator, Root-only controller and requested-module checks. A cached response
accepted by one consumer may still be rejected by another. Transport failures
and newly rejected responses are not inserted. The snapshot expires on success
or error before effects; out-of-scope reads remain fresh. Terminal inventory,
mutation order, polling and effect replay are unchanged.

## Reproduction and measured evidence

The native fixture runs the production configured-canister observation path on
fresh PendingReset assets with zero recorded balances, followed by balance
review. Responses use the normal Candid/CLI adapter. There is no simulated IC
latency, actual canister creation, installation or IC traffic.

The control removes only the fresh path's retention call from the candidate,
reproducing the released cache bypass. It leaves the fixture and observations
intact. The regression finishes all three inventories and fails on 42 redundant
calls. The candidate restores retention and passes. This is a mechanism control,
not a benchmark of the complete published .14 executable.

| Assets | Configured calls, both | Extra balance calls, control → fixed | Balance stage ms, control → fixed |
| ---: | ---: | ---: | ---: |
| 1 | 3 | 1 → 0 | 7 → 2 |
| 14 | 16 | 14 → 0 | 196 → 40 |
| 27 | 29 | 27 → 0 | 218 → 78 |

Configured calls comprise one Root status, one pool page and one inspection per
asset. They remain intact. Durations are single native fixture observations,
include local CLI launch overhead, and are not enforced as timing thresholds.
The downstream 20.771-second duplicate-stage report remains potential cost;
these measurements do not establish that amount as a real deployment saving.

The [structured receipt](canic160-fresh-observation.json) records this checkpoint's
source digest and measurements. The [evidence archive](canic160-fresh-observation-evidence.tar.gz)
contains control/candidate logs, the final 29-case run and final Clippy output.

## Validation and remaining acceptance

All 29 focused platform and bounded-observation cases pass. Coverage includes
success/failure expiry, distinct Roots/targets, stricter module/controller
rejection across consumers, changed operators, fresh transport/controller
failure recovery, and Root artifact/controller checks despite a cached response.
Existing bounded management-read tests retain ordered errors and drained batches.
Host library/tests Clippy passes with all features and warnings denied.
Formatting, document checks and .15 release-notes preflight are checked separately.
No broad suite or new PocketIC journey was run for this host observation change.

At this checkpoint pool reads still serialized. After deduplication the first
stage still required one inspection per asset. The subsequent report qualifies
bounded overlap of those configured reads.
The native fixture cannot establish Root execution queue behavior or mainnet
benefit. Measure the same real estate and then evaluate overlap of independent
reads after Root authority is established, retaining deterministic errors and
complete batch draining. Do not extend the snapshot across effects or remove
terminal checks to obtain a faster result. Un-timestamped provisioning waits
remain a separate attribution question; this change does not identify their cause.

The selected Canic correction is complete and reviewable in the open .15 draft.
CANIC-160 overall remains pending downstream equivalent-estate qualification;
no Toko Miner files, live estate, package version or Git publication changed.
