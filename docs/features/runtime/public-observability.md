# Public Status and Protected Observability

## Compile-time selection

Declared roles can omit optional Canic observation providers in `canic.toml`:

```toml
[roles.project_hub.observability]
diagnostics = false
history = false
logs = false
metrics = false
```

All four options default to `true`. Selection is per role, applies to managed
and standalone-local endpoints, and participates in the compiled capability
and protocol-profile identity. Rebuild and regenerate consumer bindings for each method's current codec.
Canonical Store and Coordinator retain their fixed infrastructure surfaces.

| Option | Optional generated reads |
| --- | --- |
| `diagnostics` | Detailed `Runtime` and `MemoryAllocations` projections |
| `history` | Public chart history, cycle history and supported top-up history |
| `logs` | Paged log reads |
| `metrics` | Public metric snapshots, protected metric pages and managed adapters' explicitly mounted metric read/reset endpoints |

Health, readiness, discovery, binding, current cycle balance and child funding
accounting remain available. The authenticated Root relay remains usable for
those financial observations. Its request vocabulary is shared; a disabled
optional observation returns `REQUEST_INVALID`. The Root-to-workload hop uses separate typed `canic_observe_*` Wire updates.
Each method has a fixed response shape across capability profiles. Disabled
providers return `REQUEST_INVALID` without invoking their collectors. The outer
operator-to-Root hop uses the corresponding typed `canic_root_observe_*` Wire
updates, with the target principal included in each request.

When both metrics and history are disabled, generated lifecycle code does not
select automatic public sampling. Public family publication settings still
apply when sampling is selected. Explicit application calls to sampling or
query APIs retain the code they use. These settings do not remove internal
accounting, metric recording, heap logs, bounded log retention or recovery
state. Existing custom application endpoints are unaffected.

### Runtime logging ownership

