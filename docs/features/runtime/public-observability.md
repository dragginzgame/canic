# Public Status and Protected Observability

Canic separates reads by caller authority. Publication never changes who may
read protected diagnostics, and Fleet admission or application-player admission
does not grant observability access.

| Method | Access | Contents |
| --- | --- | --- |
| `canic_public_status` | Public | Local responsiveness, role/build discovery, existing public topology discovery and explicitly selected cached aggregate metrics and chart history |
| `canic_observability` | Observer | Role-supported detailed runtime, readiness, logs, cycles and metrics, or infrastructure diagnostics |

Observer access uses the existing controller predicate. Operators observe
Root-controlled workload metrics through the existing authenticated Root relay;
this does not grant the operator lifecycle control over those workloads. There
is no independent observer identity store or admission-based observer grant.
Every request variant within `canic_observability` uses that same controller
rule. Enabling public publication leaves it unchanged.

Authentication/session reads, operation receipts, Store catalog access and
Coordinator Registry synchronization keep their existing protocol owners and
caller contracts on separate methods. Lifecycle, funding and admission mutations
remain in their control APIs. Consumers must regenerate Candid bindings and
route each read to its current owner. The mixed workload status endpoint and
Coordinator status endpoint are removed without aliases.

## Protocol Read Owners

These methods serve their existing control or authentication protocols; they do
not grant general diagnostic access:

| Method | Existing caller contract |
| --- | --- |
| `canic_auth_status` | Public authentication discovery and caller-bound session/token reads |
| `canic_admission_status` | Controller or bound Root |
| `canic_control_status` | Bound Root |
| `canic_root_auth_status` | Public role-attestation reads |
| `canic_root_operation_status` | Existing operation owner |
| `canic_root_status` | Controller |
| `canic_coordinator_operation_status` | Existing operation owner |
| `canic_coordinator_registry` | Controller or registered Root |
| `canic_wasm_store_catalog` | Existing Store Root/controller predicate |
| `canic_wasm_store_status` | Controller |

Methods and variants remain profile-specific. The two observability access
labels refer to the common public and protected methods above.

`Health` returns `PublicHealth.health = responding`. This establishes only that
the canister answered the query at `observed_at_ns`; it does not assess runtime
diagnostics, authority restoration or Fleet readiness. Present it as
"Responding" in public interfaces. Use the protected readiness diagnostics and
Fleet convergence evidence for readiness decisions. Regenerate bindings for the
current public enum; protected diagnostic health retains its own contract.

## Optional Public Metrics

One top-level setting in `canic.toml` selects the public aggregate families
for App-configured canisters. The independent Fleet Coordinator exposes public
health/discovery and keeps its financial/Registry diagnostics protected.
Omitting it, or selecting an empty array, disables metric publication:

```toml
public_metrics = ["cycles", "shard_occupancy", "performance"]
```

| Family | Published values |
| --- | --- |
| `cycles` | Sampled local canister cycle balance |
| `shard_occupancy` | Assigned-key count and configured capacity per shard and pool, without partition keys |
| `performance` | Recorded instruction totals and observation counts from performance metrics |
| `operations` | Existing aggregate operation and funding counters, with units |
| `application` | Explicit aggregate values supplied by trusted application code |

Shard occupancy counts assignments. An application may label these as users
only when one assignment represents one user. Canic does not infer application
user identities, active-user counts or game-specific semantics.

Instruction counts are not cycle costs. Funding amounts are transfers, not
consumption. An application with an exact accounted cost can publish an
`application` aggregate with `unit = "cycles"`; no inferred exchange rate or
balance-delta approximation is introduced.

Each `Metrics` request selects a family and page. The response includes
`sampled_at_ns`, `stale_after_ns`, `state`, `truncated` and a bounded metrics page.
Each row preserves its source observation time; the family timestamp is the
oldest included source observation, so stale rows cannot appear freshly sampled.
States distinguish `Disabled`, `Unavailable`, `Fresh` and `Stale`. Snapshots
older than six minutes are stale (five-minute sampling plus one minute of jitter). Each family retains at most 256 rows; names
and units are limited to 128 bytes. Public health reports local responsiveness,
not complete Fleet readiness.

Queries only read cached metrics. They do not sample, send calls, trigger
funding or traverse the Fleet. A nonempty family selection enables one retained
`canic/public_metrics/sample` task through the existing native timer owner.
The task samples every five minutes, skips missed slots, and stops during the
existing authority suspension. Runtime restoration reconstructs one claim.
Empty selection declares no sampling timer and retains no public history.
Sampling is independent of cycle-funding observations and decisions.

Trusted application code publishes an aggregate snapshot with
`PublicStatusApi::record_application_metrics(Vec<PublicMetric>)`. This Rust API
is not an anonymous mutation endpoint. The `application` family must be
explicitly enabled before values become public. Publishers must supply aggregate
names and canister dimensions suitable for public consumption, excluding user
identifiers and partition keys.

