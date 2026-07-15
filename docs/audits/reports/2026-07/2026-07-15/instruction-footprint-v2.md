# Instruction Footprint Audit - 2026-07-15

## Report Preamble

- Scope: Canic instruction footprint (fixed `0.92` v2 update/install roster)
- Definition path: `docs/audits/recurring/system/instruction-footprint.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `91736337fc1cfeb891f17d7d62affb5e671348e2`
- Method tag/version: `CANIC-INSTRUCTION-001/v2`
- Audit method ID: `CANIC-INSTRUCTION-001`
- Audit method version: `2`
- Audit method fingerprint: `385ea065d337781828a10a9167948309d9bafb9e126434142aeb0104eacfc584`
- Counter source: `performance_counter(1)`
- Counter ID: `1`
- Measured unit: `local_instructions`
- Counter scope: local canister WebAssembly instructions in the current call context; excludes other canisters and is not a cycle-charge measurement.
- Result validity: `valid`
- Run result: `partial`
- Comparability status: `first-v2-baseline`
- Auditor: `codex`
- Run timestamp (UTC): `2026-07-15T09:02:02Z`
- Branch: `main`
- Worktree: `dirty`
- Execution environment: `PocketIC`
- Target canisters in scope: `issuer` `root` `scale` `scale_hub` `test` `user_hub`
- Target endpoints/flows in scope: `canic_prepare_delegated_token` `canic_response_capability_v1` `canic_template_prepare_admin` `canic_template_publish_chunk_admin` `canic_template_stage_manifest_admin` `create_account` `create_worker` `request_cycles_from_parent` `root_bootstrap_init` `test_provision_chain_key_delegation_proof_for_issuer` `test_verify_delegated_token`
- Deferred from this baseline: query instruction totals require a future authoritative same-call fixture and method version. The fixed roster covers root capability/replay, root-proof provisioning, issuer prepare, verifier confirmation, scaling, sharding, publication, and root bootstrap.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Scenario manifest recorded | PASS | `artifacts/instruction-footprint-v2/scenario-manifest.json` captures the scenario identity tuple for every sampled endpoint. |
| Normalized perf rows recorded | PASS | `artifacts/instruction-footprint-v2/perf-rows.json` stores canonical endpoint rows with count and total local instructions. |
| Zero exclusive endpoint totals interpreted | PASS | 1 measured row(s) have `count > 0` and a zero exclusive total because nested/checkpoint scopes retain the attributed work; these are measured calls, not missing samples. |
| Checkpoint deltas recorded | PASS | `artifacts/instruction-footprint-v2/checkpoint-deltas.json` stores non-zero per-scenario checkpoint rows. |
| Fresh topology isolation used | PASS | Each scenario ran under a fresh smallest-profile root harness install instead of reusing one cumulative perf table. |
| Flow checkpoint coverage scanned | PASS | The Flow Checkpoints section records the current repo scan result. |
| `perf!` checkpoints available for critical flows | PASS | Current repo scan found at least one `perf!` call site. |
| Authoritative fixture build | PASS | Every scenario uses the root harness and Canic-validated `build_artifact` path; no direct Cargo probe build remains. |
| Baseline path selected | PASS | No comparable v2 report exists; this run establishes the first v2 baseline and deltas are `N/A`. |

## Comparison to Previous Relevant Run

- No previous `instruction-footprint` report was available; this report establishes the first retained baseline.
- V1 query-probe rows never executed and are not a baseline. V2 hard-cuts those direct-build probes; future query measurement needs a separately versioned authoritative same-call fixture.
- Baseline drift values are `N/A` until a prior comparable run exists.

## Counter Semantics

- Measured rows use `performance_counter(1)` and store local instruction counts, not cycle charges.
- Update and install checkpoint-group rows preserve `sample_origin` and are never compared as the same accounting shape.
- The audit intentionally omits message base fees, payload bytes, storage/reservation charges, management-call fees, callee instructions, and garbage collection.

## Endpoint Matrix

| Canister | Endpoint | Scenario | Sample origin | Count | Total local instructions | Avg local instructions | Baseline delta | Notes |
| --- | --- | --- | --- | ---: | ---: | ---: | --- | --- |
| `scale` | `request_cycles_from_parent` | `cycles-999` | `update` | 1 | 795628 | 795628 | N/A |  |
| `scale_hub` | `create_worker` | `empty-pool` | `update` | 1 | 0 | 0 | N/A |  |
| `user_hub` | `create_account` | `new-principal` | `update` | 1 | 339598 | 339598 | N/A |  |
| `root` | `test_provision_chain_key_delegation_proof_for_issuer` | `registered-new-issuer` | `update` | 1 | 16442775 | 16442775 | N/A |  |
| `issuer` | `canic_prepare_delegated_token` | `minimal-valid` | `update` | 1 | 1195696 | 1195696 | N/A |  |
| `test` | `test_verify_delegated_token` | `valid-delegated-token` | `update` | 1 | 11688 | 11688 | N/A |  |
| `root` | `canic_response_capability_v1` | `cycles-request` | `update` | 1 | 2117578 | 2117578 | N/A |  |
| `root` | `canic_response_capability_v1` | `cycles-request` | `update` | 1 | 391107 | 391107 | N/A |  |
| `root` | `canic_template_stage_manifest_admin` | `single-chunk` | `update` | 1 | 577854 | 577854 | N/A |  |
| `root` | `canic_template_prepare_admin` | `single-chunk` | `update` | 1 | 227473 | 227473 | N/A |  |
| `root` | `canic_template_publish_chunk_admin` | `single-chunk` | `update` | 1 | 417952 | 417952 | N/A |  |
| `root` | `root_bootstrap_init` | `topology-profile` | `install` | 1 | 4214023135 | 4214023135 | N/A | sum of retained bootstrap checkpoint deltas |

## Flow Checkpoints

- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:422:canic_core::perf!("publish_stage_validate_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:425:canic_core::perf!("publish_stage_upsert_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:454:canic_core::perf!("publish_store_validate_chunk");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:466:canic_core::perf!("publish_store_project_capacity");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:475:canic_core::perf!("publish_store_enforce_capacity");`
- `crates/canic-control-plane/src/ops/storage/template/chunked.rs:478:canic_core::perf!("publish_store_upsert_chunk");`
- `crates/canic-control-plane/src/storage/stable/template/chunked.rs:384:canic_core::perf!("chunk_store_insert");`
- `crates/canic-control-plane/src/storage/stable/template/chunked.rs:393:canic_core::perf!("chunk_store_accounting");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:236:canic_core::perf!("bootstrap_import_pool");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:245:canic_core::perf!("bootstrap_create_canisters");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:254:canic_core::perf!("bootstrap_rebuild_indexes");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:258:canic_core::perf!("bootstrap_validate_state");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:440:canic_core::perf!("bootstrap_ensure_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:442:canic_core::perf!("bootstrap_publish_release_set");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:745:canic_core::perf!("bootstrap_create_role");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:753:canic_core::perf!("bootstrap_ensure_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:763:canic_core::perf!("bootstrap_prune_store_catalog");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:792:canic_core::perf!("bootstrap_create_wasm_store");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:794:canic_core::perf!("bootstrap_sync_store_inventory");`
- `crates/canic-control-plane/src/workflow/bootstrap/root.rs:801:canic_core::perf!("bootstrap_import_store_catalog");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication/release/chunks.rs:111:canic_core::perf!("publish_push_store_chunk");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication/release/promote.rs:69:canic_core::perf!("publish_promote_manifest");`
- `crates/canic-control-plane/src/workflow/runtime/template/publication/release/target.rs:93:canic_core::perf!("publish_prepare_store");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:101:crate::perf!("plan_spawn");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:143:crate::perf!("observe_state");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:153:crate::perf!("plan_spawn");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:249:crate::perf!("create_canister");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:255:crate::perf!("register_worker");`
- `crates/canic-core/src/workflow/placement/scaling/mod.rs:85:crate::perf!("observe_state");`
- `crates/canic-core/src/workflow/placement/sharding/assignment.rs:115:crate::perf!("collect_registry");`
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
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:121:crate::perf!("extract_context");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:124:crate::perf!("map_request");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:127:crate::perf!("preflight");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:151:crate::perf!("execute_capability");`
- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs:167:crate::perf!("commit_replay");`
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
| `root:bootstrap:init-checkpoints` | `canic_control_plane::workflow::bootstrap::root` | `bootstrap_create_role` | 3 | 3470787704 | 1156929234 |
| `root:bootstrap:init-checkpoints` | `canic_control_plane::workflow::bootstrap::root` | `bootstrap_import_pool` | 1 | 702471237 | 702471237 |
| `root:bootstrap:init-checkpoints` | `canic_control_plane::workflow::bootstrap::root` | `bootstrap_create_wasm_store` | 1 | 39153312 | 39153312 |
| `scale_hub:create_worker:empty-pool` | `canic_core::workflow::placement::scaling` | `create_canister` | 1 | 1155062 | 1155062 |
| `root:bootstrap:init-checkpoints` | `canic_control_plane::workflow::bootstrap::root` | `bootstrap_rebuild_indexes` | 1 | 964116 | 964116 |
| `root:bootstrap:init-checkpoints` | `canic_control_plane::workflow::bootstrap::root` | `bootstrap_validate_state` | 1 | 332334 | 332334 |
| `root:canic_template_publish_chunk_admin:single-chunk` | `canic_control_plane::storage::stable::template::chunked` | `chunk_store_insert` | 1 | 180029 | 180029 |
| `root:bootstrap:init-checkpoints` | `canic_control_plane::workflow::bootstrap::root` | `bootstrap_sync_store_inventory` | 1 | 157561 | 157561 |
| `root:bootstrap:init-checkpoints` | `canic_control_plane::workflow::bootstrap::root` | `bootstrap_publish_release_set` | 1 | 139878 | 139878 |
| `user_hub:create_account:new-principal` | `canic_core::workflow::placement::sharding::assignment` | `assign_existing` | 1 | 100362 | 100362 |
| `user_hub:create_account:new-principal` | `canic_core::workflow::placement::sharding::assignment` | `collect_registry` | 1 | 73213 | 73213 |
| `user_hub:create_account:new-principal` | `canic_core::workflow::placement::sharding::assignment` | `plan_assign` | 1 | 43996 | 43996 |

