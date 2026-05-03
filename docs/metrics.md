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

## Metric Families

| `MetricsKind` | Labels | Principal | Value | Cardinality notes |
| ------------- | ------ | --------- | ----- | ----------------- |
| `Access` | `[endpoint, kind, predicate]` | `None` | `Count` | Bounded by macro-generated endpoint names and static access predicate names. |
| `CyclesFunding` | `[metric]` or `[metric, reason]` | Child principal for child-scoped rows; otherwise `None` | `U128` | Child-principal rows intentionally scale with registered children. Metric and reason dimensions are fixed enums. |
| `CyclesTopup` | `[metric]` | `None` | `Count` | Fixed auto-top-up decision and outcome labels. |
| `DelegatedAuth` | `[delegated_auth_authority]` | Verified signer authority | `Count` | Bounded by configured delegated-auth signer authorities, not request callers. |
| `Http` | `[method, label]` | `None` | `Count` | Use explicit stable labels for dynamic URLs. URL fallback labels strip query and fragment only. |
| `Icc` | `[method]` | Target canister principal | `Count` | Target cardinality grows with topology size; method names should stay static. |
| `Perf` | `[endpoint, name]`, `[timer, label]`, or `[checkpoint, scope, label]` | `None` | `CountAndU64` | `value_u64` is total instructions across samples. |
| `RootCapability` | `[capability, event_type, outcome, proof_mode]` | `None` | `Count` | All dimensions are fixed enums. |
| `System` | `[kind]` | `None` | `Count` | Fixed system operation labels. |
| `Timer` | `[mode, label]` | `None` | `CountAndU64` | `count` is executions; `value_u64` is scheduled delay in milliseconds. Timer labels should be static. |

## Family Details

### `Access`

Access rows are emitted only for denied access checks.

`kind` is one of:

- `auth`
- `custom`
- `env`
- `guard`
- `rule`

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

