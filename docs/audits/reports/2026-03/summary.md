# Audit Reports Summary: 2026-03

Monthly index of audit report runs under `docs/audits/reports/2026-03/`.

## Run Days

- [2026-03-07](2026-03-07/summary.md)
- [2026-03-08](2026-03-08/summary.md)
- [2026-03-10](2026-03-10/summary.md)
- [2026-03-16](2026-03-16/summary.md)

## Month Status

- Status: `partial` (month in progress)
- Latest note: `2026-03-16` token-trust-chain run scored `6 / 10` (no trust-chain correctness break; moderate structural pressure in auth boundary modules).
- Carry-forward follow-up:
  - monitor policy principal-coupling drift (`cdk::candid::Principal`) in next `layer-violations` recurring run
  - keep token trust-chain stage order fixed (`structure -> current_proof -> signatures`) in follow-up recurring run
  - monitor fan-in/churn pressure for `crates/canic-core/src/api/auth/mod.rs` and `crates/canic-core/src/access/auth.rs` during 0.16 proof-evolution work
  - monitor fan-in/churn trend for `crates/canic-core/src/access/expr.rs` and `crates/canic-core/src/workflow/runtime/mod.rs`
  - keep high-CAF cross-subsystem slices split to reduce blast radius in upcoming runs
