# Audience Target Binding Invariant Audit - 2026-04-05

## Report Preamble

- Scope: delegated-token audience binding, role-attestation audience binding, delegated grant target binding, and verifier proof-audience installation checks
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-10/audience-target-binding.md`
- Code snapshot identifier: `c027b8df`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T16:48:44Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Audience / target claims remain explicit | PASS | audience fields are still explicit in `crates/canic-core/src/dto/auth.rs` and `crates/canic-core/src/dto/capability/proof.rs`. |
| Runtime target binding is still enforced before authorization | PASS | delegated-token verification still runs `validate_claims_against_cert(...)` and `verify_self_audience(...)` in `crates/canic-core/src/ops/auth/verify/token_chain.rs` before any access-expression authorization path. |
| Role-attestation audience is still bound to the local verifier | PASS | `verify_role_attestation_claims(...)` in `crates/canic-core/src/ops/auth/verify/attestation.rs` still rejects a payload whose `audience` does not match `self_pid`. |
| Delegated grant target binding is still explicit | PASS | `crates/canic-core/src/api/rpc/capability/grant.rs` still rejects delegated grants whose audience does not include the current target canister. |
| Proof installation target binding still fails closed | PASS | verifier/admin proof installation still routes through `ensure_target_in_proof_audience(...)` in `crates/canic-core/src/api/auth/proof_store/mod.rs`, and the PocketIC suite proves rejection when a local canister is outside proof audience. |
| Freshness is not substituted by audience checks | PASS | audience checks are still separate from expiry/replay enforcement; replay and TTL checks remain in `crates/canic-core/src/ops/auth/token.rs`, `crates/canic-core/src/api/auth/session/mod.rs`, and the replay workflow. |

## Comparison to Previous Relevant Run

- Stable: audience and target binding remain explicit and verifier-owned.
- Stable: the current target canister, not the ingress caller, is still the comparison surface for delegated audience checks.
- Improved: runtime coverage is stronger than the March baseline because today’s PocketIC suite proves proof-audience rejection on explicit verifier-material push paths.
- Stable: no alternate fallback path was found that accepts a token, grant, or proof install outside its audience contract.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/audience.rs` | `verify_self_audience`, `validate_claims_against_cert` | canonical delegated-token audience binding | High |
| `crates/canic-core/src/ops/auth/verify/attestation.rs` | `verify_role_attestation_claims` | role-attestation local audience / subnet binding | High |
| `crates/canic-core/src/api/rpc/capability/grant.rs` | root delegated-grant verifier | capability target binding enforcement | Medium |
| `crates/canic-core/src/api/auth/proof_store/mod.rs` | `ensure_target_in_proof_audience` | verifier proof-install target gate | Medium |
| `crates/canic-core/tests/pic_role_attestation.rs` | audience-mismatch and proof-audience rejection runtime paths | strongest current end-to-end evidence | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/ops/auth/audience.rs` | `ops, dto, ids` | 3 | 1 | 5 |
| `crates/canic-core/src/ops/auth/verify/attestation.rs` | `ops, dto, ids` | 3 | 1 | 5 |
| `crates/canic-core/src/api/auth/proof_store/mod.rs` | `api, ops, dto, metrics` | 4 | 2 | 6 |
| `crates/canic-core/tests/pic_role_attestation.rs` | `PocketIC, auth DTOs, capability DTOs, proof helpers` | 4 | 2 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| enum shock radius | `crates/canic-core/src/dto/auth.rs` | `RoleAttestation` appears in `35` files | High |
| enum shock radius | `crates/canic-core/src/dto/auth.rs` | `DelegationProof` appears in `25` files | High |
| growing hub module | `crates/canic-core/tests/pic_role_attestation.rs` | the runtime audience story is still heavily concentrated in one large PocketIC suite | Medium |
| growing hub module | `crates/canic-core/src/api/auth/proof_store/mod.rs` | explicit/admin proof install normalization and target checks are coupled here | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| audience binding lane | 6 | `ops/api/tests` | Rising pressure |
| proof store target-binding lane | 4 | `api/tests` | Rising pressure |

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `RoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 35 | High |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 25 | High |

## Risk Score

Risk Score: **3 / 10**

Score contributions:
- `+1` audience/target checks are still concentrated in a few auth seams
- `+1` proof/audience DTOs have high reference radius
- `+1` end-to-end runtime coverage is still heavily concentrated in `pic_role_attestation`

Verdict: **Invariant holds with low residual coupling risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib audience_helpers_reject_claim_outside_cert_audience -- --nocapture` | PASS | delegated token claim audience outside cert audience is rejected |
| `cargo test -p canic-core --lib verify_role_attestation_claims_rejects_audience_mismatch -- --nocapture` | PASS | role-attestation audience mismatch is rejected locally |
| `cargo test -p canic-core --lib verify_root_delegated_grant_claims_rejects_audience_mismatch -- --nocapture` | PASS | delegated grant target mismatch is rejected before capability execution |
| `cargo test -p canic-core --test pic_role_attestation verifier_store_rejects_root_push_when_local_canister_is_not_in_proof_audience -- --test-threads=1 --nocapture` | PASS | verifier proof push outside local proof audience fails closed |

## Follow-up Actions

1. Keep watching `crates/canic-core/src/api/auth/proof_store/mod.rs`; it is the clearest place a convenience bypass could accidentally weaken target binding.
2. Re-run this audit after any audience schema change in `crates/canic-core/src/dto/auth.rs`, capability grant change in `crates/canic-core/src/api/rpc/capability/grant.rs`, or proof-install admin-path change in `crates/canic-core/src/api/auth/proof_store/mod.rs`.
