# Audit Reports Summary: 2026-03

Monthly index of audit report runs under `docs/audits/reports/2026-03/`.

## Run Days

- [2026-03-07](2026-03-07/summary.md)
- [2026-03-08](2026-03-08/summary.md)
- [2026-03-10](2026-03-10/summary.md)
- [2026-03-16](2026-03-16/summary.md)
- [2026-03-24](2026-03-24/summary.md)

## Month Status

- Status: `partial` (month in progress)
- Latest note: `2026-03-24` complexity-accretion rerun scored `6.5 / 10` (no audience-binding correctness break; structurally stressed auth slice, with explicit refactor order now set: extract invariants first, then decompose, while prioritizing typed rollout metrics before the next feature).
- Carry-forward follow-up:
  - monitor policy principal-coupling drift (`cdk::candid::Principal`) in next `layer-violations` recurring run
  - keep token trust-chain stage order fixed (`structure -> current_proof -> signatures`) in follow-up recurring run
  - monitor fan-in/churn pressure for `crates/canic-core/src/api/auth/mod.rs` and `crates/canic-core/src/access/auth.rs` during 0.16 proof-evolution work
  - monitor fan-in/churn trend for `crates/canic-core/src/access/expr.rs` and `crates/canic-core/src/workflow/runtime/mod.rs`
  - keep high-CAF cross-subsystem slices split to reduce blast radius in upcoming runs
  - split `crates/canic-core/src/api/auth/mod.rs` by concern before adding more 0.16 auth behavior
  - extract shared audience invariant helpers before splitting `api/auth` to avoid moving duplication around
  - replace auth rollout string-classification seams with typed/shared metric keys
  - decide and document whether verifier-local install failures should keep the `auth_signer` endpoint label
  - avoid trait-heavy abstraction, over-splitting, and public API churn during the auth refactor
