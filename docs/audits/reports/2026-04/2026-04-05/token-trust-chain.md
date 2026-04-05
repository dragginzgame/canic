# Token Trust Chain Invariant Audit - 2026-04-05

## Report Preamble

- Scope: delegated-token trust-chain verification (`root -> shard -> issuer/token`), verifier-local proof material, role-attestation key resolution, and current-proof gating
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-10/token-trust-chain.md`
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
| Trust anchors remain explicit | PASS | delegated-token verification still starts in `crates/canic-core/src/ops/auth/token.rs`, with proof/state sourcing in `crates/canic-core/src/ops/storage/auth/mod.rs` and chain-stage verification in `crates/canic-core/src/ops/auth/verify/token_chain.rs`. |
| Chain validation stages remain ordered | PASS | the current trace helpers in `crates/canic-core/src/ops/auth/verify/token_chain.rs` still enforce `structure -> current_proof -> delegation_signature -> token_signature`, and the unit trace test proves the canonical order for a valid token. |
| Cache / proof hits do not bypass cryptographic integrity checks | PASS | current-proof lookup in `crates/canic-core/src/ops/auth/verify/proof_state.rs` only resolves verifier-local proof identity; signature validation still happens in `verify_delegation_signature(...)` and `verify_token_sig(...)` in `crates/canic-core/src/ops/auth/verify/token_chain.rs`. |
| Negative trust cases still reject | PASS | unit tests reject unknown attestation key ids, PocketIC proof-miss tests fail before signature validation, and the role-attestation runtime path still rejects subject/audience/expiry mismatches. |
| Forged or mismatched issuer material fails closed | PASS | `authenticated_guard_checks_current_proof_before_signature_validation` in `crates/canic-core/tests/pic_role_attestation.rs` proves missing local proof denies the request before signature checks, and the attestation verification path still rejects invalid or mismatched authority material. |

## Comparison to Previous Relevant Run

- Stable: trust anchors are still explicit and come from verifier-local proof / key state, not caller-provided runtime context.
- Stable: signature verification is still downstream of structure and current-proof validation, not replaced by cache hits.
- Improved: current evidence is stronger than the March baseline because the auth unit suite now traces the canonical chain-stage order explicitly and the PocketIC suite proves the runtime stops at current-proof before attempting signature validation.
- Stable: no internal bypass was found that accepts token signatures without first resolving the expected root/shard trust material.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/token.rs` | `DelegatedTokenOps::verify_token`, `validate_claim_invariants` | trust-chain orchestration entrypoint | High |
| `crates/canic-core/src/ops/auth/verify/token_chain.rs` | `trace_token_trust_chain`, `verify_delegation_signature`, `verify_token_sig` | cryptographic chain verification core | High |
| `crates/canic-core/src/ops/auth/verify/proof_state.rs` | `verify_current_proof` | verifier-local proof identity gate before signatures | High |
| `crates/canic-core/src/ops/storage/auth/mod.rs` | `matching_proof_dto`, `upsert_proof_from_dto`, attestation key accessors | trust-anchor material sourcing boundary | Medium |
| `crates/canic-core/src/ops/auth/attestation.rs` | `verify_role_attestation_cached` | attestation trust path and key resolution | Medium |
| `crates/canic-core/tests/pic_role_attestation.rs` | runtime proof/attestation regression suite | strongest end-to-end trust-chain runtime evidence | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/ops/auth/token.rs` | `ops, dto, ids, crypto` | 4 | 1 | 5 |
| `crates/canic-core/src/ops/auth/verify/token_chain.rs` | `ops, crypto, dto, ids, storage` | 5 | 1 | 6 |
| `crates/canic-core/src/ops/storage/auth/mod.rs` | `ops, storage, dto, config` | 4 | 1 | 6 |
| `crates/canic-core/tests/pic_role_attestation.rs` | `PocketIC, auth DTOs, capability proofs, attestation helpers` | 4 | 2 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| growing hub module | `crates/canic-core/tests/pic_role_attestation.rs` | touched repeatedly across the March-April auth line and still owns many end-to-end negative cases | Medium |
| enum shock radius | `crates/canic-core/src/dto/auth.rs` | `DelegationProof` appears in `25` files | High |
| cross-layer struct spread | `DelegatedTokenClaims` | referenced in `10` files across auth/access/tests | Medium |
| growing hub module | `crates/canic-core/src/ops/auth/verify/token_chain.rs` | touched in the auth hardening line and now owns explicit trace helpers | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| token verification lane | 6 | `ops/access/tests` | Rising pressure |
| attestation verification lane | 35 | `ops/api/tests` via role-attestation types and verification helpers | High |

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 25 | High |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 10 | Medium |

## Risk Score

Risk Score: **3 / 10**

Score contributions:
- `+1` trust-chain code is still concentrated in a few sensitive auth modules
- `+1` `DelegationProof` has a large reference radius
- `+1` the end-to-end trust runtime is still heavily concentrated in one large PocketIC file

Verdict: **Invariant holds with low residual coupling risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib trace_token_trust_chain_stops_at_current_proof_before_signatures -- --nocapture` | PASS | proves missing local proof stops validation before signatures |
| `cargo test -p canic-core --lib trace_token_trust_chain_records_canonical_order_for_valid_token -- --nocapture` | PASS | proves canonical valid-token chain stage order |
| `cargo test -p canic-core --lib verify_role_attestation_cached_rejects_unknown_key_id -- --nocapture` | PASS | attestation trust path rejects unknown trust anchor material |
| `cargo test -p canic-core --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture` | PASS | runtime attestation path rejects subject/audience/expiry mismatches and accepts the valid path |
| `cargo test -p canic-core --test pic_role_attestation authenticated_guard_checks_current_proof_before_signature_validation -- --test-threads=1 --nocapture` | PASS | runtime path proves proof miss is rejected before signature validation |

## Follow-up Actions

1. Keep watching `crates/canic-core/tests/pic_role_attestation.rs`; it carries too much of the trust-runtime story for comfort, even though the invariant currently holds.
2. Re-run this audit after any change to `crates/canic-core/src/ops/auth/token.rs`, `crates/canic-core/src/ops/auth/verify/token_chain.rs`, or verifier proof storage in `crates/canic-core/src/ops/storage/auth/mod.rs`.
