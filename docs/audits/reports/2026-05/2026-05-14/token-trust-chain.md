# Token Trust Chain Invariant Audit - 2026-05-14

## Report Preamble

- Scope: delegated-token trust-chain verification, verifier-local root trust
  anchor resolution, shard key binding, role-attestation key verification, and
  delegated-token endpoint guard integration.
- Definition path: `docs/audits/recurring/invariants/token-trust-chain.md`
- Compared baseline report path: `N/A` (first `token-trust-chain` run on
  2026-05-14)
- Prior reference report path:
  `docs/audits/reports/2026-05/2026-05-07/token-trust-chain.md`
- Code snapshot identifier: `48213853`
- Method tag/version: `Method V4.3`
- Comparability status: `comparable` with the 2026-05-07 current-verifier
  method; same-day baseline deltas are `N/A`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-14T18:35:32Z`
- Branch: `main`
- Worktree before report write: `clean`

## Executive Summary

Verdict: **PASS**.

The delegated-token trust-chain invariant still holds. Token acceptance requires:

- delegated-token auth enabled in local config;
- shard key binding to Canic's configured delegated-token key name and shard
  derivation path;
- verifier-local root public key material from subnet state, not token input;
- root principal, key id, key hash, algorithm, and key-window checks;
- root signature over the canonical delegation certificate hash;
- token claims bound to the signed certificate by issuer shard principal and
  cert hash;
- shard signature over canonical token claims;
- endpoint guard subject/caller binding and update-call replay consumption.

No trust-chain bypass or untrusted issuer acceptance path was found. Residual
risk is structural: the runtime entrypoint remains a sensitive hotspot, and
auth DTOs such as `RoleAttestation`, `SignedRoleAttestation`, and
`DelegatedToken` have broad, expected fan-in across runtime, tests, and support
canisters.

Risk score: **4 / 10**.

## Audit Question

Can a delegated token be accepted without a valid root-to-shard-to-token issuer
trust chain?

Expected answer: **no**. Current answer: **no bypass found**.

## Method

This run used the current recurring definition and refreshed the 2026-05-07
current-verifier audit against snapshot `48213853`.

Commands used included:

- `find crates/canic-core/src/ops/auth crates/canic-core/src/access/auth crates/canic-core/src/api/auth crates/canic-tests/tests canisters/test -path '*auth*' -o -path '*attestation*'`
- `rg "verify_root|verify_shard|issuer|signature|certificate|cert_hash|root_sig|shard_sig|RootTrustAnchor|verify_delegated_token|verify_token|current_proof|proof_state|role_attestation|AttestationUnknownKeyId" crates/canic-core/src crates/canic-tests/tests canisters/test -n`
- `rg "fn verify_delegated_token_|fn resolve_root_key_|fn role_attestation_verification_paths|fn delegated_token_verification_uses_cascaded_subnet_state_root_key" crates/canic-core/src crates/canic-tests/tests -n`
- `git log --name-only -n 30 -- crates/canic-core/src/ops/auth crates/canic-core/src/api/auth crates/canic-core/src/access/auth crates/canic-tests/tests/pic_role_attestation_cases`
- `wc -l crates/canic-core/src/ops/auth/token.rs crates/canic-core/src/ops/auth/delegated/verify.rs crates/canic-core/src/ops/auth/delegated/root_key.rs crates/canic-core/src/access/auth/token.rs crates/canic-core/src/ops/auth/attestation.rs crates/canic-core/src/api/auth/verify_flow.rs`

## Comparability Notes

- Comparable with the 2026-05-07 report's current verifier method. Both runs
  inspect `ops/auth/token.rs`, `ops/auth/delegated/verify.rs`,
  `ops/auth/delegated/root_key.rs`, `access/auth/token.rs`, and
  role-attestation verification.
- Not directly comparable with older April reports that referenced removed
  proof-store/current-proof trace helpers.
- Same-day baseline deltas are `N/A` because this is the first
  `token-trust-chain` report on 2026-05-14.

## Audit Checklist

| Check | Status | Evidence |
| --- | --- | --- |
| Trust anchor is verifier-local | PASS | `AuthOps::root_trust_anchor` loads the delegated root public key from `SubnetStateOps::delegated_root_public_key` after matching the configured key name in `crates/canic-core/src/ops/auth/token.rs:167`. |
| Root identity and key metadata are checked | PASS | `resolve_root_key` checks root principal, key id, key hash, algorithm, and key validity window before returning a key in `crates/canic-core/src/ops/auth/delegated/root_key.rs:30`. |
| Root signature binds the delegation certificate | PASS | `verify_delegated_token` recomputes `cert_hash`, rejects hash drift, and verifies `proof.root_sig` over that hash in `crates/canic-core/src/ops/auth/delegated/verify.rs:97` and `crates/canic-core/src/ops/auth/delegated/verify.rs:120`. |
| Claims bind back to the signed certificate | PASS | `verify_claims` rejects issuer shard principal mismatch and cert hash mismatch in `crates/canic-core/src/ops/auth/delegated/verify.rs:161`. |
| Shard signature binds token claims | PASS | The verifier recomputes `claims_hash` and verifies `token.shard_sig` using the certificate shard key in `crates/canic-core/src/ops/auth/delegated/verify.rs:130`. |
| Runtime shard key binding matches Canic config | PASS | `verify_shard_key_binding` compares configured key-name hash and shard derivation path hash in `crates/canic-core/src/ops/auth/token.rs:199`. |
| Endpoint guard enforces subject/caller binding | PASS | `delegated_token_verified` verifies the token, then calls `enforce_subject_binding` and update replay consumption in `crates/canic-core/src/access/auth/token.rs:22`. |
| Role-attestation signatures use cached trusted keys | PASS | `verify_role_attestation_cached` rejects empty signatures, unknown key ids, invalid key windows, invalid signatures, and invalid claims in `crates/canic-core/src/ops/auth/attestation.rs:49`. |
| Unknown attestation key refresh is bounded | PASS | `verify_role_attestation_with_single_refresh` refreshes only on `AttestationUnknownKeyId` and verifies once more in `crates/canic-core/src/api/auth/verify_flow.rs:27`. |

## Current Trust-Chain Surface

| File / Module | Function / Type | Role |
| --- | --- | --- |
| `crates/canic-core/src/ops/auth/token.rs` | `AuthOps::verify_token` | Runtime delegated-token verification entrypoint. |
| `crates/canic-core/src/ops/auth/token.rs` | `root_trust_anchor` | Resolves verifier-local root public key from subnet state. |
| `crates/canic-core/src/ops/auth/token.rs` | `verify_shard_key_binding` | Checks shard key name and derivation path binding. |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_delegated_token` | Pure delegated-token trust-chain verifier. |
| `crates/canic-core/src/ops/auth/delegated/root_key.rs` | `resolve_root_key` | Validates root pid, root key identity, and root key validity window. |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified` | Endpoint guard integration, caller/subject binding, and update replay consumption. |
| `crates/canic-core/src/ops/auth/attestation.rs` | `verify_role_attestation_cached` | Role-attestation key lookup, signature verification, and claim checks. |
| `crates/canic-core/src/api/auth/verify_flow.rs` | `verify_role_attestation_with_single_refresh` | One-refresh unknown-key path. |

## Findings

| ID | Status | Severity | Area | Finding | Evidence |
| --- | --- | --- | --- | --- | --- |
| TTC-20260514-1 | PASS | High | Runtime entrypoint | Runtime verification rejects disabled delegated auth, validates shard key binding, resolves root trust, and delegates to the pure verifier before returning a verified token. | `crates/canic-core/src/ops/auth/token.rs:73`, `token.rs:90`, `token.rs:94`, `token.rs:109` |
| TTC-20260514-2 | PASS | High | Root trust anchor | Root public key material is read from verifier-local subnet state keyed by configured key name; token contents do not become the root trust source. | `crates/canic-core/src/ops/auth/token.rs:171`, `token.rs:181`, `token.rs:185` |
| TTC-20260514-3 | PASS | High | Signature chain | Certificate hash, root signature, claims binding, claims hash, and shard signature are all checked in sequence. | `crates/canic-core/src/ops/auth/delegated/verify.rs:97`, `verify.rs:120`, `verify.rs:128`, `verify.rs:130` |
| TTC-20260514-4 | PASS | High | Root key resolution | Root pid mismatch, unknown root key metadata, and invalid root key windows fail before key acceptance. | `crates/canic-core/src/ops/auth/delegated/root_key.rs:30`, `root_key.rs:41`, `root_key.rs:50` |
| TTC-20260514-5 | PASS | High | Endpoint boundary | Endpoint token verification is followed by subject binding, required-scope checks, and update-call single-use consumption. | `crates/canic-core/src/access/auth/token.rs:52`, `token.rs:61`, `token.rs:62`, `token.rs:63` |
| TTC-20260514-6 | PASS | Medium | Role attestations | Attestation verification remains key-id, key-window, signature, and claims gated; unknown key refresh is single-shot and narrow. | `crates/canic-core/src/ops/auth/attestation.rs:57`, `attestation.rs:63`, `attestation.rs:68`, `attestation.rs:73`, `crates/canic-core/src/api/auth/verify_flow.rs:36` |
| TTC-20260514-7 | PASS | Low | Audit drift | Stale proof-store/current-proof trace names are absent, matching the current self-contained token verifier model from the 2026-05-07 audit. | `rg "trace_token_trust_chain|token_chain|proof_state|verify_delegation_signature|verify_token_sig|authenticated_guard_checks_current_proof" crates -n` returned no matches. |

## Structural Hotspots

| File / Module | Lines | Public Items | Recent Commits (30d) | Risk Contribution | Status |
| --- | ---: | ---: | ---: | --- | --- |
| `crates/canic-core/src/ops/auth/token.rs` | 287 | 4 | 7 | Bridges runtime config, subnet state, shard binding, ECDSA verification, metrics, and token-use replay. | Expected hotspot. |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | 524 | 4 | 2 | Owns pure delegated-token validation order plus tests. | Expected hotspot, test-heavy. |
| `crates/canic-core/src/ops/auth/delegated/root_key.rs` | 171 | 3 | 2 | Owns root trust-anchor identity and window checks. | Focused. |
| `crates/canic-core/src/access/auth/token.rs` | 269 | 3 | 6 | Endpoint guard boundary for delegated-token decoding, subject binding, and update replay. | Expected hotspot. |
| `crates/canic-core/src/ops/auth/attestation.rs` | 87 | 0 | 2 | Focused role-attestation key/signature/claim verifier. | Low structural pressure. |
| `crates/canic-core/src/api/auth/verify_flow.rs` | 98 | 3 | 1 | Single-refresh wrapper for role-attestation unknown-key handling. | Low structural pressure. |

No structural violation was found. The main hotspot remains
`ops/auth/token.rs` because it necessarily joins runtime state, trusted root
material, signature verification, metrics, and replay consumption.

## Hub Module Pressure

| Module | Import / Reference Count | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | ---: | --- | ---: | ---: |
| `dto::auth` | 30 files | `api`, `access`, `ops`, `storage`, `tests`, `canisters/test` | 5 | 7 |
| `ops::auth` | 6 files | `api`, `access`, `ops`, `tests` | 3 | 5 |
| `ops::auth::delegated` | 4 files | `ops`, `tests` | 1 | 3 |
| `api::auth` | 4 files | `api`, `tests`, `canisters/test` | 2 | 3 |
| `access::auth` | 3 files | `access`, `tests` | 1 | 2 |

`dto::auth` is intentionally broad boundary data, but it is still the highest
fan-in pressure surface for this invariant. That pressure is structural, not a
detected validation bypass.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| Auth DTO fan-in center | `crates/canic-core/src/dto/auth.rs` | `dto::auth` references in 30 files across API/access/ops/storage/tests/test canisters. | Medium |
| Runtime verifier edit pressure | `crates/canic-core/src/ops/auth/token.rs` | 7 commits in the last 30 days touching the runtime verifier entrypoint. | Medium |
| Guard-boundary edit pressure | `crates/canic-core/src/access/auth/token.rs` | 6 commits in the last 30 days touching endpoint delegated-token guard logic. | Medium |
| Removed proof-store trace names | `crates/` | No current matches for stale proof-store/current-proof trace names. | Low |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `VerifyDelegatedTokenError` | `crates/canic-core/src/ops/auth/delegated/verify.rs` | 3 | Low |
| `RootKeyResolutionError` | `crates/canic-core/src/ops/auth/delegated/root_key.rs` | 2 | Low |

No enum shock-radius signal was detected.

### Cross-Layer Struct Spread

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `RoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 16 files | Medium |
| `SignedRoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 15 files | Medium |
| `DelegatedToken` | `crates/canic-core/src/dto/auth.rs` | 14 files | Medium |
| `DelegationCert` | `crates/canic-core/src/dto/auth.rs` | 7 files | Low |
| `RootTrustAnchor` | `crates/canic-core/src/dto/auth.rs` | 5 files | Low |

DTO spread is expected for shipped boundary data, but `RoleAttestation`,
`SignedRoleAttestation`, and `DelegatedToken` should remain passive DTOs and
must not accumulate behavior.

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `crates/canic-core/src/ops/auth/token.rs` | config, env, IC, storage, metrics, delegated verifier | 7 | Medium |
| `crates/canic-core/src/access/auth/token.rs` | access, Candid decode, auth ops, config, IC | 6 | Medium |

### Capability Surface Growth

| Module | Public Items | Risk |
| --- | ---: | --- |
| `crates/canic-core/src/ops/auth/token.rs` | 4 | Low |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | 4 | Low |
| `crates/canic-core/src/ops/auth/delegated/root_key.rs` | 3 | Low |
| `crates/canic-core/src/access/auth/token.rs` | 3 | Low |
| `crates/canic-core/src/api/auth/verify_flow.rs` | 3 | Low |

No capability surface growth signal exceeded the audit threshold.

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `dto::auth` | 30 | `api`, `access`, `ops`, `storage`, `tests`, `canisters/test` | High |
| `ops::auth` | 6 | `api`, `access`, `ops`, `tests` | Rising pressure |
| `ops::auth::delegated` | 4 | `ops`, `tests` | Low |

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `RoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 16 | Medium |
| `SignedRoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 15 | Medium |
| `DelegatedToken` | `crates/canic-core/src/dto/auth.rs` | 14 | Medium |
| `DelegationCert` | `crates/canic-core/src/dto/auth.rs` | 7 | Low |

## Risk Score

Risk Score: **4 / 10**.

Derivation:

- `+2` for sensitive hotspot concentration in `ops/auth/token.rs` and
  `access/auth/token.rs`.
- `+1` for broad auth DTO fan-in across multiple subsystems.
- `+1` for recent edit pressure in the runtime verifier and endpoint guard.
- `+0` for confirmed trust-chain validation breaks; none found.
- `+0` for public capability surface growth above threshold; none found.

Interpretation: low risk. The invariant holds, and the remaining risk is
review-cost pressure around central auth DTOs and the runtime verifier
entrypoint.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git status --short` | PASS | Worktree was clean before creating this report. |
| `git rev-parse --short HEAD` | PASS | Captured snapshot `48213853`. |
| `git branch --show-current` | PASS | Captured branch `main`. |
| `cargo test -p canic-core --lib verify_delegated_token -- --nocapture` | PASS | 10 delegated-token verifier tests passed, including valid token, root signature failure, shard signature failure, cert hash drift, noncanonical cert/claims, audience drift, missing local role, required scope, and expiry boundary. |
| `cargo test -p canic-core --lib resolve_root_key -- --nocapture` | PASS | 4 root-key resolver tests passed, including root pid binding, unknown key, explicit trusted key, and validity window. |
| `cargo test -p canic-tests --test root_suite delegated_token_verification_uses_cascaded_subnet_state_root_key -- --nocapture` | PASS | PocketIC root-key cascade path passed after building local ICP artifacts. |
| `cargo test -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture` | PASS | PocketIC role-attestation verification paths passed, including subject, audience, epoch, and expiry rejection evidence. |
| `rg "trace_token_trust_chain|token_chain|proof_state|verify_delegation_signature|verify_token_sig|authenticated_guard_checks_current_proof" crates -n` | PASS | No stale proof-store/current-proof trace path remains in active crates. |

## Follow-up Actions

No follow-up actions required.

Watchpoints for future auth changes:

1. Keep `dto::auth` passive boundary data only; do not move verifier behavior
   into DTO types.
2. Keep runtime delegated-token verification centralized in
   `AuthOps::verify_token`.
3. Rerun `audience-target-binding` with this audit after any delegated-token
   audience, role-attestation, or endpoint auth macro change.
