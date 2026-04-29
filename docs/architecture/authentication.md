# Canic Authentication Design

- **Status:** canonical current design
- **Version line:** `0.29.4`
- **Audience:** Canic maintainers and downstream application developers
- **Primary rule:** auth is enforced at endpoints; workflow, ops, policy, DTO, and model code receive already-authenticated input.

This document is the current authentication design for Canic. Historical release-slice notes live under `docs/design/`; this file is the concise canonical handoff document.

## 1. Auth Surfaces

Canic has three auth surfaces:

1. Transport/topology predicates:
   - controllers
   - whitelist principals
   - parent/root/child/same-canister checks
   - registry role checks
2. Delegated-token endpoint auth:
   - caller supplies a `DelegatedToken`
   - endpoint guard validates token and binds `claims.subject == msg_caller`
   - endpoint-required scope must appear in token claims
3. Role attestation for root-mediated capability RPC:
   - root signs a role attestation for a registered canister subject
   - verifier checks the signed attestation and optional role epoch floor

Auth code lives at the boundary:

```text
endpoint macro / access guard
  -> access::auth / api::auth
  -> ops::auth
  -> storage key/session cache only where needed
```

Model, DTO, policy, and ordinary workflow code must not introduce hidden auth checks.

## 2. Delegated Token Trust Model

Delegated auth is self-validating.

```text
configured root principal
  + configured root ECDSA key
  -> root-signed DelegationCert
  -> root-certified shard public key
  -> shard-signed DelegatedTokenClaims
  -> authenticated delegated subject
```

A verifier validates a delegated token using only:

- the token
- the embedded `DelegationProof`
- configured root identity
- configured root ECDSA key name
- delegated root public key delivered through cascaded `SubnetState`
- shard public key embedded in the root-signed certificate
- local verifier principal and configured role
- IC canister time

A verifier must not require:

- local proof presence
- proof fanout from root
- creation-time proof catch-up
- proof history replication
- registry snapshots for delegated-token audience membership
- topology placement ordering
- query-time or first-use public-key fetching

In this document, "proof" means the embedded `DelegationProof` carried inside
`DelegatedToken`. It never means verifier-local proof state.

## 3. Data Structures

Source of truth: `crates/canic-core/src/dto/auth.rs`.

```rust
pub enum SignatureAlgorithm {
    EcdsaP256Sha256,
}

pub enum DelegationAudience {
    Roles(Vec<CanisterRole>),
    Principals(Vec<Principal>),
    RolesOrPrincipals {
        roles: Vec<CanisterRole>,
        principals: Vec<Principal>,
    },
}

pub enum ShardKeyBinding {
    IcThresholdEcdsa {
        key_name_hash: [u8; 32],
        derivation_path_hash: [u8; 32],
    },
}

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

pub struct DelegationProof {
    pub cert: DelegationCert,
    pub root_sig: Vec<u8>,
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

pub struct DelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub proof: DelegationProof,
    pub shard_sig: Vec<u8>,
}
```

### Field Authority

- `root_pid`, `root_key_id`, `root_key_hash`, `alg`: set by root and signed by root.
- `shard_pid`, `shard_key_id`, `shard_public_key_sec1`, `shard_key_hash`, `shard_key_binding`: set by root after resolving the shard threshold ECDSA public key, then signed by root.
- `cert.scopes`, `cert.aud`, `verifier_role_hash`, cert time fields, and `max_token_ttl_secs`: set by root and signed by root.
- `claims.subject`, `claims.aud`, `claims.scopes`, token time fields, and `nonce`: set by shard and signed by shard.
- `claims.cert_hash`: hash of canonical `DelegationCert`; set by shard and verified by every verifier.
- `claims.issuer_shard_pid`: must equal `cert.shard_pid`; it is descriptive and does not create separate authority.

`nonce` is informational entropy only. Core delegated auth is a bearer-token system and does not track nonce reuse. Replay protection is bounded by token TTL unless an application builds a separate replay store.

## 4. Canonical Encoding

Signed payloads use Canic's auth canonical encoding in `ops/auth/delegated/canonical.rs`, not Candid bytes and not serde bytes.

Canonical hashes:

```rust
cert_hash = sha256(canonical_bytes(DelegationCert))
claims_hash = sha256(canonical_bytes(DelegatedTokenClaims))
role_hash = sha256(canonical_role_bytes(CanisterRole))
public_key_hash = sha256(public_key_sec1)
key_name_hash = sha256(key_name.as_bytes())
derivation_path_hash = sha256(canonical_derivation_path_bytes(path))
```

