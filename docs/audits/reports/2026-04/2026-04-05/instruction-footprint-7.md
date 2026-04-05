# Instruction Footprint Audit - 2026-04-05

## Report Preamble

- Scope: Canic instruction footprint (first `0.25` baseline, partial canister scope)
- Definition path: `docs/audits/recurring/system/instruction-footprint.md`
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/instruction-footprint-6.md`
- Code snapshot identifier: `1d0c6945`
- Method tag/version: `Method V1`
- Comparability status: `partial`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T19:36:39Z`
- Branch: `main`
- Worktree: `dirty`
- Execution environment: `PocketIC`
- Target canisters in scope: `audit_leaf_probe` `audit_root_probe` `audit_scaling_probe` `root` `scale_hub` `test` `user_hub`
- Target endpoints/flows in scope: `audit_env_probe` `audit_log_probe` `audit_plan_create_worker_probe` `audit_subnet_registry_probe` `audit_subnet_state_probe` `audit_time_probe` `canic_request_delegation` `canic_response_capability_v1` `canic_template_prepare_admin` `canic_template_publish_chunk_admin` `canic_template_stage_manifest_admin` `create_account` `create_worker` `test`
- Deferred from this baseline: no additional functional flows are deferred beyond first-run comparability; this run covers shared queries plus delegated auth issuance, verifier confirmation, replay/cycles, scaling worker creation, sharding account creation, and root template admin updates.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Scenario manifest recorded | PASS | `artifacts/instruction-footprint-7/scenario-manifest.json` captures the scenario identity tuple for every sampled endpoint. |
| Normalized perf rows recorded | PASS | `artifacts/instruction-footprint-7/perf-rows.json` stores canonical endpoint rows with count and total local instructions. |
| Checkpoint deltas recorded | PASS | `artifacts/instruction-footprint-7/checkpoint-deltas.json` stores non-zero per-scenario checkpoint rows. |
| Fresh topology isolation used | PASS | Each scenario ran under a fresh smallest-profile root harness install instead of reusing one cumulative perf table. |
| Flow checkpoint coverage scanned | PASS | `artifacts/instruction-footprint-7/flow-checkpoints.log` records the current repo scan result. |
| `perf!` checkpoints available for critical flows | PASS | Current repo scan found at least one `perf!` call site. |
| Query endpoint perf visibility | PASS | Sampled query scenarios were measured through same-call local-only perf probe endpoints because query-side perf rows are not committed. |
| Baseline path selected by daily baseline discipline | PARTIAL | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |

## Comparison to Previous Relevant Run

- First run of day for `instruction-footprint`; this report establishes the daily baseline.
- Query scenarios are now sampled through same-call local-only perf probes because query-side perf rows are not committed, so their rows are directly comparable to later probe-backed reruns.
- Baseline drift values are `N/A` until a same-day rerun or later comparable run exists.

## Endpoint Matrix

| Canister | Endpoint | Scenario | Count | Total local instructions | Avg local instructions | Baseline delta | Notes |
| --- | --- | --- | ---: | ---: | ---: | --- | --- |
| `audit_leaf_probe` | `audit_time_probe` | `minimal-valid` | 1 | 20680 | 20680 | N/A | same-call local-only perf probe |
| `audit_leaf_probe` | `audit_env_probe` | `minimal-valid` | 1 | 22330 | 22330 | N/A | same-call local-only perf probe |
| `audit_leaf_probe` | `audit_log_probe` | `empty-page` | 1 | 302876 | 302876 | N/A | same-call local-only perf probe |
| `audit_root_probe` | `audit_subnet_registry_probe` | `representative-valid` | 1 | 64512 | 64512 | N/A | same-call local-only perf probe |
| `audit_root_probe` | `audit_subnet_state_probe` | `minimal-valid` | 1 | 20245 | 20245 | N/A | same-call local-only perf probe |
| `audit_scaling_probe` | `audit_plan_create_worker_probe` | `empty-pool` | 1 | 52061 | 52061 | N/A | same-call local-only perf probe |
| `scale_hub` | `create_worker` | `empty-pool` | 1 | 2650150 | 2650150 | N/A |  |
| `user_hub` | `create_account` | `new-principal` | 1 | 2944415 | 2944415 | N/A |  |
| `root` | `canic_request_delegation` | `fresh-shard` | 1 | 2168632 | 2168632 | N/A |  |
| `test` | `test` | `minimal-valid` | 1 | 816 | 816 | N/A |  |
| `root` | `canic_response_capability_v1` | `cycles-request` | 1 | 1479508 | 1479508 | N/A |  |
| `root` | `canic_template_stage_manifest_admin` | `single-chunk` | 1 | 415287 | 415287 | N/A |  |
| `root` | `canic_template_prepare_admin` | `single-chunk` | 1 | 186297 | 186297 | N/A |  |
| `root` | `canic_template_publish_chunk_admin` | `single-chunk` | 1 | 335144 | 335144 | N/A |  |

