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
  -> issuer canister-signature proof
  -> reusable DelegatedToken
```

Root no longer signs `DelegationCert` with root threshold ECDSA. Verifiers do
not read `SubnetState.auth.delegated_root_public_key` when verifying a delegated
token root proof. A verifier validates locally from the token, configured root
principal, configured or runtime IC root public key, local project/role config,
and current IC time.

Delegated tokens are bearer tokens for the presenting principal. A valid token
is not consumed by verification and may authorize multiple update or query
calls by `claims.subject` until `claims.expires_at_ns`. A user token is not an
on-behalf-of credential and intentionally fails if a forwarding canister
presents it downstream.

## Canonical Payloads

Source: `crates/canic-core/src/dto/auth.rs`.

```rust
pub enum DelegationAudience {
    Canister(Principal),
    CanicSubnet(Principal),
    Project(String),
}

pub struct DelegatedRoleGrant {
    pub target: CanisterRole,
    pub scopes: Vec<String>,
}

pub enum IssuerProofAlgorithm {
    IcCanisterSignatureV1,
}

pub enum IssuerProofBinding {
    IcCanisterSignatureV1 { seed_hash: [u8; 32] },
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
    pub issuer_pid: Principal,
    pub issuer_proof_alg: IssuerProofAlgorithm,
    pub issuer_proof_binding_hash: [u8; 32],
    pub issuer_proof_binding: IssuerProofBinding,
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
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub nonce: [u8; 16],
    pub ext: Option<Vec<u8>>,
}

pub struct DelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub proof: DelegationProof,
    pub issuer_proof: IssuerProof,
}

pub enum IssuerProof {
    IcCanisterSignatureV1(IcCanisterSignatureProofV1),
}
```

All protocol timestamps are nanoseconds since Unix epoch. Human-facing config
may use seconds, but conversion happens at the boundary. Protocol structs must
not contain `*_secs` fields.

Verifier future-skew allowance is bounded and applies only to
not-from-the-future checks:

```rust
pub const AUTH_TIME_SKEW_ALLOWANCE_NS: u64 = 60_000_000_000;
```

Expiry remains strict. Verifiers must not add grace after `expires_at_ns`.

`DelegatedTokenClaims.nonce` is deterministic issuer-generated uniqueness
material. It is not secret, not a replay key, and not an authorization input.
`prepare_delegated_token` must not call the management canister, including
`raw_rand`, to produce it. The nonce derivation hashes caller, prepare
operation id, subject, issuer, and selected cert hash under
`"canic-token-nonce-v1"` and takes the first 16 bytes.

## Canonical Hashes

Signed structures use `CanicAuthCanonical` rules from
`ops/auth/delegated/canonical.rs`.

```text
cert_hash     = sha256(canonical_bytes(DelegationCert))
claims_hash   = sha256(canonical_bytes(DelegatedTokenClaims))
issuer_proof_binding_hash =
  sha256("canic-issuer-proof-binding-v1" ||
         issuer_pid ||
         issuer_proof_alg ||
         canonical_bytes(issuer_proof_binding))
```

Grant vectors must be strictly sorted by role and duplicate-free. Scope vectors
inside each grant must be strictly sorted and duplicate-free. Verifiers reject
noncanonical payloads instead of normalizing them.

`DelegatedTokenClaims.ext` is optional opaque application data. It is included
in canonical claims bytes, bounded to 4096 bytes, and interpreted only by
application endpoints after core delegated-token verification succeeds.

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
1. provisioner    -> root canic_prepare_delegation_proof_batch update
2. provisioner    -> root canic_get_delegation_proof_batch direct query
3. provisioner    -> root canic_install_delegation_proof_batch update
4. root           -> issuer canic_install_active_delegation_proof update
5. caller/session -> issuer canic_prepare_delegated_token update
6. caller/session -> issuer canic_get_delegated_token query
7. caller/session -> endpoint with DelegatedToken
```

The public delegated-token prepare endpoint is a login/session materialization
surface. It rejects requests where `subject` does not match the update caller
and only self-issues public login scopes: `session` and `verify`. Privileged
application scopes such as `read`, `write`, `admin`, or application-specific
admin labels must be issued by a separate caller-authorized path instead of
being accepted from open caller-supplied prepare payloads.

`canic_prepare_delegation_proof_batch` is request-id keyed. The same provision
request id and payload returns the same prepared batch metadata; the same
request id with a different payload is a replay conflict. The first fresh
prepare adds signature-map entries, refreshes certified data, and stores
pending batch metadata.

