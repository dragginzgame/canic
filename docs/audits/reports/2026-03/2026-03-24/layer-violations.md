# Layer Violations Audit - 2026-03-24

## Report Preamble

- Scope: `crates/canic-core/src/{api,workflow,domain,ops,storage,access,lifecycle}` and `crates/canic-dsl-macros/src`
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-24/layer-violations.md` (earlier same-day run before auth remediation slices)
- Code snapshot identifier: `97e23ab8`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-24T18:40:17Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| No upward `workflow/ops/storage/domain -> api` imports | PASS | `rg -n 'use crate::api\\|crate::api::' crates/canic-core/src/{workflow,ops,storage,domain}` returned no matches. |
| No upward `ops/storage/domain -> workflow` imports | PASS | `rg -n 'use crate::workflow\\|crate::workflow::' crates/canic-core/src/{ops,storage,domain}` returned no matches. |
| No upward `ops/storage -> domain::policy` imports | PASS | `rg -n 'use crate::domain::policy\\|crate::domain::policy::' crates/canic-core/src/{ops,storage}` returned no matches. |
| Policy purity (`ops/workflow/api` imports, async) | PASS | no `crate::ops|crate::workflow|crate::api|serde::|candid::` imports and no `async fn` in `domain/policy`. |
| DTO leakage into `domain` / `storage` | PASS | no matches for `crate::dto::|use crate::dto` in `crates/canic-core/src/domain` or `crates/canic-core/src/storage`. |
| API direct storage/infra coupling | PASS | `rg -n 'use crate::storage\\|crate::storage::\\|use crate::infra\\|crate::infra::' crates/canic-core/src/api` returned no matches. |
| Workflow direct stable-storage coupling (runtime) | PASS | `rg -n 'storage::stable\\|crate::storage::stable\\|use crate::storage' crates/canic-core/src/workflow` returned no runtime matches in this run. |
| Macro boundary policy leakage | PASS | no `crate::domain::policy|crate::ops|crate::workflow|crate::api` matches in `crates/canic-dsl-macros/src`. |
| Crate dependency cycle signal | PASS | `cargo tree -e features` completed. |

## Comparison to Previous Relevant Run

- Improved: the previous auth-slice structural heat is lower because proof reuse, bootstrap audience subset, delegated-session expiry clamping, and verifier-target derivation now sit in `ops/auth/boundary.rs` instead of `api/auth/*`.
- Improved: the previous policy-candid and workflow test-storage drift signals did not reproduce on the current tree with the same scan family.
- Stable: no upward runtime layer imports or DTO leakage into `domain` / `storage` were introduced by the remediation work.

## Violations Summary

- No concrete runtime layering violations found in this run.

## Responsibility Drift Signals

No material responsibility-drift signals were detected above low background churn in this run.

## Risk Score

Risk Score: **1 / 10**

Score contributions:
- `+1` auth boundary churn remains visible in `api/auth/mod.rs` even though the trust decisions themselves moved down into ops

Verdict: **Pass with minimal residual structural risk.**

## Architecture Health Interpretation

| Dimension | Status |
| --- | --- |
| Layer invariants | Strong |
| Policy purity | Clean |
| Workflow orchestration | Clean |
| DTO sharing | Controlled |
| Macro boundary | Stable |

Interpretation: the current `0.16` auth remediation work improved the layering story rather than weakening it. The remaining concern is churn concentration in the auth API surface, not a runtime layer break.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'use crate::api\\|crate::api::' crates/canic-core/src/{workflow,ops,storage,domain} -g '*.rs'` | PASS | no matches |
| `rg -n 'use crate::workflow\\|crate::workflow::' crates/canic-core/src/{ops,storage,domain} -g '*.rs'` | PASS | no matches |
| `rg -n 'use crate::domain::policy\\|crate::domain::policy::' crates/canic-core/src/{ops,storage} -g '*.rs'` | PASS | no matches |
| `rg -n 'ic_cdk\\|crate::ops\\|crate::workflow\\|crate::api\\|serde::\\|candid::' crates/canic-core/src/domain/policy -g '*.rs'` | PASS | no matches |
| `rg -n 'async fn' crates/canic-core/src/domain/policy -g '*.rs'` | PASS | no matches |
| `rg -n 'crate::dto::\\|use crate::dto' crates/canic-core/src/domain crates/canic-core/src/storage -g '*.rs'` | PASS | no matches |
| `rg -n 'use crate::storage\\|crate::storage::\\|use crate::infra\\|crate::infra::' crates/canic-core/src/api -g '*.rs'` | PASS | no matches |
| `rg -n 'storage::stable\\|crate::storage::stable\\|use crate::storage' crates/canic-core/src/workflow -g '*.rs'` | PASS | no matches |
| `rg -n 'crate::domain::policy\\|crate::ops\\|crate::workflow\\|crate::api' crates/canic-dsl-macros/src -g '*.rs'` | PASS | no matches |
| `cargo test -p canic-core --lib api::auth::tests -- --nocapture` | PASS | `38 passed; 0 failed` |
| `cargo test -p canic-core --lib workflow::metrics::query::tests -- --nocapture` | PASS | `2 passed; 0 failed` |
| `cargo test -p canic-core --test pic_role_attestation verifier_store_rejects_root_push_when_local_canister_is_not_in_proof_audience --locked` | PASS | verifier-local audience guard still rejects root push outside proof audience |
| `cargo clippy -p canic-core --lib -- -D warnings` | PASS | clean |
| `cargo tree -e features` | PASS | completed |

## Follow-up Actions

1. Owner boundary: `auth API`
   Action: keep reducing pure helper weight inside `api/auth/mod.rs`, which is still the highest-churn auth boundary file even though the trust logic has moved down.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/layer-violations.md`
2. Owner boundary: `recurring layer audit`
   Action: keep the current zero-match scans for policy/runtime coupling in the next run so regressions show up immediately if the auth slice grows back upward.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/layer-violations.md`
