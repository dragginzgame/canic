# Instruction Footprint Audit - 2026-05-13

## Report Preamble

- Scope: Canic instruction footprint (first `0.35` baseline, partial canister scope)
- Definition path: `docs/audits/recurring/system/instruction-footprint.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `c533afd6`
- Method tag/version: `Method V1`
- Comparability status: `partial`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-13T11:53:59Z`
- Branch: `main`
- Worktree: `dirty`
- Execution environment: `PocketIC`
- Target canisters in scope: `leaf_probe` `root` `root_probe` `scaling_probe` `test`
- Target endpoints/flows in scope: `audit_env_probe` `audit_log_probe` `audit_plan_create_worker_probe` `audit_subnet_registry_probe` `audit_subnet_state_probe` `audit_time_probe` `canic_request_delegation` `canic_response_capability_v1` `canic_template_prepare_admin` `canic_template_publish_chunk_admin` `canic_template_stage_manifest_admin` `test`
- Deferred from this baseline: no additional functional flows are deferred beyond first-run comparability; this run covers shared queries plus delegated auth issuance, verifier confirmation, replay/cycles, scaling worker creation, sharding account creation, and root template admin updates.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Scenario manifest recorded | PASS | `artifacts/instruction-footprint/scenario-manifest.json` captures the scenario identity tuple for every sampled endpoint. |
| Normalized perf rows recorded | PASS | `artifacts/instruction-footprint/perf-rows.json` stores canonical endpoint rows with count and total local instructions. |
| Checkpoint deltas recorded | PASS | `artifacts/instruction-footprint/checkpoint-deltas.json` stores non-zero per-scenario checkpoint rows. |
| Fresh topology isolation used | PASS | Each scenario ran under a fresh smallest-profile root harness install instead of reusing one cumulative perf table. |
| Flow checkpoint coverage scanned | PASS | `artifacts/instruction-footprint/flow-checkpoints.log` records the current repo scan result. |
| `perf!` checkpoints available for critical flows | PASS | Current repo scan found at least one `perf!` call site. |
| Query endpoint perf visibility | PASS | Sampled query scenarios were measured through local-only `QueryPerfSample` probe endpoints because query-side perf rows are not committed. |
| Baseline path selected by daily baseline discipline | PARTIAL | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |

## Comparison to Previous Relevant Run

- First run of day for `instruction-footprint`; this report establishes the daily baseline.
- Query scenarios are now sampled through local-only `QueryPerfSample` probes because query-side perf rows are not committed, so their rows are directly comparable to later probe-backed reruns.
- Baseline drift values are `N/A` until a same-day rerun or later comparable run exists.

## Endpoint Matrix

| Canister | Endpoint | Scenario | Count | Total local instructions | Avg local instructions | Baseline delta | Notes |
| --- | --- | --- | ---: | ---: | ---: | --- | --- |
| `leaf_probe` | `audit_time_probe` | `minimal-valid` | 1 | 30531 | 30531 | N/A | local-only QueryPerfSample probe |
| `leaf_probe` | `audit_env_probe` | `minimal-valid` | 1 | 32069 | 32069 | N/A | local-only QueryPerfSample probe |
| `leaf_probe` | `audit_log_probe` | `empty-page` | 1 | 307496 | 307496 | N/A | local-only QueryPerfSample probe |
| `root_probe` | `audit_subnet_registry_probe` | `representative-valid` | 1 | 72566 | 72566 | N/A | local-only QueryPerfSample probe |
| `root_probe` | `audit_subnet_state_probe` | `minimal-valid` | 1 | 29601 | 29601 | N/A | local-only QueryPerfSample probe |
| `scaling_probe` | `audit_plan_create_worker_probe` | `empty-pool` | 1 | 62868 | 62868 | N/A | local-only QueryPerfSample probe |
| `root` | `canic_request_delegation` | `fresh-shard` | 1 | 800834 | 800834 | N/A |  |
| `test` | `test` | `minimal-valid` | 1 | 765 | 765 | N/A |  |
| `root` | `canic_response_capability_v1` | `cycles-request` | 1 | 506816 | 506816 | N/A |  |
| `root` | `canic_template_stage_manifest_admin` | `single-chunk` | 1 | 407347 | 407347 | N/A |  |
| `root` | `canic_template_prepare_admin` | `single-chunk` | 1 | 185939 | 185939 | N/A |  |
| `root` | `canic_template_publish_chunk_admin` | `single-chunk` | 1 | 326306 | 326306 | N/A |  |

## Flow Checkpoints

- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:411:canic_core::perf!("publish_stage_validate_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:414:canic_core::perf!("publish_stage_upsert_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:443:canic_core::perf!("publish_store_validate_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:455:canic_core::perf!("publish_store_project_capacity");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:464:canic_core::perf!("publish_store_enforce_capacity");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:467:canic_core::perf!("publish_store_upsert_chunk");`
- `crates/canic-control-plane/src/storage/stable/template/chunked.rs:262:canic_core::perf!("chunk_store_insert");`
- `crates/canic-control-plane/src/storage/stable/template/chunked.rs:271:canic_core::perf!("chunk_store_accounting");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:226:canic_core::perf!("bootstrap_import_pool");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:235:canic_core::perf!("bootstrap_create_canisters");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:244:canic_core::perf!("bootstrap_rebuild_indexes");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:248:canic_core::perf!("bootstrap_validate_state");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:429:canic_core::perf!("bootstrap_ensure_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:431:canic_core::perf!("bootstrap_publish_release_set");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:733:canic_core::perf!("bootstrap_create_role");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:741:canic_core::perf!("bootstrap_ensure_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:751:canic_core::perf!("bootstrap_prune_store_catalog");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:779:canic_core::perf!("bootstrap_create_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:781:canic_core::perf!("bootstrap_sync_store_inventory");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:788:canic_core::perf!("bootstrap_import_store_catalog");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication/release/chunks.rs:115:canic_core::perf!("publish_push_store_chunk");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication/release/promote.rs:69:canic_core::perf!("publish_promote_manifest");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication/release/target.rs:95:canic_core::perf!("publish_prepare_store");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:138:crate::perf!("observe_state");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:146:crate::perf!("plan_spawn");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:242:crate::perf!("create_canister");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:248:crate::perf!("register_worker");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:81:crate::perf!("observe_state");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:96:crate::perf!("plan_spawn");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:177:crate::perf!("load_active_shards");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:220:crate::perf!("collect_registry");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:235:crate::perf!("plan_assign");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:240:crate::perf!("already_assigned");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:264:crate::perf!("assign_existing");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:304:crate::perf!("allocate_shard");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:313:crate::perf!("assign_created");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:329:crate::perf!("create_blocked");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:405:crate::perf!("bootstrap_empty_active");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:414:crate::perf!("assign_bootstrap_created");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:439:crate::perf!("load_bootstrap_pool_entries");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:447:crate::perf!("select_bootstrap_slot");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:450:crate::perf!("allocate_bootstrap_shard");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:100:crate::perf!("extract_context");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:103:crate::perf!("map_request");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:106:crate::perf!("preflight");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:125:crate::perf!("execute_capability");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:137:crate::perf!("commit_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:102:crate::perf!("duplicate_in_flight");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:115:crate::perf!("duplicate_conflict");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:128:crate::perf!("replay_expired");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:260:crate::perf!("commit_encode");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:284:crate::perf!("abort_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:52:crate::perf!("prepare_replay_input");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:61:crate::perf!("evaluate_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:81:crate::perf!("reserve_fresh");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:89:crate::perf!("decode_cached");`

## Measured Checkpoint Deltas

| Scenario | Scope | Label | Count | Total local instructions | Avg local instructions |
| --- | --- | --- | ---: | ---: | ---: |
| `root:canic_template_publish_chunk_admin:single-chunk` | `canic_control_plane::storage::stable::template::chunked` | `chunk_store_insert` | 1 | 134832 | 134832 |
| `root:canic_template_publish_chunk_admin:single-chunk` | `canic_control_plane::ops::storage::template::chunked` | `publish_stage_upsert_chunk` | 1 | 6952 | 6952 |
| `root:canic_template_publish_chunk_admin:single-chunk` | `canic_control_plane::storage::stable::template::chunked` | `chunk_store_accounting` | 1 | 6599 | 6599 |
| `root:canic_template_publish_chunk_admin:single-chunk` | `canic_control_plane::ops::storage::template::chunked` | `publish_stage_validate_chunk` | 1 | 0 | 0 |

## Checkpoint Coverage Gaps

Critical flows with checkpoints:
- `root capability dispatch`
- `replay/cached-response path`
- `sharding assignment/query flow`
- `scaling/provisioning flow`
- `bootstrap/install/publication flow`

Critical flows without checkpoints:
- `delegated auth issuance/verification`

Proposed first checkpoint insertion sites:
- `delegated auth issuance/verification` -> `crates/canic-core/src/workflow/auth.rs`

## Structural Hotspots

| Rank | Scenario | Avg local instructions | Module pressure | Evidence |
| --- | --- | ---: | --- | --- |
| 1 | `root:canic_request_delegation:fresh-shard` | 800834 | Root delegated auth request path | [root endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints/root.rs), [auth workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/auth.rs) |
| 2 | `root:canic_response_capability_v1:request-cycles-fresh` | 506816 | Root dispatcher plus replay/capability workflow | [request handler](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/mod.rs), [replay workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/replay.rs) |
| 3 | `root:canic_template_stage_manifest_admin:single-chunk` | 407347 | Root template publication admin path | [root endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints/root.rs), [template storage ops](/home/adam/projects/canic/crates/canic-control-plane/src/ops/storage/template/chunked.rs) |

## Hub Module Pressure

- `root::canic_response_capability_v1` now has measured replay/cycles stage deltas, so root capability work no longer has to be treated as an opaque endpoint total.
- `root::canic_request_delegation` remains the main shared update hotspot in the retained audit lane, so further optimization work should stay focused on shared runtime/auth cost rather than demo provisioning flows.
- `scale_hub::plan_create_worker` stays in the matrix as an audit-only dry-run probe, which keeps placement-policy visibility without turning demo `create_*` flows into default audit targets.
- `test::test` provides the current chain-key-free update floor on a non-root child canister. Drift there points back to shared runtime/update overhead rather than topology-specific logic.
- Root state/registry reads stay separate from the leaf floor. They matter for operator paths, but they should not be confused with the shared ordinary-leaf baseline.

## Dependency Fan-In Pressure

- Shared observability reads (`canic_env`, `canic_log`) are now measured through the internal `leaf_probe` canister instead of the shipped demo surface, and raw time is measured through the same internal lane. Their rows use `QueryPerfSample` counters from the measured call context rather than inferred zeroes or missing query-side perf-table commits.
- The sampled non-trivial hotspots now concentrate in shared auth/replay/root runtime and the audit-only placement dry-run probe. The local `test::test` update acts as the baseline floor for update overhead on an ordinary child canister.
- Flow-stage checkpoints now exist in the scaling, sharding, publication, and replay workflows. This matrix records non-zero checkpoint deltas for sampled update scenarios, so the next optimization pass can target concrete stages instead of endpoint totals alone.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Flow checkpoint coverage present | INFO | Current repo scan found 56 `perf!` call sites under `crates/`. |
| Highest sampled endpoint currently highest-cost | WARN | `root:canic_request_delegation:fresh-shard` averages 800834 local instructions in this first baseline. |
| Baseline drift not yet available | INFO | First run of day; deltas remain `N/A` until the next comparable rerun. |

## Risk Score

Risk Score: **3 / 10**

Interpretation: query visibility and stage attribution are now working for the sampled matrix. The remaining audit risk is mostly first-run comparability (`N/A` baseline deltas) plus a few endpoint-only paths that still do not have deeper internal stage attribution, not missing coverage of the critical flows themselves.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture` | PASS | PocketIC runner completed and wrote the report plus normalized artifacts. |
| `fresh root harness profile per scenario` | PASS | Each scenario used a fresh smallest-profile root bootstrap instead of sharing one cumulative perf table. |
| `canic_metrics(MetricsKind::Runtime, PageRequest { limit=512, offset=0 })` | PASS | Update scenarios were sampled before/after through persisted perf rows, and query scenarios used local-only `QueryPerfSample` probe endpoints because query-side perf rows are not committed; normalized rows saved under `/home/adam/projects/canic/docs/audits/reports/2026-05/2026-05-13/artifacts/instruction-footprint/perf-rows.json`. |
| `repo checkpoint scan` | PASS | Found 56 checkpoint call sites. |
| `checkpoint delta capture` | PASS | 4 non-zero checkpoint delta rows were captured under `/home/adam/projects/canic/docs/audits/reports/2026-05/2026-05-13/artifacts/instruction-footprint/checkpoint-deltas.json`. |
| `query perf visibility` | PASS | All sampled query scenarios returned `QueryPerfSample` local instruction counters through the local-only probe endpoints, which avoids relying on non-persisted query-side perf state. |
| `baseline comparison` | BLOCKED | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |

## Follow-up Actions

1. Owner boundary: `flow observability`
   Action: rerun this audit after one concrete perf change so the next report has real comparable baseline deltas instead of first-run `N/A`, and only add deeper verifier-side auth checkpoints if that endpoint-total starts to matter.
2. Owner boundary: `shared update hotspots`
   Action: compare `root::canic_request_delegation`, `root::canic_response_capability_v1`, and the local `test::test` update floor before/after any shared-runtime cleanup, using this report as the `0.35` baseline.
3. Owner boundary: `shared observability floor`
   Action: keep the internal standalone query probes in the matrix so shared-runtime drift does not hide behind root-only or coordinator-only endpoints.

## Report Files

- [instruction-footprint.md](./instruction-footprint.md)
- [scenario-manifest.json](artifacts/instruction-footprint/scenario-manifest.json)
- [perf-rows.json](artifacts/instruction-footprint/perf-rows.json)
- [endpoint-matrix.tsv](artifacts/instruction-footprint/endpoint-matrix.tsv)
- [checkpoint-deltas.json](artifacts/instruction-footprint/checkpoint-deltas.json)
- [flow-checkpoints.log](artifacts/instruction-footprint/flow-checkpoints.log)
- [checkpoint-coverage-gaps.json](artifacts/instruction-footprint/checkpoint-coverage-gaps.json)
- [verification-readout.md](artifacts/instruction-footprint/verification-readout.md)
- [method.json](artifacts/instruction-footprint/method.json)
- [environment.json](artifacts/instruction-footprint/environment.json)
