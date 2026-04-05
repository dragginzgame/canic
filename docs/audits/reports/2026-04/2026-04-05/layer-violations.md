# Layer Violations Audit - 2026-04-05

## Report Preamble

- Scope: `crates/canic-core/src/{api,workflow,domain,ops,storage,access}` and `crates/canic-dsl-macros/src`
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-24/layer-violations.md`
- Code snapshot identifier: `30807142`
- Method tag/version: `Method V5.0`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T14:36:23Z`
- Branch: `main`
- Worktree: `dirty`

## Executive Summary

- Risk Score: `1 / 10`
- Delta summary: runtime layering remains clean, and the only same-run hard guard failure was remediated by renaming the registry response mapper conversions to response-oriented names.
- Largest growth contributor: replay/capability and registry-response paths still carry the highest structural pressure, but no current hard layer break remains.
- Over-bundled families: `none` for this audit scope.
- Follow-up required: `yes`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| No upward `workflow/ops/storage/domain -> api` imports | PASS | `rg -n 'use crate::api\|crate::api::' crates/canic-core/src/{workflow,ops,storage,domain} -g '*.rs'` returned no matches. |
| No upward `ops/storage/domain -> workflow` imports | PASS | `rg -n 'use crate::workflow\|crate::workflow::' crates/canic-core/src/{ops,storage,domain} -g '*.rs'` returned no matches. |
| No upward `ops/storage -> domain::policy` imports | PASS | `rg -n 'use crate::domain::policy\|crate::domain::policy::' crates/canic-core/src/{ops,storage} -g '*.rs'` returned no matches. |
| Policy purity (`ops/workflow/api` imports, async) | PASS | No `crate::ops|crate::workflow|crate::api|serde::|candid::` matches beyond `crate::cdk::candid::Principal` value-type imports in policy, and `rg -n 'async fn' crates/canic-core/src/domain/policy -g '*.rs'` returned no matches. |
| DTO leakage into `domain` / `storage` | PASS | `rg -n 'crate::dto::\|use crate::dto' crates/canic-core/src/domain crates/canic-core/src/storage -g '*.rs'` returned no matches. |
| API direct storage/infra coupling | PASS | `rg -n 'use crate::storage\|crate::storage::\|use crate::infra\|crate::infra::' crates/canic-core/src/api -g '*.rs'` returned no matches. |
| Workflow direct stable-storage coupling (runtime) | PASS | `rg -n 'storage::stable\|crate::storage::stable\|use crate::storage' crates/canic-core/src/workflow -g '*.rs'` only matched `#[cfg(test)]` imports in `workflow/rpc/request/handler/replay.rs:22` and `workflow/rpc/request/handler/tests.rs`. |
| Macro boundary policy leakage | PASS | `rg -n 'crate::domain::policy\|crate::ops\|crate::workflow\|crate::api' crates/canic-dsl-macros/src -g '*.rs'` returned no matches. |
| Layering guard script | PASS | `bash scripts/ci/run-layering-guards.sh` passed after renaming the registry response mapper conversions to `record_to_response`. |
| `canic-core` library clippy | PASS | `cargo clippy -p canic-core --lib -- -D warnings` passed. |
| Replay/request-handler unit slice | PASS | `cargo test -p canic-core --lib workflow::rpc::request::handler -- --nocapture` passed: `42 passed; 0 failed`. |

## Comparison to Previous Relevant Run

- Improved: the same-run registry mapper naming violation was removed by renaming the response conversions to `record_to_response`, so the layer guard is green again.
- Stable: no upward runtime imports were introduced between `api`, `workflow`, `ops`, `policy`, and `storage`.
- Stable: policy remains side-effect free and async-free.
- Stable: DTO leakage into `domain` and `storage` is still absent.

## Hard Violations Summary

