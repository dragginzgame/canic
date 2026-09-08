# Public Status and Protected Observability

Canic separates reads by caller authority. Publication never changes who may
read protected diagnostics, and Fleet admission or application-player admission
does not grant observability access.

| Method | Access | Contents |
| --- | --- | --- |
| `canic_public_status` | Public | Summary health, role/build discovery, existing public topology discovery and explicitly selected cached aggregate metrics |
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
States distinguish `Disabled`, `Unavailable`, `Fresh` and `Stale`. Snapshots
older than five minutes are stale. Each family retains at most 256 rows; names
and units are limited to 128 bytes. Public health reports local responsiveness,
not complete Fleet readiness.

Queries only read cached metrics. They do not sample, send calls, trigger
funding or traverse the Fleet. Existing update-side cycle observations refresh
selected built-in aggregates. Applications that need a predictable display
cadence call `canic::api::public_status::PublicStatusApi::sample_metrics()` from
an existing update or application-owned timer. This does not create a second
Canic lifecycle or timer owner.

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
input construction and disposal. No new sampling task or historical retention
is provided.

## Adoption

A consuming frontend should label public status as Public and detailed
observability as Observer. Use the public snapshot state and sample time when
rendering charts. Regenerate method bindings, update authentication and control
read routes, enable only the intended families, and connect any application
sampling or aggregate producers. App-specific dashboards and live timing
qualification belong in the consuming repository.