## Checkpoint Coverage Gaps

Critical flows with checkpoints:
- `root capability dispatch`
- `replay/cached-response path`
- `sharding assignment flow`
- `scaling/provisioning flow`
- `bootstrap/install/publication flow`

Critical flows without checkpoints:
- `root proof provisioning`
- `issuer delegated-token prepare and verification`

Proposed first checkpoint insertion sites:
- `root proof provisioning` -> `crates/canic-core/src/workflow/runtime/auth/provisioning`
- `issuer delegated-token prepare and verification` -> `crates/canic-core/src/workflow/runtime/auth/prepare`

## Structural Hotspots

| Rank | Scenario | Avg local instructions | Module pressure | Evidence |
| --- | --- | ---: | --- | --- |
| 1 | `root:bootstrap:init-checkpoints` | 4214023135 | Root installation checkpoint group | `crates/canic-control-plane/src/workflow/bootstrap/root.rs` |
| 2 | `root:test_provision_chain_key_delegation_proof_for_issuer:new-issuer` | 16442775 | Root proof provisioning workflow | `fleets/test/root/src/lib.rs`; `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs` |
| 3 | `root:canic_response_capability_v1:request-cycles-fresh` | 2117578 | Root dispatcher plus replay/capability workflow | `crates/canic-core/src/workflow/rpc/request/handler/{mod,replay}.rs` |

