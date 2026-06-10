# Auth Contract: Self-Validating Delegated Tokens

This document defines the current hard-cut delegated-token authentication
contract implemented by Canic.

## Trust Model

Canonical trust chain:

```text
configured root principal
  + configured raw IC root public key
  -> embedded root canister-signature proof
  -> root-certified DelegationCert
  -> shard secp256k1 signature
  -> reusable DelegatedToken
```

Root no longer signs `DelegationCert` with root threshold ECDSA. Verifiers do
not read `SubnetState.auth.delegated_root_public_key` when verifying a delegated
token root proof. A verifier validates locally from the token, configured root
principal, configured or runtime IC root public key, local project/role config,
and current IC time.

Delegated tokens are bearer tokens. A valid token is not consumed by
verification and may authorize multiple update or query calls until
`claims.expires_at_ns`.

## Canonical Payloads

Source: `crates/canic-core/src/dto/auth.rs`.

```rust
pub enum DelegationAudience {
    Canic,
    Project(String),
}

pub struct DelegatedRoleGrant {
    pub target: CanisterRole,
    pub scopes: Vec<String>,
}

pub enum ShardKeyBinding {
    IcThresholdEcdsaSecp256k1 {
        key_name_hash: [u8; 32],
        derivation_path_hash: [u8; 32],
    },
}

pub enum ShardSignatureAlgorithm {
    IcThresholdEcdsaSecp256k1,
}

pub enum RootProof {
    IcCanisterSignatureV1(IcCanisterSignatureProofV1),
}

pub struct IcCanisterSignatureProofV1 {
    pub signature_cbor: Vec<u8>,
    pub public_key_der: Vec<u8>,
}

pub struct DelegationCert {
    pub root_pid: Principal,
    pub shard_pid: Principal,
    pub shard_key_id: String,
    pub shard_sig_alg: ShardSignatureAlgorithm,
    pub shard_public_key_sec1: Vec<u8>,
    pub shard_key_hash: [u8; 32],
    pub shard_key_binding: ShardKeyBinding,
    pub issued_at_ns: u64,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub max_token_ttl_ns: u64,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
}

pub struct DelegationProof {
    pub cert: DelegationCert,
    pub root_proof: RootProof,
}

pub struct DelegatedTokenClaims {
    pub subject: Principal,
    pub issuer_shard_pid: Principal,
    pub cert_hash: [u8; 32],
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub nonce: [u8; 16],
}

pub struct DelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub proof: DelegationProof,
    pub shard_sig: Vec<u8>,
}
```

All protocol timestamps are nanoseconds since Unix epoch. Human-facing config
may use seconds, but conversion happens at the boundary. Protocol structs must
not contain `*_secs` fields.

## Canonical Hashes

Signed structures use `CanicAuthCanonical` rules from
`ops/auth/delegated/canonical.rs`.

```text
cert_hash     = sha256(canonical_bytes(DelegationCert))
claims_hash   = sha256(canonical_bytes(DelegatedTokenClaims))
shard_key_hash =
  sha256("canic-shard-key-v1" || shard_sig_alg ||
         canonical_bytes(shard_public_key_sec1) ||
         canonical_bytes(shard_key_binding))
```

Grant vectors must be strictly sorted by role and duplicate-free. Scope vectors
inside each grant must be strictly sorted and duplicate-free. Verifiers reject
noncanonical payloads instead of normalizing them.

## Root Canister Signature

Root proof creation uses the IC canister-signature helper with a single
certified-data tree under the `"sig"` label.

Creation input:

```rust
CanisterSigInputs {
    seed: b"canic-root-delegation-cert",
    domain: b"canic-root-delegation-cert",
    message: &cert_hash,
}
```

The signature map stores the hash of:

```text
domain_len || domain || cert_hash
```

Verification therefore passes those exact message bytes to
`ic_signature_verification::verify_canister_sig`, not raw `cert_hash`:

```rust
verify_canister_sig(
    domain_len || domain || cert_hash,
    signature_cbor,
    public_key_der,
    ic_root_public_key_raw,
)
```

`ic_root_public_key_raw` is the 96-byte raw IC BLS root key. TOML may encode it
as hex, and local/PocketIC tests must use the local root key instead of silently
using the mainnet key.

## Issuance Flow

Root proof issuance is update-then-query because IC certified data is committed
during update execution and the data certificate is available during query
execution.

```text
1. shard/client -> root canic_prepare_delegation_proof update
2. shard/client -> root canic_get_delegation_proof query
3. caller       -> shard issue_token update with DelegationProof
4. caller       -> endpoint with DelegatedToken
```

