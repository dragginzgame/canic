# Audit Summary - 2026-03-10

## Run Contexts

- Branch: `main`
- Commit: `fa06bfef`
- Worktree: `dirty`
- Method: `Method V4.0`
- Comparability: `non-comparable` versus prior reports (method expansion for hotspots/fan-in/early-warning scoring)

Audits regenerated in this run:

- `complexity-accretion`
- `change-friction`
- `layer-violations`
- `bootstrap-lifecycle-symmetry`
- `subject-caller-binding`
- `canonical-auth-boundary`
- `capability-scope-enforcement`
- `token-trust-chain`
- `expiry-replay-single-use`
- `auth-abstraction-equivalence`
- `audience-target-binding`

## Risk Index Summary

| Audit | Risk Score |
| --- | ---: |
| `complexity-accretion` | 5 / 10 |
| `change-friction` | 5 / 10 |
| `layer-violations` | 3 / 10 |
| `bootstrap-lifecycle-symmetry` | 3 / 10 |
| `subject-caller-binding` | 2 / 10 |
| `canonical-auth-boundary` | 2 / 10 |
| `capability-scope-enforcement` | 2 / 10 |
| `token-trust-chain` | 2 / 10 |
| `expiry-replay-single-use` | 3 / 10 |
| `auth-abstraction-equivalence` | 2 / 10 |
| `audience-target-binding` | 2 / 10 |

Overall day posture: **low to moderate structural risk, no auth invariant break detected**.

## Key Findings by Severity

### High

- No confirmed invariant or boundary failures.

### Medium

- `workflow/runtime/mod.rs` and auth/workflow dispatch files show continuing hub pressure and churn.
- Recent high-CAF slices touched auth/replay/workflow/config surfaces together.
- Enum and struct coupling around auth and RPC DTOs remains a recurring complexity vector.

### Low

- Invariant suite checks all passed in this run.
- Lifecycle symmetry and boundary ordering checks passed.

## Verification Readout Rollup

| Command | Status | Notes |
| --- | --- | --- |
| `cargo tree -e features` | PASS | dependency graph resolved |
| `cargo test -p canic-core --lib --locked` | PASS | `230 passed; 0 failed` |
| `cargo test -p canic --test delegation_flow --locked` | PASS | `7 passed; 0 failed` |
| `cargo test -p canic --test lifecycle_boundary --locked` | PASS | `3 passed; 0 failed` |

## Follow-up Actions

1. Track fan-in and churn trend for `access/expr.rs` and `workflow/runtime/mod.rs` in next recurring run.
2. Keep high-CAF cross-subsystem work split into smaller slices where feasible.
3. Re-run lifecycle symmetry audit immediately after any lifecycle macro/adapter edits.

## Report Files

- `docs/audits/reports/2026-03/2026-03-10/complexity-accretion.md`
- `docs/audits/reports/2026-03/2026-03-10/change-friction.md`
- `docs/audits/reports/2026-03/2026-03-10/layer-violations.md`
- `docs/audits/reports/2026-03/2026-03-10/bootstrap-lifecycle-symmetry.md`
- `docs/audits/reports/2026-03/2026-03-10/subject-caller-binding.md`
- `docs/audits/reports/2026-03/2026-03-10/canonical-auth-boundary.md`
- `docs/audits/reports/2026-03/2026-03-10/capability-scope-enforcement.md`
- `docs/audits/reports/2026-03/2026-03-10/token-trust-chain.md`
- `docs/audits/reports/2026-03/2026-03-10/expiry-replay-single-use.md`
- `docs/audits/reports/2026-03/2026-03-10/auth-abstraction-equivalence.md`
- `docs/audits/reports/2026-03/2026-03-10/audience-target-binding.md`
- `docs/audits/reports/2026-03/2026-03-10/summary.md`
