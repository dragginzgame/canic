# Instruction Footprint Audit - 2026-04-04

## Report Preamble

- Scope: Canic instruction footprint (first `0.20` baseline, partial canister scope)
- Definition path: `docs/audits/recurring/system/instruction-footprint.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `4d32edea`
- Method tag/version: `Method V1`
- Comparability status: `partial`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-04T11:27:09Z`
- Branch: `main`
- Worktree: `dirty`
- Execution environment: `PocketIC`
- Target canisters in scope: `app` `root` `scale_hub` `test`
- Target endpoints/flows in scope: `canic_env` `canic_log` `canic_subnet_registry` `canic_subnet_state` `canic_template_prepare_admin` `canic_template_publish_chunk_admin` `canic_template_stage_manifest_admin` `canic_time` `plan_create_worker` `test`
- Deferred from this baseline: `scale_hub::create_worker` and sharding assignment updates require chain-key ECDSA in PocketIC; the default root harness does not provision that key yet.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Scenario manifest recorded | PASS | `artifacts/instruction-footprint/scenario-manifest.json` captures the scenario identity tuple for every sampled endpoint. |
| Normalized perf rows recorded | PASS | `artifacts/instruction-footprint/perf-rows.json` stores canonical endpoint rows with count and total local instructions. |
| Fresh topology isolation used | PASS | Each scenario ran under a fresh `setup_root()` install instead of reusing one cumulative perf table. |
| Flow checkpoint coverage scanned | PASS | `artifacts/instruction-footprint/flow-checkpoints.log` records the current repo scan result. |
| `perf!` checkpoints available for critical flows | PASS | Current repo scan found at least one `perf!` call site. |
| Query endpoint perf visibility | PARTIAL | 6 successful query scenarios left no persisted `MetricsKind::Perf` delta; those rows are method-limited rather than true zero-cost measurements. |
| Baseline path selected by daily baseline discipline | PARTIAL | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |

## Comparison to Previous Relevant Run

- First run of day for `instruction-footprint`; this report establishes the daily baseline.
- Current query scenarios are not yet comparable through persisted `MetricsKind::Perf` rows, so this baseline should be treated as update-visible only until query accounting is widened.
- Baseline drift values are `N/A` until a same-day rerun or later comparable run exists.

## Endpoint Matrix

| Canister | Endpoint | Scenario | Count | Total local instructions | Avg local instructions | Baseline delta | Notes |
| --- | --- | --- | ---: | ---: | ---: | --- | --- |
| `app` | `canic_time` | `minimal-valid` | 0 | 0 | 0 | N/A | method-limited: successful query left no persisted perf delta |
| `app` | `canic_env` | `minimal-valid` | 0 | 0 | 0 | N/A | method-limited: successful query left no persisted perf delta |
| `app` | `canic_log` | `empty-page` | 0 | 0 | 0 | N/A | method-limited: successful query left no persisted perf delta |
| `root` | `canic_subnet_registry` | `representative-valid` | 0 | 0 | 0 | N/A | method-limited: successful query left no persisted perf delta |
| `root` | `canic_subnet_state` | `minimal-valid` | 0 | 0 | 0 | N/A | method-limited: successful query left no persisted perf delta |
| `scale_hub` | `plan_create_worker` | `empty-pool` | 0 | 0 | 0 | N/A | method-limited: successful query left no persisted perf delta |
| `test` | `test` | `minimal-valid` | 1 | 421 | 421 | N/A |  |
| `root` | `canic_template_stage_manifest_admin` | `single-chunk` | 1 | 587050 | 587050 | N/A |  |
| `root` | `canic_template_prepare_admin` | `single-chunk` | 1 | 222348 | 222348 | N/A |  |
| `root` | `canic_template_publish_chunk_admin` | `single-chunk` | 1 | 18236972 | 18236972 | N/A |  |

## Flow Checkpoints

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
| 1 | `root:canic_template_publish_chunk_admin:single-chunk` | 18236972 | Shared runtime surface | [endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs) |
| 2 | `root:canic_template_stage_manifest_admin:single-chunk` | 587050 | Shared runtime surface | [endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs) |
| 3 | `root:canic_template_prepare_admin:single-chunk` | 222348 | Shared runtime surface | [endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs) |

