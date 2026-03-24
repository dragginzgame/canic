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

Audits generated in this run:

- `complexity-accretion`
- `complexity-accretion-2`
- `complexity-accretion-3`

## Risk Index Summary

| Audit | Risk Score |
| --- | ---: |
| `complexity-accretion` | 6 / 10 |
| `complexity-accretion-2` | 6.5 / 10 |
| `complexity-accretion-3` | 6.5 / 10 |

Overall day posture: **moderate-to-rising structural pressure in the 0.16 auth slice, no audience-binding correctness break found**.

## Key Findings by Severity

### High

- No confirmed invariant failures.

### Medium

- `crates/canic-core/src/api/auth/mod.rs` is acting as a multi-responsibility auth control hub.
- Auth rollout metrics are string-coupled across producer, classifier, and query layers, and the rerun classifies that as the highest latent failure mode because drift can remove observability silently.
- Refactor order is now explicit: extract shared audience invariants first, then decompose modules.

### Low

- Install-path metrics currently flatten verifier-local failures into the `auth_signer` endpoint label.
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

## Follow-up Actions

1. Extract shared audience invariant helpers first, then split `api/auth` by invariant boundary before additional 0.16 auth features land.
2. Treat typed rollout metrics as high-priority follow-up before the next auth feature slice.
3. Document metric label semantics in a README or module header before changing verifier/install endpoint labels.
4. Avoid trait-heavy abstraction, over-splitting, or public API churn during the refactor.

## Report Files

- `docs/audits/reports/2026-03/2026-03-24/complexity-accretion.md`
- `docs/audits/reports/2026-03/2026-03-24/complexity-accretion-2.md`
- `docs/audits/reports/2026-03/2026-03-24/complexity-accretion-3.md`
- `docs/audits/reports/2026-03/2026-03-24/summary.md`
