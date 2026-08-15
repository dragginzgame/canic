# Canic Metrics Reference

`canic_metrics(kind, page)` returns a paginated `Page<MetricEntry>`.
Rows are sorted by `labels`, then `principal`, before pagination.

Each row has:

| Field | Meaning |
| ----- | ------- |
| `labels` | Ordered string dimensions. The first label is always the metric family inside the selected tier. |
| `principal` | Optional principal dimension when the family is naturally principal-scoped. |
| `value` | Metric payload: `Count`, `CountAndU64`, or `U128`. |

`CountAndU64` uses `count` as the event/sample count. The `value_u64`
meaning is family-specific.

`canic info metrics` keeps the default text table compact. It renders
`CountAndU64` rows as `COUNT` plus `AVG/CALL`, omits raw totals, and keeps
full canister ids, principal dimensions, and raw totals behind `--verbose`.
`--json` preserves the raw metric payload shape.

## Query Perf Samples

Query calls can update in-memory perf tables during the call, but those updates
are not committed after the query returns. For audit probes that need comparable
query-side instruction measurements, return a `QueryPerfSample<T>` from the
same query call:

```rust
#[canic_query(requires(env::build_local_only()))]
async fn audit_env_probe() -> Result<QueryPerfSample<EnvSnapshotResponse>, Error> {
    Ok(MetricsQuery::sample_query(EnvQuery::snapshot()))
}
```

`QueryPerfSample::local_instructions` is the local call-context instruction
counter observed before the query response is returned. Use this for explicit
audit/probe endpoints; use `canic_metrics(MetricsKind::Runtime, ...)` for
persisted update and timer rows.

Audit reports should treat a zero `local_instructions` value as unobservable
rather than as a successful zero-cost query measurement.

## Metric Tiers

Canic keeps metrics enabled by default for generated canisters, but each
canister compiles only the tiers needed by its inferred metrics profile:

| Profile | Selected by default | Enabled tiers |
| ------- | ------------------- | ------------- |
| `root` | Root canister | `Core`, `Placement`, `Platform`, `Runtime`, `Security`, `Storage` |
| `storage` | `wasm_store` role | `Core`, `Runtime`, `Storage` |
| `hub` | Canisters with scaling, sharding, or directory config | `Core`, `Placement`, `Runtime`, `Security` |
| `leaf` | Other non-root canisters | `Core`, `Runtime`, `Security` |
| `full` | Explicit override only | `Core`, `Placement`, `Platform`, `Runtime`, `Security`, `Storage` |

Use an override only when a role needs more visibility than its default:

```toml
[component_specs.app.metrics]
profile = "full"
```

Unsupported tier requests keep the same Candid enum shape and return a Canic
invalid-input error for that canister.

| `MetricsKind` | Families | Notes |
| ------------- | -------- | ----- |
| `Core` | `lifecycle`, `canister_ops`, `cycles_funding`, `cycles_topup` | Operator-facing lifecycle, canister operation, and cycles rows. |
| `Placement` | `cascade`, `placement_index`, `scaling`, `sharding` | Component placement and topology rows. `sharding` is present only when the sharding feature is enabled. |
| `Platform` | `platform_call`, `inter_canister_call` | Low-cardinality IC/platform I/O rows. |
| `Runtime` | `intent`, `perf`, `timer` | Runtime reservation, instruction, and timer rows. |
| `Security` | `access`, `auth`, `delegated_auth`, `replay`, `root_capability` | Access, delegated auth, replay, and capability rows. |
| `Storage` | `wasm_store` | Wasm-store source, chunk, and publication rows. |

### `Core`

Core rows cover lifecycle, canister operation, cycles behavior, and ICP refill
record observability through the existing funding family.

### `Placement`

Placement rows cover direct-child topology propagation, keyed placement indexes,
scaling pools, and feature-gated sharding pools.

The Fleet Subnet Root prepaid empty-Canister inventory is not a Component
placement metric family. Its exact current policy and asset states are exposed
through the bounded controller-only pool status query; Fleet-wide counts are
included in `canic info subnets`.

### `Platform`

Platform rows cover IC/platform call outcomes and inter-canister calls.

### `Runtime`

Runtime rows cover intent reservation, persisted perf counters, checkpoints,
and timers.

### `Security`

Security rows cover access denials, auth/session behavior, delegated auth,
replay, and root-capability authorization.

### `Storage`

Storage rows cover wasm-store source resolution, chunk movement, and
publication.

## Family Labels