## Hub Module Pressure

- `root::canic_response_capability_v1` now has measured replay/cycles stage deltas, so root capability work no longer has to be treated as an opaque endpoint total.
- `root::test_provision_chain_key_delegation_proof_for_issuer` measures explicit first-proof provisioning through the maintained root facade.
- `scale_hub::create_worker` measures the maintained scaling update through observe, plan, creation, and registration.
- `scale::request_cycles_from_parent` measures the maintained child-to-parent capability round trip.
- Root bootstrap is a checkpoint-group install row, not an endpoint total; it remains separate from update comparisons.

## Dependency Fan-In Pressure

- The sampled non-trivial hotspots concentrate in shared auth/replay/root runtime, child-to-parent capability, placement updates, and publication.
- Flow-stage checkpoints now exist in the scaling, sharding, publication, and replay workflows. This matrix records non-zero checkpoint deltas for sampled update scenarios, so the next optimization pass can target concrete stages instead of endpoint totals alone.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Flow checkpoint coverage present | INFO | Current repo scan found 57 `perf!` call sites under `crates/`. |
| Highest sampled endpoint currently highest-cost | WARN | `root:bootstrap:init-checkpoints` averages 4214023135 local instructions in this run. |
| Zero exclusive endpoint totals | INFO | `scale_hub:create_worker:empty-pool` retain measured call counts while nested/checkpoint scopes own the instruction attribution. |
| Baseline drift not yet available | INFO | No prior comparable report was selected; deltas remain `N/A`. |