| Hard Check | Result | Evidence |
| --- | --- | --- |
| Guarded conversion naming rule | PASS | [mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/storage/registry/mapper.rs#L16) and [mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/storage/registry/mapper.rs#L38) now use `record_to_response`, and [subnet.rs](/home/adam/projects/canic/crates/canic-core/src/ops/storage/registry/subnet.rs#L208) calls the response-named helper. |
| Upward runtime layer imports | PASS | No matches in the three core upward-import scans. |
| DTO leakage into forbidden layers | PASS | No matches in `domain` or `storage`. |
| Macro boundary leakage | PASS | No boundary-layer imports in `canic-dsl-macros`. |

## Structural Hotspots

1. Registry response assembly remains the clearest structural hotspot.
   Evidence: [mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/storage/registry/mapper.rs#L34) and [subnet.rs](/home/adam/projects/canic/crates/canic-core/src/ops/storage/registry/subnet.rs#L204) still concentrate the response conversion path, even though the naming drift was fixed in this round.

2. Replay/capability handling is still the densest workflow pressure zone.
   Evidence: [replay.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/replay.rs#L1) has one of the highest non-test import counts in the layer scan, and the fan-in scan still lights up `workflow/rpc/request/handler/{replay,authorize,nonroot_cycles}.rs`.

3. Root capability metrics and registry ops remain broad shared hubs.
   Evidence: the dependency pressure scan shows `ops/runtime/metrics/root_capability.rs` and `ops/storage/registry/subnet.rs` among the highest-frequency files touched by the selected hub symbols.

## Hub Module Pressure

Non-test import-density scan (`rg -n '^use ' ... | rg -v '/tests?\.rs:'`):

| Module | Import Count | Pressure |
| --- | ---: | --- |
| `ops/rpc/mod.rs` | 6 | medium |
| `ops/rpc/request/dispatch.rs` | 5 | medium |
| `workflow/rpc/request/handler/replay.rs` | 4 | medium |
| `workflow/rpc/request/handler/nonroot_cycles.rs` | 4 | medium |
| `ops/runtime/env/mod.rs` | 4 | medium |
| `domain/policy/placement/sharding/mod.rs` | 4 | medium |

Interpretation: no single file is exploding in raw import count, but the replay/capability and registry-response lanes still concentrate a lot of structural responsibility.

## Dependency Fan-In Pressure

Hub-symbol reference scan (`SubnetRegistryOps|SubnetRegistryResponseMapper|ReplaySlotKey|RootCapabilityMetrics|AppRegistryResponseMapper`), excluding `tests.rs` files:

| Module | Reference Count | Pressure |
| --- | ---: | --- |
| `ops/storage/registry/subnet.rs` | 16 | high |
| `ops/runtime/metrics/root_capability.rs` | 14 | high |
| `workflow/rpc/request/handler/nonroot_cycles.rs` | 13 | high |
| `workflow/rpc/request/handler/replay.rs` | 9 | medium |
| `workflow/canister_lifecycle/mod.rs` | 9 | medium |
| `workflow/ic/provision.rs` | 8 | medium |
| `storage/stable/replay.rs` | 8 | medium |
| `ops/storage/registry/mapper.rs` | 6 | medium |

Interpretation: the main fan-in is still around registry operations and replay/capability plumbing, which matches the hotspot picture above.

## Responsibility Drift Signals

- `PASS`: policy still uses `Principal` as a pure value type rather than importing runtime/storage behavior.
- `WARN`: registry response assembly spans both [mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/storage/registry/mapper.rs) and [subnet.rs](/home/adam/projects/canic/crates/canic-core/src/ops/storage/registry/subnet.rs), so it remains a concentrated ops conversion boundary even after the naming fix.
- `WARN`: replay handler test-only imports of `ReplaySlotKey` from stable storage in [replay.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/replay.rs#L22) are not a runtime break, but they do keep the replay/storage seam close enough that future drift there should be watched.

## Risk Score

Risk Score: **1 / 10**

Score contributions:
- `+1` replay/registry responsibility pressure remains concentrated in a few shared ops/workflow files

Verdict: **Pass with low residual structural risk.**

## Architecture Health Interpretation

| Dimension | Status |
| --- | --- |
| Runtime layer invariants | Strong |
| Policy purity | Clean |
| Workflow orchestration | Stable |
| DTO sharing | Controlled |
| Ops conversion discipline | Stable |

Interpretation: the current tree respects the important runtime layer boundaries, and the only same-run naming drift that surfaced in the registry response mapping path was corrected before the final report state.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `bash scripts/ci/run-layering-guards.sh` | PASS | clean after renaming registry response conversions to `record_to_response` |
| `rg -n 'use crate::api\|crate::api::' crates/canic-core/src/{workflow,ops,storage,domain} -g '*.rs'` | PASS | no matches |
| `rg -n 'use crate::workflow\|crate::workflow::' crates/canic-core/src/{ops,storage,domain} -g '*.rs'` | PASS | no matches |
| `rg -n 'use crate::domain::policy\|crate::domain::policy::' crates/canic-core/src/{ops,storage} -g '*.rs'` | PASS | no matches |
| `rg -n 'ic_cdk\|crate::ops\|crate::workflow\|crate::api\|serde::\|candid::' crates/canic-core/src/domain/policy -g '*.rs'` | PASS | only `crate::cdk::candid::Principal` value-type imports matched |
| `rg -n 'async fn' crates/canic-core/src/domain/policy -g '*.rs'` | PASS | no matches |
| `rg -n 'crate::dto::\|use crate::dto' crates/canic-core/src/domain crates/canic-core/src/storage -g '*.rs'` | PASS | no matches |
| `rg -n 'use crate::storage\|crate::storage::\|use crate::infra\|crate::infra::' crates/canic-core/src/api -g '*.rs'` | PASS | no matches |
| `rg -n 'storage::stable\|crate::storage::stable\|use crate::storage' crates/canic-core/src/workflow -g '*.rs'` | PASS | only `#[cfg(test)]` matches |
| `rg -n 'crate::domain::policy\|crate::ops\|crate::workflow\|crate::api' crates/canic-dsl-macros/src -g '*.rs'` | PASS | no matches |
| `cargo clippy -p canic-core --lib -- -D warnings` | PASS | clean |
| `cargo test -p canic-core --lib workflow::rpc::request::handler -- --nocapture` | PASS | `42 passed; 0 failed` |

## Follow-up Actions

1. Owner boundary: `workflow/replay + storage seam`
   Action: keep the replay/storage seam under watch, especially any future widening of `#[cfg(test)]` stable-storage imports in workflow files.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-06/layer-violations.md`
