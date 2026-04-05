# Capability Scope Enforcement Invariant Audit - 2026-04-05

## Report Preamble

- Scope: authenticated scope ordering, root capability proof handling, and capability authorization flow
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-10/capability-scope-enforcement.md`
- Code snapshot identifier: `c027b8df`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T16:34:13Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Authentication before scope enforcement | PASS | [token.rs](/home/adam/projects/canic/crates/canic-core/src/ops/auth/token.rs#L78) verifies token trust/freshness, [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L252) binds caller to subject, and only then [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L253) enforces required scopes. |
| Scope uses verified claims/context only | PASS | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L120) reads scopes from `VerifiedDelegatedToken`, and [expr.rs](/home/adam/projects/canic/crates/canic-core/src/access/expr.rs#L838) calls `delegated_token_verified(...)` before the authenticated predicate succeeds. |
| Scope cannot substitute identity | PASS | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L258) rejects subject mismatch before any scope path, and [pic_role_attestation.rs](/home/adam/projects/canic/crates/canic-core/tests/pic_role_attestation.rs#L1648) proves policy denial occurs after a valid proof instead of replacing auth. |
| Failure semantics ordered correctly | PASS | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L278) emits `missing required scope` only after caller-bound verification has succeeded; local unit and PocketIC tests still show auth failures before authorization denials. |
| Positive/negative scope coverage present | PASS | unit test [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L496) covers required-scope allow/deny, [pic_role_attestation.rs](/home/adam/projects/canic/crates/canic-core/tests/pic_role_attestation.rs#L1498) proves valid role-attestation capability success, and [pic_role_attestation.rs](/home/adam/projects/canic/crates/canic-core/tests/pic_role_attestation.rs#L1658) proves subject-mismatch policy rejection. |

## Comparison to Previous Relevant Run

- Stable: auth/scope ordering remains correct and explicit.
- Stable: scope decisions still read verified claims, not request payload fields.
- Improved: local PocketIC coverage around the capability endpoint is stronger than the March baseline, and now directly proves both successful capability authorization and policy rejection ordering in the same runtime.
- Stable: no capability or scope path was found that can create identity or substitute for caller binding.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs) | `verify_token`, `enforce_required_scope` | canonical subject/scope ordering enforcement | High |
| [authorize.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/authorize.rs) | `authorize`, `authorize_issue_role_attestation` | capability authorization decision seam after authentication | Medium |
| [mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/rpc/capability/mod.rs) | root/non-root capability envelope validation and proof-mode routing | upstream capability filtering before workflow authorization | Medium |
| [pic_role_attestation.rs](/home/adam/projects/canic/crates/canic-core/tests/pic_role_attestation.rs) | capability endpoint proof/policy regression paths | strongest current end-to-end proof of ordering semantics | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs) | `access, ops, dto, config, ids` | 5 | 2 | 6 |
| [authorize.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/authorize.rs) | `workflow, ops, dto, config` | 4 | 2 | 6 |
| [mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/rpc/capability/mod.rs) | `api, dto, metrics, proof routing` | 4 | 2 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| enum shock radius | [mod.rs](/home/adam/projects/canic/crates/canic-core/src/dto/capability/mod.rs) | `CapabilityService` has `49` references and `CapabilityProof` has `67` | High |
| cross-layer struct spread | [mod.rs](/home/adam/projects/canic/crates/canic-core/src/dto/capability/mod.rs) | `RootCapabilityEnvelopeV1` appears in `33` references across api/ops/workflow/tests | High |
| growing hub module | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs) | touched in `19` recent commits | Medium |
| growing hub module | [authorize.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/authorize.rs) | touched in `12` recent commits | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `access::auth` | 22 | `access/api/workflow/tests/macros` | High |
| capability DTO / envelope lane | broad reference surface via `CapabilityService`, `CapabilityProof`, `RootCapabilityEnvelopeV1` | `api/ops/workflow/tests` | High |

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `CapabilityProof` | [mod.rs](/home/adam/projects/canic/crates/canic-core/src/dto/capability/mod.rs) | 67 | High |
| `CapabilityService` | [mod.rs](/home/adam/projects/canic/crates/canic-core/src/dto/capability/mod.rs) | 49 | High |
| `RootCapabilityEnvelopeV1` | [mod.rs](/home/adam/projects/canic/crates/canic-core/src/dto/capability/mod.rs) | 33 | High |

## Risk Score

Risk Score: **4 / 10**

Score contributions:
- `+2` medium/high hotspot pressure across auth ordering and capability authorization seams
- `+1` capability DTO shock radius is large
- `+1` cross-layer capability envelope spread is large

Verdict: **Invariant holds with low-but-real coupling pressure.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib required_scope_rejects_when_scope_missing -- --nocapture` | PASS | local unit proof for negative scope path |
| `cargo test -p canic-core --test pic_role_attestation capability_endpoint_role_attestation_proof_paths -- --test-threads=1 --nocapture` | PASS | valid bound identity + valid capability succeeds; broken proofs fail closed |
| `cargo test -p canic-core --test pic_role_attestation capability_endpoint_policy_and_structural_paths -- --test-threads=1 --nocapture` | PASS | policy denials still occur after authenticated capability proof handling |

## Follow-up Actions

1. Keep watching the capability DTO surface in [mod.rs](/home/adam/projects/canic/crates/canic-core/src/dto/capability/mod.rs); the invariant is still sound, but the reference radius is high enough to make future drift expensive.
2. Re-run this audit after any change to [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs), [authorize.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/authorize.rs), or [mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/rpc/capability/mod.rs).
