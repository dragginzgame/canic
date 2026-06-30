# Auth Contract: Self-Validating Delegated Tokens

This document defines the current hard-cut delegated-token authentication
contract implemented by Canic.

## Trust Model

Canonical trust chain:

```text
configured root principal
  + configured chain-key root verifier policy
  -> embedded RootProof::IcChainKeyBatchSignatureV1
  -> root-signed batch header
  -> issuer Merkle witness
  -> root-authorized DelegationCert
  -> issuer canister-signature proof
  -> reusable DelegatedToken
```

Root delegation proofs are IC chain-key threshold ECDSA proofs over a canonical
delegation batch, not IC certified-data canister-signature proofs. Verifiers do
not read `SubnetState.auth.delegated_root_public_key` when verifying a delegated
token root proof. A verifier validates locally from the token, configured root
principal, configured chain-key root verifier policy, configured or runtime IC
root public key for the issuer canister-signature proof, local project/role
config, and current IC time.

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

pub enum ChainKeyAlgorithm {
    EcdsaSecp256k1,
}

pub struct ChainKeyKeyId {
    pub name: String,
}

pub enum RootProof {
    IcCanisterSignatureV1(IcCanisterSignatureProofV1),
    IcChainKeyBatchSignatureV1(IcChainKeyBatchSignatureProofV1),
}

pub struct IcCanisterSignatureProofV1 {
    pub signature_cbor: Vec<u8>,
    pub public_key_der: Vec<u8>,
}

pub struct IcChainKeyBatchSignatureProofV1 {
    pub header: ChainKeyBatchHeaderV1,
    pub delegation_cert: ChainKeyDelegationCertV1,
    pub issuer_witness: ChainKeyBatchWitnessV1,
    pub signature: ChainKeyRootSignatureV1,
}

pub struct ChainKeyBatchHeaderV1 {
    pub schema_version: u16,
    pub root_canister_id: Principal,
    pub batch_id: [u8; 32],
    pub proof_epoch: u64,
    pub registry_epoch: u64,
    pub registry_hash: [u8; 32],
    pub tree_root: [u8; 32],
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub algorithm: ChainKeyAlgorithm,
    pub key_id: ChainKeyKeyId,
    pub derivation_path_hash: [u8; 32],
    pub key_version: u64,
}

pub struct ChainKeyDelegationCertV1 {
    pub root_canister_id: Principal,
    pub issuer_canister_id: Principal,
    pub proof_epoch: u64,
    pub issuer_proof_algorithm: IssuerProofAlgorithm,
    pub issuer_proof_binding_hash: [u8; 32],
    pub issuer_proof_binding: IssuerProofBinding,
    pub max_token_ttl_ns: u64,
    pub audience: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub registry_epoch: u64,
    pub registry_hash: [u8; 32],
}

