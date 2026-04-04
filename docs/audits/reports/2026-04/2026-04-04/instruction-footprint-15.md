# Instruction Footprint Audit - 2026-04-04

## Report Preamble

- Scope: Canic instruction footprint (first `0.24` baseline, partial canister scope)
- Definition path: `docs/audits/recurring/system/instruction-footprint.md`
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-04/instruction-footprint-14.md`
- Code snapshot identifier: `4d32edea`
- Method tag/version: `Method V1`
- Comparability status: `partial`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-04T13:47:20Z`
- Branch: `main`
- Worktree: `dirty`
- Execution environment: `PocketIC`
- Target canisters in scope: `app` `root` `scale_hub` `test` `user_hub`
- Target endpoints/flows in scope: `canic_env` `canic_log` `canic_request_delegation` `canic_response_capability_v1` `canic_subnet_registry` `canic_subnet_state` `canic_template_prepare_admin` `canic_template_publish_chunk_admin` `canic_template_stage_manifest_admin` `canic_time` `create_account` `create_worker` `plan_create_worker` `test` `test_verify_delegated_token`
- Deferred from this baseline: no additional functional flows are deferred beyond first-run comparability; this run covers shared queries plus delegated auth issuance, verifier confirmation, replay/cycles, scaling worker creation, sharding account creation, and root template admin updates.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Scenario manifest recorded | PASS | `artifacts/instruction-footprint-15/scenario-manifest.json` captures the scenario identity tuple for every sampled endpoint. |
| Normalized perf rows recorded | PASS | `artifacts/instruction-footprint-15/perf-rows.json` stores canonical endpoint rows with count and total local instructions. |
| Checkpoint deltas recorded | PASS | `artifacts/instruction-footprint-15/checkpoint-deltas.json` stores non-zero per-scenario checkpoint rows. |
| Fresh topology isolation used | PASS | Each scenario ran under a fresh `setup_root()` install instead of reusing one cumulative perf table. |
| Flow checkpoint coverage scanned | PASS | `artifacts/instruction-footprint-15/flow-checkpoints.log` records the current repo scan result. |
| `perf!` checkpoints available for critical flows | PASS | Current repo scan found at least one `perf!` call site. |
| Query endpoint perf visibility | PASS | Sampled query scenarios were measured through same-call local-only perf probe endpoints. |
| Baseline path selected by daily baseline discipline | PARTIAL | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |

## Comparison to Previous Relevant Run

- First run of day for `instruction-footprint`; this report establishes the daily baseline.
- Query scenarios are now sampled through same-call local-only perf probes, so their rows are directly comparable to later probe-backed reruns.
- Baseline drift values are `N/A` until a same-day rerun or later comparable run exists.

## Endpoint Matrix

