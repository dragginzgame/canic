# Audit Summary - 2026-03-24

## Run Contexts

- Audit run: `complexity-accretion`
  - Baseline: `N/A` (first run for this scope on 2026-03-24)
  - Branch: `main`
  - Commit: `76eba1a1`
  - Worktree: `dirty`
  - Method: `Method V4.1`
  - Comparability: `non-comparable` versus earlier month complexity baselines (targeted 0.16 auth-slice review)
- Audit run: `complexity-accretion-2`
  - Baseline: `docs/audits/reports/2026-03/2026-03-24/complexity-accretion.md`
  - Branch: `main`
  - Commit: `76eba1a1`
  - Worktree: `dirty`
  - Method: `Method V4.1`
  - Comparability: `comparable`
- Audit run: `complexity-accretion-3`
  - Baseline: `docs/audits/reports/2026-03/2026-03-24/complexity-accretion.md`
  - Branch: `main`
  - Commit: `76eba1a1`
  - Worktree: `dirty`
  - Method: `Method V4.1`
  - Comparability: `comparable`
- Audit run: `layer-violations`
  - Definition: `docs/audits/recurring/system/layer-violations.md`
  - Baseline: `docs/audits/reports/2026-03/2026-03-24/layer-violations.md` (earlier same-day run before remediation)
  - Branch: `main`
  - Commit: `97e23ab8`
  - Worktree: `dirty`
  - Method: `Method V4.1`
  - Comparability: `comparable`
- Audit run: `token-trust-chain`
  - Definition: `docs/audits/recurring/invariants/token-trust-chain.md`
  - Baseline: `docs/audits/reports/2026-03/2026-03-24/token-trust-chain.md` (earlier same-day run before remediation)
  - Branch: `main`
  - Commit: `97e23ab8`
  - Worktree: `dirty`
  - Method: `Method V4.1`
  - Comparability: `comparable`

Audits generated in this run:

- `complexity-accretion`
- `complexity-accretion-2`
- `complexity-accretion-3`
- `layer-violations`
- `token-trust-chain`

## Risk Index Summary

| Audit | Risk Score |
| --- | ---: |
| `complexity-accretion` | 6 / 10 |
| `complexity-accretion-2` | 6.5 / 10 |
| `complexity-accretion-3` | 6.5 / 10 |
| `layer-violations` | 1 / 10 |
| `token-trust-chain` | 4 / 10 |

Overall day posture: **lower structural pressure in the 0.16 auth slice after the remediation passes, with no audience-binding correctness break, no hard layering violation, and no trust-chain validation break found**.

## Key Findings by Severity

### High

- No confirmed invariant failures.

### Medium

- `DelegationProof` remains the main cross-layer dependency center, even though storage-facing and verified-claims internals are now much narrower.
- `api/auth/mod.rs` remains the main auth churn hotspot, even after the pure trust helpers moved down into ops.
- Verifier-local install failures are still flattened under the `auth_signer` endpoint label, which keeps metric-semantics cleanup on the follow-up list.

### Low

- Layer scans were fully clean in this rerun; the previous policy-candid and workflow-storage drift signals did not reproduce on the remediated tree.
- Label cleanup should not proceed without module-level or README-level metric semantics documentation.

## Verification Readout Rollup

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n "DelegationAdminCommand|prepare_explicit_verifier_push|ensure_target_in_proof_audience|ensure_token_claim_audience_subset|record_delegation_install_|AuthRollout"` | PASS | accreted auth seams located and reviewed |
| `cargo test -p canic-core --lib api::auth::tests -- --nocapture` | PASS | `38 passed; 0 failed` |
| `cargo clippy -p canic-core --lib -- -D warnings` | PASS | audited slice lint-clean |
| `wc -l crates/canic-core/src/api/auth/mod.rs ...` | PASS | hotspot size baselines captured |
| rerun seam confirmation scans (`api/auth`, `metrics/auth`, `metrics/mapper`) | PASS | same-day comparison confirms unchanged code, refined interpretation |
| rerun 3 report comparison reads | PASS | baseline and rerun-2 re-read before tightening action ordering |
| layer import / DTO / macro scans | PASS | no hard runtime layer violations detected on current auth split |
| `cargo test -p canic-core --lib api::auth::tests -- --nocapture` | PASS | `38 passed; 0 failed` |
| `cargo test -p canic-core --lib workflow::metrics::query::tests -- --nocapture` | PASS | `2 passed; 0 failed` |
| `cargo test -p canic-core --test pic_role_attestation verifier_store_rejects_root_push_when_local_canister_is_not_in_proof_audience --locked` | PASS | `1 passed; 0 failed` |
| `cargo tree -e features` | PASS | no dependency-cycle signal detected |
| trust-chain stage/order scan in `ops/auth` and `api/auth` | PASS | structure -> current proof -> signatures remains intact |
| `cargo test -p canic-core --lib ops::auth::tests -- --nocapture` | PASS | `25 passed; 0 failed` |
| `cargo test -p canic --test delegation_flow authenticated_guard_rejects_bogus_token_on_local --locked` | PASS | bogus token rejected on local verifier path |
| rerun trust/layer evidence scan after remediation slices | PASS | split verifier modules, thinner API helpers, and narrower internal proof/claim shapes confirmed |

## Follow-up Actions

1. Keep shrinking direct `DelegationProof` dependence outside explicit boundary seams.
2. Document metric label semantics in a README or module header before changing verifier/install endpoint labels.
3. Keep `api/auth/mod.rs` from re-accumulating pure trust helpers now that those decisions live in ops.
4. Keep trust-chain stage order locked and re-audit `DelegationProof` / `VerifiedTokenClaims` / `StoredDelegationProof` spread in the next `token-trust-chain` run.

## Report Files

- `docs/audits/reports/2026-03/2026-03-24/complexity-accretion.md`
- `docs/audits/reports/2026-03/2026-03-24/complexity-accretion-2.md`
- `docs/audits/reports/2026-03/2026-03-24/complexity-accretion-3.md`
- `docs/audits/reports/2026-03/2026-03-24/layer-violations.md`
- `docs/audits/reports/2026-03/2026-03-24/token-trust-chain.md`
- `docs/audits/reports/2026-03/2026-03-24/summary.md`
