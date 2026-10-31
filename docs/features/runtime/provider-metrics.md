# Provider metrics storage

Canic selects storage for component-owned diagnostic counters and measurements.
Heap is the default. A component declares a fixed roster and records through
`canic::api::metrics::providers::MetricProviderApi`; it does not open a store or
select its own persistence policy. IC Metrics owns arithmetic and overflow
quality. Persistence's published `ic-metrics-stable` owns the retained format.

Select the policy in the existing metrics table for the component's canister:

```toml
[component_specs.backend.metrics]
storage = "heap"

[component_specs.backend.metrics.providers.blob]
storage = "stable"
```

This retains Blob's observations while other registered providers remain on
heap. Child roles use their own existing `children.<role>.metrics` table. The
configuration is compiled into runtime authority and its deployment digest.
There is no runtime storage switch, reset endpoint or cross-release migration.
Adding these fields changes the maintained configuration contract; rebuild all
managed artifacts and use the normal pre-1.0 clean-reinstall release procedure.

## Registration and lifecycle

Declare at most 16 providers and 256 metrics in total. Names and units are fixed,
bounded canonical identifiers; never derive them from users or requests. Declare
`MetricKind::Counter` for `add` and `MetricKind::Measurement` for `record`.
Measurements preserve sample count, total, maximum and independent overflow flags;
an empty measurement is distinct from a real zero sample. Counter totals are u128.

Register definitions in the application's **synchronous lifecycle participant**,
on both installation and same-release restoration. Deferred `init` hooks run too
late. Canic activates the registry after those participants and before deferred
work. Unknown provider overrides and duplicate declarations refuse activation.
Merely linking the library exports no endpoint and opens no metrics store.

Roles whose compiled variants select only heap declare no metrics allocation.
Variants selecting stable reserve one governed `canic.core.metrics.providers.v1` allocation,
shared by all its providers. Activation writes explicit empty records on fresh
installation, and requires every retained record on restoration. Missing, malformed,
wrong-unit or incompatible records refuse restoration rather than reset totals.
This uses the existing host Memory composition, not a second memory manager.
A shared role artifact can reserve that allocation for a stable variant while its
active heap variant never opens the map.

## Recording and reporting

Both storage modes use the same `register`, `add`, `record` and `snapshot` API.
Stable observations are checkpointed synchronously on each accepted recording;
heap recording performs no stable writes. There is no background flush or second
business journal. A typed storage error makes provider reporting unavailable and
must not become a false zero or a new authority to retry a paid effect.

Components own logical operation and replay semantics. The Blob adapter observes
admission and reference RPC attempts, applied outcomes, replays, refusals, request
bytes and call-context workflow instructions. Replays do not increment applied
outcomes. Instructions exclude decoding, encoding and telemetry overhead and do
not estimate remote work or cycle billing. These diagnostic observations are not
an atomic business ledger; ordinary traps and asynchronous callback boundaries
retain their normal IC rollback semantics.

Snapshots are read-only projections. Record only from update/callback paths:
query mutations are not durable observations. Live quotas, balances and fences
remain gauges from the authoritative component state, not copied counters. The
Blob participant projects into the existing Application metrics family; public
publication still requires explicit `public_metrics = ["application"]`. Storage
selection does not change authentication or publication authority.

## Querying metrics and logs

The Blob sampler publishes provider observations through `canic_public_metrics`
with the `application` family when `public_metrics = ["application"]` is selected.
The query reads the bounded cache; it does not record or resample. Embedded hosts
compose `canic_blob_service::metrics::sample` into their application sampler.
Other providers supply their own projection through that same host sampler.

The existing `canic_metrics` query remains controller-only for compiled framework
metric families. `canic_logs` remains controller-only and accepts source, topic,
severity and pagination filters. The role's observability selection must retain
the relevant endpoints. These are the existing Wire methods, with no new public
access grant. See [public observability](public-observability.md).

Provider storage selection applies to metrics. Logs use the separate heap recorder
and start empty on upgrade or reinstall; this feature does not enable stable log
checkpoints.

Heap totals reset on ordinary same-release code restoration; stable totals reopen.
Both report the **current platform canister version** as their comparison window,
including after a snapshot restore. A different version forbids interval deltas
even when stable totals survived. Restoring an old snapshot may restore old totals;
stable metrics do not prove continuity, authorize a stale business instance or
replace its recovery fences. No external recovery journal is introduced.

## Qualification

The provider unit cases cover mixed storage, exact caps, missing/malformed records,
overflow quality and store refusal. The managed Blob cases use separate builds:
dedicated defaults to heap, embedded opts Blob into stable. Their same-release
upgrade checks distinguish reset from retained totals, exact replay from fresh
application and new comparison windows. Public cache queries do not resample.
Framework checks qualify these fixtures, not application-specific cardinality,
production provider effects or macOS execution.

See [public publication qualification](public-metrics-qualification.md) for the
separate sampler and public access contract.