| Canister | Endpoint | Scenario | Count | Total local instructions | Avg local instructions | Baseline delta | Notes |
| --- | --- | --- | ---: | ---: | ---: | --- | --- |
| `app` | `canic_time` | `minimal-valid` | 1 | 19818 | 19818 | N/A | same-call local-only perf probe |
| `app` | `canic_env` | `minimal-valid` | 1 | 21418 | 21418 | N/A | same-call local-only perf probe |
| `app` | `canic_log` | `empty-page` | 1 | 359325 | 359325 | N/A | same-call local-only perf probe |
| `root` | `canic_subnet_registry` | `representative-valid` | 1 | 180689 | 180689 | N/A | same-call local-only perf probe |
| `root` | `canic_subnet_state` | `minimal-valid` | 1 | 20895 | 20895 | N/A | same-call local-only perf probe |
| `scale_hub` | `plan_create_worker` | `empty-pool` | 1 | 56253 | 56253 | N/A | same-call local-only perf probe |
| `scale_hub` | `create_worker` | `empty-pool` | 1 | 2650463 | 2650463 | N/A |  |
| `user_hub` | `create_account` | `new-principal` | 1 | 2955954 | 2955954 | N/A |  |
| `root` | `canic_request_delegation` | `fresh-shard` | 1 | 5516827 | 5516827 | N/A |  |
| `test` | `test_verify_delegated_token` | `valid-delegated-token` | 1 | 4191 | 4191 | N/A |  |
| `test` | `test` | `minimal-valid` | 1 | 421 | 421 | N/A |  |
| `root` | `canic_response_capability_v1` | `cycles-request` | 1 | 2275000 | 2275000 | N/A |  |
| `root` | `canic_template_stage_manifest_admin` | `single-chunk` | 1 | 587029 | 587029 | N/A |  |
| `root` | `canic_template_prepare_admin` | `single-chunk` | 1 | 221740 | 221740 | N/A |  |
| `root` | `canic_template_publish_chunk_admin` | `single-chunk` | 1 | 391757 | 391757 | N/A |  |

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
- `crates/canic-core/src/workflow/auth.rs:125:crate::perf!("issue_proof");`
- `crates/canic-core/src/workflow/auth.rs:146:crate::perf!("push_signers");`
- `crates/canic-core/src/workflow/auth.rs:158:crate::perf!("push_verifiers");`
- `crates/canic-core/src/workflow/auth.rs:252:crate::perf!("prepare_call");`
- `crates/canic-core/src/workflow/auth.rs:263:crate::perf!("execute_call");`
- `crates/canic-core/src/workflow/auth.rs:274:crate::perf!("decode_response");`
- `crates/canic-core/src/workflow/auth.rs:285:crate::perf!("finalize_result");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:36:crate::perf!("observe_state");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:44:crate::perf!("plan_spawn");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:64:crate::perf!("create_canister");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:69:crate::perf!("register_worker");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:80:crate::perf!("observe_state");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:82:crate::perf!("plan_spawn");`
- `crates/canic-core/src/workflow/rpc/request/handler/execute.rs:137:crate::perf!("cache_root_verifier_keys");`
- `crates/canic-core/src/workflow/rpc/request/handler/execute.rs:139:crate::perf!("cache_root_verifier_proof");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:108:crate::perf!("execute_capability");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:120:crate::perf!("commit_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:83:crate::perf!("extract_context");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:88:crate::perf!("map_request");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:91:crate::perf!("preflight");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:162:crate::perf!("commit_encode");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:171:crate::perf!("abort_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:48:crate::perf!("prepare_replay_input");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:53:crate::perf!("evaluate_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:59:crate::perf!("reserve_fresh");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:67:crate::perf!("decode_cached");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:75:crate::perf!("duplicate_in_flight");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:83:crate::perf!("duplicate_conflict");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:91:crate::perf!("replay_expired");`
- `crates/canic-sharding-runtime/src/workflow/mod.rs:123:canic_core::perf!("load_active_shards");`
- `crates/canic-sharding-runtime/src/workflow/mod.rs:134:canic_core::perf!("bootstrap_empty_active");`
- `crates/canic-sharding-runtime/src/workflow/mod.rs:160:canic_core::perf!("collect_registry");`
- `crates/canic-sharding-runtime/src/workflow/mod.rs:174:canic_core::perf!("plan_assign");`
- `crates/canic-sharding-runtime/src/workflow/mod.rs:178:canic_core::perf!("already_assigned");`
- `crates/canic-sharding-runtime/src/workflow/mod.rs:194:canic_core::perf!("assign_existing");`
- `crates/canic-sharding-runtime/src/workflow/mod.rs:216:canic_core::perf!("allocate_shard");`
- `crates/canic-sharding-runtime/src/workflow/mod.rs:219:canic_core::perf!("assign_created");`
- `crates/canic-sharding-runtime/src/workflow/mod.rs:231:canic_core::perf!("create_blocked");`

## Measured Checkpoint Deltas

