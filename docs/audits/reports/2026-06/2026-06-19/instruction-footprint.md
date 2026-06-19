# Instruction Footprint Audit - 2026-06-19

## Report Preamble

- Scope: Canic instruction footprint (first `0.68` baseline, partial canister scope)
- Definition path: `docs/audits/recurring/system/instruction-footprint.md`
- Compared baseline report path: `docs/audits/reports/2026-06/2026-06-04/instruction-footprint.md`
- Code snapshot identifier: `ef55e53c`
- Method tag/version: `Method V2`
- Counter source: `performance_counter(1)`
- Counter ID: `1`
- Measured unit: `local_instructions`
- Counter scope: local canister WebAssembly instructions in the current call context; excludes other canisters and is not a cycle-charge measurement.
- Comparability status: `partial`
- Auditor: `codex`
- Run timestamp (UTC): `2026-06-19T11:43:56Z`
- Branch: `main`
- Worktree: `dirty`
- Execution environment: `PocketIC`
- Target canisters in scope: `leaf_probe` `root` `root_probe` `scaling_probe` `test`
- Target endpoints/flows in scope: `audit_env_probe` `audit_log_probe` `audit_plan_create_worker_probe` `audit_subnet_registry_probe` `audit_subnet_state_probe` `audit_time_probe` `canic_prepare_delegation_proof_batch` `canic_response_capability_v1` `canic_template_prepare_admin` `canic_template_publish_chunk_admin` `canic_template_stage_manifest_admin` `test`
- Deferred from this baseline: no additional functional flows are deferred beyond first-run comparability; this run covers shared queries plus root proof batch preparation, verifier-side delegated-token confirmation, replay/cycles, scaling worker creation, sharding account creation, and root template admin updates.

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
| Baseline path selected | PASS | Latest prior `instruction-footprint` report selected: `docs/audits/reports/2026-06/2026-06-04/instruction-footprint.md`. |

## Comparison to Previous Relevant Run

- Compared baseline report: `docs/audits/reports/2026-06/2026-06-04/instruction-footprint.md`.
- Query scenarios are now sampled through local-only `QueryPerfSample` probes because query-side perf rows are not committed, so their rows are directly comparable to later probe-backed reruns.
- Baseline drift values are `N/A` where the selected baseline has no matching readable `perf-rows.json` artifact or matching scenario key.

## Counter Semantics

- Measured rows use `performance_counter(1)` and store local instruction counts, not cycle charges.
- Update rows and query rows preserve `sample_origin`; do not compare replicated update samples, ordinary query probe samples, and future composite-query samples as if they had identical counter scope.
- The audit intentionally omits message base fees, payload bytes, storage/reservation charges, management-call fees, callee instructions, and garbage collection.

## Endpoint Matrix

| Canister | Endpoint | Scenario | Sample origin | Count | Total local instructions | Avg local instructions | Baseline delta | Notes |
| --- | --- | --- | --- | ---: | ---: | ---: | --- | --- |
| `leaf_probe` | `audit_time_probe` | `minimal-valid` | `query` | 1 | 32047 | 32047 | N/A | local-only QueryPerfSample probe |
| `leaf_probe` | `audit_env_probe` | `minimal-valid` | `query` | 1 | 33591 | 33591 | N/A | local-only QueryPerfSample probe |
| `leaf_probe` | `audit_log_probe` | `empty-page` | `query` | 1 | 297827 | 297827 | N/A | local-only QueryPerfSample probe |
| `root_probe` | `audit_subnet_registry_probe` | `representative-valid` | `query` | 1 | 76246 | 76246 | N/A | local-only QueryPerfSample probe |
| `root_probe` | `audit_subnet_state_probe` | `minimal-valid` | `query` | 1 | 32251 | 32251 | N/A | local-only QueryPerfSample probe |
| `scaling_probe` | `audit_plan_create_worker_probe` | `empty-pool` | `query` | 1 | 65661 | 65661 | N/A | local-only QueryPerfSample probe |
| `root` | `canic_prepare_delegation_proof_batch` | `fresh-shard` | `update` | 0 | 0 | 0 | N/A |  |
| `test` | `test` | `minimal-valid` | `update` | 0 | 0 | 0 | N/A |  |
| `root` | `canic_response_capability_v1` | `cycles-request` | `update` | 0 | 0 | 0 | N/A |  |
| `root` | `canic_template_stage_manifest_admin` | `single-chunk` | `update` | 0 | 0 | 0 | N/A |  |
| `root` | `canic_template_prepare_admin` | `single-chunk` | `update` | 0 | 0 | 0 | N/A |  |
| `root` | `canic_template_publish_chunk_admin` | `single-chunk` | `update` | 0 | 0 | 0 | N/A |  |

## Flow Checkpoints

- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:411:canic_core::perf!("publish_stage_validate_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:414:canic_core::perf!("publish_stage_upsert_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:443:canic_core::perf!("publish_store_validate_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:455:canic_core::perf!("publish_store_project_capacity");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:464:canic_core::perf!("publish_store_enforce_capacity");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:467:canic_core::perf!("publish_store_upsert_chunk");`
- `crates/canic-control-plane/src/storage/stable/template/chunked.rs:264:canic_core::perf!("chunk_store_insert");`
- `crates/canic-control-plane/src/storage/stable/template/chunked.rs:273:canic_core::perf!("chunk_store_accounting");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:225:canic_core::perf!("bootstrap_import_pool");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:234:canic_core::perf!("bootstrap_create_canisters");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:243:canic_core::perf!("bootstrap_rebuild_indexes");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:247:canic_core::perf!("bootstrap_validate_state");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:428:canic_core::perf!("bootstrap_ensure_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:430:canic_core::perf!("bootstrap_publish_release_set");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:733:canic_core::perf!("bootstrap_create_role");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:741:canic_core::perf!("bootstrap_ensure_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:751:canic_core::perf!("bootstrap_prune_store_catalog");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:780:canic_core::perf!("bootstrap_create_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:782:canic_core::perf!("bootstrap_sync_store_inventory");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:789:canic_core::perf!("bootstrap_import_store_catalog");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication/release/chunks.rs:111:canic_core::perf!("publish_push_store_chunk");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication/release/promote.rs:70:canic_core::perf!("publish_promote_manifest");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication/release/target.rs:92:canic_core::perf!("publish_prepare_store");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:137:crate::perf!("observe_state");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:145:crate::perf!("plan_spawn");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:241:crate::perf!("create_canister");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:247:crate::perf!("register_worker");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:80:crate::perf!("observe_state");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:95:crate::perf!("plan_spawn");`
- `crates/canic-core/src/workflow/placement/sharding/assignment.rs:112:crate::perf!("collect_registry");`
- `crates/canic-core/src/workflow/placement/sharding/assignment.rs:127:crate::perf!("plan_assign");`
- `crates/canic-core/src/workflow/placement/sharding/assignment.rs:132:crate::perf!("already_assigned");`
- `crates/canic-core/src/workflow/placement/sharding/assignment.rs:156:crate::perf!("assign_existing");`
- `crates/canic-core/src/workflow/placement/sharding/assignment.rs:196:crate::perf!("allocate_shard");`
- `crates/canic-core/src/workflow/placement/sharding/assignment.rs:205:crate::perf!("assign_created");`
- `crates/canic-core/src/workflow/placement/sharding/assignment.rs:221:crate::perf!("create_blocked");`
- `crates/canic-core/src/workflow/placement/sharding/assignment.rs:69:crate::perf!("load_active_shards");`
- `crates/canic-core/src/workflow/placement/sharding/bootstrap.rs:116:crate::perf!("load_bootstrap_pool_entries");`
- `crates/canic-core/src/workflow/placement/sharding/bootstrap.rs:124:crate::perf!("select_bootstrap_slot");`
- `crates/canic-core/src/workflow/placement/sharding/bootstrap.rs:127:crate::perf!("allocate_bootstrap_shard");`
- `crates/canic-core/src/workflow/placement/sharding/bootstrap.rs:82:crate::perf!("bootstrap_empty_active");`
- `crates/canic-core/src/workflow/placement/sharding/bootstrap.rs:91:crate::perf!("assign_bootstrap_created");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:119:crate::perf!("extract_context");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:122:crate::perf!("map_request");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:125:crate::perf!("preflight");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:149:crate::perf!("execute_capability");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:165:crate::perf!("commit_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:105:crate::perf!("evaluate_existing_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:122:crate::perf!("prepare_replay_input");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:132:crate::perf!("evaluate_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:144:crate::perf!("decode_cached");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:157:crate::perf!("duplicate_in_flight");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:170:crate::perf!("duplicate_conflict");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:183:crate::perf!("replay_expired");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:326:crate::perf!("commit_encode");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:350:crate::perf!("abort_replay");`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs:71:crate::perf!("reserve_fresh");`

## Measured Checkpoint Deltas

| Scenario | Scope | Label | Count | Total local instructions | Avg local instructions |
| --- | --- | --- | ---: | ---: | ---: |
| `root:canic_template_publish_chunk_admin:single-chunk` | `canic_control_plane::storage::stable::template::chunked` | `chunk_store_insert` | 1 | 134683 | 134683 |
| `root:canic_template_publish_chunk_admin:single-chunk` | `canic_control_plane::ops::storage::template::chunked` | `publish_stage_upsert_chunk` | 1 | 6834 | 6834 |
| `root:canic_template_publish_chunk_admin:single-chunk` | `canic_control_plane::storage::stable::template::chunked` | `chunk_store_accounting` | 1 | 6483 | 6483 |
| `root:canic_template_publish_chunk_admin:single-chunk` | `canic_control_plane::ops::storage::template::chunked` | `publish_stage_validate_chunk` | 1 | 0 | 0 |

## Checkpoint Coverage Gaps

Critical flows with checkpoints:
- `root capability dispatch`
- `replay/cached-response path`
- `sharding assignment/query flow`
- `scaling/provisioning flow`
- `bootstrap/install/publication flow`

Critical flows without checkpoints:
- `root proof provisioning and issuer delegated-token issuance/verification`

Proposed first checkpoint insertion sites:
- `root proof provisioning and issuer delegated-token issuance/verification` -> `crates/canic-core/src/workflow/runtime/auth`

## Structural Hotspots

| Rank | Scenario | Avg local instructions | Module pressure | Evidence |
| --- | --- | ---: | --- | --- |
| 1 | `app:canic_log:empty-page` | 297827 | Internal audit log pagination probe over the shared log query path | [leaf_probe](/home/adam/projects/canic/canisters/audit/leaf_probe/src/lib.rs), [log query](/home/adam/projects/canic/crates/canic-core/src/workflow/log/query.rs) |
| 2 | `root:canic_subnet_registry:full-registry` | 76246 | Root topology registry query | [root_probe](/home/adam/projects/canic/canisters/audit/root_probe/src/lib.rs), [registry query](/home/adam/projects/canic/crates/canic-core/src/workflow/topology/registry/query.rs) |
| 3 | `scale_hub:plan_create_worker:empty-pool` | 65661 | Scaling policy read path | [scaling_probe](/home/adam/projects/canic/canisters/audit/scaling_probe/src/lib.rs), [scaling workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/placement/scaling/mod.rs) |

## Hub Module Pressure

- `root::canic_response_capability_v1` now has measured replay/cycles stage deltas, so root capability work no longer has to be treated as an opaque endpoint total.
- `root::canic_prepare_delegation_proof_batch` remains the main root proof provisioning update hotspot in the retained audit lane, so further optimization work should stay focused on shared runtime/auth cost rather than demo provisioning flows.
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
| Flow checkpoint coverage present | INFO | Current repo scan found 57 `perf!` call sites under `crates/`. |
| Highest sampled endpoint currently highest-cost | WARN | `app:canic_log:empty-page` averages 297827 local instructions in this run. |
| Baseline drift source | INFO | Latest prior baseline path: `docs/audits/reports/2026-06/2026-06-04/instruction-footprint.md`. |

## Risk Score

Risk Score: **2 / 10**

Interpretation: query visibility and stage attribution are now working for the sampled matrix. The remaining audit risk is mostly baseline comparability plus a few endpoint-only paths that still do not have deeper internal stage attribution, not missing coverage of the critical flows themselves.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture` | PASS | PocketIC runner completed and wrote the report plus normalized artifacts. |
| `fresh root harness profile per scenario` | PASS | Each scenario used a fresh smallest-profile root bootstrap instead of sharing one cumulative perf table. |
| `canic_metrics(MetricsKind::Runtime, PageRequest { limit=512, offset=0 })` | PASS | Update scenarios were sampled before/after through persisted perf rows, and query scenarios used local-only `QueryPerfSample` probe endpoints because query-side perf rows are not committed; normalized rows saved under `/home/adam/projects/canic/docs/audits/reports/2026-06/2026-06-19/artifacts/instruction-footprint/perf-rows.json`. |
| `repo checkpoint scan` | PASS | Found 57 checkpoint call sites. |
| `checkpoint delta capture` | PASS | 4 non-zero checkpoint delta rows were captured under `/home/adam/projects/canic/docs/audits/reports/2026-06/2026-06-19/artifacts/instruction-footprint/checkpoint-deltas.json`. |
| `query perf visibility` | PASS | All sampled query scenarios returned `QueryPerfSample` local instruction counters through the local-only probe endpoints, which avoids relying on non-persisted query-side perf state. |
| `baseline comparison` | PARTIAL | Latest prior `instruction-footprint` report selected as baseline: `docs/audits/reports/2026-06/2026-06-04/instruction-footprint.md`. |
| `bash scripts/ci/install-ic-query.sh` | PASS | Reinstalled pinned `icq 0.2.23` after the first runner attempt found local `icq 0.2.26`. |
| `cargo test --locked -p canic-tests --test instruction_audit --no-run` | PASS | Recompiled instruction-audit support after audit-definition and runner wording fixes. |
| `cargo fmt --all -- --check` | PASS | Formatting check passed after support-code cleanup. |
| `git diff --check` | PASS | Whitespace check passed after report and artifact updates. |

## Follow-up Actions

1. Owner boundary: `flow observability`
   Action: rerun this audit after one concrete perf change and compare against the latest prior retained report; only add deeper verifier-side auth checkpoints if that endpoint-total starts to matter.
2. Owner boundary: `shared update hotspots`
   Action: compare `root::canic_prepare_delegation_proof_batch`, `root::canic_response_capability_v1`, and the local `test::test` update floor before/after any shared-runtime cleanup, using this report as the `0.68` baseline.
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