## Flow Checkpoints

- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:413:canic_core::perf!("publish_stage_validate_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:416:canic_core::perf!("publish_stage_upsert_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:445:canic_core::perf!("publish_store_validate_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:457:canic_core::perf!("publish_store_project_capacity");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:466:canic_core::perf!("publish_store_enforce_capacity");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:469:canic_core::perf!("publish_store_upsert_chunk");`
- `crates/canic-control-plane/src/storage/stable/template/chunked.rs:256:canic_core::perf!("chunk_store_insert");`
- `crates/canic-control-plane/src/storage/stable/template/chunked.rs:266:canic_core::perf!("chunk_store_accounting");`
- `crates/canic-control-plane/src/storage/stable/template/chunked.rs:280:canic_core::perf!("chunk_store_insert");`
- `crates/canic-control-plane/src/storage/stable/template/chunked.rs:287:canic_core::perf!("chunk_store_accounting");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:150:canic_core::perf!("bootstrap_import_pool");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:159:canic_core::perf!("bootstrap_create_canisters");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:168:canic_core::perf!("bootstrap_rebuild_directories");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:172:canic_core::perf!("bootstrap_validate_state");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:331:canic_core::perf!("bootstrap_ensure_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:333:canic_core::perf!("bootstrap_publish_release_set");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:606:canic_core::perf!("bootstrap_create_role");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:614:canic_core::perf!("bootstrap_ensure_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:624:canic_core::perf!("bootstrap_prune_store_catalog");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:646:canic_core::perf!("bootstrap_create_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:648:canic_core::perf!("bootstrap_sync_store_inventory");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:655:canic_core::perf!("bootstrap_import_store_catalog");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication.rs:1324:canic_core::perf!("publish_prepare_store");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication.rs:1353:canic_core::perf!("publish_push_store_chunk");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication.rs:1391:canic_core::perf!("publish_promote_manifest");`
- `crates/canic-core/src/workflow/auth.rs:131:crate::perf!("issue_proof");`
- `crates/canic-core/src/workflow/auth.rs:153:crate::perf!("push_signers");`
- `crates/canic-core/src/workflow/auth.rs:166:crate::perf!("push_verifiers");`
- `crates/canic-core/src/workflow/auth.rs:277:crate::perf!("prepare_call");`
- `crates/canic-core/src/workflow/auth.rs:288:crate::perf!("execute_call");`
- `crates/canic-core/src/workflow/auth.rs:299:crate::perf!("decode_response");`
- `crates/canic-core/src/workflow/auth.rs:310:crate::perf!("finalize_result");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:36:crate::perf!("observe_state");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:44:crate::perf!("plan_spawn");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:64:crate::perf!("create_canister");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:69:crate::perf!("register_worker");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:80:crate::perf!("observe_state");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:82:crate::perf!("plan_spawn");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:118:crate::perf!("load_active_shards");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:153:crate::perf!("collect_registry");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:167:crate::perf!("plan_assign");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:171:crate::perf!("already_assigned");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:187:crate::perf!("assign_existing");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:209:crate::perf!("allocate_shard");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:212:crate::perf!("assign_created");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:224:crate::perf!("create_blocked");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:289:crate::perf!("bootstrap_empty_active");`
- `crates/canic-core/src/workflow/placement/sharding/mod.rs:292:crate::perf!("assign_bootstrap_created");`
- `crates/canic-core/src/workflow/rpc/request/handler/execute.rs:150:crate::perf!("cache_root_verifier_keys");`
- `crates/canic-core/src/workflow/rpc/request/handler/execute.rs:156:crate::perf!("cache_root_verifier_proof");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:100:crate::perf!("map_request");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:103:crate::perf!("preflight");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:122:crate::perf!("execute_capability");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:134:crate::perf!("commit_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:95:crate::perf!("extract_context");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:172:crate::perf!("commit_encode");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:181:crate::perf!("abort_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:47:crate::perf!("prepare_replay_input");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:52:crate::perf!("evaluate_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:58:crate::perf!("reserve_fresh");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:66:crate::perf!("decode_cached");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:74:crate::perf!("duplicate_in_flight");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:82:crate::perf!("duplicate_conflict");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:90:crate::perf!("replay_expired");`

## Measured Checkpoint Deltas

| Scenario | Scope | Label | Count | Total local instructions | Avg local instructions |
| --- | --- | --- | ---: | ---: | ---: |
| `user_hub:create_account:first-account` | `canic_core::workflow::placement::sharding` | `bootstrap_empty_active` | 1 | 4076258 | 4076258 |
| `scale_hub:create_worker:first-worker` | `canic_core::workflow::placement::scaling` | `create_canister` | 1 | 3888030 | 3888030 |
| `root:canic_response_capability_v1:request-cycles-fresh` | `canic_core::workflow::rpc::request::handler` | `commit_replay` | 1 | 900819 | 900819 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::auth` | `issue_proof` | 1 | 777665 | 777665 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::rpc::request::handler::execute` | `cache_root_verifier_proof` | 1 | 262205 | 262205 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::auth` | `decode_response` | 1 | 253418 | 253418 |
| `root:canic_response_capability_v1:request-cycles-fresh` | `canic_core::workflow::rpc::request::handler` | `execute_capability` | 1 | 250336 | 250336 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::rpc::request::handler` | `commit_replay` | 1 | 246419 | 246419 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::rpc::request::handler::replay` | `prepare_replay_input` | 1 | 140933 | 140933 |
| `root:canic_template_publish_chunk_admin:single-chunk` | `canic_control_plane::storage::stable::template::chunked` | `chunk_store_insert` | 1 | 139762 | 139762 |
| `root:canic_response_capability_v1:request-cycles-fresh` | `canic_core::workflow::rpc::request::handler` | `preflight` | 1 | 110494 | 110494 |
| `user_hub:create_account:first-account` | `canic_core::workflow::placement::sharding` | `assign_bootstrap_created` | 1 | 74769 | 74769 |

## Checkpoint Coverage Gaps

Critical flows with checkpoints:
- `root capability dispatch`
- `delegated auth issuance/verification`
- `replay/cached-response path`
- `sharding assignment/query flow`
- `scaling/provisioning flow`
- `bootstrap/install/publication flow`

Critical flows without checkpoints:
- none

Proposed first checkpoint insertion sites:
- none

## Structural Hotspots

| Rank | Scenario | Avg local instructions | Module pressure | Evidence |
| --- | --- | ---: | --- | --- |
| 1 | `user_hub:create_account:first-account` | 2944415 | Sharding coordinator plus `canic-core` sharding workflow | [user_hub/lib](/home/adam/projects/canic/canisters/user_hub/src/lib.rs), [sharding workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/placement/sharding/mod.rs) |
| 2 | `scale_hub:create_worker:first-worker` | 2650150 | Scaling coordinator plus `canic-core` placement workflow | [scale_hub/lib](/home/adam/projects/canic/canisters/scale_hub/src/lib.rs), [scaling workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/placement/scaling/mod.rs) |
| 3 | `root:canic_request_delegation:fresh-shard` | 2168632 | Shared runtime surface | [endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs) |

## Hub Module Pressure

- `scale_hub::create_worker` concentrates cost in the scaling coordinator surface plus `canic-core` placement workflow. That makes scaling one of the first shared instruction hot paths worth reducing.
- `user_hub::create_account` is now measurable as a real sharding update, and its first-account path is dominated by `canic-core::workflow::placement::sharding::bootstrap_empty_active`.
- `root::canic_response_capability_v1` now has measured replay/cycles stage deltas, so root capability work no longer has to be treated as an opaque endpoint total.
- `test::test` provides the current chain-key-free update floor on a non-root child canister. Drift there points back to shared runtime/update overhead rather than topology-specific logic.
- Root state/registry reads stay separate from the leaf floor. They matter for operator paths, but they should not be confused with the shared ordinary-leaf baseline.

## Dependency Fan-In Pressure

- Shared observability reads (`canic_env`, `canic_log`) are now measured through the internal `audit_leaf_probe` canister instead of the shipped demo surface, and raw time is measured through the same internal lane. Their rows still reflect actual query counters from the measured call context rather than inferred zeroes or missing query-side perf-table commits.
- The sampled non-trivial hotspot fans into `canic-core` placement orchestration (`workflow/placement/scaling`). The local `test::test` update acts as the baseline floor for update overhead on an ordinary child canister.
- Flow-stage checkpoints now exist in the scaling, sharding, auth, and replay workflows. This matrix records non-zero checkpoint deltas for sampled update scenarios, so the next optimization pass can target concrete stages instead of endpoint totals alone.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Flow checkpoint coverage present | INFO | Current repo scan found 64 `perf!` call sites under `crates/`. |
| Highest sampled endpoint currently highest-cost | WARN | `user_hub:create_account:first-account` averages 2944415 local instructions in this first baseline. |
| Baseline drift not yet available | INFO | First run of day; deltas remain `N/A` until the next comparable rerun. |

## Risk Score

Risk Score: **4 / 10**

Interpretation: query visibility and stage attribution are now working for the sampled matrix. The remaining audit risk is mostly first-run comparability (`N/A` baseline deltas) plus a few endpoint-only paths that still do not have deeper internal stage attribution, not missing coverage of the critical flows themselves.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture` | PASS | PocketIC runner completed and wrote the report plus normalized artifacts. |
| `fresh root harness profile per scenario` | PASS | Each scenario used a fresh smallest-profile root bootstrap instead of sharing one cumulative perf table. |
| `canic_metrics(MetricsKind::Perf, PageRequest { limit=512, offset=0 })` | PASS | Update scenarios were sampled before/after through persisted perf rows, and query scenarios used same-call local-only probe endpoints because query-side perf rows are not committed; normalized rows saved under `/home/adam/projects/canic/docs/audits/reports/2026-04/2026-04-05/artifacts/instruction-footprint-7/perf-rows.json`. |
| `repo checkpoint scan` | PASS | Found 64 checkpoint call sites. |
| `checkpoint delta capture` | PASS | 38 non-zero checkpoint delta rows were captured under `/home/adam/projects/canic/docs/audits/reports/2026-04/2026-04-05/artifacts/instruction-footprint-7/checkpoint-deltas.json`. |
| `query perf visibility` | PASS | All sampled query scenarios returned same-call local instruction counters through the local-only probe endpoints, which avoids relying on non-persisted query-side perf state. |
| `baseline comparison` | BLOCKED | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |

## Follow-up Actions

1. Owner boundary: `flow observability`
   Action: rerun this audit after one concrete perf change so the next report has real comparable baseline deltas instead of first-run `N/A`, and only add deeper verifier-side auth checkpoints if that endpoint-total starts to matter.
2. Owner boundary: `shared update hotspots`
   Action: compare `scale_hub::create_worker` and `user_hub::create_account` against the `test::test` update floor before/after any placement/sharding cleanup, using this report as the `0.25` baseline.
3. Owner boundary: `shared observability floor`
   Action: keep the internal standalone query probes in the matrix so shared-runtime drift does not hide behind root-only or coordinator-only endpoints.

## Report Files

- [instruction-footprint-7.md](./instruction-footprint-7.md)
- [scenario-manifest.json](artifacts/instruction-footprint-7/scenario-manifest.json)
- [perf-rows.json](artifacts/instruction-footprint-7/perf-rows.json)
- [endpoint-matrix.tsv](artifacts/instruction-footprint-7/endpoint-matrix.tsv)
- [checkpoint-deltas.json](artifacts/instruction-footprint-7/checkpoint-deltas.json)
- [flow-checkpoints.log](artifacts/instruction-footprint-7/flow-checkpoints.log)
- [checkpoint-coverage-gaps.json](artifacts/instruction-footprint-7/checkpoint-coverage-gaps.json)
- [verification-readout.md](artifacts/instruction-footprint-7/verification-readout.md)
- [method.json](artifacts/instruction-footprint-7/method.json)
- [environment.json](artifacts/instruction-footprint-7/environment.json)
