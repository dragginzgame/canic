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
[subnets.prime.canisters.app.metrics]
profile = "full"
```

Unsupported tier requests keep the same Candid enum shape and return a Canic
invalid-input error for that canister.

| `MetricsKind` | Families | Notes |
| ------------- | -------- | ----- |
| `Core` | `lifecycle`, `canister_ops`, `cycles_funding`, `cycles_topup` | Operator-facing lifecycle, canister operation, and cycles rows. |
| `Placement` | `cascade`, `directory`, `pool`, `scaling`, `sharding` | Fleet placement and topology rows. `sharding` is present only when the sharding feature is enabled. |
| `Platform` | `platform_call`, `http`, `inter_canister_call` | Low-cardinality IC/platform I/O rows. |
| `Runtime` | `intent`, `perf`, `timer` | Runtime reservation, instruction, and timer rows. |
| `Security` | `access`, `auth`, `delegated_auth`, `replay`, `root_capability` | Access, delegated auth, replay, and capability rows. |
| `Storage` | `wasm_store` | Wasm-store source, chunk, and publication rows. |

### `Core`

Core rows cover lifecycle, canister operation, and cycles behavior.

### `Placement`

Placement rows cover topology propagation, directory placement, reusable pools,
scaling, and feature-gated sharding.

### `Platform`

Platform rows cover IC/platform call outcomes, HTTP outcalls, and
inter-canister calls.

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
| `cycles_funding` | `[metric]` or `[metric, reason]` | Child principal for child-scoped rows | `U128` |
| `cycles_topup` | `[metric]` | `None` | `Count` |
| `delegated_auth` | `[delegated_auth_authority]` or `[operation, outcome, reason]` | Verified signer authority for authority rows | `Count` |
| `directory` | `[operation, outcome, reason]` | `None` | `Count` |
| `http` | `[method, label]` | `None` | `Count` |
| `intent` | `[surface, operation, outcome, reason]` | `None` | `Count` |
| `inter_canister_call` | `[method]` | Target canister principal | `Count` |
| `lifecycle` | `[phase, role, stage, outcome]` | `None` | `Count` |
| `perf` | `[endpoint, name]`, `[timer, label]`, or `[checkpoint, scope, label]` | `None` | `CountAndU64` |
| `platform_call` | `[surface, mode, outcome, reason]` | `None` | `Count` |
| `pool` | `[operation, outcome, reason]` | `None` | `Count` |
| `replay` | `[operation, outcome, reason]` | `None` | `Count` |
| `root_capability` | `[capability, event_type, outcome, proof_mode]` | `None` | `Count` |
| `scaling` | `[operation, outcome, reason]` | `None` | `Count` |
| `sharding` | `[operation, outcome, reason]` | `None` | `Count` |
| `timer` | `[mode, label]` | `None` | `CountAndU64` |
| `wasm_store` | `[operation, source, outcome, reason]` | `None` | `Count` |

## Internal Counters

The runtime still records detailed internal counters for management-canister
calls, provisioning workflow phases, and coarse system operations. Those tables
are intentionally not exposed as separate public `MetricsKind` values because
they overlap the public operator tiers:

- Management-call progress is visible through `platform_call` and higher-level
  `canister_ops` rows.
- Provisioning workflow progress is folded into public canister operation and
  placement rows where it is operator-relevant.
- Coarse system counters are redundant with `platform_call`, `http`,
  `inter_canister_call`, and `timer`.