The first label in every row identifies the concrete family. Remaining labels
use the existing family-specific dimensions:

| Family | Labels after family prefix | Principal | Value |
| ------ | -------------------------- | --------- | ----- |
| `access` | `[endpoint, kind, predicate]` | `None` | `Count` |
| `auth` | `[surface, operation, outcome, reason]` | `None` | `Count` |
| `canister_ops` | `[operation, role, outcome, reason]` | `None` | `Count` |
| `cascade` | `[operation, snapshot, outcome, reason]` | `None` | `Count` |
| `cycles_funding` | `[metric]`, `[metric, reason]`, or `[icp_refill, phase, metric, value]` | Child principal for child-scoped rows; root principal for root-refill rows | `Count` or `U128` |
| `cycles_topup` | `[metric]` | `None` | `Count` |
| `delegated_auth` | `[delegated_auth_authority]` or `[operation, outcome, reason]` | Verified signer authority for authority rows | `Count` |
| `intent` | `[surface, operation, outcome, reason]` | `None` | `Count` |
| `inter_canister_call` | `[method]` | Target canister principal | `Count` |
| `lifecycle` | `[phase, role, stage, outcome]` | `None` | `Count` |
| `perf` | `[endpoint, call_kind, name]`, `[timer, owner, subsystem, name]`, or `[checkpoint, scope, label]` | `None` | `CountAndU64` |
| `platform_call` | `[surface, mode, outcome, reason]` | `None` | `Count` |
| `placement_index` | `[operation, outcome, reason]` | `None` | `Count` |
| `replay` | `[operation, outcome, reason]` | `None` | `Count` |
| `root_capability` | `[capability, event_type, outcome, proof_mode]` | `None` | `Count` |
| `scaling` | `[operation, outcome, reason]` | `None` | `Count` |
| `sharding` | `[operation, outcome, reason]` | `None` | `Count` |
| `timer` | `[policy, owner, subsystem, name]` or `[inventory, available]` | `None` | `CountAndU64` for timer rows; `Count` for availability |
| `wasm_store` | `[operation, source, outcome, reason]` | `None` | `Count` |

Delegated-auth renewal rows use the existing `delegated_auth` family with
the bounded operation label `renewal_sweep`. Outcomes are
`started`/`completed`/`failed`; reasons reuse bounded auth reasons such as
`ok`, `invalid_state`, `cert_expired`, `issuer_proof_unavailable`,
`cert_hash_mismatch`, `disabled`, and `root_proof_prepare_failed`.

For `timer`, `count` is accepted consumer-work starts and `value_u64` is the
latest armed delay in milliseconds. Delay is deliberately a value rather than
a key, so exact-deadline rescheduling does not create unbounded metric rows.
Timer rows come from the shared `ic-timers` inventory, so they include Canic,
application, and other framework owners in the same canister. Timer `perf`
rows use completed work-instruction samples as `count` and their saturating
instruction total as `value_u64`. These samples bracket the complete accepted
shared-runtime callback path, including registry acceptance, consumer work,
completion accounting, and successor binding; they are not isolated
application-function benchmarks. Runtime timer status schema 3 separately
projects scheduler and work instruction aggregates plus each role's bounded
latest Wasm/stable-memory page extents and maximum observed page growth. Memory
page extents are epoch-local high-water observations, not exact live bytes or
exclusive attribution for asynchronous work. A transient `RemoveWhenStopped`
declaration disappears from inventory at terminal state, so its final status
and performance sample are not retained; retained declarations preserve their
normally completed observations.
`timer/inventory/available` is `1` when the complete
registry was observed and `0` when observation failed; an unavailable registry
is never represented as a successful empty inventory.

Endpoint perf `call_kind` labels are `query`, `composite_query`, or `update`.
Query and composite-query endpoint perf rows are only durable when sampled by a
call path that commits state; ordinary query calls should use same-call
`QueryPerfSample<T>` probes instead.

Root-only ICP-refill `cycles_funding` rows use bounded `phase` labels:
`preflight`, `transfer`, or `notify`. Status and error labels are bounded by
the refill status and error-code DTO enums. Non-root canisters do not open or
project the refill record allocation.

## Internal Counters

The runtime still records detailed internal counters for management-canister
calls and coarse system operations. Those tables
are intentionally not exposed as separate public `MetricsKind` values because
they overlap the public operator tiers:

- Management-call progress is visible through `platform_call` and higher-level
  `canister_ops` rows.
- Coarse system counters are redundant with `platform_call`,
  `inter_canister_call`, and `timer`.
