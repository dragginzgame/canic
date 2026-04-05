# Instruction Footprint Audit - 2026-04-05

## Report Preamble

- Scope: Canic instruction footprint
- Definition path: `docs/audits/recurring/system/instruction-footprint.md`
- Retained summary policy: `0.25` keeps one retained summary per audit and drops same-day duplicates/artifacts by default.
- Code snapshot identifier: `c027b8df`
- Method tag/version: `Method V1`
- Comparability status: `partial`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T16:30:13Z`
- Branch: `main`
- Worktree: `dirty`
- Execution environment: `PocketIC`
- Target canisters in scope: `audit_leaf_probe` `audit_root_probe` `audit_scaling_probe` `root` `scale_hub` `test` `user_hub`
- Target endpoints/flows in scope: `audit_env_probe` `audit_log_probe` `audit_plan_create_worker_probe` `audit_subnet_registry_probe` `audit_subnet_state_probe` `audit_time_probe` `canic_request_delegation` `canic_response_capability_v1` `canic_template_prepare_admin` `canic_template_publish_chunk_admin` `canic_template_stage_manifest_admin` `create_account` `create_worker` `test`

## Summary

- This retained summary keeps the final same-day `0.25` instruction readout after the demo-vs-test split moved query perf sampling onto internal audit probes.
- Comparability is `partial` because the earlier same-day run still sampled demo canister probe endpoints; this retained run reflects the cleaner final audit surface.
- Query probe lanes are measured through same-call local-only audit probes because query-side perf rows are not committed to stable state.

## Findings

| Check | Result | Evidence |
| --- | --- | --- |
| Fresh smallest-profile topology used | PASS | Scenarios ran under standalone probes or the smallest root profile needed for the flow instead of one cumulative perf table. |
| Query endpoint perf visibility preserved | PASS | Query scenarios were sampled through dedicated internal audit probes rather than shipped demo endpoints. |
| Delegated auth / replay / cycles path still measurable | PASS | `root::canic_request_delegation` and `root::canic_response_capability_v1` remain directly sampled in the retained matrix. |
| Root template admin publication path still measurable | PASS | `canic_template_stage_manifest_admin`, `canic_template_prepare_admin`, and `canic_template_publish_chunk_admin` remain in scope. |
| Same-day duplicate reports removed | PASS | This file is the single retained `instruction-footprint` summary for `2026-04-05`. |

## Current Endpoint Matrix

| Canister | Endpoint | Scenario | Count | Total local instructions | Avg local instructions | Notes |
| --- | --- | --- | ---: | ---: | ---: | --- |
| `audit_leaf_probe` | `audit_time_probe` | `minimal-valid` | 1 | 20680 | 20680 | same-call local-only perf probe |
| `audit_leaf_probe` | `audit_env_probe` | `minimal-valid` | 1 | 22330 | 22330 | same-call local-only perf probe |
| `audit_leaf_probe` | `audit_log_probe` | `empty-page` | 1 | 302876 | 302876 | same-call local-only perf probe |
| `audit_root_probe` | `audit_subnet_registry_probe` | `representative-valid` | 1 | 64512 | 64512 | same-call local-only perf probe |
| `audit_root_probe` | `audit_subnet_state_probe` | `minimal-valid` | 1 | 20245 | 20245 | same-call local-only perf probe |
| `audit_scaling_probe` | `audit_plan_create_worker_probe` | `empty-pool` | 1 | 52061 | 52061 | same-call local-only perf probe |
| `scale_hub` | `create_worker` | `empty-pool` | 1 | 2633882 | 2633882 | real product-facing create path |
| `user_hub` | `create_account` | `new-principal` | 1 | 2933361 | 2933361 | demo/reference provisioning path |
| `root` | `canic_request_delegation` | `fresh-shard` | 1 | 3230286 | 3230286 | shared auth/control path |
| `test` | `test` | `minimal-valid` | 1 | 816 | 816 | standalone baseline floor |
| `root` | `canic_response_capability_v1` | `cycles-request` | 1 | 1478634 | 1478634 | replay/cycles response path |
| `root` | `canic_template_stage_manifest_admin` | `single-chunk` | 1 | 410889 | 410889 | root template admin path |
| `root` | `canic_template_prepare_admin` | `single-chunk` | 1 | 188423 | 188423 | root template admin path |
| `root` | `canic_template_publish_chunk_admin` | `single-chunk` | 1 | 334994 | 334994 | root template admin path |

## Readout

- The heaviest retained sampled flow is still `root::canic_request_delegation` at `3230286` local instructions.
- `user_hub::create_account` at `2933361` and `scale_hub::create_worker` at `2633882` remain the largest non-root provisioning paths in this retained matrix.
- The local-only query floor is now measured on internal audit probes rather than on shipped demo endpoints, which keeps the demo surface cleaner without losing perf coverage.
- Because the earlier same-day run used a broader older probe surface, this retained summary records the final cleaner matrix rather than a numeric same-day delta table.

## Conclusion

- The `0.25` instruction audit is retained as a single summary at this canonical path.
- The main instruction hotspots remain root delegation plus the create-account / create-worker provisioning flows.
- The test-vs-demo split improved audit hygiene without removing meaningful performance visibility.
