# Capability Scope Enforcement Invariant Audit - 2026-03-10

## Report Preamble

- Scope: auth ordering and scope/capability enforcement path
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
| Authentication before scope enforcement | PASS | `verify_token` + subject bind run before `enforce_required_scope` |
| Scope uses verified claims/context only | PASS | scope checks read verified token scopes, not request payload |
| Scope cannot substitute identity | PASS | mismatched subject/caller is rejected before scope path |
| Failure semantics ordered correctly | PASS | auth failures surface before authorization denials |
| Positive/negative scope coverage present | PASS | `delegation_flow` includes missing-scope rejection and valid-scope success |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/auth.rs` | `enforce_required_scope` | canonical scope gate after identity bind | High |
| `crates/canic-core/src/workflow/rpc/request/handler/authorize.rs` | capability authorization decisions | policy/authorization seam | Medium |
| `crates/canic-core/src/api/rpc/capability/mod.rs` | capability envelope validation | upstream capability filtering before workflow | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/auth.rs` | `access, ops, dto, config` | 4 | 2 | 6 |
| `crates/canic-core/src/workflow/rpc/request/handler/authorize.rs` | `workflow, ops, policy, dto` | 4 | 2 | 6 |
| `crates/canic-core/src/api/rpc/capability/mod.rs` | `api, dto, ops, workflow` | 4 | 2 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| enum shock radius | `crates/canic-core/src/dto/capability.rs` | `CapabilityService` referenced in `7` files | Medium |
| cross-layer struct spread | `RoleAttestation` | references across `api/ops/workflow` | Medium |
| growing hub module | `crates/canic-core/src/access/auth.rs` | touched in `19` recent commits | Medium |

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

Ordering and enforcement checks passed; no scope-before-auth or identity substitution path was found.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib --locked` | PASS | ordering and auth-scope tests passed |
| `cargo test -p canic --test delegation_flow --locked` | PASS | missing-scope rejection test passed |

## Follow-up Actions

No immediate follow-up required for this run.
