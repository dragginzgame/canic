# Token Trust Chain Invariant Audit - 2026-05-07

## Report Preamble

- Scope: delegated-token trust-chain verification, root trust-anchor resolution, shard key binding, role-attestation key verification, and runtime delegated-token guard integration.
- Definition path: `docs/audits/recurring/invariants/token-trust-chain.md`
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/token-trust-chain.md`
- Code snapshot identifier: `6e72960b`
- Method tag/version: `Method V4.2`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-07T17:42:18Z`
- Branch: `main`
- Worktree: `dirty`
- Dirty files observed: `CHANGELOG.md`, `Makefile`, `canisters/test/delegation_root_stub/src/lib.rs`, `crates/canic-cli/**`, `crates/canic-core/**`, `crates/canic-host/**`, `crates/canic-testing-internal/**`, `crates/canic-tests/**`, `docs/**`, `scripts/**`

## Executive Summary

Verdict: **PASS**.

The token trust-chain invariant still holds. Delegated-token acceptance is now
implemented through a self-contained token verifier rather than the April
`token_chain` / `proof_state` trace path. The current verifier requires:

- a verifier-local root trust anchor loaded from subnet state, not from token
  claims;
- a root key matching root principal, key id, key hash, algorithm, and validity
  window;
- a root signature over the canonical delegation certificate hash;
- claims whose issuer shard and cert hash match the signed certificate;
- a shard signature over canonical token claims;
- a shard key binding that matches Canic's configured IC threshold ECDSA key and
  derivation path.

The main risk is audit-method drift, not a detected verifier break. The April
baseline tests named `trace_token_trust_chain_*` and
`verify_role_attestation_cached_rejects_unknown_key_id`; those filters now match
zero tests. Replacement verifier and PocketIC tests cover the current behavior.

Risk score: **3 / 10**.

## Audit Question

Can a delegated token be accepted without a valid root-to-shard-to-token issuer
trust chain?

Expected answer: **no**.

## Method

This run refreshed the April 2026 token trust-chain audit against the current
implementation:

1. Located current delegated-token verification and runtime guard paths.
2. Compared them with the April baseline hotspots.
3. Traced trust-anchor resolution, root key validation, root signature
   verification, claims binding, shard signature verification, and runtime
   subject binding.
4. Checked role-attestation key verification because it shares root-issued auth
   material and capability proof paths.
5. Ran targeted unit and PocketIC tests for positive verification, invalid root
   signature, invalid shard signature, cert hash drift, noncanonical cert data,
   root pid binding, cascaded root-key propagation, and role-attestation
   rejection paths.

Commands used included:

- `find crates/canic-core/src/ops/auth crates/canic-core/src/access/auth crates/canic-core/src/api/auth crates/canic-tests/tests -path '*auth*' -o -path '*attestation*' | sort`
- `rg "verify_root|verify_shard|issuer|signature|certificate|cert_hash|root_sig|shard_sig|RootTrustAnchor|verify_delegated_token|verify_token|current_proof|proof_state|role_attestation|AttestationUnknownKeyId" crates/canic-core/src crates/canic-tests/tests canisters/test -n`
- `rg "trace_token_trust_chain|token_chain|proof_state|verify_delegation_signature|verify_token_sig|authenticated_guard_checks_current_proof" crates -n`
- `git log --name-only -n 30 -- crates/canic-core/src/ops/auth crates/canic-core/src/api/auth crates/canic-core/src/access/auth crates/canic-tests/tests/pic_role_attestation_cases`

## Comparability Notes

This report is only partially comparable with the April baseline:

- The April verifier hotspots `ops/auth/verify.rs`,
  `ops/auth/verify/token_chain.rs`, and `ops/auth/verify/proof_state.rs` are no
  longer the active implementation.
- The current delegated-token trust chain lives in
  `ops/auth/delegated/verify.rs`, `ops/auth/delegated/root_key.rs`,
  `ops/auth/token.rs`, and `access/auth/token.rs`.
- The old trace tests for current-proof ordering now match zero tests because
  delegated tokens are verified as self-contained credentials without local
  proof-store lookup.

## Current Trust-Chain Surface

| File / Module | Function | Role |
| --- | --- | --- |
| `crates/canic-core/src/ops/auth/token.rs` | `AuthOps::verify_token` | Runtime delegated-token verification entrypoint. |
| `crates/canic-core/src/ops/auth/token.rs` | `root_trust_anchor` | Resolves verifier-local root public key from subnet state. |
| `crates/canic-core/src/ops/auth/token.rs` | `verify_shard_key_binding` | Checks shard key name and derivation path binding. |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_delegated_token` | Pure delegated-token trust-chain verifier. |
| `crates/canic-core/src/ops/auth/delegated/root_key.rs` | `resolve_root_key` | Validates root pid, key identity, hash, algorithm, and time window. |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified` | Endpoint guard integration and caller/subject enforcement. |
| `crates/canic-core/src/ops/auth/attestation.rs` | `verify_role_attestation_cached` | Role-attestation signature, key id, and claim verification. |
| `crates/canic-core/src/api/auth/verify_flow.rs` | `verify_role_attestation_with_single_refresh` | Unknown-key refresh path; retries only after unknown key id. |

## Findings

