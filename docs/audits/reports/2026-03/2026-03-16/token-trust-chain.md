# Token Trust Chain Invariant Audit - 2026-03-16

## Report Preamble

- Scope: delegated token trust chain (`root -> shard -> token issuer`)
- Compared baseline report path: `N/A` (first run for this scope on 2026-03-16)
- Code snapshot identifier: `a0d6ce65`
- Method tag/version: `Method V4.0`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-16T18:48:35Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Trust anchors explicit | PASS | `verify_token` requires `authority_pid` + local `self_pid`; delegation structure/signature checks are explicit in `ops/auth/token.rs` and `ops/auth/verify.rs`. |
| Chain validation stages present | PASS | `verify_token` executes: structure/time/audience/claims checks -> `verify_current_proof` -> delegation signature -> token signature. |
| Cache integrity path present | PASS | verifier fail-closed path enforces stored proof equality via `verify_current_proof` (`ProofUnavailable` / `ProofMismatch`). |
| Negative trust cases covered | PASS | typed rejection errors exist for invalid root/sig/audience/expiry (`DelegationValidationError`, `DelegationSignatureError`, `DelegationScopeError`, `DelegationExpiryError`) and are exercised in `canic-core` tests. |
| Forged token rejection path | PASS | signature path hard-fails in `verify_delegation_signature` and `verify_token_sig`; integration test `authenticated_guard_rejects_bogus_token_on_local` passes. |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/token.rs` | `DelegatedTokenOps::verify_token` | trust-chain orchestration entrypoint | High |
| `crates/canic-core/src/ops/auth/verify.rs` | `verify_delegation_signature`, `verify_token_sig`, `verify_current_proof` | cryptographic + local trust-anchor enforcement core | High |
| `crates/canic-core/src/api/auth/mod.rs` | `set_delegated_session_subject`, `verify_token*` wrappers | user-facing auth API boundary and bootstrap gate | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/ops/auth/token.rs` | `ops, dto, cdk` | 3 | 1 | 5 |
| `crates/canic-core/src/ops/auth/verify.rs` | `ops, dto, log` | 3 | 1 | 5 |
| `crates/canic-core/src/api/auth/mod.rs` | `access, dto, ops, workflow, protocol` | 5 | 2 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| enum shock radius | `crates/canic-core/src/ops/auth/error.rs` | `DelegationValidationError` referenced in `11` files (`rg -l "DelegationValidationError" ... | wc -l`) | Medium |
| cross-layer struct spread | `DelegationProof` | referenced across `dto/storage/ops/api/workflow` (`10` files) | Medium |
| growing auth boundary churn | `crates/canic-core/src/access/auth.rs`, `crates/canic-core/src/api/auth/mod.rs` | recent change concentration (`11` and `9` appearances in last 20 commits for scoped paths) | Medium |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `DelegationValidationError` | `crates/canic-core/src/ops/auth/error.rs` | 11 | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | `dto`, `storage`, `ops`, `api`, `workflow` | Medium |

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `crates/canic-core/src/access/auth.rs` | `access, dto, config, ops` | 11/20 scoped commits | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | `access, dto, ops, workflow` | 9/20 scoped commits | Medium |

### Capability Surface Growth

| Module | Public Items | Risk |
| --- | ---: | --- |
| `crates/canic-core/src/api/auth/mod.rs` | 14 public functions | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

No module fan-in pressure detected in this run above the `6+` import threshold.

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 10 | Medium |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 7 | Low |

## Red Flags

No confirmed trust-chain red flags were found in this run.

## Risk Score

Risk Score: **6 / 10**

Interpretation: no trust-chain correctness break was detected, but trust-critical logic remains concentrated in a small set of auth modules with medium coupling/churn pressure. Score is driven by hotspot concentration and cross-layer spread, not by acceptance of invalid tokens.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg "verify_token\\(|verify_current_proof|verify_delegation_signature|verify_token_sig" crates/canic-core/src/ops/auth -g '*.rs'` | PASS | all trust-chain stages present and ordered in code paths |
| `rg "ProofUnavailable|ProofMismatch|AudienceNotAllowed|SelfAudienceMissing|TokenSignatureInvalid|CertSignatureInvalid|TokenExpired|CertExpired" crates/canic-core/src/ops/auth -g '*.rs'` | PASS | typed fail-closed rejection classes present |
| `rg '^use ' crates/ -g '*.rs'` + related hotspot scans | PASS | structural/hub scans executed successfully |
| `git log --pretty=format: --name-only -n 20 -- crates/` | PASS | recent-change pressure evidence collected |
| `cargo test -p canic-core --lib --locked` | PASS | 256 tests passed, including auth regression/verification coverage |
| `cargo test -p canic --test delegation_flow --locked` | PASS | delegated token flow + bogus token rejection path passed |

## Follow-up Actions

1. Owner boundary: `canic-core auth boundary`
   Action: keep `verify_token` stage order locked (`structure -> current_proof -> signatures`) and add an explicit stage-order regression test if verification pipeline changes.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-17/token-trust-chain.md`
2. Owner boundary: `canic-core auth API/workflow`
   Action: monitor fan-in/churn for `api/auth/mod.rs` and `access/auth.rs` during 0.16-proof-evolution work; split if pressure score reaches `>= 7`.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-17/token-trust-chain.md`
3. Owner boundary: `design governance`
   Action: ensure 0.16 proof-model changes do not introduce any auth-path trust refresh that bypasses local fail-closed verification.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-17/token-trust-chain.md`
