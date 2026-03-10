# Token Trust Chain Invariant Audit - 2026-03-10

## Report Preamble

- Scope: delegated token trust chain (`root -> shard -> token issuer`)
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
| Trust anchors explicit | PASS | verifier code uses explicit authority/shard inputs (`ops/auth/token.rs`, `ops/auth/verify.rs`) |
| Chain validation stages present | PASS | delegation signature, token signature, audience, time bounds, claim/cert consistency checks present |
| Cache integrity path present | PASS | current-proof validation enforced before acceptance |
| Negative trust cases covered | PASS | invalid issuer/audience/time/signature tests present in core auth test suite |
| Forged token rejection path | PASS | `verify_token_sig`/`verify_delegation_signature` hard-fail on signature mismatch |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/token.rs` | `DelegatedTokenOps::verify_token` | trust-chain orchestration entrypoint | High |
| `crates/canic-core/src/ops/auth/verify.rs` | `verify_delegation_signature`, `verify_token_sig` | cryptographic chain verification core | High |
| `crates/canic-core/src/ops/storage/auth/mod.rs` | delegation proof state accessors | trust material source boundary | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/ops/auth/token.rs` | `ops, dto, cdk` | 3 | 1 | 5 |
| `crates/canic-core/src/ops/auth/verify.rs` | `ops, dto, ids, log` | 4 | 1 | 5 |
| `crates/canic-core/src/access/auth.rs` | `access, ops, dto, config` | 4 | 2 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| enum shock radius | `crates/canic-core/src/ops/auth/error.rs` | `DelegationValidationError` refs in `11` files | Medium |
| cross-layer struct spread | `DelegationProof` | referenced across `api/ops/workflow` (`9` files) | Medium |
| growing hub module | `crates/canic-core/src/access/auth.rs` | touched in `19` recent commits | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `dto` | 8 | `api/dto` | Hub forming |

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 9 | Medium |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 7 | Low |

## Risk Score

Risk Score: **2 / 10**

No trust-chain break was detected. Score stays low with minor residual complexity pressure in auth error and DTO fan-in surfaces.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib --locked` | PASS | auth signature/claims validation tests passed |
| `cargo test -p canic --test delegation_flow --locked` | PASS | delegated token flow passed |

## Follow-up Actions

No immediate follow-up required for this run.