pub struct ChainKeyRootSignatureV1 {
    pub algorithm: ChainKeyAlgorithm,
    pub key_id: ChainKeyKeyId,
    pub derivation_path: Vec<Vec<u8>>,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

pub struct ChainKeyBatchWitnessV1 {
    pub steps: Vec<ChainKeyBatchWitnessStepV1>,
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
chain_key_batch_header_hash =
  sha256(canonical_bytes(ChainKeyBatchHeaderV1))
chain_key_delegation_cert_hash =
  sha256(canonical_bytes(ChainKeyDelegationCertV1))
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

## Root Chain-Key Batch Proof

Delegated-token root proof creation uses IC management-canister chain-key
threshold ECDSA. The root canister signs the canonical
`ChainKeyBatchHeaderV1`, and each issuer receives an issuer-specific
`ChainKeyDelegationCertV1` plus Merkle witness proving that issuer leaf is
covered by the signed batch `tree_root`.

The root signing message is:

```text
sha256(canonical_bytes(ChainKeyBatchHeaderV1))
```

The batch header binds:

- root canister id
- batch id
- proof epoch
- registry epoch and registry hash
- Merkle tree root
- batch validity window
- algorithm
- key id
- derivation path hash
- key version

`ChainKeyRootSignatureV1.signature` is the raw secp256k1 ECDSA `r || s`
signature returned by `sign_with_ecdsa` after signer-side low-s normalization.
Verifiers reject malformed length, zero components, and high-s signatures.
`ChainKeyRootSignatureV1.public_key` is the configured SEC1 chain-key public
key for the root canister, key id, and derivation path. The verifier must not
treat the proof-supplied public key as authority; it must match configured
policy.

`DelegationCert` remains the token-facing root certificate and is still hashed
into `claims.cert_hash`. The embedded `ChainKeyDelegationCertV1` must cohere
with `DelegationCert` for issuer id, issuer proof algorithm and binding,
audience, grants, time window, max token TTL, registry epoch, and registry
hash.

`RootProof::IcCanisterSignatureV1` is a retained historical variant and is
rejected for 0.76 delegated-token root proof verification. It remains relevant
only to non-delegated-token surfaces that still use root canister signatures,
such as role attestation.

## Issuance Flow

Root proof issuance and renewal are canister-owned. Auth liveness must not
depend on a bridge worker, CLI, cron job, host daemon, external signer, direct
root query, or client-side provisioning step.

```text
1. root timer/update
   -> prepare due issuer renewal templates
   -> build canonical chain-key batch
   -> call management canister sign_with_ecdsa
   -> persist signed batch
   -> install issuer-specific proof/witness on issuer canisters
2. caller/session -> issuer canic_prepare_delegated_token update
3. caller/session -> issuer canic_get_delegated_token query
4. caller/session -> endpoint with DelegatedToken
```

Lazy repair uses the same proof primitive:

```text
1. issuer canic_prepare_delegated_token update sees missing/stale active proof
2. issuer -> root canic_get_or_create_chain_key_delegation_proof update
3. root returns a cached signed proof when possible
4. root signs at most one in-flight batch when no valid batch exists
5. issuer verifies and stores the proof
6. token preparation continues or the retry uses the stored proof
```

The public delegated-token prepare endpoint is a login/session materialization
surface. It rejects requests where `subject` does not match the update caller
and only self-issues public login scopes: `session` and `verify`. Privileged
application scopes such as `read`, `write`, `admin`, or application-specific
admin labels must be issued by a separate caller-authorized path instead of
being accepted from open caller-supplied prepare payloads.

`canic_upsert_root_issuer_policy` is a root controller update that registers
or updates the issuer policy used by batch prepare. It records the issuer
principal, enabled state, allowed audiences, allowed grants, maximum
certificate TTL, and refresh-after ratio.

Root-managed renewal stores enabled issuer renewal templates, a delegated-auth
registry epoch/hash, proof epoch state, and signed chain-key root delegation
batches. The root timer prepares due issuer templates, signs a batch once per
proof epoch, installs issuer-specific active proof material, retries partial
install failures, discards stale signing callbacks after registry changes, and
prunes expired batches before install.

The old bridge-backed canister-signature provisioning endpoints are not part of
the active delegated-token root proof contract. In
`root_proof_mode = "chain_key_batch"`, delegated-token liveness comes from root
timer renewal and issuer lazy repair through chain-key batch proofs, not from
external bridge or direct-query install workflows.

The retired single-proof root prepare/get endpoints are not part of the active
protocol. Root proof retrieval must not be hidden behind issuer composite-query
wrappers, and the current root proof flow must not require root query data
certificates.

The normal auth surface does not expose a one-shot fresh-proof `mint_token`
path. Client/test helpers may choreograph the calls above, but they must not
hide bridge-backed or direct-query root proof renewal inside one update.

## Issuer Token Proof

Issuer token proofs still use canister-signature update-then-query mechanics.
The root delegation proof no longer uses this primitive. The issuer signs the
canonical claims hash, not a secp256k1 payload.

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
The 0.76 hard cut reintroduces management-canister ECDSA only at root renewal
and lazy-repair batch boundaries. `prepare_delegated_token` and protected
endpoint verification must not call management-canister signing. The expected
usage model is to reuse root proofs and delegated tokens for their TTL.
Protected endpoint verification may use a bounded positive cache for the
expensive root chain-key and issuer canister-signature checks, but
endpoint-specific authorization still runs after cache hits.

## Verification Contract

Verification entrypoint:

- `access::auth::authenticated(required_scope)`

Checks before authorization:

- delegated token decodes from ingress first argument
- `cert_hash` and `claims_hash` recompute from canonical bytes
- `cert.root_pid` equals configured root canister id
- root proof variant is `RootProof::IcChainKeyBatchSignatureV1`
- configured chain-key root policy is valid for verifier time
- root proof header, delegation cert leaf, and `DelegationCert` all bind the
  configured root canister id
- header algorithm, key id, derivation path hash, key version, proof epoch,
  registry epoch, and root public key match configured policy
- proof epoch, key version, and registry epoch meet configured minimums
- batch and delegation cert windows are valid and do not exceed
  `max_revocation_latency_ns`
- issuer Merkle witness reconstructs the signed batch tree root
- raw secp256k1 signature is well formed, low-s, and verifies over
  `sha256(canonical_bytes(ChainKeyBatchHeaderV1))`
- `ChainKeyDelegationCertV1` coheres with `DelegationCert`
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
root_proof_mode = "chain_key_batch"

[auth.delegated_tokens.chain_key_root_proof]
key_id = "key_1"
derivation_path_hash_hex = "..."
derivation_path_hex = ["63616e6963", "64656c65676174696f6e"]
public_key_hex = "..."
key_version = 1
min_accepted_key_version = 1
min_accepted_proof_epoch = 1
min_accepted_registry_epoch = 1
valid_from_ns = 0
accept_until_ns = 4102444800000000000
max_revocation_latency_ns = 60000000000
```

`root_canister_id` may fall back to initialized Canic root env. The raw IC root
key is paired with `network` for issuer canister-signature verification:
mainnet requires the configured known mainnet raw key, while local/PocketIC/test
verification requires a non-mainnet raw root key from
`ic_root_public_key_raw_hex` and rejects the mainnet key.

0.76 delegated-token auth requires `root_proof_mode = "chain_key_batch"`.
`chain_key_root_proof` is the root delegation proof trust anchor. Its
`public_key_hex` must be a secp256k1 SEC1 public key for the configured root
canister id, `key_id`, and `derivation_path_hex`; `derivation_path_hash_hex`
must match the canonical hash of `derivation_path_hex`. Mainnet rejects
`test_key_1`; local/test use of `test_key_1` requires `allow_test_key = true`.

If delegated-token verification is enabled, startup must have chain-key root
proof verification support, issuer canister-signature verification support, an
effective root principal, a raw IC root key for issuer proof verification, a
complete chain-key root proof policy, and
`delegated_token_verifier = true` on the current canister before endpoint token
verification can proceed.

## Revocation and TTL

Delegated proofs and tokens are self-contained. A verifier with the token,
configured chain-key root policy, and configured IC root key for issuer
canister-signature proof verification can verify without online root or issuer
state.
Emergency revocation before `expires_at_ns` is not guaranteed. The hard-cut
mitigation is short cert/token TTLs and strict `max_ttl_secs`.

## Forbidden Patterns

The auth architecture must not introduce:

- bridge, worker, CLI, cron, host daemon, external signer, direct root query,
  or client-side provisioning dependency for delegated-auth liveness
- per-login, per-user, per-token, or per-session root threshold signing
- legacy `root_sig` verifier branches
- legacy `RootProof::IcCanisterSignatureV1` acceptance for delegated-token root
  proofs
- `SubnetState.auth.delegated_root_public_key` as delegated-token root proof
  authority
- proof-supplied chain-key public key as delegated-token root proof authority
- verifier-local proof lookup as an acceptance condition
- proof distribution as an authentication correctness requirement
- query-time root or management calls from endpoint guards
- endpoint APIs that return generic raw signatures
- single-call fresh-proof `mint_token` on the normal auth surface
- relay envelope auth modes that skip delegated subject binding

Normal delegated-token prepare/get and protected endpoint auth must not call
management-canister threshold ECDSA. Only root renewal and lazy-repair batch
creation may call management-canister signing, and those calls must be bounded
by delegation epoch rather than login volume.

## Test-Only Paths

The following are explicitly test-only or demo-local:

- `create_account` and `plan_create_account` in fleet demo canisters
