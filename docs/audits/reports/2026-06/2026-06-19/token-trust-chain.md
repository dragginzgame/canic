# Token Trust Chain Invariant Audit - 2026-06-19

## Report Preamble

- Definition path:
  `docs/audits/recurring/invariants/token-trust-chain.md`
- Scope: delegated-token root/issuer canister-signature trust chain,
  configured root canister/root-key verifier config, canonical cert/claims
  hashes, active proof install, endpoint guard ordering, and role-attestation
  proof verification.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-29/token-trust-chain.md`
- Code snapshot identifier: `16894709`
- Method tag/version: `token-trust-chain-current`
- Comparability status: `non-comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-19`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This audit was selected as the next stale recurring invariant after the
subject-caller-binding refresh. It is non-comparable with the May baseline
because the old report audited root/shard ECDSA-style trust-chain wording,
while the current implementation uses a configured root canister signature over
a canonical delegation certificate and an issuer canister signature over
canonical delegated-token claims.

## Audit Definition Maintenance

The audit definition was updated before the run. Stale terms and search paths
for root/shard signatures, shard keys, current-proof helpers, and delegated
root-key resolution were removed. The live definition now checks the current
chain:

```text
configured IC root key + configured root canister id
  -> root canister signature over canonical delegation cert hash
  -> cert issuer and issuer-proof binding
  -> issuer canister signature over canonical claims hash
  -> claims bound to the certified delegation cert