Strict canonical rules:

- roles, scopes, and principal vectors must already be strictly sorted and duplicate-free
- role and scope strings must be non-empty ASCII strings using only `[a-z0-9_:-]`
- principal audiences must not contain the anonymous principal
- no `Any` audience exists
- verifier rejects noncanonical vectors rather than normalizing them at verification time

This is intentional: one semantic token must have one valid canonical encoding.

## 5. Root Certificate Issuance

Entrypoint path:

```text
DelegationApi::request_delegation
  -> root canic_request_delegation
  -> DelegationApi::issue_delegation_proof
  -> DelegatedTokenOps::sign_delegation_proof
```

Root issuance steps:

1. Require local canister is root.
2. Load `auth.delegated_tokens` config.
3. Resolve active root ECDSA public key from configured `ecdsa_key_name`.
4. Resolve target shard ECDSA public key using:
   - key name: `auth.delegated_tokens.ecdsa_key_name`
   - derivation path: `["canic", "shard", shard_pid_bytes]`
   - canister id: `shard_pid`
5. Build `DelegationCert`.
6. Enforce:
   - `cert.version == 2`
   - `cert.root_pid == self`
   - `cert.expires_at > cert.issued_at`
   - `cert_ttl <= auth.delegated_tokens.max_ttl_secs`
   - `cert.max_token_ttl_secs > 0`
   - `cert.max_token_ttl_secs <= cert_ttl`
   - audience is non-empty and canonical
   - role-targeted certificate has exact `verifier_role_hash`
   - `sha256(cert.shard_public_key_sec1) == cert.shard_key_hash`
   - `cert.shard_key_binding` equals the expected key-name and shard-derivation binding
7. Sign `cert_hash` with the root ECDSA path:

```text
["canic", "root"]
```

8. Return `DelegationProof`.

Root does not push the proof to verifiers.

Root does publish verifier trust material through `SubnetState`. That state is
separate from `AppState` and is the subnet-scoped cascade surface for data that
all canisters in one subnet need and that may change under root control.

Current delegated-auth `SubnetState` target:

```rust
pub struct SubnetStateRecord {
    pub auth: SubnetAuthStateRecord,
}

pub struct SubnetAuthStateRecord {
    pub delegated_root_public_key: Option<RootPublicKeyRecord>,
}

pub struct RootPublicKeyRecord {
    pub public_key_sec1: Vec<u8>,
    pub key_name: String,
    pub key_hash: [u8; 32],
}
```

`delegated_root_public_key` is not proof state. It is the root trust anchor
needed to verify `DelegationCert.root_sig`.

Root owns this value. The active code path is:

1. Root resolves the delegated root public key from current
   `auth.delegated_tokens.ecdsa_key_name`.
2. Root stores the identity-bound key record in `SubnetState`.
3. Root cascades `SubnetState` through the existing state propagation path.
4. Non-root verifier canisters store the cascaded `SubnetState`.
5. Access guards read the root trust anchor directly from local `SubnetState`.

If root changes the delegated auth key name or key material, it updates
`SubnetState`; normal subnet-state cascade is the synchronization mechanism.

Root publish entrypoints:

```text
canic_setup on root
  -> DelegationApi::publish_root_auth_material
  -> DelegatedTokenOps::publish_root_auth_material

state propagation / canister lifecycle snapshots
  -> RuntimeAuthWorkflow::publish_root_delegated_key_to_subnet_state
  -> DelegatedTokenOps::publish_root_delegated_key_material
```

`publish_root_auth_material` also warms root-owned role-attestation material.
Verifier canisters never call a delegated-root-key prewarm API.

## 6. Shard Token Minting

Entrypoint paths:

```text
DelegationApi::mint_token
  -> request root proof
  -> DelegationApi::issue_token

DelegationApi::issue_token
  -> DelegatedTokenOps::sign_token
```

Shard minting steps:

1. Require `proof.cert.shard_pid == self`.
2. Prepare `DelegatedTokenClaims`.
3. Enforce:
   - cert is currently valid
   - token TTL is greater than zero
   - token TTL does not exceed `cert.max_token_ttl_secs`
   - token expiry does not exceed cert expiry
   - token audience is a subset of cert audience
   - token scopes are a subset of cert scopes
   - claims are canonical
4. Sign `claims_hash` with the shard ECDSA path:

```text
["canic", "shard", self_pid_bytes]
```

5. Return `DelegatedToken`.

Minting does not depend on verifier creation order or registry propagation.

Signer lifecycle prewarm:

```text
non-root bootstrap
  -> RuntimeAuthWorkflow::prewarm_signer_key_material
  -> DelegatedTokenOps::local_shard_public_key_sec1
```

This warms local shard signing key material only. It does not request, store, or
fan out delegation proofs.

## 7. Verifier Algorithm

Endpoint delegated auth is reached through endpoint guards in `access::auth`.

Verifier steps:

1. Decode the first ingress argument as `DelegatedToken`.
2. Resolve the expected root public key from local state:
   - require `cert.root_pid == EnvOps::root_pid()`
   - require `cert.root_key_id == auth.delegated_tokens.ecdsa_key_name`
   - require local `SubnetState` contains the cascaded delegated root public key
   - the key may be used only when its identity matches current config and
     `cert.root_key_hash`
   - never fetch root threshold ECDSA public key from an access guard
   - reject cleanly if the key is missing or stale
3. Verify certificate policy:
   - version
   - configured root principal
   - cert time window
   - cert TTL policy
   - max token TTL policy
   - audience shape
   - verifier role hash
   - shard public-key hash
4. Resolve root trust anchor:
   - one configured root key only
   - unknown root keys are rejected
   - no embedded root key fallback
   - no root-key certificate authority
5. Verify root signature over `cert_hash`.
6. Verify shard key binding:
   - key name hash equals current delegated-token ECDSA key name hash
   - derivation path hash equals `["canic", "shard", cert.shard_pid]`
7. Verify claims:
   - `claims.issuer_shard_pid == cert.shard_pid`
   - `claims.cert_hash == cert_hash`
   - token window is valid
   - token does not outlive cert
   - token TTL does not exceed `cert.max_token_ttl_secs`
   - `claims.aud` is subset of `cert.aud`
   - `claims.scopes` is subset of `cert.scopes`
8. Verify shard signature over `claims_hash` using `cert.shard_public_key_sec1`.
9. Verify verifier audience membership:
   - principal branch: `self_pid` exact match
   - role branch: configured local role exact match
   - if any role branch is used, local role is required and must hash to `cert.verifier_role_hash`
10. Verify endpoint-required scopes are present in `claims.scopes`.
11. Verify transport caller binding:

```rust
claims.subject == ic_cdk::caller()
```

If all checks pass, the endpoint receives a delegated subject identity.

No step checks local proof presence.

Plain query, composite-query, and update guards share this same verification
path. A cold verifier without cascaded `SubnetState.auth.delegated_root_public_key`
must fail as a Canic auth error; it must not attempt an IC call from the guard.

## 8. Delegated Sessions

Delegated sessions allow a wallet caller to temporarily bind an authenticated delegated subject.

Entrypoint:

```text
DelegationApi::set_delegated_session_subject
```

Rules:

- delegated token must verify through the same self-validating token path
- token subject must equal requested delegated subject
- wallet caller and delegated subject must not be infrastructure/canister principals rejected by `validate_delegated_session_subject`
- session expiry is clamped to:
  - token expiry
  - configured delegated-token max TTL
  - optional requested session TTL
- expiry boundary is strict: `now_secs >= expires_at` means expired
- bootstrap token fingerprint is stored to reject replay conflicts and allow idempotent same-session replay

Session storage is not delegated-token proof storage.

## 9. Role Attestation

Role attestation is separate from delegated-token proof validation. It is used for root-mediated capability RPC authorization.

Data:

```rust
pub struct RoleAttestation {
    pub subject: Principal,
    pub role: CanisterRole,
    pub subnet_id: Option<Principal>,
    pub audience: Option<Principal>,
    pub issued_at: u64,
    pub expires_at: u64,
    pub epoch: u64,
}

pub struct SignedRoleAttestation {
    pub payload: RoleAttestation,
    pub signature: Vec<u8>,
    pub key_id: u32,
}
```

Root signs role attestations with:

```text
["canic", "attestation", "root"]
```

Verifier behavior:

- verify attestation signature under the cached role-attestation key
- on unknown attestation key id, refresh the key set from root once and retry
- preserve root-provided `AttestationKey.key_name` and `AttestationKey.key_hash`
- do not retag root-provided key bytes with verifier-local config
- enforce subject, role, audience, subnet, time window, and minimum accepted epoch

