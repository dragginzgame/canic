# 0.65 Status: Zero Management-ECDSA Auth Hard Cut

Last updated: 2026-06-11

## Purpose

This file is the implementation status log for the 0.65 auth hard-cut design.
The design document captures the intended contract; this file records what has
landed, what drifted, and what remains open.

Design: [0.65-design.md](0.65-design.md)

## Current State

Design correction pending: 0.65 is now zero-management-ECDSA auth, not
root-proof-only canister signatures.

Delegated-token root proof hard cut is locally closed, but delegated-token auth
is not closeout-clean while token issuer signatures remain
threshold-ECDSA-backed.

`SignedRoleAttestation` now uses root canister-signature prepare/get plus local
embedded-proof verification.

Delegated-grant capability proofs are not retained in the active protocol.
Standalone capability proof DTOs and proof-mode branches are removed. Token
grants remain `DelegatedRoleGrant` values inside delegation certs and
delegated-token claims.

## Audit Status

Audit status from the root-proof hard-cut remains useful but is no longer
sufficient for 0.65 closeout:

- active install, architecture, contract, and getting-started docs no longer
  instruct users to enable `auth-crypto`
- no active code contains `DelegationProof.root_sig`,
  `EcdsaP256Sha256`, or `sign_prepared_delegation_proof`
- residual `RootPublicKeyRecord` / `delegated_root_public_key` stable state and
  auth key publication helpers are removed from the active codebase
- the only active root `certified_data_set` call is the root
  canister-signature certified-data owner helper for the exact `"sig"` tree
  shape
- runtime sharding, scaling, and directory child-management RPCs no longer
  request one-shot role attestations; registered non-root parents use
  structural root capability proofs for `ThisCanister` child provisioning and
  direct-child upgrade/recycle

Required zero-ECDSA audit scan:

```text
rg -n 'sign_with_ecdsa|EcdsaOps::sign_bytes|auth-threshold-ecdsa-sign|ThresholdEcdsaSign|IcThresholdEcdsaSecp256k1'
```

Expected result:

- no normal-auth code matches
- no auth feature matches
- allowed matches only in explicit negative tests, historical docs, or non-auth
  external-chain modules that no auth code imports or calls

Current local result: this release-blocking audit is not yet clean. Delegated
token issuer certs, test fleet token issuance, and `SignedRoleAttestation`
verification no longer match the shard/root ECDSA signing path. Remaining
active-code matches are the public threshold-ECDSA feature definitions, the
isolated ECDSA ops module, historical replay external effect records/tests, and
legacy internal-invocation/attestation-key compatibility helpers. The checklist
below tracks their removal or conversion.

## Implementation Checklist

- [x] Remove delegated-auth protocol epoch/version fields for the hard-cut DTOs.
- [x] Ensure all 0.65 auth/replay protocol DTO TTLs use `_ns`, with checked
      config/API conversion from seconds only at the boundary.
- [x] Replace broad `auth-crypto` with precise auth features.
- [x] Make canister-signature creation/verification dependencies optional and
      feature-owned.
- [x] Add delegated-token verifier config for root canister id, raw IC root key,
      and network label.
- [x] Add startup trap when a delegated-token verifier lacks
      `auth-root-canister-sig-verify` or effective verifier trust anchors.
- [x] Define `root_canister_sig_verification_message(kind, payload_hash)`.
- [x] Add a golden test proving verifier passes
      `domain_len || domain || cert_hash`, not raw `cert_hash`.
- [x] Define `root_sig_seed(kind)` separately from `root_sig_domain(kind)`.
- [x] Add bounded positive delegated-token verification cache keyed by proof
      hash, claims hash, `issuer_proof_hash`, and caller; cache values contain
      only `valid_until_ns` and `verified_at_ns`; endpoint-specific
      authorization still runs after cache hit.
- [x] Add opaque issuer-signed `ext: Option<Vec<u8>>` to delegated-token issue
      requests and claims, include it in canonical claims hashing, and bound it
      to 4096 bytes.
- [x] Replace the legacy global `DelegationAudience::Canic` token audience with
      explicit canister, Canic-subnet, and project audiences, and require
      verifier-local audience context for token audience acceptance.
- [x] Bind signed token `ext` bytes into token-issue replay payload hashes.
- [x] Add issuer-proof DTOs, canonical `IssuerProof` hashing,
      `issuer_proof_binding_hash`, issuer canister-signature
      seed/domain/message helpers, and the future issuer-proof verifier cache
      key helper.
- [x] Add issuer canister-signature SignatureMap prepare/get/verify primitive
      behind issuer-specific create/verify feature gates.
- [x] Add persisted `ActiveDelegationProof` DTO/storage foundation with stable
      auth records and fail-closed active proof lookup outside the proof
      validity window.
- [x] Add active-delegation-proof install validation around the persisted active
      proof store.