Caches carry no authority and may be rebuilt after restoration. Until sampled,
an enabled family reports `Unavailable`. A rejected replacement leaves the
previous snapshot intact; its original sample time remains visible.

Sampling attempts every selected family even if one fails, then returns the first
family error in selector order. Failed families retain their original timestamps
and become stale normally; successful families still refresh. Optional sampling
errors do not change cycle-tracking or funding decisions.

Collection bounds apply before formatting and sorting. Operations read at most
257 entries from each of four counter owners and each of the three ICP-refill
aggregate indexes. Each target entry yields at most two refill rows. Performance
reads at most 129 recorded counters plus the upstream timer inventory, capped by
ic-timers at 64 registrations with bounded identities; it builds no intent or
timer-diagnostic projection. Occupancy reads at most 129 bounded shard records
and no assignment keys. These ordered prefixes are independent of insertion
order. Performance and occupancy emit two public rows per input; a sentinel row
signals truncation. Each family retains the first 256 selected rows and sorts
that bounded selection by name and canister dimension for reads.

The complete public series name, including family prefix and suffix, must fit
128 bytes. Oversized checkpoint/role labels remain valid internal instrumentation
but reject that family's public sample without cloning unbounded text. The
application publisher processes only its first 257 supplied rows, retaining up
to 256; applications remain responsible for bounding their own collection and
input construction and disposal. The timer records its cost through existing
performance instrumentation after collection; the current run is visible on the
next sample and does not recursively sample itself.

## Bounded chart history

`History(PublicHistoryRequest)` selects one exact family/name/canister series
and a page. Each admitted series has 288 fixed slots (24 hours). Multiple samples
in a slot coalesce to the latest source observation. Missing slots remain gaps;
there is no interpolation or catch-up queue. Reads filter expired slots without
mutating the cache. Update-side sampling releases fully expired series.

The total history cap is 256 series and 8 MiB of conservatively accounted storage
per canister, including full ring allocation and bounded names, units and node
overhead. Admission stops at either cap and exposes `truncated`; latest family
snapshots still retain their independent 256-row cap. Admission follows selected
family order and each family's sorted bounded rows. An admitted series retains
its allocation until it expires; changing units starts new series coverage.
The response reports limits, reserved bytes, coverage, cadence and staleness.
Replies contain at most 288 points, regardless of the requested limit. The history
budget excludes the independently bounded latest snapshots, query reply buffers
and application-owned input allocation.

Every point carries the original `observed_at_ns`, unit (on the series) and a
`Gauge` or `Counter { window_id, saturated }` interpretation. A counter producer
must change `window_id` on every reset, including two resets in one timestamp.
`delta` is present only between adjacent slots with increasing observation time,
the same counter window, no saturation and a nondecreasing value. It contains an
exact amount and elapsed nanoseconds; consumers may calculate a rate from those
fields. Gauge values, gaps and resets have no counter delta. Funding-record
inventories and timer lifetime summaries without a reset identity remain gauges.
A window maximum is a gauge whose name must identify the window maximum; it is
not an interval maximum or a cumulative counter.

Heap-only snapshots and history start empty after reinstall or a same-release
lifecycle upgrade. `canister_version`, `heap_started_at_ns` and `coverage_start_ns`
identify the current coverage; do not join charts across canister identities or
versions. There is no stable-memory history or cross-release import.

## Application and IcyDB composition

Keep IcyDB as a measurement provider. An application's synchronous lifecycle
participant can register one `ApplicationMetricsSampler` through
`PublicStatusApi::set_application_sampler(Some(ApplicationMetricsSampler::new(sample)))`.
The function returns `Result<Vec<PublicMetric>, canic::Error>` and runs only when
`application` is selected. Re-register it after restoration. The adapter may
compose bounded local application counters and IcyDB measurements; it must not
start another metrics timer, query tables, walk users, or make remote calls.
Canic has no IcyDB dependency or schema knowledge. Existing explicit publication
through `record_application_metrics` remains supported.

Supply each row's source observation time and counter window. Reading an old
application cache must preserve its old time. Repeated data cannot create later
history slots or refresh staleness. Invalid replacements preserve the previous
snapshot; history ignores observations older than its retained series. Other
selected families continue.
Provider code owns its collection budget; Canic cannot bound arbitrary application
code. Qualify the actual selected producer names, cardinality and instruction cost
before enabling publication. IcyDB's current report API needs a bounded selection
and exact reset identity before it can support safe cumulative chart deltas.

## Adoption

A consuming frontend should label public status as Public and detailed
observability as Observer. Use the public snapshot state and sample time when
rendering charts. Regenerate method bindings, update authentication and control
read routes, enable only the intended families, and connect any application
sampling or aggregate producers. App-specific dashboards and live timing
qualification belong in the consuming repository.
