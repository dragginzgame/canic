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
| `Auth` | `[surface, operation, outcome, reason]` | `None` | `Count` | All dimensions are fixed enums for session bootstrap, session lifecycle, identity fallback, and attestation verifier visibility. |
| `CanisterOps` | `[operation, role, outcome, reason]` | `None` | `Count` | Operation, outcome, and reason are fixed enums. Role labels come from configured canister roles, plus `unknown` and `unscoped` fallbacks. |
| `Cascade` | `[operation, snapshot, outcome, reason]` | `None` | `Count` | All dimensions are fixed enums for state/topology cascade fanout, local apply, route resolution, and child-send visibility. |
| `CyclesFunding` | `[metric]` or `[metric, reason]` | Child principal for child-scoped rows; otherwise `None` | `U128` | Child-principal rows intentionally scale with registered children. Metric and reason dimensions are fixed enums. |
| `CyclesTopup` | `[metric]` | `None` | `Count` | Fixed auto-top-up decision and outcome labels. |
| `DelegatedAuth` | `[delegated_auth_authority]` or `[operation, outcome, reason]` | Verified signer authority for authority rows; otherwise `None` | `Count` | Authority rows are bounded by configured signer authorities. Outcome rows use fixed enums for token verification progress and failure reasons. |
| `Directory` | `[operation, outcome, reason]` | `None` | `Count` | All dimensions are fixed enums for keyed directory resolution, claims, stale repair, cleanup, and binding. |
| `Http` | `[method, label]` | `None` | `Count` | Use explicit stable labels for dynamic URLs. URL fallback labels strip query and fragment only. |
| `Intent` | `[surface, operation, outcome, reason]` | `None` | `Count` | All dimensions are fixed enums for call, pool, and cleanup intent reservation visibility. |
| `InterCanisterCall` | `[method]` | Target canister principal | `Count` | Target cardinality grows with topology size; method names should stay static. |
| `Lifecycle` | `[phase, role, stage, outcome]` | `None` | `Count` | All dimensions are fixed enums for lifecycle runtime seeding and async bootstrap visibility. |
| `Perf` | `[endpoint, name]`, `[timer, label]`, or `[checkpoint, scope, label]` | `None` | `CountAndU64` | `value_u64` is total instructions across samples. |
| `PlatformCall` | `[surface, mode, outcome, reason]` | `None` | `Count` | All dimensions are fixed enums for generic IC calls, management calls, ledgers, ECDSA, HTTP outcalls, and XRC. |
| `Pool` | `[operation, outcome, reason]` | `None` | `Count` | All dimensions are fixed enums for pool create/import/recycle/reset/scheduler visibility. |
| `Provisioning` | `[operation, role, outcome, reason]` | `None` | `Count` | Operation, outcome, and reason are fixed enums for create/install/upgrade workflow phases. Role labels come from configured canister roles, plus `unknown` when registry lookup fails. |
| `Replay` | `[operation, outcome, reason]` | `None` | `Count` | All dimensions are fixed enums for root capability replay checks, reservation, cached decode, commit, and abort visibility. |
| `RootCapability` | `[capability, event_type, outcome, proof_mode]` | `None` | `Count` | All dimensions are fixed enums. |
| `Scaling` | `[operation, outcome, reason]` | `None` | `Count` | All dimensions are fixed enums for scaling policy planning, startup warmup, worker creation, and registry updates. |
| `Sharding` | `[operation, outcome, reason]` | `None` | `Count` | Feature-gated by `sharding`; all dimensions are fixed enums for shard assignment and startup shard bootstrap visibility. |
| `System` | `[kind]` | `None` | `Count` | Fixed system operation labels. |
| `Timer` | `[mode, label]` | `None` | `CountAndU64` | `count` is executions; `value_u64` is scheduled delay in milliseconds. Timer labels should be static. |
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

### `Auth`

Auth rows expose session and attestation verifier outcomes without using caller,
subject, key id, or token material as dimensions. Existing auth compatibility
rows also remain visible under `Access`.

Surfaces:

- `attestation`
- `session`

Operations:

- `bootstrap`
- `identity_fallback`
- `refresh`
- `session`
- `verify`

Outcomes:

- `completed`
- `failed`
- `idempotent`
- `rejected`

