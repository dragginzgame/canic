# Audit Reports Summary: 2026-03

Monthly index of audit report runs under `docs/audits/reports/2026-03/`.

## Run Days

- [2026-03-07](2026-03-07/summary.md)
- [2026-03-08](2026-03-08/summary.md)
- [2026-03-10](2026-03-10/summary.md)
- [2026-03-16](2026-03-16/summary.md)
- [2026-03-24](2026-03-24/summary.md)
- [2026-03-25](2026-03-25/summary.md)

## Month Status

- Status: `partial` (month in progress)
- Latest note: `2026-03-25` first `wasm-footprint` baseline for `0.17` recorded a heavy shared-runtime floor across leaf canisters and a clear `root` bundle-canister outlier (`11267613` shrunk bytes), establishing the measurement base for root payload decomposition and `0.18` cutover inputs.
- Carry-forward follow-up:
  - keep token trust-chain stage order fixed (`structure -> current_proof -> signatures`) in the next `token-trust-chain` recurring run
  - keep shrinking direct `DelegationProof` dependence outside explicit boundary seams
  - keep `api/auth/mod.rs` from re-accumulating pure trust helpers now that those decisions live in ops
  - monitor `DelegationProof`, `StoredDelegationProof`, and `VerifiedTokenClaims` spread so future trust-model work does not increase cross-layer dependency pressure
  - monitor fan-in/churn trend for `crates/canic-core/src/access/expr.rs` and `crates/canic-core/src/workflow/runtime/mod.rs`
  - keep high-CAF cross-subsystem slices split to reduce blast radius in upcoming runs
  - document whether verifier-local install failures should keep the `auth_signer` endpoint label before changing metric semantics
  - avoid trait-heavy abstraction and public API churn during the remaining auth cleanup
  - complete the `0.17` root breakdown: runtime bytes versus embedded payload bytes, metadata/custom-section bytes, and growth slope versus deployable artifact count
  - add explicit IC ceiling headroom reporting and warning bands to the next `wasm-footprint` run
  - identify the irreducible bootstrap/recovery artifact set and ordered extraction list required by `docs/design/0.18-canister-templates/0.18-design.md`