`canic_prepare_delegation_proof` is replay-protected. The same caller,
operation id, and payload returns the same prepared response; the same caller
and operation id with a different payload is a replay conflict. The first fresh
prepare adds one signature-map entry, refreshes certified data once, and stores
pending proof metadata.

`canic_get_delegation_proof` is not separately replay-protected. It is a query
over an existing pending proof. The caller must match the preparing caller, the
requested `cert_hash` must match pending metadata, and
`now_ns < retrieval_expires_at_ns`.

The normal auth surface does not expose a one-shot fresh-proof `mint_token`
path. Client/test helpers may choreograph the three calls above, but they must
not hide query certificate retrieval inside one shard update.

## Shard Token Signature

Shard token signatures remain threshold ECDSA secp256k1 for this slice.

```text
shard_token_hash =
  sha256(domain_len ||
         b"canic-shard-delegated-token" ||
         canonical_bytes(DelegatedTokenClaims))
```

Threshold ECDSA signs `shard_token_hash`. Verifiers require a compressed SEC1
secp256k1 shard public key of 33 bytes and verify the fixed secp256k1 signature
bytes through the local ECDSA verifier.

The 0.65 hard cut removes the root threshold ECDSA signing cost from delegated
root proof issuance. If shards mint a fresh delegated token for every endpoint
request, shard threshold ECDSA signing can still dominate cost. The expected
usage model is to reuse root proofs and delegated tokens for their TTL.

## Verification Contract

Verification entrypoint:

- `access::auth::authenticated(required_scope)`

Checks before authorization:

- delegated token decodes from ingress first argument
- `cert_hash` and `claims_hash` recompute from canonical bytes
- `cert.root_pid` equals configured root canister id
- root proof variant is `RootProof::IcCanisterSignatureV1`
- root canister-signature public key DER embeds the configured root canister id
  and expected seed
- root canister signature verifies under the configured raw IC root key
- shard key binding matches configured delegated-token ECDSA key name and shard
  derivation path
- `cert.shard_key_hash` matches algorithm, shard public key, and binding
- shard token signature verifies under `cert.shard_public_key_sec1`
- certificate and token time windows are valid using strict expiry:
  `now_ns >= expires_at_ns` means expired
- token does not outlive certificate or `cert.max_token_ttl_ns`
- `claims.aud` is a subset of `cert.aud`
- local project accepts both token and cert audiences
- `claims.grants` is a subset of `cert.grants`
- configured local role is present in `claims.grants`
- endpoint-required scopes are present in the grant for the local role
- delegated session subject binding is enforced before replacing caller identity

No verification step checks for local proof presence, fetches root key material,
or calls root.

## Configuration Contract

```toml
[auth.delegated_tokens]
enabled = true
root_canister_id = "..."
ic_root_public_key_raw_hex = "..."
network = "mainnet"
max_ttl_secs = 3600
```

`root_canister_id` may fall back to initialized Canic root env. The raw IC root
key may come from config or a test/runtime root-key provider. If delegated-token
verification is enabled, startup must have the `auth-root-canister-sig-verify`
feature and an effective root principal plus raw IC root key.

`auth.delegated_tokens.ecdsa_key_name` remains the shard token-signing key name.
It is not the root proof trust anchor.

## Revocation and TTL

Delegated proofs and tokens are self-contained. A verifier with the token and
configured IC root key can verify without online root state. Emergency
revocation before `expires_at_ns` is not guaranteed unless a separate
root-certified revocation epoch is introduced.

The hard-cut mitigation is short cert/token TTLs, strict `max_ttl_secs`, and
shard key rotation. Stronger revocation is a separate protocol addition and
would weaken the no-required-verifier-state contract.

## Forbidden Patterns

The auth architecture must not introduce:

- root threshold ECDSA signing for delegated root proofs
- legacy `root_sig` verifier branches
- `SubnetState.auth.delegated_root_public_key` as delegated-token root proof
  authority
- verifier-local proof lookup as an acceptance condition
- proof distribution as an authentication correctness requirement
- query-time root or management calls from endpoint guards
- endpoint APIs that return generic raw signatures
- single-call fresh-proof `mint_token` on the normal auth surface
- relay envelope auth modes that skip delegated subject binding

Allowed ECDSA use remains:

- shard token signing
- shard token signature verification
- role-attestation and internal-invocation proof signing/verification

## Test-Only Paths

The following are explicitly test-only or demo-local:

- `user_shard_issue_token` in `fleets/test/user_shard/src/lib.rs`
- `signer_issue_token` in `canisters/test/delegation_signer_stub/src/lib.rs`
- `create_account` and `plan_create_account` in fleet demo canisters
