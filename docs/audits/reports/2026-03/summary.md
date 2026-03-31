# Audit Reports Summary: 2026-03

Monthly index of audit report runs under `docs/audits/reports/2026-03/`.

## Run Days

- [2026-03-07](2026-03-07/summary.md)
- [2026-03-08](2026-03-08/summary.md)
- [2026-03-10](2026-03-10/summary.md)
- [2026-03-16](2026-03-16/summary.md)
- [2026-03-24](2026-03-24/summary.md)
- [2026-03-25](2026-03-25/summary.md)
- [2026-03-29](2026-03-29/summary.md)
- [2026-03-31](2026-03-31/summary.md)

## Month Status

- Status: `partial` (month in progress)
- Latest note: `2026-03-31` recorded both the first `0.20` wasm baseline and the first clean instruction baseline. `wasm-footprint` confirmed the current shared shrunk leaf floor at about `1.90 MB` and exposed the shrink-path inversion, while `instruction-footprint-3` showed that update-floor measurement works but query lanes and flow checkpoints are still method-limited.
- Carry-forward follow-up:
  - document the `CapabilityProofBlob` wire-shape change in release or migration notes
  - track `minimal.did` shared-method count and `root.did` admin count as explicit capability-surface guardrails in future runs
  - keep new control-plane additions root-local unless they clearly need to widen shared DTO/protocol families
  - keep token trust-chain stage order fixed (`structure -> current_proof -> signatures`) in the next `token-trust-chain` recurring run
  - keep shrinking direct `DelegationProof` dependence outside explicit boundary seams
  - keep `api/auth/mod.rs` from re-accumulating pure trust helpers now that those decisions live in ops
  - monitor `DelegationProof`, `StoredDelegationProof`, and `VerifiedTokenClaims` spread so future trust-model work does not increase cross-layer dependency pressure
  - monitor fan-in/churn trend for `crates/canic-core/src/access/expr.rs` and `crates/canic-core/src/workflow/runtime/mod.rs`
  - keep high-CAF cross-subsystem slices split to reduce blast radius in upcoming runs
  - document whether verifier-local install failures should keep the `auth_signer` endpoint label before changing metric semantics
  - avoid trait-heavy abstraction and public API churn during the remaining auth cleanup
  - investigate why the current `dfx` shrink path makes ordinary leaf canisters larger than the raw Cargo wasm artifacts
  - improve `0.20` hotspot attribution so the next wasm rerun produces a useful shared-runtime shortlist instead of `table[0]`
  - add first stable `perf!` checkpoints so the new instruction audit can attribute flow-stage cost
  - decide whether query lanes should be measured through widened persisted perf accounting or a separate query-focused audit method
  - add a key-provisioned PocketIC audit mode so chain-key-dependent update flows can join the instruction baseline
  - use the clean `instruction-footprint-3` update-floor baseline to choose the first perf/instruction reduction slice