Root batch provisioning is bounded in the MVP: 64 issuers per batch, 128
pending batches, and 16 pending root delegation proofs per issuer. Expired
pending metadata is pruned opportunistically during prepare and install.
Uninstalled pending entries are removed after their retrieval window expires;
installed entries remain available for idempotent reinstall until certificate
expiry. Signature-map leaf pruning is not part of the current MVP.

`canic_get_delegation_proof_batch` is not separately replay-protected. It is a
direct root query over existing pending batch metadata. The requested
`batch_id`, issuer, and `cert_hash` must match pending metadata, and
`now_ns < retrieval_expires_at_ns`.

The retired single-proof root prepare/get endpoints are not part of the active
protocol. Root proof retrieval must not be hidden behind issuer composite-query
wrappers because root needs its direct query data certificate to assemble the
canister-signature proof.

The normal auth surface does not expose a one-shot fresh-proof `mint_token`
path. Client/test helpers may choreograph the calls above, but they must not
hide query certificate retrieval inside one update.

## Issuer Token Proof

Issuer token proofs use the same canister-signature update-then-query
mechanics as root proofs. The issuer signs the canonical claims hash, not a
secp256k1 payload.

```text
claims_hash = sha256(canonical_bytes(DelegatedTokenClaims))
issuer verifier message =
  domain_len || b"canic-issuer-delegated-token" || claims_hash
```

The issuer `SignatureMap` stores the domain-separated claims hash under seed
`b"canic-issuer-delegated-token"`. Verification passes
`domain_len || domain || claims_hash` to `verify_canister_sig`, checks the
issuer proof public key DER embeds `cert.issuer_pid` and the expected seed, and
uses the configured raw IC root key.

The 0.65 hard cut removes management-canister ECDSA from normal delegated auth.
The expected usage model is to reuse root proofs and delegated tokens for their
TTL. Protected endpoint verification may use a bounded positive cache for the
expensive root and issuer canister-signature checks, but endpoint-specific
authorization still runs after cache hits.

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
- `cert.issuer_pid` matches `claims.issuer_pid`
- `cert.issuer_proof_alg == IssuerProofAlgorithm::IcCanisterSignatureV1`
- `cert.issuer_proof_binding` carries the expected issuer seed hash
- `cert.issuer_proof_binding_hash` matches the canonical issuer proof binding
- issuer proof variant is `IssuerProof::IcCanisterSignatureV1`
- issuer canister-signature public key DER embeds `cert.issuer_pid` and
  expected seed
- issuer canister signature verifies over
  `domain_len || b"canic-issuer-delegated-token" || claims_hash`
- certificate and token time windows are valid using strict expiry:
  `now_ns >= expires_at_ns` means expired
- certificate `not_before_ns` and token `issued_at_ns` may be at most
  `AUTH_TIME_SKEW_ALLOWANCE_NS` ahead of verifier time
- token does not outlive certificate or `cert.max_token_ttl_ns`
- `claims.aud` is a subset of `cert.aud`
- local project accepts both token and cert audiences
- `claims.grants` is a subset of `cert.grants`
- `claims.subject` equals the transport caller
- configured local role is present in `claims.grants`
- endpoint-required scopes are present in the grant for the local role
- delegated session subject binding is enforced before replacing caller identity

No verification step checks for local proof presence, fetches root key material,
or calls root. Forwarded user tokens fail with subject/caller mismatch because
the downstream verifier sees the forwarding canister as caller.

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
key is paired with `network`: mainnet requires the configured known mainnet raw
key, while local/PocketIC/test verification requires a non-mainnet raw root key
from `ic_root_public_key_raw_hex` and rejects the mainnet key. If delegated-token
verification is enabled, startup must have root and
issuer canister-signature verification features, an effective root principal
plus raw IC root key, and the current canister must set
`delegated_token_verifier = true` before endpoint token verification can
proceed.

## Revocation and TTL

Delegated proofs and tokens are self-contained. A verifier with the token and
configured IC root key can verify without online root or issuer state.
Emergency revocation before `expires_at_ns` is not guaranteed. The hard-cut
mitigation is short cert/token TTLs and strict `max_ttl_secs`.

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

Normal delegated-token auth must not call management-canister threshold ECDSA.

## Test-Only Paths

The following are explicitly test-only or demo-local:

- `create_account` and `plan_create_account` in fleet demo canisters