## Hub Module Pressure

- `scale_hub::plan_create_worker` concentrates cost in the scaling coordinator surface plus `canic-core` placement workflow. That makes scaling one of the first shared instruction hot paths worth reducing even before live worker provisioning is measurable in PocketIC.
- `test::test` provides the current chain-key-free update floor on a non-root child canister. Drift there points back to shared runtime/update overhead rather than topology-specific logic.
- Root state/registry reads stay separate from the leaf floor. They matter for operator paths, but they should not be confused with the shared ordinary-leaf baseline.

## Dependency Fan-In Pressure

- Shared lifecycle/observability endpoints (`canic_time`, `canic_env`, `canic_log`) all route through the default `start!` bundle, but the current persisted perf transport does not yet expose comparable query deltas. Their zero rows in this report are method-limited, not true zero-cost measurements.
- The sampled non-trivial hotspot fans into `canic-core` placement orchestration (`workflow/placement/scaling`). The local `test::test` update acts as the baseline floor for update overhead on an ordinary child canister.
- Flow-stage checkpoints now exist in the scaling, sharding, auth, and replay workflows, but the current sampled matrix still does not hit enough update paths to rank checkpoint-stage costs directly.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Flow checkpoint coverage present | INFO | Current repo scan found 51 `perf!` call sites under `crates/`. |
| Query endpoint deltas currently not persisted | WARN | 6 sampled query scenarios returned successfully but left no persisted `MetricsKind::Perf` delta. |
| Highest sampled endpoint currently highest-cost | WARN | `root:canic_template_publish_chunk_admin:single-chunk` averages 18236972 local instructions in this first baseline. |
| Baseline drift not yet available | INFO | First run of day; deltas remain `N/A` until the next comparable rerun. |

## Risk Score

Risk Score: **6 / 10**

Interpretation: the main current risk is observability incompleteness rather than one measured endpoint spike. The first baseline is good enough to rank entrypoints, but not yet good enough to localize flow stages.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture` | PASS | PocketIC runner completed and wrote the report plus normalized artifacts. |
| `setup_root() per scenario` | PASS | Each scenario used a fresh root bootstrap instead of sharing one cumulative perf table. |
| `canic_metrics(MetricsKind::Perf, PageRequest { limit=512, offset=0 })` | PASS | Perf rows were sampled before and after each scenario; normalized rows saved under `/home/adam/projects/canic/docs/audits/reports/2026-04/2026-04-04/artifacts/instruction-footprint/perf-rows.json`. |
| `repo checkpoint scan` | PASS | Found 51 checkpoint call sites. |
| `query perf visibility` | PARTIAL | 6 successful query scenarios left no persisted `MetricsKind::Perf` delta; they are treated as method-limited rather than zero-cost. |
| `baseline comparison` | BLOCKED | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |

## Follow-up Actions

1. Owner boundary: `flow observability`
   Action: extend the audit matrix with update scenarios that actually traverse the new scaling, sharding, replay, and auth checkpoints so the next rerun can rank stage-level costs instead of just scan-site presence.
2. Owner boundary: `shared update hotspots`
   Action: compare `scale_hub::plan_create_worker` against the `test::test` update floor before/after any placement-runtime cleanup, using this report as the `0.20` baseline.
3. Owner boundary: `shared observability floor`
   Action: keep `app` query surfaces in the matrix so shared-runtime drift does not hide behind root-only or coordinator-only endpoints.

## Report Files

- [instruction-footprint.md](./instruction-footprint.md)
- [scenario-manifest.json](artifacts/instruction-footprint/scenario-manifest.json)
- [perf-rows.json](artifacts/instruction-footprint/perf-rows.json)
- [endpoint-matrix.tsv](artifacts/instruction-footprint/endpoint-matrix.tsv)
- [flow-checkpoints.log](artifacts/instruction-footprint/flow-checkpoints.log)
- [checkpoint-coverage-gaps.json](artifacts/instruction-footprint/checkpoint-coverage-gaps.json)
- [verification-readout.md](artifacts/instruction-footprint/verification-readout.md)
- [method.json](artifacts/instruction-footprint/method.json)
- [environment.json](artifacts/instruction-footprint/environment.json)
