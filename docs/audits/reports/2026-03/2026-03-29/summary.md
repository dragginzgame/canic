# Audit Summary - 2026-03-29

## Run Contexts

- Audit run: `capability-surface`
  - Definition: `docs/audits/recurring/system/capability-surface.md`
  - Baseline: `N/A` (first run for this scope on 2026-03-29)
  - Branch: `main`
  - Commit: `f26eccd6`
  - Worktree: `dirty`
  - Method: `Method V1.0`
  - Comparability: `non-comparable` (first recorded baseline)
- Audit run: `capability-surface-2`
  - Definition: `docs/audits/recurring/system/capability-surface.md`
  - Baseline: `docs/audits/reports/2026-03/2026-03-29/capability-surface.md`
  - Branch: `main`
  - Commit: `f26eccd6`
  - Worktree: `dirty`
  - Method: `Method V2.0`
  - Comparability: `non-comparable` versus earlier same-day baseline (hard/drift split, GAF/utilization sections, and fully refreshed `.did` artifacts)

Audits generated in this run:

- `capability-surface`
- `capability-surface-2`

## Risk Index Summary

| Audit | Risk Score |
| --- | ---: |
| `capability-surface` | 6 / 10 |
| `capability-surface-2` | 2 / 10 |

Overall day posture: **improved after the rerun. The initial baseline overstated shared proof-tree fan-out because generated `.did` files were stale; with refreshed interfaces and the stricter method, the current capability surface is mostly stable, with one explicit compatibility-risk signal from the `CapabilityProofBlob` wire-shape change.**

## Key Findings by Severity

### High

- No confirmed capability-surface invariant failures.

### Medium

- `canic_response_capability_v1` and the core proof DTO families still propagate across every canister interface, which keeps the shared `.did` floor high.
- `root` is the clear control-plane growth hotspot, with all `11` admin methods and the largest `canic_*` surface in the fleet.

### Low

- Delegated proof installation endpoints are already tightly scoped; current pressure is mostly root control-plane concentration, not endpoint sprawl.
- The current rerun shows compact `CapabilityProofBlob` fan-out instead of the earlier concrete proof-tree fan-out.

## Verification Readout Rollup

| Command | Status | Notes |
| --- | --- | --- |
| endpoint inventory `python3` scan over `crates/canic/src/macros/endpoints.rs` | PASS | bundle, endpoint, admin, controller-only, and internal counts captured |
| RPC/protocol `python3` scan over `crates/canic-core/src/dto/{rpc,capability}.rs` and `protocol.rs` | PASS | constant and enum-variant baselines captured |
| generated `.did` scans over `crates/canisters/*/*.did` | PASS | per-canister surface counts and endpoint-family spread captured |
| `rg -n '^  canic_.*_admin :' crates/canisters -g '*.did'` | PASS | root-only admin clustering confirmed |
| churn scan via `git log --format='' --name-only -n 20 -- ...` | PASS | hotspot pressure confirmed in `endpoints.rs` and `protocol.rs` |
| refreshed `.did` rerun inventory with full canister build loop | PASS | stale proof-tree types no longer present in generated non-root `.did` files |
| `CARGO_TARGET_DIR=/tmp/canic-capability-audit-ws-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | warmed-target workspace clippy passed |

## Follow-up Actions

1. Document the `CapabilityProofBlob` wire-shape change in release notes or migration notes because the rerun flags it as a compatibility-risk signal.
2. Track `minimal.did` shared-method count and `root.did` admin count as explicit recurring capability-surface guardrails in future runs.
3. Keep new control-plane additions root-local unless there is a clear reason they must widen shared DTO/protocol families.

## Report Files

- `docs/audits/reports/2026-03/2026-03-29/capability-surface.md`
- `docs/audits/reports/2026-03/2026-03-29/capability-surface-2.md`
- `docs/audits/reports/2026-03/2026-03-29/summary.md`
