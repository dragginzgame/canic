# Auth Contract: Root-Anchored Delegated ECDSA Tokens

This document defines the delegated-token authentication model implemented in
Canic today.

## Trust Model

Canonical trust chain:

```text
IC root key -> root canister -> shard canister -> user-bound delegated token
```

- Root signs a `DelegationCert` for one `shard_pid`.
- Shard signs `DelegatedToken` claims for one user (`sub`).
- Verifiers validate locally without inter-canister auth round-trips.

## Canonical Payloads

Source: `crates/canic-core/src/dto/auth.rs`

```rust
pub struct DelegationCert {
    pub root_pid: Principal,
    pub shard_pid: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
    pub scopes: Vec<String>,
    pub aud: Vec<Principal>,
}

pub struct DelegatedTokenClaims {
    pub sub: Principal,
    pub shard_pid: Principal,
    pub scopes: Vec<String>,
    pub aud: Vec<Principal>,
    pub iat: u64,
    pub exp: u64,
}
```

`sub` is explicit and required.

## Crypto Backend and Signing Rules

- Signing uses threshold ECDSA management APIs via `ops/ic/ecdsa.rs`:
  - `sign_with_ecdsa`
  - `ecdsa_public_key`
- Signature verification is pure Rust (`k256`) and runs locally:
  - no management canister calls during token verification
  - pre-hash verification (`verify_prehash`) over a single SHA-256 hash

Deterministic derivation paths:
- root path: `["canic", "root"]`
- shard path: `["canic", "shard", shard_pid_bytes]`

Domain-separated hashing:
- cert domain: `CANIC_DELEGATION_CERT_V1`
- token domain: `CANIC_DELEGATED_TOKEN_V1`

Token signature preimage includes:
- all `DelegatedTokenClaims`
- `cert_hash` (SHA-256 over domain-separated, candid-encoded `DelegationCert`)

Implementation: `crates/canic-core/src/ops/auth.rs`

## Issuance and Provisioning Flow

### Canonical flow (shard-initiated)
1. Shard calls root: `DelegationApi::request_delegation`.
2. Root verifies:
   - delegated auth feature enabled
   - `self == root_pid`
   - `caller == request.shard_pid`
   - cert policy constraints (expiry, aud/scopes non-empty, shard registered, etc.)
3. Root signs delegation cert in one step (`sign_delegation_cert`).
4. Root provisions proof to signer/verifier targets via workflow.

### Token minting
- Shard loads stored proof (`DelegationApi::require_proof`).
- Shard signs token in one step (`DelegatedTokenOps::sign_token`).

No prepare/get two-step signature API is used.

## Verification Contract (Verifier Canister)

Verification entrypoint:
- `access::auth::authenticated(required_scope)`

Checks enforced before authorization:
- delegated token decodes from ingress first argument
- proof/cert structure valid
- cert root authority matches expected root principal
- cert signature valid under cached root public key
- cert temporal validity (`issued_at < expires_at`, not expired)
- token signature valid under cached shard public key
- `claims.shard_pid == cert.shard_pid`
- `claims.aud` subset of `cert.aud`
- `claims.scopes` subset of `cert.scopes`
- verifier audience binding: `self_pid in claims.aud`
- proof matches verifier’s locally stored current proof
- subject binding: `claims.sub == ic_cdk::caller()`
- endpoint binding: required scope is present in `claims.scopes`

## Endpoint Type Contract

### Direct endpoints
- Supported model.
- Token subject must bind to transport caller (`sub == caller`).

### Relayed endpoints
- Not supported.
- No `presenter_pid` contract.
- No relay envelope auth mode.

## Root Authority Contract

Root authority source:
- `EnvOps::root_pid()`

Immutability:
- `EnvOps::import` treats `root_pid` as write-once.
- Re-import with a different `root_pid` is rejected.

## Forbidden Patterns

The auth architecture must not introduce:
- two-step signature materialization (`prepare`/`get`)
- detached verification with caller-supplied arbitrary public keys
- endpoint APIs that return raw signatures for later submission
- relay envelope auth modes (`AuthenticatedRequest`/`presenter_pid`)
- auth paths that skip `sub == caller`

Allowed internal use:
- `sign_with_ecdsa` / `ecdsa_public_key` only inside the IC ECDSA ops façade
  and call paths that perform root/shard signing or key caching.

## Test-Only Paths

The following are explicitly test-only and guarded in non-debug builds:
- `user_shard_mint_token` in `crates/canisters/user_shard/src/lib.rs`
- `register_principal` in `crates/canisters/shard_hub/src/lib.rs`

Admin provisioning API (`DelegationApi::provision`) is retained as a
root-only tooling/test escape hatch and is not the canonical user auth flow.
