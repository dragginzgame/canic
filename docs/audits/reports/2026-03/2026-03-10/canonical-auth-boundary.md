# Canonical Auth Boundary Invariant Audit - 2026-03-10

## Report Preamble

- Scope: authenticated entrypoints and verifier convergence path
- Compared baseline report path: `N/A` (first run for this scope on 2026-03-10)
- Code snapshot identifier: `fa06bfef`
- Method tag/version: `Method V4.0`
- Comparability status: `non-comparable` (method expanded with hotspots, predictive signals, fan-in pressure)
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-10T14:30:36Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Authenticated paths converge on canonical verifier | PASS | macro path -> `eval_access` -> `delegated_token_verified` -> `verify_token` |
| Canonical ordering preserved | PASS | token verify -> subject bind -> scope authorize -> handler execution |
| No weaker internal auth branch detected | PASS | no alternate endpoint path observed with partial auth |
| Freshness/expiry stage invoked from boundary | PASS | `verify_time_bounds` invoked in delegated token verification stack |
| Integration path coverage present | PASS | `delegation_flow` authenticated endpoint tests passed |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/auth.rs` | `delegated_token_verified`, `verify_token` | canonical boundary implementation | High |
| `crates/canic-core/src/access/expr.rs` | `eval_access` | central auth dispatch surface | Medium |
| `crates/canic-dsl-macros/src/endpoint/expand.rs` | generated endpoint auth wiring | abstraction entrypoint into canonical boundary | Medium |
| `crates/canic-core/src/ops/auth/token.rs` | `DelegatedTokenOps::verify_token` | verifier stage orchestration | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/auth.rs` | `access, ops, dto, config` | 4 | 2 | 6 |
| `crates/canic-core/src/access/expr.rs` | `access, ids, log` | 3 | 1 | 7 |
| `crates/canic-core/src/ops/auth/token.rs` | `ops, dto, cdk` | 3 | 1 | 5 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| growing hub module | `crates/canic-core/src/access/auth.rs` | touched in `19` recent commits | Medium |
| enum shock radius | `crates/canic-core/src/ops/auth/error.rs` | `DelegationValidationError` referenced in `11` files | Medium |
| dependency fan-in hub | `dto` | imported in `8` files across `api/dto` | Low |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `dto` | 8 | `api/dto` | Hub forming |

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `RoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 15 | Medium |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 9 | Medium |

## Risk Score

Risk Score: **2 / 10**

Boundary convergence and ordering checks passed; no bypass path found. Residual risk is low and mainly tied to churn in auth dispatcher files.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib --locked` | PASS | verifier and ordering unit tests passed |
| `cargo test -p canic --test delegation_flow --locked` | PASS | authenticated endpoint flow passed |

## Follow-up Actions

No immediate follow-up required for this run.