## Risk Score

Risk Score: **6 / 10**

Interpretation: the first valid v2 measurement has no comparable predecessor, and root-proof plus delegated-token flows still lack product checkpoints. Those limitations are recorded rather than scored as zero.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test --offline --locked -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture` | PASS | PocketIC runner completed through authoritative root-harness artifacts and wrote the report plus normalized artifacts. |
| `fresh authoritative root harness profile per scenario` | PASS | Each scenario used a fresh topology/capability/scaling/sharding root bootstrap instead of sharing one cumulative perf table. |
| `canic_metrics(MetricsKind::Runtime, PageRequest { limit=512, offset=0 })` | PASS | Update scenarios were sampled before/after through persisted perf rows; the install scenario groups retained bootstrap checkpoints. Normalized rows are under `artifacts/instruction-footprint-v2/perf-rows.json`. |
| `repo checkpoint scan` | PASS | Found 57 checkpoint call sites. |
| `checkpoint delta capture` | PASS | 21 non-zero checkpoint delta rows were captured under `artifacts/instruction-footprint-v2/checkpoint-deltas.json`. |
| `fixed v2 update/install scenario roster` | PASS | All twelve required scenarios completed; query instruction totals are outside this method version. |
| `baseline comparison` | PASS | No comparable v2 report exists; this valid run establishes the first v2 baseline and deltas are `N/A`. |

## Follow-up Actions

1. Owner boundary: `flow observability`
   Action: rerun this audit after one concrete perf change and compare against the latest prior retained report; only add deeper verifier-side auth checkpoints if that endpoint-total starts to matter.
2. Owner boundary: `shared update hotspots`
   Action: compare `root::test_provision_chain_key_delegation_proof_for_issuer`, `root::canic_response_capability_v1`, and `scale::request_cycles_from_parent` before/after any shared-runtime cleanup, using this report as the `0.92` baseline.
3. Owner boundary: `query measurement`
   Action: add query rows only through a future authoritative same-call fixture and method version.

## Report Files

- [instruction-footprint-v2.md](./instruction-footprint-v2.md)
- [scenario-manifest.json](artifacts/instruction-footprint-v2/scenario-manifest.json)
- [perf-rows.json](artifacts/instruction-footprint-v2/perf-rows.json)
- [checkpoint-deltas.json](artifacts/instruction-footprint-v2/checkpoint-deltas.json)
- [checkpoint-coverage-gaps.json](artifacts/instruction-footprint-v2/checkpoint-coverage-gaps.json)
- [verification-readout.md](artifacts/instruction-footprint-v2/verification-readout.md)
- [method.json](artifacts/instruction-footprint-v2/method.json)
- [environment.json](artifacts/instruction-footprint-v2/environment.json)
- [evidence-manifest.yml](artifacts/instruction-footprint-v2/evidence-manifest.yml)