Published [IC Logging](https://crates.io/crates/ic-logging/0.17.1) owns the heap
recorder, UTF-8-safe message bounds, count eviction, age retention and filtered
cursor scans. Canic supplies its existing log configuration, source/topic labels,
Wire DTO conversion, controller authorization and the sole retention timer.
Source and topic labels each have a 256-byte ceiling. The existing message/count
configuration limits remain authoritative; zero retained entries disables logging.

Log history is heap-only and starts a new window on upgrade or reinstall. Canic
has no stable log allocation or log snapshot schema. Diagnostic history is not
spending, recovery or completion authority. Canic's public log contract still uses
offset/limit and exact totals: the adapter scans upstream pages to count all
retained matches while copying only the selected reply rows, capped at 1,000.
Disabling the generated log read does not disable internal recording.

Component counters and measurements can select heap or stable storage through
the [provider metrics API](provider-metrics.md). Their projections use the same
application sampler and cached `canic_public_metrics` query. Stable selection
does not publish a provider, grant log access or change the controller-only
`canic_logs` and `canic_metrics` methods.

### Adapter reporting contract

Managed adapters use the same `observability.metrics` capability as the framework.
Configuration selects whether reporting may be exposed; Rust `mount!` selects the
adapter and its explicit endpoint declarations. A false metrics setting suppresses
both metric reads and reset, including exports, contract metadata and ingress-limit
registration. A true setting never mounts an undeclared endpoint or widens access.
IcyDB supports explicit controller/public reads and always controller-only reset.
Canic-hosted applications must use its `managed;` mount form. Standalone mounts
retain their explicit source policy because they have no Canic role configuration.

Recording, capacity limits, heap/stable retention and reset continuity remain
upstream-owned. Adapters do not add a recorder or sampling timer. Blob contributes
its bounded projection to the application's one sampler; history remains selected
separately and may continue sampling when direct metrics reads are disabled.
Auth's unpublished scaffold and host-side Backup do not gain synthetic endpoints.
Future managed adapter metric read/reset declarations must use the shared managed
entrypoint's `observability = metrics` classification; that owner gates the entire
expansion using the existing capability. Keep public safety and authorization in
the endpoint code, not in a configurable permission override.

The ownership boundary is:

| Owner | Responsibility |
| --- | --- |
| TOML schema | Reporting selection and its default; no endpoint declarations or access grants |
| Core role policy | Resolve configuration into typed capabilities |
| Build integration | Emit Rust capability flags from the resolved contract |
| Adapter `mount!` | Declare endpoints and their explicit authorization |
| Shared entrypoint macro | Gate the complete generated endpoint using its capability classification |
| Upstream recorder/storage | Recording, capacity, retention and reset semantics |
| Canic lifecycle/sampler | Sampling and history scheduling; adapters add no second timer |

`observability = metrics` classifies an endpoint; it is not a second configuration
switch. Managed endpoints require both an explicit declaration and the resolved
capability. Enabling the capability alone creates no adapter endpoint.

Canic separates reads by caller authority. Publication never changes who may
read protected diagnostics, and Fleet admission or application-player admission
does not grant observability access.

| Method | Access | Contents |
| --- | --- | --- |
| `canic_public_health` | Public | Typed Wire responsiveness and local identity |
| `canic_public_history` | Public | Typed Wire query for one bounded cached series |
| `canic_public_metrics` | Public | Typed Wire query for selected cached aggregate metrics |
| `canic_public_overview` | Public | Typed Wire role/build identity and bootstrap readiness |
| `canic_public_children` | Public | Typed Wire bounded direct-child inventory |
| `canic_component_directory_page` | Registered Component member | Typed Wire page under exact current Component authority |
| `canic_root_directory_head`, `canic_root_directory_page` | Controller | Typed Wire directory authority and pages |
| `canic_cycle_balance` | Controller | Typed Wire current canister balance |
| `canic_cycle_history` | Controller | Typed Wire bounded cycle samples; optional on application roles |
| `canic_logs` | Controller | Typed Wire filtered logs; requires log capability |
| `canic_metrics` | Controller | Typed Wire exact metric pages; requires metrics capability |
| `canic_memory_allocations` | Controller | Typed Wire current allocation diagnostics; Root/managed diagnostics capability |
| `canic_child_funding` | Controller | Typed Wire exact child accounting; available to Root while Prepared |
| `canic_readiness` | Controller | Typed Wire readiness and exact fixture evidence; Root refuses while Prepared |
| `canic_runtime` | Controller | Typed Wire runtime/timer diagnostics; requires diagnostic capability |
| `canic_binding` | Controller | Typed Wire Registry-issued identity; managed roles only |
| `canic_health` | Controller | Typed Wire runtime health diagnostics; Root refuses while Prepared |
| `canic_cycle_topups` | Controller | Typed Wire funding history; requires history and automatic-topup capabilities |
| `canic_root_inspection_reserve` | Controller | Typed Wire indicative management-inspection reserve; available while Prepared |
| `canic_coordinator_root_acknowledgements` | Controller | Typed Wire retained Root acknowledgements under exact registry authority |
| `canic_coordinator_funding` | Controller | Typed Wire Coordinator treasury and Root funding evidence |

Observer access uses the existing controller predicate. Operators observe
Root-controlled workload metrics through the existing authenticated Root relay;
this does not grant the operator lifecycle control over those workloads. There
is no independent observer identity store or admission-based observer grant.
The protected cycle, metric and funding methods use that same controller rule.
Enabling public publication leaves it unchanged.

Authentication/session reads, operation receipts, Store catalog access and
Coordinator Registry synchronization keep their existing protocol owners and
caller contracts on separate methods. Lifecycle, funding and admission mutations
remain in their control APIs. Consumers must regenerate bindings for each method's current codec and
route each read to its current owner. The mixed workload status endpoint and
Coordinator status endpoint are removed without aliases.

## Protocol Read Owners

These methods serve their existing control or authentication protocols; they do
not grant general diagnostic access:

| Method | Existing caller contract |
| --- | --- |
| `canic_active_delegation_proof` (Wire) | Public issuer-local proof status; no credentials or secret material |
| `canic_admission_status` | Controller or bound Root |
| `canic_runtime_operation` (Wire) | Bound Root; complete retained runtime operation evidence |
| `canic_caller_authority_status` (Wire) | Bound Root; original publication receipt and receiver authority |
| `canic_root_role_attestation` | Public role-attestation reads |
| `canic_root_membership` | Root controller or active local managed member; read-only update for inter-canister discovery |
| `canic_root_removal`, `canic_root_component_provisioning` (Wire) | Coordinator or controller; exact retained operation |
| `canic_root_component_allocation`, `canic_root_child_allocation` (Wire) | Existing allocation authority; exact retained operation |
| `canic_root_component_removal`, `canic_root_subtree_removal`, `canic_root_store_adoption`, `canic_root_store_bootstrap`, `canic_root_activation_status`, `canic_root_cycle_refill_status`, `canic_root_registry_synchronization` (Wire) | Controller; exact retained operation |
| `canic_root_config` (Wire) | Controller; Root must be active |
| `canic_root_fleet_state` (Wire) | Controller; Root must be active |
| `canic_authority_restore_status` (Wire) | Root/Coordinator controller; Root must be active |
| `canic_root_inventory` (Wire) | Controller; typed retained Root inventory |
| `canic_coordinator_component_provisioning`, `canic_coordinator_root_removal`, `canic_coordinator_admission_operation`, `canic_coordinator_funding_rotation`, `canic_coordinator_retirement_status` | Existing operation owner |
| `canic_coordinator_registry` (Wire) | Controller or registered Root; typed Fleet registry snapshot |
| `canic_wasm_store_fixture_status` | Existing Store Root/controller predicate; typed fixture progress |
| `canic_wasm_store_fixture_grant` | Existing Store Root/controller predicate; exact retained grant or absence |
| `canic_wasm_store_template_status` | Existing Store Root/controller predicate; manifest and per-chunk evidence |
| `canic_wasm_store_inventory` (Wire) | Existing Store Root/controller predicate; template inventory |
| `canic_wasm_store_storage` (Wire) | Existing Store Root/controller predicate; storage/GC inventory |
| `canic_wasm_store_authority` | Controller; complete retained Store installation authority |
| `canic_wasm_store_activation_status` | Controller; exact activation operation |
| `canic_wasm_store_gc_status` | Controller; exact GC operation |

Caller-authority mutations use the separate Wire updates
`canic_caller_authority_prepare`, `canic_caller_authority_commit`,
`canic_caller_authority_complete` and `canic_application_startup_release`.
Each requires the bound Root after the Fleet update admission check and retains
the original operation identity and replay behavior.

Methods and variants remain profile-specific. The two observability access
labels refer to the common public and protected methods above.

`canic_public_health` is a typed Wire query with a unit request and a direct
`PublicHealth` reply. It returns `PublicHealth.health = Responding`. This establishes only that
the canister answered the query at `observed_at_ns`; it does not assess runtime
diagnostics, authority restoration or Fleet readiness. Present it as
"Responding" in public interfaces. Use the protected readiness diagnostics and
Fleet convergence evidence for readiness decisions. Regenerate bindings for the
current typed public-health contract; protected diagnostic health retains its own contract.

ICRC discovery and consent endpoints are opt-in through the
[ICRC adapter](icrc-adapter.md), not role startup or configuration.
Default roles expose neither endpoint. Standards discovery does not grant
protected inspection access.

Canonical Store and Coordinator Candid must be generated with the host's
`build_artifact --refresh-canonical-did` flow after changing shared public DTOs or
endpoint macros. Structural Candid equality does not establish the exact bytes
used by protocol-profile hashing. After refreshing, build the affected artifacts
without the refresh flag to verify the ordinary materialization path as well.

## Current Memory Allocations

Root and managed canisters expose the typed Wire `canic_memory_allocations` query
under the existing controller predicate. For a controlled Workload, send
`canic_root_observe_memory_allocations({ canister_id, input: () })` through its
Root. Root uses the replicated Wire `canic_observe_memory_allocations` update for
the managed target. Host telemetry verifies the native schema from the terminal
Fleet’s finalized release, checking Root’s installed module and protocol identity
before transport. No Candid sidecar or local preparation is required for this
read. Paid balance inspection retains its separate management/reserve contract.
Store does not expose memory allocations. There is no public memory-ledger endpoint.

The response reports the committed generation, actual persisted bucket size,
manager capacity and allocation totals, and all 255 usable manager IDs in order.
Each row separates virtual extent, allocated buckets and bytes, bucket slack,
and a current stable-key/owner binding, the ic-memory ledger binding, or an
explicit unknown binding. Range claims describe allocation policy; they do not
establish an unknown allocation's owner. Zero-page entries remain visible.

Collection uses ic-memory's owned runtime report. The 0.13.3 qualification read
34,848 bytes of validated manager metadata and bounded current declarations without decoding
history, constructing stores, growing memory, writing, or advancing the ledger
generation. Canic requires an already bootstrapped runtime.
The 0.14.1 adoption preserves this reporting path and passes its native accounting
regressions; it does not supply a new IC instruction/cycle measurement.
`MemoryQuery::ledger()` is the separate current-ledger diagnostic API. With
`ic-memory 0.27`, it reports the exact ledger memory ID, checked commit counter
and recovery slots, retained ownership/state and latest declared schema version.
It does not report generation events, allocation observation/retirement counters
or schema history. Retired allocations retain their identity and memory ID.
The API/ledger hard cut requires updated diagnostic consumers and clean reinstall
for earlier releases; it adds no predecessor decoding or migration path.

`physical_extent.bytes` measures allocated IC stable memory in canister
execution and the supplied backing-memory extent in native execution. Format
bytes in binary units and label this metric **allocated stable memory**.
Virtual extent includes structure capacity and metadata. `payload_bytes` is
unavailable; bucket slack is allocation beyond virtual extent, not unused
record-payload capacity.

The report preserves these conservation identities:

- Physical extent = manager metadata + allocated bucket bytes + unmanaged bytes.
- Allocated bucket bytes = the sum of every row's allocated bytes.
- Allocated bucket bytes = known-binding bytes + unknown-binding bytes.
- Allocated bucket bytes = virtual extent + bucket slack.

The ledger is included in the rows and known-binding total; do not add it twice.
Unknown-binding bytes are managed buckets without a current binding.
Unmanaged bytes are physical extent outside the manager's accounted extent.

Canic defaults to 16-page (1 MiB) buckets. The build-time
`CANIC_MEMORY_BUCKET_PAGES` setting selects larger buckets for applications
requiring more manager capacity. See the [stable-memory layout](stable-memory-layout.md)
for the capacity tradeoff, consolidated stores and measured template decisions.
Storage collections use `ic_storage::adapter` and consume exclusive
`ic_memory::RuntimeMemory` leases. Capacity reservation and observation use the
collection owner’s API; they do not open a second handle to its allocation.

## Optional Public Metrics

Metrics-capable artifacts expose `canic_public_metrics` as a typed Wire query:
`PublicMetricsRequest` returns `PublicMetricsSnapshot` directly. The request selects
one family and page; querying never refreshes the cache. Use the method identity
and request/reply budgets from `canic::protocol::public_metrics`. Metric dimensions
use native `ic_platform::Principal`; application samplers return that type too.
The endpoint is absent when the artifact lacks the metrics capability. An enabled
capability with publication disabled returns the existing `Disabled` snapshot.
History uses the independent typed Wire `canic_public_history` query.

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
| `performance` | Recorded instruction totals, observation counts, memory extents and cached anonymous allocation summaries |
| `operations` | Existing aggregate operation and funding counters, with units |
| `application` | Explicit aggregate values supplied by trusted application code |

Shard occupancy counts assignments. An application may label these as users
only when one assignment represents one user. Canic does not infer application
user identities, active-user counts or game-specific semantics.

### Public allocation summaries

Selecting `performance` also selects the local allocation summary. The existing
sampler reads bounded memory-manager metadata; public queries and history reads
only read the cache. There is no additional endpoint, selector, remote collector
or application callback. Omitting `performance` skips this collection and hides
its values. Raw keys, owner strings, memory IDs and range claims remain protected.
The sampler uses ic-memory 0.15.3's numeric `MemoryAllocationSummary`, avoiding
per-ID rows and copied binding names. It shares accounting with the protected
detailed report and retains the 34,848-byte metadata-read bound.

All summary names have the `memory.allocations.` prefix and no canister label.
Rows use `Gauge`, preserve `observed_at_ns`, and use the existing bounded history.

| Suffix | Unit | Meaning |
| --- | --- | --- |
| `state` | `state` | `1` = available, `2` = unsupported, `3` = collection failed or incomplete |
| `physical_extent` | `bytes` | Allocated IC stable-memory extent |
| `bucket_size` | `bytes` | Actual persisted bucket size |
| `allocated_bucket_bytes`, `allocated_buckets` | `bytes`, `count` | Assigned manager capacity |
| `bucket_capacity`, `remaining_buckets` | `count` | Manager bucket limit and remaining capacity |
| `virtual_extent`, `bucket_slack` | `bytes` | Addressable virtual capacity and allocation beyond it |
| `manager_metadata`, `unmanaged` | `bytes` | Manager metadata and physical extent outside its assigned region |
| `known_binding`, `unknown_binding` | `bytes` | Allocated capacity with and without a current binding |
| `current_binding.allocated`, `current_binding.slack` | `bytes` | Totals for current declared bindings |
| `ledger_binding.allocated`, `ledger_binding.slack` | `bytes` | Totals for the substrate ledger binding |
| `unknown_binding.allocated`, `unknown_binding.slack` | `bytes` | Totals without a current binding |
| `ids_measured`, `ids_total` | `count` | Coverage of all usable manager IDs, including unopened IDs |
| `metadata_read` | `bytes` | Bounded manager metadata read by the owner |
| `payload_available` | `count` | `0`: allocation metadata cannot measure payload occupancy |

Absence of `state` means no allocation sample has been published. An unsupported
role publishes only the allocation state; Store is explicitly unsupported.
A first failure also publishes only state. Later failures retain the last good
allocation values with their original timestamps and publish a new failure state.
The containing Performance family may still contain other valid metrics: check
allocation state and row age before presenting these values as current. Do not
render missing values as zero. History retains original successful source times
on failure and reports its existing series/byte truncation limits.

The physical, capacity and ownership identities described above are validated
independently before publication. Current-binding totals include any owner with
a current declaration; they are not a crate-specific breakdown. The ledger is
already part of known-binding bytes. Do not add ownership partitions to capacity
partitions. Virtual extent is not payload use, and bucket slack is not evidence
of a leak or reclaimable memory. No payload byte gauge is fabricated.

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
257 entries from lifecycle, canister-operation and top-up owners. Global funding
uses four fixed enum groups with at most the fixed denial-reason set; ordered
range lookup skips all child-principal groups before applying its row limit.
Root reads bounded prefixes of the three ICP-refill indexes, but publishes only
normalized status/error counts. Per-child funding and per-target refill amounts
remain in protected Core diagnostics. Refill status grouping removes automatic
sequences and folds error phases before bounded selection, preserving one emitted
identity. Public operation units and counter/gauge interpretation come from
these typed projections; display names do not recover either fact. Performance
reads at most 129 recorded counters plus the upstream timer inventory, capped by
ic-timers at 64 registrations with bounded identities. Each registration yields
separate scheduler and work entries; the performance path builds no protected
intent or timer-diagnostic projection. Bounded operations aggregates are described below. Occupancy reads at most 129 bounded shard records
and no assignment keys. These ordered prefixes are independent of insertion
order. Performance and occupancy emit two public rows per input; a sentinel row
signals truncation. Each family retains the first 256 selected rows and sorts
that bounded selection by name and canister dimension for reads.

Endpoint instruction measurements belong to each invocation, including its async
resumptions. Synchronous nested endpoint work is subtracted from its parent;
unrelated async calls never share frames or checkpoint baselines. `perf!` measures
from the start of its owned invocation or the previous checkpoint in that same
invocation. Generated endpoints and framework lifecycle/renewal tasks establish
this context. Application background futures can wrap their work with
`canic::api::ops::with_async_perf_context`. Keep each wrapped future within one
IC call context; migratory tasks need a fresh scope for each method context.
An unscoped checkpoint produces no sample. A measured zero remains distinct from
missing interval evidence.

The complete public series name, including family prefix and suffix, must fit
128 bytes. Checkpoint admission checks the complete `perf.checkpoint` name,
including its `.calls` suffix, before retaining scope or label text. Use static,
low-cardinality labels rather than user, canister or partition identifiers.
The recorder retains at most 128 distinct checkpoint identities in lexicographic
order; existing retained counters continue updating at capacity. Samples with an
oversized identity, or outside that retained prefix, contribute to the explicit
`perf.checkpoint_overflow` instruction total and its `.calls` sample count.
The public overflow total retains its full `u128` width; a wide total does not
hide the sample count. Runtime-tier diagnostics expose the overflow sample count.
Eviction moves all prior samples into that aggregate once, preserving accounting
without merging distinct retained names. The overflow counter is selected before
ordinary counters so a bounded Performance sample can report admission loss.
Checkpoint counters remain ephemeral heap state. Oversized role labels
still reject their family's public sample without cloning unbounded text. The
application publisher processes only its first 257 supplied rows, retaining up
to 256; applications remain responsible for bounding their own collection and
input construction and disposal. The timer records its cost through existing
performance instrumentation after collection; the current run is visible on the
next sample and does not recursively sample itself.

## Sampling cost qualification

Measure the complete sampling callback in Wasm, including application collection,
validation, publication and history. Retain the exact source/artifact, build
profile, selected families, entity/name cardinality and instruction counts.
Cover first allocation, steady sampling, full history retention/rollover and
rejection/recovery. Native counters and a small ordinary fixture do not establish
the maximum supported producer cost.

The historical **20,000,000 instructions** is an advisory comparison point.
It first appeared in Canic's 0.110.11 sampling regression on 2026-09-08; that
fixture measured 4,647,992 instructions. It was retained for history and then
copied into Toko Miner's complete-callback qualification. No operating-cost or
latency calculation establishes that exact value. It is not a runtime cutoff,
protocol limit or universal application budget, and crossing it alone must not
block a Canic release. The timer regression reports the measurement and whether
it exceeds the reference without an absolute-cost assertion.

Correctness and bounded work remain required: enforce collection/label limits,
history capacity, exact timestamps, family isolation, continued scheduling and
recovery. The current scaling regression also compares the same retained prefix
with 256 and 4,096 source checkpoints: a sixteenfold source increase may use at
most twice the first sample's instructions. This fixture-specific tolerance
detects a return to source-proportional collection; it does not certify a global
instruction bound or replace structural bounds. Revise that comparison when
its workload or measurement meaning changes, retaining the supporting evidence.

An absolute regression threshold needs an explicit workload, measured baseline,
justified margin and consequence for exceeding it. An application operating
budget additionally needs its sampling cadence, deployment scale and an actual
latency or accounted cycle-cost requirement. Record and review those assumptions
when the producer, profile or platform changes. Do not silently raise a failing
limit, infer cycles from instructions, or treat an unexplained historical number
as permanent authority. Until such a budget exists, absolute cost is reported
for review alongside the maintained structural and behavioural checks.

The original [sampling evidence](../../audits/reports/2026-09/2026-09-08/public-sampling-bounds.md)
and [Toko ceiling evidence](../../audits/reports/2026-09/2026-09-09/metrics-history-index.md)
retain their original measurements and test verdicts. A historical pass/fail
against 20M is not a new product-cost requirement.

## Bounded chart history

`canic_public_history(PublicHistoryRequest)` selects one exact family/name/canister series
and a page. Each admitted series has 288 fixed slots (24 hours). Multiple samples
in a slot coalesce to the latest source observation. Missing slots remain gaps;
there is no interpolation or catch-up queue. Reads filter expired slots without
mutating the cache. Update-side sampling releases fully expired series.

The total history cap is 256 series and 8 MiB of conservatively accounted storage
per canister, including full reserved group allocations, unused positions,
validity/index metadata and bounded names, units and node overhead. Admission stops at either cap and exposes `truncated`; latest family
snapshots still retain their independent 256-row cap. Admission follows selected
family order and each family's sorted bounded rows. Eight series share each
contiguous history allocation; vacant positions are reused and an allocation is
released when its final series expires. Changing units starts new series
coverage. The [layout qualification](../../audits/reports/2026-09/2026-09-21/history-locality.md)
records sampling/read improvements and sparse/gap tradeoffs.
The response reports limits, reserved bytes, coverage, cadence and staleness.
Replies contain at most 288 points, regardless of the requested limit. The history
budget excludes the independently bounded latest snapshots, query reply buffers
and application-owned input allocation.

History `truncated` describes admission during the selected family's latest
accepted sampling pass. A pass with no history rejection, including an empty
pass, clears that family's flag. Sampling another family preserves its result;
failed snapshot validation preserves the prior result. Queries only read it.
Missing retained slots remain gaps; coverage and points describe that history.
The latest snapshot's independent row-cap flag belongs to its own response.

Every point carries the original `observed_at_ns`, unit (on the series) and a
`Gauge`, `Counter { window_id, saturated }` or `TimerCounter` interpretation. A counter producer
must change `window_id` on every reset, including two resets in one timestamp.
`delta` is present only between adjacent slots with increasing observation time,
the same counter window, no saturation and a nondecreasing value. It contains an
exact amount and elapsed nanoseconds; consumers may calculate a rate from those
fields. Gauge values, timer counters, gaps and resets have no public history
delta. Timer counters retain the full registration metadata for qualified offline
comparison. Funding-record inventories and aggregate timer events remain gauges.
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

## Public process aggregates

The existing `operations` selection also includes anonymous
`process.platform_call`, `process.inter_canister_call`, `process.intent`,
`process.placement_index` and `process.wasm_store` aggregates. Platform, intent,
placement and Store rows sum recorded event counts by outcome across the owning
metric table. Inter-canister `started` reads the existing fixed system counter;
it counts polled framework call attempts, including Platform admission refusals,
without enumerating target/method identities. Constructing an unused request
records no event. Direct application calls retain application-owned instrumentation.
Generic platform-call rows describe transport; protocol clients own codec failures.
Unobserved outcome combinations
have no row. These are instrumentation events, not proof of Fleet readiness or
an inventory of all outstanding work. Store and placement activity appears only
on roles that execute those owners. No security records, callers, credentials,
raw intents, canister targets or application row contents are published by these
new projections.

Each of the four enum-keyed owners admits at most 256 source rows for a sample.
An oversized owner rejects the operations sample before allocation or scanning;
previous cached data ages and other selected families continue. Aggregation uses
fixed outcome labels and IC Metrics wide counters. Both source and grouped
actual-overflow flags reach public history; an exact numeric cap remains exact.
Diagnostic funding totals use the same arithmetic with cycle units. Counts use
the existing heap counter window (`window_id = 0`); heap coverage and canister
version delimit resets.
The common 256-row snapshot and global history budgets still apply.

`operations` also includes `timer.state.{disabled,idle,active,retrying,failed}`
counts and `timer.events` scheduling, scheduler/work starts, work completion, retryable/invariant failure,
unacknowledged and coalescing totals. Collection visits the existing inventory's
maximum 64 registrations. These aggregates are gauges because cancellation and
registration changes can reset individual source counters. Missing state rows
mean no currently observed declaration in that state. They describe timer work,
not every durable application intent. Timer inventory failure rejects only the
operations sample. Endpoint performance remains available without timer inventory;
offline comparison reports missing timer evidence rather than inventing measurements.

Timer performance uses `perf.timer.<owner>.<subsystem>.<name>.scheduler` and
`.work`, in instructions, with a `.calls` row for each phase's completed
measurement samples. This is a hard cut of the previous work-only metric name.
The protected performance projection uses the same explicit phase labels.
The public sampler reads typed source measurements directly: instruction totals
retain `u128`, and count/total saturation flags report actual overflow. Reaching
an exact numeric cap alone does not mark an observation saturated. Registration
is carried from the timer snapshot, never recovered from the formatted name.
The protected diagnostic view also retains wide totals and source overflow flags
in its explicit measurement payload; neither view narrows those totals to `u64`.
`timer.events.scheduler_started` and `timer.events.work_started` independently
count starts across the current registry; starts need not equal completed
measurement samples. Scheduler and work measurements cover the timer owner's
measured envelopes, not all execution or billed cycles.

Timer performance rows use `PublicMetricKind::TimerCounter`: the source runtime
epoch (canister version and start time), checked registration sequence and
per-field saturation accompany each instruction total and completed-sample
count from the same inventory observation. Cancellation preserves that identity;
unregister/re-register changes it. Public history retains raw samples without
deriving timer rates. Aggregate timer event and state rows remain gauges.
The host's offline comparison admits timer deltas only for an unchanged
registration, advancing exact source times and unsaturated fields. Instruction
totals retain the full `u128` range; sample counts remain `u64`. Host comparison
uses the carried actual-overflow flag rather than treating an exact cap as loss.
A full registry can
exceed the existing public row budget; truncation remains explicit and missing
rows are not zero.
No extra sampler or polling schedule is introduced. The private
[observatory cost evidence](../operations/fleet-observatory.md#cost-investigation)
collects these values with balances, grants and source-window metadata.

`performance` adds `memory.wasm_extent` and `memory.stable_extent`, both gauges
in bytes, obtained from the executing canister's memory page counts. Wasm extent
is allocated linear memory, not live Rust heap occupancy; stable extent is the
allocated stable memory, not application data size. Native hosts omit these IC
measurements. All added rows use the sampling observation timestamp. They share
the single five-minute sampler, default-off family selection and bounded history.

See [matching-build qualification](public-metrics-qualification.md) before
measuring enabled versus disabled publication or an application participant.

## Continue From Here

- [Read the metrics reference](../../metrics.md)
- [Browse runtime features](README.md)
- [Browse all documentation](../../README.md)
- [Back to the main README](../../../README.md)

Store publication controls use native updates `canic_wasm_store_inspect_template`,
`canic_wasm_store_prepare_chunk_set`, `canic_wasm_store_stage_manifest`,
`canic_wasm_store_prepare_fixture` and `canic_wasm_store_run_gc`.
Inspection, fixture preparation and GC require Root; chunk-set preparation and
manifest staging retain the Store publication-caller predicate. These operations
remain available to a Prepared Store under their existing monotonic/replay rules.
They do not change the chunk upload contract or grant deletion authority.

Store credential preparation (`canic_wasm_store_prepare_credential`) and activation
(`canic_wasm_store_activate`) are separate Root-authorized native updates. Both
retain the operation ID in their receipt; activation also validates the retained
credential generation and cascade evidence before scheduling initialization once.
These transitions remain available to Prepared Stores and do not grant admission
policy mutation authority. Their status reconciliation retains its existing read
contract until that read surface completes the coordinated transport cut.

The Store's remaining mutations are typed native updates:
`canic_wasm_store_set_fixture_grant` compares the exact installation binding and
expected grant revision, and `canic_wasm_store_reclaim_deletion_cycles` returns
observed transfer evidence under the existing controlled-destination, quota and
reserve rules. Both require the Root caller. The Store no longer exports a
command dispatcher. Status reads and publication byte lanes retain their current transports pending
the rest of the coordinated cut. The three catalog queries use typed native Wire.

Fixture byte transfer uses native Wire on `canic_wasm_store_publish_fixture` and
`canic_wasm_store_fixture_chunk`. Publication retains the existing Root/controller
predicate. Pulls are bounded replicated updates authenticated against the complete
caller-bound grant and revision; they return opaque bytes or a typed Store refusal.
The importer rechecks its installed assignment and recovery lease after the await,
then verifies each chunk against the retained descriptor before applying it.
The native envelope permits the existing 1 MiB chunk plus framing; storage and
content-hash encodings are unchanged. Template publication and Store status reads
remain separate transport migration work.

Store authority and operation queries use native Wire. The operation methods take
an explicit operation ID and reject zero or unavailable identities; they do not
return a polymorphic status union. Prepared Store reads retain controller protection.
