# Canic Metrics Reference

`canic_metrics(kind, page)` returns a paginated `Page<MetricEntry>`.
Rows are sorted by `labels`, then `principal`, before pagination.

Each row has:

| Field | Meaning |
| ----- | ------- |
| `labels` | Ordered string dimensions for the selected metric family. |
| `principal` | Optional principal dimension when the family is naturally principal-scoped. |
| `value` | Metric payload: `Count`, `CountAndU64`, or `U128`. |

`CountAndU64` uses `count` as the event/sample count. The `value_u64` meaning is family-specific.

## Query Perf Samples

Query calls can update in-memory perf tables during the call, but those updates
are not committed after the query returns. For audit probes that need comparable
query-side instruction measurements, return a `QueryPerfSample<T>` from the same
query call:

```rust
#[canic_query(requires(env::build_local_only()))]
async fn audit_env_probe() -> Result<QueryPerfSample<EnvSnapshotResponse>, Error> {
    Ok(MetricsQuery::sample_query(EnvQuery::snapshot()))
}
```

`QueryPerfSample::local_instructions` is the local call-context instruction
counter observed before the query response is returned. Use this for explicit
audit/probe endpoints; use `canic_metrics(MetricsKind::Perf, ...)` for persisted
update and timer rows.

Audit reports should treat a zero `local_instructions` value as unobservable
rather than as a successful zero-cost query measurement.

## Metric Families

| `MetricsKind` | Labels | Principal | Value | Cardinality notes |
| ------------- | ------ | --------- | ----- | ----------------- |
| `Access` | `[endpoint, kind, predicate]` | `None` | `Count` | Bounded by macro-generated endpoint names and static access predicate names. |
| `CanisterOps` | `[operation, role, outcome, reason]` | `None` | `Count` | Operation, outcome, and reason are fixed enums. Role labels come from configured canister roles, plus `unknown` and `unscoped` fallbacks. |
| `CyclesFunding` | `[metric]` or `[metric, reason]` | Child principal for child-scoped rows; otherwise `None` | `U128` | Child-principal rows intentionally scale with registered children. Metric and reason dimensions are fixed enums. |
| `CyclesTopup` | `[metric]` | `None` | `Count` | Fixed auto-top-up decision and outcome labels. |
| `DelegatedAuth` | `[delegated_auth_authority]` | Verified signer authority | `Count` | Bounded by configured delegated-auth signer authorities, not request callers. |
| `Http` | `[method, label]` | `None` | `Count` | Use explicit stable labels for dynamic URLs. URL fallback labels strip query and fragment only. |
| `Icc` | `[method]` | Target canister principal | `Count` | Target cardinality grows with topology size; method names should stay static. |
| `Lifecycle` | `[phase, role, stage, outcome]` | `None` | `Count` | All dimensions are fixed enums for lifecycle runtime seeding and async bootstrap visibility. |
| `Perf` | `[endpoint, name]`, `[timer, label]`, or `[checkpoint, scope, label]` | `None` | `CountAndU64` | `value_u64` is total instructions across samples. |
| `RootCapability` | `[capability, event_type, outcome, proof_mode]` | `None` | `Count` | All dimensions are fixed enums. |
| `System` | `[kind]` | `None` | `Count` | Fixed system operation labels. |
| `Timer` | `[mode, label]` | `None` | `CountAndU64` | `count` is executions; `value_u64` is scheduled delay in milliseconds. Timer labels should be static. |
| `WasmStore` | `[operation, source, outcome, reason]` | `None` | `Count` | All dimensions are fixed enums for module-source resolution and wasm-store publication visibility. |
| `WasmStore` | `[operation, source, outcome, reason]` | `None` | `Count` | All dimensions are fixed enums for source resolution, bootstrap chunk sync, and managed store publication. |

## Family Details

### `Access`

Access rows are emitted only for denied access checks.

`kind` is one of:

- `auth`
- `custom`
- `env`
- `guard`
- `rule`

### `CanisterOps`

Canister operation rows expose higher-level fleet operation outcomes above the
raw management-canister system counters.

