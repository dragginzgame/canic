# Token Trust Chain Invariant Audit - 2026-05-29

## Report Preamble

- Scope: delegated-token issuer trust chain, root trust-anchor resolution,
  shard certificate verification, shard signature verification, endpoint guard
  ordering, and role-attestation verifier refresh behavior.
- Definition path:
  `docs/audits/recurring/invariants/token-trust-chain.md`
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-07/token-trust-chain.md`
- Code snapshot identifier: `a43e27f8`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp: `2026-05-29`
- Worktree: `dirty`

## Executive Summary

Verdict: **PASS**.

The delegated-token trust chain still fails closed from verifier-local state:
runtime auth configuration is checked before token acceptance, the verifier
requires a locally configured shard signing key binding, root trust anchors come
from cascaded subnet state, root key identity is resolved against root pid, key
id, key hash, algorithm, and validity window, and token claims must bind back to
the signed certificate before the shard signature is accepted.

Endpoint authorization still verifies the token before subject binding, scope
checks, update replay consumption, and handler execution. Role-attestation
verification still uses cached trusted root-issued keys and only refreshes on an
unknown key id; signature, subject, audience, epoch, and expiry failures do not
trigger a broad retry path.

Risk score: **3 / 10**.

## Ordered Chain Evidence

| Stage | Evidence | Status |
| --- | --- | --- |
| Runtime config gate | `AuthOps::verify_token` rejects when delegated-token auth is disabled. | PASS |
| Shard key binding | `verify_shard_key_binding` compares key name, derivation path, algorithm, and public key bytes against local auth material. | PASS |
| Verifier-local root trust anchor | `root_trust_anchor` reads root pid and trusted root keys from local subnet state. | PASS |
| Root key identity/window resolution | `resolve_root_key` enforces root pid, key id, key hash, algorithm, and validity window before root signature verification. | PASS |
| Canonical certificate hash | `verify_delegated_token` recomputes the canonical certificate hash and rejects drift. | PASS |
| Root signature | Root signature verification covers the canonical certificate hash. | PASS |
| Claims-to-cert binding | Claims issuer shard pid, cert hash, cert audiences, and local role membership must match the signed certificate. | PASS |
| Canonical claims hash | `verify_delegated_token` recomputes the canonical claims hash before shard signature verification. | PASS |
| Shard signature | Shard signature verification uses the root-certified shard public key. | PASS |
| Endpoint subject/scope/replay boundary | `access/auth/token.rs` verifies token material before subject binding, scope enforcement, update replay consumption, and handler dispatch. | PASS |

## Findings

| ID | Status | Severity | Area | Finding | Resolution |
| --- | --- | --- | --- | --- | --- |
| TTC-2026-05-29-1 | PASS | High | Delegated-token verifier | Root and shard signatures remain chained through canonical cert and claims hashes; token-provided key material cannot replace verifier-local root trust. | Existing verifier and tests still pass. |
| TTC-2026-05-29-2 | PASS | High | Root trust anchor | Root key resolution rejects root pid mismatch, unknown key id, key hash drift, unsupported algorithm, and invalid key windows. | Existing root-key tests still pass. |
| TTC-2026-05-29-3 | PASS | High | Runtime propagation | The sharding root-suite path accepts a token verified through cascaded subnet-state root-key propagation. | Integration test passed. |
| TTC-2026-05-29-4 | PASS | Medium | Role attestation | Role-attestation verification refreshes only for unknown key id and rejects subject, audience, epoch, expiry, and signature failures without broad fallback. | Unit and integration tests still pass. |
| TTC-2026-05-29-5 | PASS | Medium | Stale trace helpers | No stale proof-store/current-proof token-chain helpers remain in `crates/`. | Targeted source scan found no matches. |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/token.rs` | `verify_token`, `root_trust_anchor`, `verify_shard_key_binding` | Runtime trust-chain orchestration entrypoint. | High |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_delegated_token`, `verify_claims` | Pure root/shard/token verification order. | High |
| `crates/canic-core/src/ops/auth/delegated/root_key.rs` | `resolve_root_key` | Root trust-anchor identity and validity checks. | High |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `enforce_subject_binding` | Endpoint guard integration after token verification. | Medium |
| `crates/canic-core/src/ops/auth/attestation.rs` | `verify_role_attestation_cached` | Role-attestation key and signature verification. | Medium |
| `crates/canic-core/src/api/auth/verify_flow.rs` | `verify_role_attestation_with_single_refresh` | Controls the only allowed key-refresh retry path. | Medium |

## Verification Readout

Commands passed:

- `cargo +1.96.0 test -p canic-core --lib verify_delegated_token --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib resolve_root_key --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib role_attestation_ --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-tests --test root_suite delegated_token_verification_uses_cascaded_subnet_state_root_key --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-tests --test pic_role_attestation role_attestation_verification_paths --locked -- --test-threads=1 --nocapture`

Commands used as source scans:

- `rg "verify_root|verify_shard|issuer|signature|certificate|cert_hash|root_sig|shard_sig|RootTrustAnchor|verify_delegated_token|verify_token|current_proof|proof_state|role_attestation|AttestationUnknownKeyId" crates/canic-core/src crates/canic-tests/tests canisters/test fleets/test -n`
- `rg "trace_token_trust_chain|token_chain|proof_state|verify_delegation_signature|verify_token_sig|authenticated_guard_checks_current_proof" crates -n`

Commands with no functional coverage:

- `cargo +1.96.0 test -p canic-core --lib verify_role_attestation_with_single_refresh --locked -- --nocapture`

  This stale filter matched zero tests. The concrete replacement filter
  `role_attestation_` matched and passed the current role-attestation unit
  coverage.

## Residual Risk

No blocker remains. The main watchpoint is keeping `AuthOps::verify_token`,
the pure delegated-token verifier, root-key resolution, endpoint access guards,
and role-attestation refresh behavior aligned whenever delegated issuer
formats, subnet-state propagation, or endpoint authentication macros change.
