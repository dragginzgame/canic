# Instruction Footprint Audit - 2026-03-31

## Report Preamble

- Scope: Canic instruction footprint (first `0.20` baseline, partial canister scope)
- Definition path: `docs/audits/recurring/system/instruction-footprint.md`
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-31/instruction-footprint-3.md`
- Code snapshot identifier: `55b2c0f5`
- Method tag/version: `Method V1`
- Comparability status: `partial`
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-31T09:54:36Z`
- Branch: `main`
- Worktree: `clean`
- Execution environment: `PocketIC`
- Target canisters in scope: `app` `root` `scale_hub` `test`
- Target endpoints/flows in scope: `canic_time` `canic_env` `canic_log` `canic_subnet_registry` `canic_subnet_state` `plan_create_worker` `test`
- Deferred from this baseline: `scale_hub::create_worker` and sharding assignment updates require chain-key ECDSA in PocketIC; the default root harness does not provision that key yet.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Scenario manifest recorded | PASS | `artifacts/instruction-footprint-4/scenario-manifest.json` captures the scenario identity tuple for every sampled endpoint. |
| Normalized perf rows recorded | PASS | `artifacts/instruction-footprint-4/perf-rows.json` stores canonical endpoint rows with count and total local instructions. |
| Fresh topology isolation used | PASS | Each scenario ran under a fresh `setup_root()` install instead of reusing one cumulative perf table. |
| Flow checkpoint coverage scanned | PASS | `artifacts/instruction-footprint-4/flow-checkpoints.log` records the current repo scan result. |
| `perf!` checkpoints available for critical flows | PARTIAL | Current repo scan found zero `perf!` call sites under `crates/`, so flow-stage attribution is not yet measurable. |
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
| `test` | `test` | `minimal-valid` | 1 | 384 | 384 | N/A |  |

## Flow Checkpoints

- No current `perf!` checkpoints were found under `crates/`; no per-stage flow deltas are available yet.
- Flow checkpoint evidence file: `artifacts/instruction-footprint-4/flow-checkpoints.log`

## Checkpoint Coverage Gaps

Critical flows with checkpoints:
- none

Critical flows without checkpoints:
- `root capability dispatch`
- `delegated auth issuance/verification`
- `replay/cached-response path`
- `sharding assignment/query flow`
- `scaling/provisioning flow`
- `bootstrap/install/publication flow`

Proposed first checkpoint insertion sites:
- `root capability dispatch` -> `crates/canic-core/src/workflow/rpc/request/handler/mod.rs`
- `delegated auth issuance/verification` -> `crates/canic-core/src/workflow/auth.rs`
- `replay/cached-response path` -> `crates/canic-core/src/workflow/rpc/request/handler/replay.rs`
- `sharding assignment/query flow` -> `crates/canic-sharding-runtime/src/workflow/mod.rs`
- `scaling/provisioning flow` -> `crates/canic-core/src/workflow/placement/scaling/mod.rs`
- `bootstrap/install/publication flow` -> `crates/canic-tests/tests/root/harness.rs`

## Structural Hotspots

| Rank | Scenario | Avg local instructions | Module pressure | Evidence |
| --- | --- | ---: | --- | --- |
| 1 | `test:test:minimal-valid` | 384 | Local/dev update floor on the test helper canister | [test/lib](/home/adam/projects/canic/canisters/test/src/lib.rs) |

## Hub Module Pressure

- `scale_hub::plan_create_worker` concentrates cost in the scaling coordinator surface plus `canic-core` placement workflow. That makes scaling one of the first shared instruction hot paths worth reducing even before live worker provisioning is measurable in PocketIC.
- `test::test` provides the current chain-key-free update floor on a non-root child canister. Drift there points back to shared runtime/update overhead rather than topology-specific logic.
- Root state/registry reads stay separate from the leaf floor. They matter for operator paths, but they should not be confused with the shared ordinary-leaf baseline.

## Dependency Fan-In Pressure

- Shared lifecycle/observability endpoints (`canic_time`, `canic_env`, `canic_log`) all route through the default `start!` bundle, but the current persisted perf transport does not yet expose comparable query deltas. Their zero rows in this report are method-limited, not true zero-cost measurements.
- The sampled non-trivial hotspot fans into `canic-core` placement orchestration (`workflow/placement/scaling`). The local `test::test` update acts as the baseline floor for update overhead on an ordinary child canister.
- There is currently no flow-stage attribution because `perf!` coverage is absent. That is itself a dependency-pressure signal: optimization work is bottlenecked by missing internal checkpoints.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Flow checkpoint coverage absent | WARN | Current repo scan found zero `perf!` call sites under `crates/`. |
| Query endpoint deltas currently not persisted | WARN | 6 sampled query scenarios returned successfully but left no persisted `MetricsKind::Perf` delta. |
| Highest sampled endpoint currently highest-cost | WARN | `test:test:minimal-valid` averages 384 local instructions in this first baseline. |
| Baseline drift not yet available | INFO | First run of day; deltas remain `N/A` until the next comparable rerun. |

## Risk Score

Risk Score: **6 / 10**

Interpretation: the main current risk is observability incompleteness rather than one measured endpoint spike. The first baseline is good enough to rank entrypoints, but not yet good enough to localize flow stages.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture` | PASS | PocketIC runner completed and wrote the report plus normalized artifacts. |
| `setup_root() per scenario` | PASS | Each scenario used a fresh root bootstrap instead of sharing one cumulative perf table. |
| `canic_metrics(MetricsKind::Perf, PageRequest { limit=512, offset=0 })` | PASS | Perf rows were sampled before and after each scenario; normalized rows saved under `/home/adam/projects/canic/docs/audits/reports/2026-03/2026-03-31/artifacts/instruction-footprint-4/perf-rows.json`. |
| `repo checkpoint scan` | PASS | No `perf!` call sites are present in the current repo scan; flow checkpoint coverage remains partial. |
| `query perf visibility` | PARTIAL | 6 successful query scenarios left no persisted `MetricsKind::Perf` delta; they are treated as method-limited rather than zero-cost. |
| `baseline comparison` | BLOCKED | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |

## Follow-up Actions

1. Owner boundary: `flow observability`
   Action: add first stable `perf!` checkpoints to the scaling, sharding, and root-capability workflows so the next rerun can move from endpoint-only totals to real flow-stage attribution.
2. Owner boundary: `shared update hotspots`
   Action: compare `scale_hub::plan_create_worker` against the `test::test` update floor before/after any placement-runtime cleanup, using this report as the `0.20` baseline.
3. Owner boundary: `shared observability floor`
   Action: keep `app` query surfaces in the matrix so shared-runtime drift does not hide behind root-only or coordinator-only endpoints.

## Report Files

- [instruction-footprint-4.md](./instruction-footprint-4.md)
- [scenario-manifest.json](artifacts/instruction-footprint-4/scenario-manifest.json)
- [perf-rows.json](artifacts/instruction-footprint-4/perf-rows.json)
- [endpoint-matrix.tsv](artifacts/instruction-footprint-4/endpoint-matrix.tsv)
- [flow-checkpoints.log](artifacts/instruction-footprint-4/flow-checkpoints.log)
- [checkpoint-coverage-gaps.json](artifacts/instruction-footprint-4/checkpoint-coverage-gaps.json)
- [verification-readout.md](artifacts/instruction-footprint-4/verification-readout.md)
- [method.json](artifacts/instruction-footprint-4/method.json)
- [environment.json](artifacts/instruction-footprint-4/environment.json)