Operations:

- `create`
- `delete`
- `install`
- `reinstall`
- `restore`
- `snapshot`
- `upgrade`

Outcomes:

- `started`
- `completed`
- `failed`
- `skipped`

Reasons:

- `already_exists`
- `cycles`
- `invalid_state`
- `management_call`
- `missing_wasm`
- `new_allocation`
- `not_found`
- `ok`
- `policy_denied`
- `pool_reuse`
- `pool_topup`
- `state_propagation`
- `topology`
- `topology_propagation`
- `unknown`

Current rows are emitted by root create/upgrade workflows, install/delete
provisioning helpers, and root bootstrap create skips.

### `CyclesFunding`

Metric labels:

- `cycles_denied_global_kill_switch`
- `cycles_denied_to_child`
- `cycles_denied_total`
- `cycles_granted_to_child`
- `cycles_granted_total`
- `cycles_requested_by_child`
- `cycles_requested_total`

Denial reason labels:

- `child_not_found`
- `cooldown_active`
- `execution_error`
- `insufficient_cycles`
- `kill_switch_disabled`
- `max_per_child_exceeded`
- `not_direct_child`

### `CyclesTopup`

Metric labels:

- `above_threshold`
- `config_error`
- `policy_missing`
- `request_err`
- `request_in_flight`
- `request_ok`
- `request_scheduled`

### `Http`

Prefer:

```rust
HttpApi::get_with_label(url, headers, "provider_route").await
```

over unlabeled calls when `url` may contain IDs, account names, timestamps, or other request-specific path segments.

Unlabeled HTTP metrics normalize the URL by removing query strings and fragments, but they do not rewrite dynamic path segments.

### `Lifecycle`

Lifecycle rows expose synchronous runtime seeding and asynchronous bootstrap
progress.

Phases:

- `init`
- `post_upgrade`

Roles:

- `root`
- `nonroot`

Stages:

- `runtime`
- `bootstrap`

Outcomes:

- `scheduled`
- `started`
- `completed`
- `failed`
- `waiting`
- `skipped`

### `Perf`

Perf rows use `CountAndU64`:

- `count`: number of samples
- `value_u64`: total instructions

Endpoint perf uses exclusive instruction accounting, so nested endpoint dispatch subtracts child endpoint work from the parent row.

### `Timer`

Timer rows use `CountAndU64`:

- `count`: number of timer executions
- `value_u64`: timer delay in milliseconds

Scheduling is also counted separately in `MetricsKind::System` as `TimerScheduled`.

### `WasmStore`

Wasm-store rows expose module-source resolution, bootstrap chunk sync, and
managed publication progress.

Operations:

- `bootstrap_chunk_sync`
- `chunk_publish`
- `chunk_upload`
- `manifest_promote`
- `prepare`
- `release_publish`
- `source_resolve`

Sources:

- `bootstrap`
- `embedded`
- `managed_fleet`
- `resolver`
- `store`
- `target_store`

Outcomes:

- `started`
- `completed`
- `failed`
- `skipped`

Reasons:

- `cache_hit`
- `cache_miss`
- `capacity`
- `hash_mismatch`
- `invalid_state`
- `management_call`
- `missing_chunk`
- `missing_manifest`
- `ok`
- `store_call`
- `unsupported_inline`

### `WasmStore`

Wasm-store rows expose install-source resolution and store publication progress
without using template IDs, versions, canister IDs, or bindings as labels.

Operations:

- `bootstrap_chunk_sync`
- `chunk_publish`
- `chunk_upload`
- `manifest_promote`
- `prepare`
- `release_publish`
- `source_resolve`

Sources:

- `bootstrap`
- `embedded`
- `managed_fleet`
- `resolver`
- `store`
- `target_store`

Outcomes:

- `started`
- `completed`
- `failed`
- `skipped`

Reasons:

- `cache_hit`
- `cache_miss`
- `capacity`
- `hash_mismatch`
- `invalid_state`
- `management_call`
- `missing_chunk`
- `missing_manifest`
- `ok`
- `store_call`
- `unsupported_inline`
