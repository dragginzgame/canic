# Deployment speed investigation

Date: 2026-09-24. Current Canic source baseline: 0.110.40,
`1ee0f742f79af572f134ed10aa893f0b5aea3492`.
Disposition: prioritized candidates, not implemented or measured improvements.
The existing issue #28 test changes and dependency reminder are preserved.

Subsequent work: the [implementation report](deployment-speed-implementation.md)
records the accepted Store/query changes and their focused qualification. This
investigation remains the original evidence and prioritization snapshot.

## Measured starting point

Toko's retained 0.3.10 staging recovery used Canic .38. Its read-only evidence is
`docs/status/evidence/2026-09-23-staging-0.3.10-recovery.md` and
`docs/upstream/artifacts/staging-0.3.10-completion-2026-09-23.json` in Toko Miner.

| Work | Recorded time | Scope |
| --- | ---: | --- |
| Release build | 471.46 s | Included in preparation/qualification, not additional to it |
| Desired-authority generation | 88.253 s | Preparation phase |
| Full successor review | 50.586 s | Before publication |
| Fleet convergence | 829.612 s | Canic-managed publication phase |
| Terminal replay | 47.955 s | Zero new journalled effects/operator debit; still performs fresh observations |
| Frontend reinstall/sync | 385.232 s | Toko asset publisher, separate from Canic convergence |

These are retained observations, not a current .40 benchmark. The convergence
receipt includes 198 status requests, 140 identity lookups, 86 Root commands,
57 observability queries, 40 Store catalog queries and 14 Store chunk updates.
Identity lookup time sums to 56.297 seconds and Store catalog requests to 30.083
seconds. These totals are inclusive and may overlap; neither is a claim of
recoverable wall time. The .38 receipt cannot provide .40 child attribution.

## Recommended first slice: Store batch observation work

`fleet_ensure/workflow/independent_effects/mod.rs::apply_batch` submits admitted
Store chunks concurrently, but prepares each action, reconciles every action
before submission, then reconciles every result sequentially. Reconciliation
can request both template staging state and live cycle status. The single-action
`current_protocol::observe` creates a new `StoreStagingObservations` each time;
the existing planning-sequence cache therefore does not share these reads.

Investigate one exact Store/template observation per bounded reconciliation
phase, projecting every chunk predicate from the same response. The batch policy
already binds Store Principal, Candid path/hash, template/version and distinct
chunk indices, with preparation chunk zero completed before parallel uploads.
Preparation, pre-submit reconciliation and post-submit reconciliation must have
separate observations. Never reuse pre-submit state after updates. Any cycle
snapshot sharing needs a separate proof against the existing per-effect
conservation records; reducing catalog reads does not authorize that change.

Keep intent-before-effect, exact chunk hashes, independent-effect bounds,
drainage, individual durable results and lost-response reconciliation. A missing
or contradictory chunk remains incomplete. Test partial success, lost replies,
crash/restart, changed authority and immediate effect-free replay. Qualify call
count reduction first, then wall time against a matched run. The 40 catalog
queries are not all proven redundant, so no fixed saving is claimed.

## Second slice: authenticated query setup

`icp/query/mod.rs::query_candid_readonly` constructs an authenticated Agent on
every logical query, then `query_bytes` constructs a Tokio runtime.
`icp/management.rs::build_authenticated_agent` resolves the network, exports the
selected signer and verifies its Principal for each new Agent. Selected identity
names are bound, but that is not an immutable binding to key material.

Measure setup separately from the query, including subprocess, signer,
connection and runtime costs. First assess sharing transport/runtime resources;
then assess a verified query-session lifetime within an explicit authority
boundary. Preserve exact network/root key/signer bindings, response limits,
query verification and typed retry deadlines. A signer-name cache alone cannot
justify retaining authority after a key or network change. Effect admission
continues to require its own current authority checks. Fresh query results must
not become a long-lived status cache, and this work must not add update retries.

The retained run made 41 authenticated Coordinator-status queries. The existing
Agent builder supports an explicit HTTP client, but sharing a client alone is
not proof of connection reuse while the driving runtime is recreated each call.
Savings require measurement; 56.297 seconds of total identity activity includes
other required checks and is not a promised reduction.

## Larger opportunity: avoid recompiling for release packaging

The [controlled .39 build matrix](toko-performance-followup.md) reports a
925.54-second initial build and 459.95–507.28-second changed builds. Unchanged
repeats take 1.84–1.92 seconds. Application runtime Cargo/link dominates.

Even adding a qualification note changes the input identity, allocates another
release identity, and changes all eight Wasm hashes. `WorkspaceBuildContext`
supplies `CANIC_RELEASE_BUILD_ID` to compilation; `canic::start!` embeds it and
activation validates it. Ignoring Markdown or assigning old Wasm a new release
identity would not preserve that contract. Build scripts may consume non-Rust
inputs too.

A substantial improvement needs a design separating reusable compilation from
release-specific binding/finalization, with exact compiler inputs and output
verification. This is larger than a cache-key tweak. Benchmark unchanged,
qualification-only, gameplay, dependency and relocated inputs, preserving
artifact/activation binding and isolated mutable Cargo targets. Do not infer
that a faster build profile preserves accepted Wasm footprint or runtime cost.

## Lower priority and excluded shortcuts

- The read collector uses batches of four and waits for each whole batch.
  Uneven request latencies can leave slots idle. A rolling queue may help, but
  changes which reads are issued before a failure is known; protected reads
  also consume cycles. It needs explicit failure/drainage and debit evidence.
  Increasing the concurrency limit alone is not the first recommendation.
- Terminal replay intentionally performs fresh inventory, planning and final
  authority/conservation checks. The earlier 24 final inspections were for 24
  distinct retained assets. Do not describe them as proven duplicates or skip
  them because the journal already records completion.
- Host observation pacing grows from 250 ms to 5 s. Root provisioning schedules
  successful continuation with zero delay and uses recorded deadlines on retry.
  Faster polling does not itself advance remote provisioning. Attribute backoff
  before changing it, and retain the owner's retry authority.
- [Contract parsing measurements](../2026-09-23/performance-controls.md) put 33
  local Candid checks at roughly 0.096 seconds. Another parsing cache is low
  priority. Candid extraction is likewise small in the real build matrix.

## Next implementation boundary

Start with the bounded Store catalog observation candidate, then query transport
setup. Treat compilation/release binding as a separately reviewed design.
Current sources and retained evidence support these priorities, not a numerical
deployment-speed claim. The existing .40 receipt instrumentation is sufficient
for the first comparisons; do not spend another release solely renaming timing
fields or adding arbitrary timing thresholds.

This investigation ran no build, benchmark, live request or deployment and made
no production changes. Toko Miner remained read-only. The candidates do not
replace the accepted runtime-contraction batches or close issue #28's downstream
acceptance. No version or changelog entry is allocated for this investigation.