| ID | Status | Severity | Area | Finding | Evidence |
| --- | --- | --- | --- | --- | --- |
| TTC-1 | PASS | High | Runtime entrypoint | Runtime token verification checks delegated auth is enabled, validates shard key binding, resolves root trust, and then calls the pure verifier. | `ops/auth/token.rs:69`, `ops/auth/token.rs:86`, `ops/auth/token.rs:90`, `ops/auth/token.rs:105` |
| TTC-2 | PASS | High | Root trust anchor | Root public key material is loaded from verifier-local subnet state for the configured key name. Token contents do not provide the trusted root key. | `ops/auth/token.rs:137`, `ops/auth/token.rs:147`, `ops/auth/token.rs:151` |
| TTC-3 | PASS | High | Root key validation | Root key resolution checks root principal, key id, key hash, algorithm, and validity window before returning a key. | `ops/auth/delegated/root_key.rs:30`, `root_key.rs:34`, `root_key.rs:41`, `root_key.rs:50` |
| TTC-4 | PASS | High | Root signature | Verification recomputes the canonical cert hash and verifies the root signature over that hash before claims acceptance. | `ops/auth/delegated/verify.rs:97`, `verify.rs:120` |
| TTC-5 | PASS | High | Claims binding | Token claims must bind back to the signed certificate through matching issuer shard pid and cert hash. | `ops/auth/delegated/verify.rs:168`, `verify.rs:171` |
| TTC-6 | PASS | High | Shard signature | Verification recomputes the canonical claims hash and verifies the shard signature using the certificate shard public key. | `ops/auth/delegated/verify.rs:130`, `verify.rs:131` |
| TTC-7 | PASS | High | Shard authority | Runtime verification checks the cert shard key binding against Canic's configured IC threshold ECDSA key name and shard derivation path. | `ops/auth/token.rs:165`, `token.rs:167`, `token.rs:168`, `token.rs:175` |
| TTC-8 | PASS | High | Guard binding | Endpoint guard integration decodes only the delegated token first, verifies it, then enforces token subject equals caller. | `access/auth/token.rs:21`, `access/auth/token.rs:43`, `access/auth/token.rs:52` |
| TTC-9 | PASS | Medium | Role attestations | Role-attestation verification rejects empty signatures, unknown key ids, invalid key windows, invalid signatures, and invalid claims. | `ops/auth/attestation.rs:57`, `attestation.rs:63`, `attestation.rs:68`, `attestation.rs:73`, `attestation.rs:76` |
| TTC-10 | PASS | Medium | Key refresh | Role-attestation verification refreshes keys once only for unknown key id, then verifies again. Other errors do not trigger refresh. | `api/auth/verify_flow.rs:36`, `verify_flow.rs:39`, `verify_flow.rs:41`, `verify_flow.rs:47` |
| TTC-11 | PASS | Low | Audit maintenance | The recurring audit definition and April baseline tests were stale after the verifier implementation moved. Replacement tests passed. | `rg "trace_token_trust_chain|token_chain|proof_state" crates -n`; test rollup below |

## Verification Readout

| Command | Result | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib verify_delegated_token_accepts_self_validating_token_without_proof_lookup -- --nocapture` | PASS | Positive self-contained delegated-token verifier path. |
| `cargo test -p canic-core --lib verify_delegated_token_rejects_root_signature_failure -- --nocapture` | PASS | Invalid root signature rejected. |
| `cargo test -p canic-core --lib verify_delegated_token_rejects_shard_signature_failure -- --nocapture` | PASS | Invalid shard signature rejected. |
| `cargo test -p canic-core --lib verify_delegated_token_rejects_cert_hash_drift -- --nocapture` | PASS | Claims/cert hash drift rejected. |
| `cargo test -p canic-core --lib verify_delegated_token_rejects_noncanonical_cert_vectors -- --nocapture` | PASS | Noncanonical cert vector rejected. |
| `cargo test -p canic-core --lib resolve_root_key_enforces_root_pid_binding_before_key_lookup -- --nocapture` | PASS | Root pid mismatch rejected before key acceptance. |
| `cargo test -p canic-tests --test root_suite delegated_token_verification_uses_cascaded_subnet_state_root_key -- --nocapture` | PASS | PocketIC root-key cascade path passed. |
| `cargo test -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture` | PASS | PocketIC role-attestation verifier rejection paths passed. |
| `cargo test -p canic-core --lib verify_role_attestation_cached_rejects_unknown_key_id -- --nocapture` | BLOCKED | Stale April test filter matched zero tests. |
| `cargo test -p canic-core --lib trace_token_trust_chain_stops_at_current_proof_before_signatures -- --nocapture` | BLOCKED | Stale April test filter matched zero tests. |

## Structural Hotspots

| File / Module | Risk Contribution | Status |
| --- | --- | --- |
| `ops/auth/token.rs` | Bridges runtime config, subnet state, ECDSA verification, and pure verifier. | Expected hotspot. |
| `ops/auth/delegated/verify.rs` | Owns pure delegated-token validation order. | Expected hotspot. |
| `ops/auth/delegated/root_key.rs` | Owns trusted root key identity and validity checks. | Small and focused. |
| `access/auth/token.rs` | Endpoint guard integration and subject binding. | Small and focused. |
| `ops/auth/attestation.rs` | Role-attestation key lookup and signature verification. | Expected hotspot. |

No structural violation was found in this run.

## Risk Score

Risk Score: **3 / 10**.

The trust-chain invariant holds and targeted tests pass. Residual risk is low
because the implementation moved since the April audit, making some old test
filters and audit hotspots stale. The recurring definition has been updated to
track the current verifier paths.

## Follow-up Actions

1. Keep delegated-token trust-chain tests named around current behavior, not the
   removed proof-store implementation.
2. On future delegated auth changes, rerun this audit together with
   `audience-target-binding` because root/shard trust and audience binding are
   now tightly coupled.
