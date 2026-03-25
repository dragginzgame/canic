# Token Trust Chain Invariant Audit - 2026-03-24

## Report Preamble

- Scope: delegated token trust chain (`root -> shard -> token issuer`)
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-24/token-trust-chain.md` (earlier same-day run before remediation slices)
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
| Trust anchors explicit | PASS | `verify_token` still requires explicit `authority_pid` and local `self_pid`; root/shard key material stays in verifier-local trust state and is enforced from `ops/auth/verify/token_chain.rs` and `ops/auth/verify/proof_state.rs`. |
| Chain validation stages present | PASS | canonical stage order is still `structure/time/audience -> current_proof -> delegation signature -> token signature`, now sealed in `ops/auth/verify/token_chain.rs`. |
| Cache integrity path present | PASS | fail-closed verifier-local proof matching remains in `ops/auth/verify/proof_state.rs` with exact-match `ProofMiss` / `ProofMismatch`. |
| Negative trust cases covered | PASS | typed rejections remain present for invalid root/audience/signature/expiry paths; stage-order regressions still assert signatures never run before `current_proof`. |
| Forged token rejection path | PASS | `authenticated_guard_rejects_bogus_token_on_local` still rejects a bogus token before handler execution. |

## Comparison to Previous Relevant Run

- Improved: the earlier `6 / 10` hotspot in `ops/auth/verify.rs` is gone; trust logic is now split across `verify/token_chain.rs`, `verify/proof_state.rs`, and `verify/attestation.rs`.
- Improved: `DelegatedTokenClaims` is no longer the default internal trust shape across auth helpers; `VerifiedTokenClaims` now appears in `7` files while `DelegatedTokenClaims` dropped to `12` references across `crates/`.
- Improved: API boundary helpers for proof reuse, bootstrap audience subset, delegated-session expiry clamping, and verifier-target derivation now route through `DelegatedTokenOps`.
- Mixed: `DelegationProof` spread remains broad at `27` Rust files, so the boundary DTO is still the main remaining dependency center even after `StoredDelegationProof` narrowed storage-facing use to `8` files.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/verify/token_chain.rs` | `verify_token_trust_chain` | canonical stage-order owner and crypto-chain coordinator | Medium |
| `crates/canic-core/src/ops/auth/verify/proof_state.rs` | `verify_current_proof` | fail-closed local-proof enforcement | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | `issue_token` / delegation setup glue | still large and still sees repeated churn, even though trust decisions moved down | Medium |

## Hub Module Pressure

| Module | Line Count | Role | Pressure Score |
| --- | ---: | --- | ---: |
| `crates/canic-core/src/ops/auth/token.rs` | 72 | thin auth entrypoint over config + canonical trust-chain call | 2 |
| `crates/canic-core/src/ops/auth/verify/token_chain.rs` | 296 | ordered token-chain validation and crypto checks | 4 |
| `crates/canic-core/src/ops/auth/verify/proof_state.rs` | 84 | verifier-local proof cache integrity | 2 |
| `crates/canic-core/src/ops/auth/verify/attestation.rs` | 96 | attestation-specific invariants | 2 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| enum shock radius | `crates/canic-core/src/ops/auth/error.rs` | `DelegationValidationError` now appears in `13` Rust files | Medium |
| boundary DTO spread | `crates/canic-core/src/dto/auth.rs` | `DelegationProof` now appears in `27` Rust files across `dto`, `ops`, `api`, `workflow`, and tests | Medium |
| auth boundary churn | `crates/canic-core/src/api/auth/mod.rs` | still appears in `9` of the last `20` crate-scoped file touches | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 27 | High |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 12 | Medium |
| `StoredDelegationProof` | `crates/canic-core/src/ops/storage/auth/mod.rs` | 8 | Low |
| `VerifiedTokenClaims` | `crates/canic-core/src/ops/auth/types.rs` | 7 | Low |

## Red Flags

No confirmed trust-chain red flags were found in this run.

## Risk Score

Risk Score: **4 / 10**

Interpretation: no trust-chain correctness break was detected, and the structural pressure dropped materially from the earlier `6 / 10` same-day run. Remaining pressure is mostly boundary DTO spread around `DelegationProof` plus continued churn in `api/auth/mod.rs`, not a weakness in the verification order or local trust-anchor enforcement.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'verify_token\\(|verify_current_proof|verify_delegation_signature|verify_token_sig|verify_token_trust_chain|trace_token_trust_chain|StoredDelegationProof|VerifiedTokenClaims|required_verifier_targets_from_audience|bootstrap_token_audience_subset|clamp_delegated_session_expires_at' crates/canic-core/src/ops/auth crates/canic-core/src/api/auth crates/canic-core/src/access -g '*.rs'` | PASS | canonical trust-chain ownership, proof-state split, and API thinning evidence captured |
| `cargo test -p canic-core --lib ops::auth::tests -- --nocapture` | PASS | `27 passed; 0 failed` |
| `cargo test -p canic-core --lib api::auth::tests -- --nocapture` | PASS | `38 passed; 0 failed` |
| `cargo test -p canic-core --lib workflow::metrics::query::tests -- --nocapture` | PASS | `2 passed; 0 failed` |
| `cargo test -p canic --test delegation_flow authenticated_guard_rejects_bogus_token_on_local --locked` | PASS | bogus token rejected on local verifier path |
| `rg -l 'DelegationValidationError' crates -g '*.rs' | wc -l` | PASS | enum reference count captured (`13`) |
| `rg -l 'DelegationProof' crates -g '*.rs' | wc -l` | PASS | boundary DTO spread count captured (`27`) |
| `rg -l 'DelegatedTokenClaims' crates -g '*.rs' | wc -l` | PASS | remaining DTO spread count captured (`12`) |
| `rg -l 'StoredDelegationProof' crates -g '*.rs' | wc -l` | PASS | storage-owned proof shape spread captured (`8`) |
| `rg -l 'VerifiedTokenClaims' crates -g '*.rs' | wc -l` | PASS | ops-local verified claims spread captured (`7`) |
| `wc -l crates/canic-core/src/ops/auth/token.rs crates/canic-core/src/ops/auth/verify/mod.rs crates/canic-core/src/ops/auth/verify/token_chain.rs crates/canic-core/src/ops/auth/verify/proof_state.rs crates/canic-core/src/ops/auth/verify/attestation.rs crates/canic-core/src/api/auth/mod.rs crates/canic-core/src/api/auth/session/mod.rs crates/canic-core/src/ops/auth/boundary.rs` | PASS | hotspot size and split evidence captured |
| `git log --pretty=format: --name-only -n 20 -- crates/ | sed '/^$/d' | sort | uniq -c | sort -nr | sed -n '1,40p'` | PASS | recent-change pressure evidence captured |

## Follow-up Actions

1. Owner boundary: `canic-core auth DTO boundary`
   Action: keep shrinking direct `DelegationProof` dependence outside explicit boundary seams so future auth work does not push the DTO spread back into trust-critical helpers.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/token-trust-chain.md`
2. Owner boundary: `canic-core auth API`
   Action: continue reducing non-boundary helper weight inside `api/auth/mod.rs`, which remains the main churn hotspot even after the trust helpers moved down into ops.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/token-trust-chain.md`
3. Owner boundary: `design governance`
   Action: keep stage-order tests and fail-closed local-proof enforcement intact if any future refresh/install flow changes the trust path again.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/token-trust-chain.md`
