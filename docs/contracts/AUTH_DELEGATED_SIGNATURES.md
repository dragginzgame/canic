# Auth Contract: Self-Validating Delegated Tokens

This document defines the delegated-token authentication model implemented in
Canic today.

## Trust Model

Canonical trust chain:

```text
configured root principal + root public key -> root certificate -> shard signature -> delegated token
```

- Root signs a `DelegationCert` for one `shard_pid`.
- Root certifies the shard public key inside the certificate.
- Root publishes the delegated root public key through cascaded `SubnetState`.
- Shard signs `DelegatedTokenClaims` for one delegated subject.
- Verifiers validate locally without proof distribution, proof fanout, topology
  catch-up, or verifier-local proof lookup.

## Canonical Payloads

Source: `crates/canic-core/src/dto/auth.rs`

```rust
pub struct DelegationCert {
    pub version: u16,
    pub root_pid: Principal,
    pub root_key_id: String,
    pub root_key_hash: [u8; 32],
    pub alg: SignatureAlgorithm,
    pub shard_pid: Principal,
    pub shard_key_id: String,
    pub shard_public_key_sec1: Vec<u8>,
    pub shard_key_hash: [u8; 32],
    pub shard_key_binding: ShardKeyBinding,
    pub issued_at: u64,
    pub expires_at: u64,
    pub max_token_ttl_secs: u64,
    pub scopes: Vec<String>,
    pub aud: DelegationAudience,
    pub verifier_role_hash: Option<[u8; 32]>,
}

pub struct DelegatedTokenClaims {
    pub version: u16,
    pub subject: Principal,
    pub issuer_shard_pid: Principal,
    pub cert_hash: [u8; 32],
    pub issued_at: u64,
    pub expires_at: u64,
    pub aud: DelegationAudience,
    pub scopes: Vec<String>,
    pub nonce: [u8; 16],
}

pub struct AttestationKey {
    pub key_id: u32,
    pub public_key: Vec<u8>,
    pub key_name: String,
    pub key_hash: [u8; 32],
    pub status: AttestationKeyStatus,
    pub valid_from: Option<u64>,
    pub valid_until: Option<u64>,
}
```

Signed structures use `CanicAuthCanonical`. Canonical vectors are strict:
roles, principals, and scopes must already be sorted and duplicate-free.

## Crypto Backend and Signing Rules

- Signing uses threshold ECDSA management APIs via `ops/ic/ecdsa.rs`.
- Signature verification is pure Rust (`k256`) and runs locally.
- Verification uses pre-hash verification over canonical SHA-256 hashes.

Deterministic derivation paths:

- root path: `["canic", "root"]`
- shard path: `["canic", "shard", shard_pid_bytes]`
- role-attestation path: `["canic", "attestation", "root"]`

Delegated-token signing domains are defined in
`crates/canic-core/src/ops/auth/delegated/canonical.rs`.

## Issuance Flow

### Root certificate issuance

1. Shard calls root through `AuthApi::request_delegation`.
2. Root validates:
   - delegated auth is enabled
   - root authority is the local canister
   - certificate TTL and token TTL policy
   - audience shape and role-hash binding
   - shard public-key hash and deterministic shard derivation binding
3. Root signs the canonical certificate hash.
4. Root returns a self-contained `DelegationProof`.

Root does not push proof state to verifiers.

Root does cascade root trust-anchor state to verifiers:

```rust
pub struct SubnetStateRecord {
    pub auth: SubnetAuthStateRecord,
}

pub struct SubnetAuthStateRecord {
    pub delegated_root_public_key: Option<RootPublicKeyRecord>,
}
```

`SubnetState` remains separate from `AppState`. App state controls app-mode
runtime behavior. Subnet state carries subnet-scoped shared control-plane data.
The delegated root public key is the first auth entry in `SubnetState` because
all verifier canisters in the subnet need it and root may change it when the
delegated-auth key config changes.

### Token minting