```

The definition also now covers active proof install/status and role-attestation
proof verification, since both share the configured root verifier discipline.

## Audit Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Explicit verifier trust anchors | PASS | `AuthProofVerifierConfig` carries `root_canister_id` and `ic_root_public_key_raw`; config validation rejects missing or wrong network keys. |
| No implicit root-key fallback in protected verifier paths | PASS | Static scans found root-key use in config/bootstrap/validation paths, not as a verifier shortcut. |
| Canonical delegation cert hash | PASS | `cert_hash` is recomputed by verifier tests; cert-hash drift and noncanonical cert grants reject. |
| Root canister id binding | PASS | Cert issuance rules and embedded-proof verification bind the certified `root_pid` to configured root canister id. |
| Root proof canister signature | PASS | `verify_root_canister_signature_proof` checks DER canister id, seed/domain, payload message, and configured raw IC root public key. |
| Issuer proof binding | PASS | `issuer_proof_binding_hash` is recomputed and checked before token acceptance. |
| Claims-to-cert binding | PASS | Claims issuer and claims cert hash must match the certified delegation cert. |
| Issuer proof canister signature | PASS | `verify_issuer_canister_signature_proof` checks DER canister id, seed/domain, canonical claims hash, and configured raw IC root public key. |
| Positive cache identity | PASS | Cached hits still rerun canonical token, local audience/grant, subject, and scope checks. |
| Endpoint guard ordering | PASS | Access guard verifies token material before subject binding and required-scope authorization. |
| Active proof install | PASS | Install verifies root proof and local issuer canister binding before storing active proof state. |
| Role attestation root proof | PASS | Role-attestation proof verification uses the same configured root verifier discipline with distinct root payload seed/domain. |
| Retired helper names absent | PASS | Static scan found no stale current-proof, root/shard signature, or delegated root-key helper names in runtime/test surfaces. |

## Ordered Chain Evidence

| Stage | Evidence | Status |
| --- | --- | --- |
| Runtime verifier gate | Delegated-token verification requires runtime verifier support before proof acceptance. | PASS |
| Explicit trust anchor | `AuthProofVerifierConfig` supplies the expected root canister id and raw IC root public key. | PASS |
| Network root-key policy | `validate_network_root_key_pair` enforces mainnet known-key and explicit non-mainnet keys. | PASS |
| Canonical delegation cert hash | `cert_hash` is recomputed over canonical cert material. | PASS |
| Root pid binding | Cert rules require the certificate root principal to match configured root canister id. | PASS |
| Root canister signature | Root proof verification binds DER canister id, seed/domain, message, and configured IC root key. | PASS |
| Issuer proof binding hash | Cert rules recompute issuer proof binding material before issuer proof acceptance. | PASS |
| Claims-to-cert binding | Claims issuer and cert hash are bound to the certified delegation cert. | PASS |
| Canonical claims hash | `claims_hash` is recomputed before issuer signature verification. | PASS |
| Issuer canister signature | Issuer proof verification binds DER issuer canister id, seed/domain, message, and configured IC root key. | PASS |
| Positive cache | Cache identity binds proof/claims/issuer proof/caller, then reruns local checks on hit. | PASS |
| Endpoint access guard | Token verification precedes subject binding and scope authorization. | PASS |
| Active proof install | Stored active proof state is derived only after root proof and issuer-self checks pass. | PASS |
| Role attestation | Role-attestation verification uses root canister-signature proof with a distinct payload kind. | PASS |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/token.rs` | `AuthOps::verify_token`, `verify_with_embedded_proofs` | Runtime config, root proof, issuer proof, and cache orchestration | High |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_delegated_token`, `verify_claims` | Pure cert/claims/root/issuer verification ordering | High |
| `crates/canic-core/src/ops/auth/root_canister_sig.rs` | `verify_root_canister_signature_proof` | Root canister-signature proof construction and verification | High |
| `crates/canic-core/src/ops/auth/issuer_canister_sig.rs` | `verify_issuer_canister_signature_proof` | Issuer canister-signature proof construction and verification | High |
| `crates/canic-core/src/ops/auth/delegated/cert_rules.rs` | `validate_cert_issuance_rules` | Root pid, TTL, audience/grant shape, and issuer binding checks | High |
| `crates/canic-core/src/ops/auth/delegated/canonical.rs` | `cert_hash`, `claims_hash`, `issuer_proof_binding_hash` | Canonical hash material for root and issuer signatures | High |
| `crates/canic-core/src/ops/auth/delegated/active_proof.rs` | `install_active_delegation_proof` | Issuer-local active proof validation before storage | Medium |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | Endpoint guard integration after trust-chain verification | Medium |
| `crates/canic-core/src/ops/auth/attestation.rs` | `verify_role_attestation_cached` | Role-attestation root proof verification shares trust anchors | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| verifier trust-anchor config | trust config/root-key scan found 15 files | 5 | 4 | 6 |
| root canister-signature proof | root proof scan found 18 files | 5 | 3 | 6 |
| issuer canister-signature proof | issuer proof scan found 11 files | 4 | 3 | 5 |
| cert/claims canonical binding | cert/hash/binding scan found 28 files | 5 | 4 | 6 |
| delegated auth proof DTO family | proof/token/attestation scan found 59 files | 6 | 4 | 6 |

The broadest surface is the passive token/proof DTO family. That is a
watchpoint, but the audit did not find behavior or validation moving onto DTO
types.

## Risk Score

Risk Score: **3 / 10**

Score contributions:

- `+1` root/issuer canister-signature verification remains security-sensitive.
- `+1` active proof install and provisioning paths recently changed and share
  root proof material.
- `+1` passive delegated auth DTOs have broad fan-in across production and
  test surfaces.

Verdict: **Invariant holds with low residual trust-chain complexity risk.**

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| verifier config spread | config/bootstrap/ops/workflow auth paths | 15 files reference verifier config/root-key terms | Medium |
| root proof spread | root proof ops, token verifier, attestation, active proof install | 18 files reference root proof terms | Medium |
| issuer proof spread | issuer proof ops, token verifier, canonical/cache tests | 11 files reference issuer proof terms | Medium |
| canonical hash binding spread | delegated cert/claims/proof DTOs and ops | 28 files reference cert/claims/binding hash terms | Medium |
| passive proof DTO spread | DTO, ops, API, workflow, storage, protocol tests | 59 files reference token/proof/attestation DTO terms | Medium |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `RootPayloadKind` | `crates/canic-core/src/ops/auth/root_canister_sig.rs` | 4 | Low |
| `IssuerPayloadKind` | `crates/canic-core/src/ops/auth/issuer_canister_sig.rs` | 4 | Low |
| `CanonicalDomain` | `crates/canic-core/src/ops/auth/delegated/canonical.rs` | 1 | Low |
| `VerifyDelegatedTokenError` | `crates/canic-core/src/ops/auth/types.rs` | 3 | Low |

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `AuthProofVerifierConfig` | `crates/canic-core/src/domain/auth.rs` | config, bootstrap, ops, workflow | Medium |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | dto, ops, access tests, protocol tests | Medium |
| `ActiveDelegationProof` | `crates/canic-core/src/dto/auth.rs` | api, workflow, ops, storage, protocol tests | Medium |

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | root proof batch prepare/get/install, root issuer policy, active proof status | high recent auth-provisioning churn | Medium |
| `crates/canic-core/src/ops/auth/token.rs` | runtime verifier config, delegated-token verification, positive cache | focused auth verifier owner | Medium |
| `crates/canic-core/src/ops/auth/delegated/canonical.rs` | cert hash, claims hash, issuer proof binding hash | focused canonical owner | Medium |
| `crates/canic-core/src/dto/auth.rs` | token/proof/cert/active-proof boundary shapes | broad passive fan-in | Medium |

### Capability Surface Growth

No trust-chain capability surface growth was detected in this run. The broad
surface is boundary DTO usage, not new public verifier commands.

## Dependency Fan-In Pressure

| Module / Struct | Evidence | Risk |
| --- | --- | --- |
| verifier trust-anchor config | Direct count scan found 15 files | Medium |
| root canister-signature proof terms | Direct count scan found 18 files | Medium |
| issuer canister-signature proof terms | Direct count scan found 11 files | Medium |
| canonical cert/claims/binding terms | Direct count scan found 28 files | Medium |
| delegated auth proof DTO family | Direct count scan found 59 files | Medium |
| `AuthProofVerifierConfig` | Direct struct count found 3 files | Low |
| `DelegatedTokenClaims` | Direct struct count found 11 files | Medium |
| `ActiveDelegationProof` | Direct struct count found 13 files | Medium |

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test --locked -p canic-core --lib verify_delegated_token -- --nocapture` | PASS | 17 delegated-token verifier tests passed, including root proof, issuer proof, cert hash drift, and noncanonical data rejection. |
| `cargo test --locked -p canic-core --lib auth_proof_verifier_config -- --nocapture` | PASS | 11 root-key/network verifier config tests passed. |
| `cargo test --locked -p canic-core --lib root_canister_sig -- --nocapture` | PASS | 3 root canister-signature seed/domain/message/root-key tests passed. |
| `cargo test --locked -p canic-core --lib issuer_canister_sig -- --nocapture` | PASS | 6 issuer canister-signature seed/domain/feature tests passed. |
| `cargo test --locked -p canic-core --lib cert_rules -- --nocapture` | PASS | 7 cert issuance and issuer-binding rule tests passed. |
| `cargo test --locked -p canic-core --lib install_active_delegation_proof -- --nocapture` | PASS | 4 active proof install tests passed, including wrong issuer/time/root proof rejection. |
| `cargo test --locked -p canic-core --lib delegated_auth_guard_preserves_verify_bind_scope_order -- --nocapture` | PASS | Endpoint guard order remains verify, bind subject, then scope. |
| `cargo test --locked -p canic-tests --test root_suite root_batch_provisioning_installs_active_proof_on_user_shard -- --nocapture` | PASS | PocketIC root batch provisioning installed an active proof and verified signer-local token issuance. |
| `cargo test --locked -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture` | PASS | PocketIC role-attestation proof and rejection paths passed. |
| `rg -n 'AuthProofVerifierConfig\|auth_proof_verifier_config\|validate_network_root_key_pair\|ic_root_public_key_raw\|root_canister_id' crates/canic-core/src -g '*.rs'` | PASS | Trust-anchor config and root-key paths are concentrated in config/bootstrap/workflow/ops. |
| `rg -n 'verify_root_canister_signature_proof\|root_canister_sig_seed\|root_canister_sig_domain\|RootPayloadKind\|RootProof::IcCanisterSignatureV1' crates/canic-core/src -g '*.rs'` | PASS | Root proof paths are concentrated in root proof ops, token verifier, attestation, delegation install, and tests. |
| `rg -n 'verify_issuer_canister_signature_proof\|issuer_canister_sig_seed\|issuer_canister_sig_seed_hash\|IssuerPayloadKind\|IssuerProof::IcCanisterSignatureV1' crates/canic-core/src -g '*.rs'` | PASS | Issuer proof paths are concentrated in issuer proof ops, token verifier, delegated proof/cert code, and tests. |
| `rg -n 'cert_hash\|claims_hash\|issuer_proof_binding_hash\|VerifyDelegatedTokenError\|IssuerPidMismatch\|CertHashMismatch' crates/canic-core/src/ops/auth -g '*.rs'` | PASS | Canonical hash and typed rejection paths are owned by auth ops. |
| `rg -n 'trace_token_trust_chain\|token_chain\|proof_state\|current_proof\|verify_delegation_signature\|verify_token_sig\|authenticated_guard_checks_current_proof\|root_sig\|shard_sig\|shard_key\|resolve_root_key' crates/canic-core/src crates/canic-tests/tests canisters/test fleets/test -g '*.rs'` | PASS | No stale root/shard/current-proof helper names were found. |
| `git log --name-only -n 20 -- crates/canic-core/src/ops/auth crates/canic-core/src/api/auth crates/canic-core/src/config/validation/auth.rs crates/canic-core/src/domain/auth.rs crates/canic-tests/tests/root_cases crates/canic-tests/tests/pic_role_attestation_cases` | PASS | Recent churn is concentrated in root-proof provisioning and auth proof verification areas. |
| Early-warning and fan-in count scans from the recurring definition | PASS | Counts recorded in Hub Module Pressure and Dependency Fan-In Pressure tables. |

The `canic-core` test commands emitted known
`unfulfilled_lint_expectations` warnings in
`crates/canic-core/src/ops/runtime/metrics/delegated_auth.rs`; they did not
affect the focused test outcomes.

## Follow-up Actions

No follow-up actions required.