The attestation key set is shared key state, not delegated-token proof state.

## 10. Shared Key State

Delegated root public key state lives in `SubnetState`, not in the auth key
cache:

```rust
SubnetState.auth.delegated_root_public_key.key_name == current_configured_key_name
sha256(SubnetState.auth.delegated_root_public_key.public_key_sec1)
  == SubnetState.auth.delegated_root_public_key.key_hash
```

Stable auth key caches still exist for shard and role-attestation material:

```rust
cache_hit => cached.key_name == current_configured_key_name
cache_hit => sha256(cached.public_key_sec1) == cached.key_hash
```

Applies to:

- `ShardPublicKeyRecord`
- `AttestationPublicKeyRecord`

Identity-less key-cache records are invalid state. The system does not keep legacy deserializers for old key-cache shapes.

Cache invalidation is deterministic:

```text
config key-name change -> cache miss -> immediate refetch at signing/attestation boundary
```

Delegated root trust-anchor invalidation is `SubnetState` based:

```text
root delegated-key config change -> root republishes SubnetState.auth
cascade reaches verifier          -> verifier accepts certs for that root key
cascade not yet received          -> verifier denies cleanly
```

No background delegated-root-key refresh loop, non-root delegated-root-key
prewarm API, TTL, verifier fallback authority, or proof-cache repair path
exists. Access guards do not perform key fetches.

## 11. Configuration

Delegated tokens:

```toml
[auth.delegated_tokens]
enabled = true
ecdsa_key_name = "test_key_1"
max_ttl_secs = 86400
```

Role attestation:

```toml
[auth.role_attestation]
ecdsa_key_name = "test_key_1"
max_ttl_secs = 300

[auth.role_attestation.min_accepted_epoch_by_role]
project_instance = 1
```

Security boundaries:

- `EnvOps::root_pid()` is the root identity trust boundary.
- `auth.delegated_tokens.ecdsa_key_name` defines the root and shard delegated-token ECDSA key family.
- `auth.role_attestation.ecdsa_key_name` defines role-attestation key material.
- verifier `local_role` config is trusted; a canister configured with the wrong role is compromised for delegated-auth purposes.

## 12. Removed Concepts

These concepts are not part of current Canic delegated auth:

- V1 delegated-token acceptance
- local verifier proof cache as an auth condition
- proof fanout from root to verifiers
- creation-time verifier proof catch-up
- proof equality matching
- root-key fallback from embedded token material
- verifier-side root-key fetch-on-verify
- query-time key fetch from `requires(auth::authenticated())`
- delegated root-key background warmup timers
- `RootKeyAuthority`
- root-key certificates
- implicit revocation by deleting proof/cache state
- relay envelope delegated auth

Authenticated endpoint guards require a first argument of type `DelegatedToken`.
Candid `Reserved` placeholders are not part of the current auth surface.

## 13. Failure Modes

Expected failures:

- disabled delegated auth config
- malformed Candid token argument
- noncanonical audience/scope/role/principal vectors
- unknown or mismatched root key
- missing or stale cascaded `SubnetState` root key
- root signature failure
- shard signature failure
- shard key binding mismatch
- certificate expired or not yet valid
- token expired or not yet valid
- token TTL exceeds cert or config policy
- audience subset failure
- verifier not in token/cert audience
- missing local role for role audience
- required scope missing
- token subject does not match transport caller
- delegated-session bootstrap replay conflict
- role-attestation unknown key after refresh
- role-attestation epoch below configured minimum

These failures are cryptographic, temporal, policy, or config failures. There is no "delegation proof miss" correctness failure in current Canic auth.

Operationally, a verifier created or upgraded before receiving the latest
`SubnetState` is temporarily unable to validate delegated tokens for that root
key. That is an explicit state-propagation failure, not a proof-replication
failure, and it must surface as a normal Canic auth denial.

## 14. Developer Checklist

When changing auth code:

- keep auth checks at endpoint/access/API auth boundaries
- use one canonical encoding implementation for mint and verify
- reject noncanonical vectors instead of sorting during verification
- preserve `now_secs >= expires_at` as the expiry boundary everywhere
- do not add verifier-local proof lookup
- do not add proof distribution as a correctness requirement
- do not retag root-provided attestation keys
- do not accept caller-provided arbitrary public keys
- update `docs/contracts/AUTH_DELEGATED_SIGNATURES.md` when wire structs or verification rules change
- update this document when trust boundaries or auth flows change