1. Shard obtains a `DelegationProof`.
2. Shard validates audience, scope, and TTL attenuation.
3. Shard signs canonical token claims with its deterministic shard ECDSA path.
4. Shard returns a self-contained `DelegatedToken`.

`AuthApi::mint_token` performs proof request and token signing in one
API call. `AuthApi::issue_token` signs from an explicit proof.

## Verification Contract

Verification entrypoint:

- `access::auth::authenticated(required_scope)`

Checks enforced before authorization:

- delegated token decodes from ingress first argument
- certificate and claim canonical hashes recompute successfully
- `cert.root_pid == EnvOps::root_pid()`
- `cert.root_key_id == auth.delegated_tokens.ecdsa_key_name`
- delegated root public key exists in cascaded local `SubnetState`
- delegated root public key identity matches configured key name and `cert.root_key_hash`
- root certificate signature verifies
- shard key binding matches configured key name and deterministic shard path
- `hash(cert.shard_public_key_sec1) == cert.shard_key_hash`
- shard token signature verifies under `cert.shard_public_key_sec1`
- `claims.issuer_shard_pid == cert.shard_pid`
- `claims.cert_hash == hash(cert)`
- certificate and token time windows are valid
- token does not outlive certificate or root token-TTL policy
- `claims.aud` is a subset of `cert.aud`
- `claims.scopes` is a subset of `cert.scopes`
- local principal or configured local role is in `claims.aud`
- role audiences match `cert.verifier_role_hash`
- endpoint required scopes are present in `claims.scopes`
- delegated session subject binding is enforced before replacing caller identity

No verification step checks for local proof presence.

No verification step may fetch root key material. Plain queries, composite
queries, and updates use the same local root trust-anchor lookup. If the
`SubnetState` cascade has not populated the delegated root public key yet,
verification fails cleanly as root key unavailable.

## Shared Key State Contract

Delegated root public key state is subnet-scoped control-plane state:

```rust
SubnetState.auth.delegated_root_public_key.key_name == configured_key_name
sha256(SubnetState.auth.delegated_root_public_key.public_key_sec1)
  == SubnetState.auth.delegated_root_public_key.key_hash
```

Stable key caches remain caches, not authority, for shard and role-attestation
material:

```rust
cache_hit => cached.key_name == configured_key_name
cache_hit => sha256(cached.public_key_sec1) == cached.key_hash
```

This applies to:

- `ShardPublicKeyRecord`
- `AttestationPublicKeyRecord`

Role-attestation key-set refresh preserves root-provided `AttestationKey`
identity. A verifier must not retag root-provided attestation key bytes with
its own local key name.

Identity-less key-cache records are invalid state.

For delegated-token verification, `SubnetState` is the only supported
distribution mechanism for the root public key. Verifier-side first-use ECDSA
public-key fetches and background delegated-root-key warmup loops are not part
of the contract.

## Endpoint Type Contract

### Direct endpoints

- Supported model.
- Token subject must bind to transport caller before delegated session state is
  accepted.

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

- verifier-local proof lookup as an acceptance condition
- proof distribution as an authentication correctness requirement
- verifier-side root-key fetch-on-verify
- query-time key fetch from an authenticated access guard
- delegated-root-key background warmup timers
- implicit revocation by deleting proof or cache state
- two-step signature materialization (`prepare`/`get`)
- detached verification with caller-supplied arbitrary public keys
- endpoint APIs that return raw signatures for later submission
- relay envelope auth modes (`AuthenticatedRequest`/`presenter_pid`)
- auth paths that skip delegated subject binding

Allowed internal use:

- `sign_with_ecdsa` / `ecdsa_public_key` only inside the IC ECDSA ops facade
  and call paths that perform root/shard signing or key caching.

## Test-Only Paths

The following are explicitly test-only or demo-local:

- `user_shard_issue_token` in `canisters/demo/user_shard/src/lib.rs`
- `create_account` in `canisters/demo/user_hub/src/lib.rs`
- `plan_create_account` in `canisters/demo/user_hub/src/lib.rs`