Reasons:

- `cleared`
- `created`
- `disabled`
- `epoch_rejected`
- `invalid_subject`
- `pruned`
- `raw_caller`
- `refresh_failed`
- `replay`
- `replay_conflict`
- `replay_reused`
- `replaced`
- `subject_mismatch`
- `subject_rejected`
- `token_invalid`
- `ttl_invalid`
- `unknown_key_id`
- `verify_failed`
- `wallet_caller_rejected`

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
provisioning helpers, root bootstrap create skips, and low-level canister
snapshot/restore management calls. Snapshot and restore rows use the `unscoped`
role label at the low-level management boundary when no configured role is
available.

### `Cascade`

Cascade rows expose state/topology propagation progress without using target
canister IDs, template IDs, or role labels as dimensions. Use `InterCanisterCall` rows when
target-principal visibility is needed.

Operations:

- `child_send`
- `local_apply`
- `nonroot_fanout`
- `root_fanout`
- `route_resolve`

Snapshots:

- `state`
- `topology`

Outcomes:

- `started`
- `completed`
- `failed`
- `skipped`

Reasons:

- `empty_snapshot`
- `invalid_state`
- `management_call`
- `no_route`
- `ok`
- `partial_failure`
- `policy_denied`
- `send_failed`
- `unknown`

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

### `DelegatedAuth`

Delegated-auth rows expose both successful authority attribution and bounded
token-verification outcomes. Authority rows keep the legacy
`[delegated_auth_authority]` label with the signer principal in `principal`.
Outcome rows avoid caller principals and token subjects.

Operations:

- `verify_token`

Outcomes:

- `started`
- `completed`
- `failed`

Reasons:

- `audience`
- `audience_not_subset`
- `canonical`
- `cert_audience_rejected`
- `cert_expired`
- `cert_hash_mismatch`
- `cert_not_yet_valid`
- `cert_policy`
- `disabled`
- `invalid_state`
- `issuer_shard_pid_mismatch`
- `local_role_hash_mismatch`
- `missing_local_role`
- `ok`
- `root_key`
- `root_signature_invalid`
- `root_signature_unavailable`
- `scope_rejected`
- `shard_key_binding`
- `shard_signature_invalid`
- `shard_signature_unavailable`
- `token_audience_rejected`
- `token_expired`
- `token_invalid_window`
- `token_issued_before_cert`
- `token_not_yet_valid`
- `token_outlives_cert`
- `token_ttl_exceeded`

### `Directory`

Directory rows expose keyed placement progress without using directory pool
names, key values, roles, or canister IDs as labels.

Operations:

- `bind`
- `claim`
- `classify`
- `cleanup_stale`
- `create_instance`
- `finalize`
- `recover`
- `recycle_abandoned`
- `repair_stale`
- `resolve`

Outcomes:

- `started`
- `completed`
- `failed`
- `skipped`

Reasons:

- `already_bound`
- `claim_lost`
- `claimed`
- `invalid_child`
- `invalid_state`
- `management_call`
- `missing`
- `ok`
- `pending_current`
- `pending_fresh`
- `policy_denied`
- `registry_missing`
- `released_stale`
- `role_mismatch`
- `stale_cleanup`
- `stale_repairable`
- `unknown`

### `Http`

Prefer:

```rust
HttpApi::get_with_label(url, headers, "provider_route").await
```

over unlabeled calls when `url` may contain IDs, account names, timestamps, or other request-specific path segments.

Unlabeled HTTP metrics normalize the URL by removing query strings and fragments, but they do not rewrite dynamic path segments.

### `PlatformCall`

Platform-call rows expose low-cardinality platform-call outcomes without using target
principals, method names, URLs, ledger IDs, key names, or asset pairs as
dimensions. Use `InterCanisterCall` when target-principal and method-level call volume is
needed.

Surfaces:

- `ecdsa`
- `generic`
- `http`
- `ledger`
- `management`
- `xrc`

Modes:

- `bounded_wait`
- `local_verify`
- `query`
- `unbounded_wait`
- `update`

Outcomes:

- `completed`
- `failed`
- `started`

Reasons:

- `candid_decode`
- `candid_encode`
- `http_status`
- `infra`
- `invalid_public_key`
- `invalid_signature`
- `ledger_rejected`
- `ok`
- `rejected`
- `unavailable`

### `Intent`

Intent rows expose reservation lifecycle outcomes without using resource keys,
intent IDs, call methods, or canister principals as dimensions.

Surfaces:

- `call`
- `cleanup`
- `pool`

Operations:

- `abort`
- `capacity_check`
- `cleanup`
- `commit`
- `reserve`

Outcomes:

- `completed`
- `failed`

Reasons:

- `capacity`
- `expired`
- `idle`
- `no_expired`
- `ok`
- `overflow`
- `storage_failed`

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

### `Pool`

Pool rows expose the lifecycle of reusable empty canisters without using
canister IDs as labels.

Operations:

- `create_empty`
- `import_immediate`
- `import_queued`
- `recycle`
- `reset`
- `scheduler`
- `select_ready`

Outcomes:

- `started`
- `scheduled`
- `completed`
- `failed`
- `requeued`
- `skipped`

Reasons:

- `already_present`
- `empty`
- `failed_entry`
- `in_progress`
- `invalid_state`
- `management_call`
- `non_importable_local`
- `not_found`
- `ok`
- `policy_denied`
- `registered_in_subnet`
- `unknown`

### `Provisioning`

Provisioning rows expose workflow-level create, install, propagation, and
upgrade progress without using canister principals, module hashes, chunk hashes,
or parent principals as dimensions. Use `CanisterOps` for lower-level
management operation visibility and `PlatformCall` for platform-call outcomes.

Operations:

- `allocate`
- `create`
- `install`
- `propagate_state`
- `propagate_topology`
- `resolve_module`
- `upgrade`

Outcomes:

- `completed`
- `failed`
- `skipped`
- `started`

Reasons:

- `already_current`
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

### `Replay`

Replay rows expose root capability replay safety outcomes without using caller
principals, request IDs, payload hashes, or capability names as dimensions. Use
`RootCapability` rows when capability-level replay visibility is needed.

Operations:

- `abort`
- `check`
- `commit`
- `decode`
- `reserve`

Outcomes:

- `completed`
- `failed`

Reasons:

- `capacity`
- `conflict`
- `decode_failed`
- `duplicate`
- `encode_failed`
- `expired`
- `fresh`
- `in_flight`
- `invalid_ttl`
- `missing_metadata`
- `ok`

### `Scaling`

Scaling rows expose worker pool planning and bootstrap progress without using
pool names, worker roles, or canister IDs as labels. Use canister operation rows
when role-level create visibility is needed.

Operations:

- `bootstrap_config`
- `bootstrap_pool`
- `create_worker`
- `plan_create`
- `register_worker`

Outcomes:

- `started`
- `completed`
- `failed`
- `skipped`

Reasons:

- `at_max_workers`
- `below_min_workers`
- `invalid_state`
- `management_call`
- `missing_worker_entry`
- `no_initial_workers`
- `ok`
- `policy_denied`
- `scaling_disabled`
- `target_satisfied`
- `unknown`
- `within_bounds`

### `Sharding`

Sharding rows expose shard assignment and configured startup shard progress
without using pool names, partition keys, shard roles, or canister IDs as labels.
This family is available when the `sharding` feature is enabled.

Operations:

- `assign`
- `assign_key`
- `bootstrap_active`
- `bootstrap_config`
- `bootstrap_pool`
- `create_shard`
- `plan_assign`

Outcomes:

- `started`
- `completed`
- `failed`
- `skipped`

Reasons:

- `already_assigned`
- `create_allowed`
- `existing_capacity`
- `invalid_state`
- `management_call`
- `no_free_slots`
- `no_initial_shards`
- `ok`
- `policy_denied`
- `pool_at_capacity`
- `sharding_disabled`
- `target_satisfied`
- `unknown`

### `Timer`

Timer rows use `CountAndU64`:

- `count`: number of timer executions
- `value_u64`: timer delay in milliseconds

Scheduling is also counted separately in `MetricsKind::System` as `TimerScheduled`.

### `WasmStore`

Wasm-store rows expose install-source resolution, bootstrap chunk sync, and
managed store publication progress without using template IDs, versions,
canister IDs, or bindings as labels.

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