| Scenario | Scope | Label | Count | Total local instructions | Avg local instructions |
| --- | --- | --- | ---: | ---: | ---: |
| `user_hub:create_account:first-account` | `canic_sharding_runtime::workflow` | `bootstrap_empty_active` | 1 | 3995647 | 3995647 |
| `scale_hub:create_worker:first-worker` | `canic_core::workflow::placement::scaling` | `create_canister` | 1 | 3878402 | 3878402 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::rpc::request::handler` | `commit_replay` | 1 | 1151412 | 1151412 |
| `root:canic_response_capability_v1:request-cycles-fresh` | `canic_core::workflow::rpc::request::handler` | `commit_replay` | 1 | 1024239 | 1024239 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::auth` | `issue_proof` | 1 | 908828 | 908828 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::rpc::request::handler::replay` | `prepare_replay_input` | 1 | 778701 | 778701 |
| `root:canic_response_capability_v1:request-cycles-fresh` | `canic_core::workflow::rpc::request::handler::replay` | `prepare_replay_input` | 1 | 673186 | 673186 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::auth` | `prepare_call` | 2 | 644116 | 322058 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::auth` | `decode_response` | 2 | 519840 | 259920 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::rpc::request::handler::execute` | `cache_root_verifier_keys` | 1 | 421478 | 421478 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::rpc::request::handler::replay` | `evaluate_replay` | 1 | 378896 | 378896 |
| `root:canic_request_delegation:fresh-shard` | `canic_core::workflow::rpc::request::handler::execute` | `cache_root_verifier_proof` | 1 | 264876 | 264876 |

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
| 1 | `root:canic_request_delegation:fresh-shard` | 5516827 | Shared runtime surface | [endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs) |
| 2 | `user_hub:create_account:first-account` | 2955954 | Sharding coordinator plus `canic-sharding-runtime` workflow | [user_hub/lib](/home/adam/projects/canic/canisters/user_hub/src/lib.rs), [sharding workflow](/home/adam/projects/canic/crates/canic-sharding-runtime/src/workflow/mod.rs) |
| 3 | `scale_hub:create_worker:first-worker` | 2650463 | Scaling coordinator plus `canic-core` placement workflow | [scale_hub/lib](/home/adam/projects/canic/canisters/scale_hub/src/lib.rs), [scaling workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/placement/scaling/mod.rs) |

## Hub Module Pressure

- `scale_hub::create_worker` concentrates cost in the scaling coordinator surface plus `canic-core` placement workflow. That makes scaling one of the first shared instruction hot paths worth reducing.
- `user_hub::create_account` is now measurable as a real sharding update, and its first-account path is dominated by `canic-sharding-runtime::workflow::bootstrap_empty_active`.
- `root::canic_response_capability_v1` now has measured replay/cycles stage deltas, so root capability work no longer has to be treated as an opaque endpoint total.
- `test::test` provides the current chain-key-free update floor on a non-root child canister. Drift there points back to shared runtime/update overhead rather than topology-specific logic.
- Root state/registry reads stay separate from the leaf floor. They matter for operator paths, but they should not be confused with the shared ordinary-leaf baseline.

## Dependency Fan-In Pressure

- Shared lifecycle/observability endpoints (`canic_time`, `canic_env`, `canic_log`) all route through the default `start!` bundle, and this matrix now samples them through same-call local-only perf probes. Their rows reflect actual query counters from the measured call context rather than inferred zeroes.
- The sampled non-trivial hotspot fans into `canic-core` placement orchestration (`workflow/placement/scaling`). The local `test::test` update acts as the baseline floor for update overhead on an ordinary child canister.
- Flow-stage checkpoints now exist in the scaling, sharding, auth, and replay workflows. This matrix records non-zero checkpoint deltas for sampled update scenarios, so the next optimization pass can target concrete stages instead of endpoint totals alone.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Flow checkpoint coverage present | INFO | Current repo scan found 63 `perf!` call sites under `crates/`. |
| Highest sampled endpoint currently highest-cost | WARN | `root:canic_request_delegation:fresh-shard` averages 5516827 local instructions in this first baseline. |
| Baseline drift not yet available | INFO | First run of day; deltas remain `N/A` until the next comparable rerun. |

## Risk Score

Risk Score: **4 / 10**

Interpretation: query visibility and stage attribution are now working for the sampled matrix. The remaining audit risk is mostly first-run comparability (`N/A` baseline deltas) plus a few endpoint-only paths that still do not have deeper internal stage attribution, not missing coverage of the critical flows themselves.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture` | PASS | PocketIC runner completed and wrote the report plus normalized artifacts. |
| `setup_root() per scenario` | PASS | Each scenario used a fresh root bootstrap instead of sharing one cumulative perf table. |
| `canic_metrics(MetricsKind::Perf, PageRequest { limit=512, offset=0 })` | PASS | Update scenarios were sampled before/after through persisted perf rows, and query scenarios used same-call local-only probe endpoints; normalized rows saved under `/home/adam/projects/canic/docs/audits/reports/2026-04/2026-04-04/artifacts/instruction-footprint-15/perf-rows.json`. |
| `repo checkpoint scan` | PASS | Found 63 checkpoint call sites. |
| `checkpoint delta capture` | PASS | 40 non-zero checkpoint delta rows were captured under `/home/adam/projects/canic/docs/audits/reports/2026-04/2026-04-04/artifacts/instruction-footprint-15/checkpoint-deltas.json`. |
| `query perf visibility` | PASS | All sampled query scenarios returned same-call local instruction counters through the local-only probe endpoints. |
| `baseline comparison` | BLOCKED | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |

## Follow-up Actions

1. Owner boundary: `flow observability`
   Action: rerun this audit after one concrete perf change so the next report has real comparable baseline deltas instead of first-run `N/A`, and only add deeper verifier-side auth checkpoints if that endpoint-total starts to matter.
2. Owner boundary: `shared update hotspots`
   Action: compare `scale_hub::create_worker` and `user_hub::create_account` against the `test::test` update floor before/after any placement/sharding-runtime cleanup, using this report as the `0.24` baseline.
3. Owner boundary: `shared observability floor`
   Action: keep `app` query surfaces in the matrix so shared-runtime drift does not hide behind root-only or coordinator-only endpoints.

## Report Files

- [instruction-footprint-15.md](./instruction-footprint-15.md)
- [scenario-manifest.json](artifacts/instruction-footprint-15/scenario-manifest.json)
- [perf-rows.json](artifacts/instruction-footprint-15/perf-rows.json)
- [endpoint-matrix.tsv](artifacts/instruction-footprint-15/endpoint-matrix.tsv)
- [checkpoint-deltas.json](artifacts/instruction-footprint-15/checkpoint-deltas.json)
- [flow-checkpoints.log](artifacts/instruction-footprint-15/flow-checkpoints.log)
- [checkpoint-coverage-gaps.json](artifacts/instruction-footprint-15/checkpoint-coverage-gaps.json)
- [verification-readout.md](artifacts/instruction-footprint-15/verification-readout.md)
- [method.json](artifacts/instruction-footprint-15/method.json)
- [environment.json](artifacts/instruction-footprint-15/environment.json)