- [x] Add pending-proof metadata and enforce `retrieval_expires_at_ns`.
- [x] Add overflow-safe time checks.
- [ ] Apply `AUTH_TIME_SKEW_ALLOWANCE_NS = 60_000_000_000` to delegated cert
      `not_before_ns`, delegated token `issued_at_ns`, and role-attestation
      `issued_at_ns` not-from-the-future checks while preserving strict expiry
      with no grace.
- [ ] Add verifier skew tests proving an issuer clock 30 seconds ahead of the
      verifier passes and an issuer clock 120 seconds ahead fails for delegated
      token and role-attestation verification.
- [ ] Remove caller-provided delegated-token nonce input and derive
      `DelegatedTokenClaims.nonce` issuer-side from caller, prepare operation
      id, subject, issuer, and selected cert hash without `raw_rand`.
- [ ] Add a source guard proving `prepare_delegated_token` contains no
      `raw_rand`, management-canister call, or `.await`.
- [ ] Add a source guard proving no root or token issuer module outside the auth
      certified-data owner calls `set_certified_data`.
- [ ] Add a negative verifier test proving a user token presented by a
      forwarding canister is rejected with `SubjectCallerMismatch`.
- [x] Replace `DelegationProof.root_sig` with `DelegationProof.root_proof`.
- [x] Add `RootProof::IcCanisterSignatureV1`.
- [x] Remove legacy threshold-ECDSA root-proof verification.
- [x] Remove root ECDSA key fields from `DelegationCert`.
- [x] Ensure delegated-token root proof verification does not read
      `SubnetState::delegated_root_public_key`.
- [x] Remove residual `RootPublicKeyRecord` / `delegated_root_public_key`
      stable state and publication helpers.
- [x] Rename auth/replay protocol timestamps and TTLs with `_ns` suffixes.
- [x] Specify `ic_root_public_key_raw`, not DER.
- [x] Add explicit `cert_id == cert_hash` naming in the spec; log/metric naming
      can be expanded after the release-critical hard cut.
- [x] Extend existing canonical auth encoding for the hard-cut DTOs.
- [x] Add root signature-map module.
- [x] Add certified-data owner helper.
- [x] Keep 0.65 root canister-signature certified data to exact `"sig"` shape.
- [x] Add prepare/get root proof endpoints.
- [x] Update `issue_token` to accept the new root proof.
- [x] Remove single-call fresh-proof `mint_token` from the normal auth surface.
- [x] Add canister-signature verification benchmarks.
- [x] Add delegated-token encoded-size and endpoint-decode benchmarks.
- [x] Add explicit revocation/TTL tradeoff.
- [x] Add deployment check: root canister-signature issuer is not on
      `cloud_engine`.
- [x] Remove old one-shot root ECDSA role-attestation and
      internal-invocation-proof issuance from normal auth.
- [x] Remove old inbound standalone capability proof DTOs and proof-mode
      branches.
- [x] Route non-root placement create/upgrade/recycle away from one-shot role
      attestations; structural child provision is limited to
      `CreateCanisterParent::ThisCanister`.
- [x] Remove the stale protected-internal client PIC case that required fresh
      root-issued internal-invocation proofs.
- [x] Replace delegated-token issuer ECDSA signature with zero-ECDSA issuer
      proof, preferably `IssuerProof::IcCanisterSignatureV1`.
- [x] Remove `auth-threshold-ecdsa-sign` and threshold-ECDSA public-key fetching
      from the normal auth feature graph.
- [x] Remove `IcThresholdEcdsaSecp256k1` issuer proof algorithm/binding from
      normal delegated-token auth DTOs.
- [x] Add issuer prepare/get delegated-token canister-signature flow.
- [x] Add `install_active_delegation_proof` endpoint.
- [x] Add issuer canister-signature local verification against issuer canister
      id plus raw IC root key.
- [x] Extend canister-signature issuer deployment checks to token issuer canisters
      as well as root issuers.
- [x] Add required `SignedRoleAttestation = RootCertified<RoleAttestation>`
      prepare/get root proof flow using `RootProof::IcCanisterSignatureV1`.
- [x] Add local `SignedRoleAttestation` verification against configured root
      canister id plus raw IC root key, with no root or management-canister call
      on the protected endpoint hot path.
- [x] Hard-fail/remove delegated-grant capability proofs from normal auth.
- [ ] Add zero-ECDSA normal-auth audit scan and tests proving no normal auth
      path reaches `sign_with_ecdsa`.
- [ ] Add verifier-module CI gate proving verification code has no `.await`,
      issuer client imports, or management-canister imports.
- [x] Update metrics/cost classes so normal auth has no `ThresholdEcdsaSign`
      class.
- [ ] Update Candid, endpoint macros, architecture/auth docs, and
      getting-started docs for zero-ECDSA auth.

Do not mark 0.65 closeout complete until:

- delegated-token root proof uses IC canister signatures
- delegated-token issuer proof uses `IssuerProof::IcCanisterSignatureV1`
- `SignedRoleAttestation` prepare/get and verification are implemented
- delegated grants no longer retain a normal auth ECDSA path
- zero-ECDSA scans/tests pass
- protected endpoints verify self-contained proofs locally with no root or
  management-canister calls
