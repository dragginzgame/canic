# Canic Authentication Design

- **Status:** canonical current design
- **Version line:** current hard-cut delegated-token contract
- **Audience:** Canic maintainers and downstream application developers
- **Primary rule:** auth is enforced at endpoints; workflow, ops, policy, DTO, and model code receive already-authenticated input.

This document is the current authentication design for Canic. Historical
release-slice notes live under `docs/design/`; exact runtime/wire contracts live
under `docs/contracts/`.

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
   - endpoint-required scope must appear in the token grant for the local role
3. Role-attestation endpoint auth:
   - caller supplies a `SignedRoleAttestation`
   - endpoint guard validates the embedded root canister-signature proof
   - service-role checks stay local and make no root or management-canister call

Auth code lives at the boundary:

```text
endpoint macro / access guard
  -> access::auth / api::auth
  -> ops::auth
  -> storage key/session cache only where needed
```

Model, DTO, policy, and ordinary workflow code must not introduce hidden auth
checks.

## 2. Delegated Token Trust Model

Delegated auth is self-validating.

```text
configured root principal
  + configured raw IC root public key
  -> root canister-signature proof over cert_hash
  -> root-certified DelegationCert
  -> issuer canister-signature proof over claims_hash
  -> authenticated delegated subject
```

A verifier validates a delegated token using only:

- the token
- the embedded `DelegationProof`
- configured root identity
- configured network label paired with the effective raw IC root public key
- issuer proof embedded in the token
- local project id and configured role
- IC canister time

A verifier must not require:

- local proof presence
- proof fanout from root
- creation-time proof catch-up
- proof history replication
- registry snapshots for delegated-token audience membership
- topology placement ordering
- query-time or first-use public-key fetching
- cascaded `SubnetState.auth.delegated_root_public_key`

In this document, "proof" means the embedded `DelegationProof` carried inside
`DelegatedToken`. It never means verifier-local proof state.

Delegated tokens are not one-shot receipts. A token that verifies may authorize
multiple update or query calls until `claims.expires_at_ns`.

## 3. Data Structures

Source of truth: `crates/canic-core/src/dto/auth.rs`.

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
may use seconds; protocol DTOs and canonical encodings use `_ns` fields.

### Field Authority

- `root_pid`: set by root and checked against verifier config.
- `issuer_pid`, `issuer_proof_alg`, `issuer_proof_binding_hash`,
  and `issuer_proof_binding`: set by root after binding the issuer
  canister-signature authority, then certified by the root canister-signature
  proof.
- `cert.aud`, `cert.grants`, cert time fields, and `max_token_ttl_ns`: set by
  root and certified by the root proof.
- `claims.subject`, `claims.aud`, `claims.grants`, token time fields, and
  `nonce`: set by the issuer and signed by the issuer.
- `claims.ext`: opaque application data set by the issuer, signed as part of
  `DelegatedTokenClaims`, and interpreted only by application endpoints.
- `claims.cert_hash`: hash of canonical `DelegationCert`; set by issuer and
  verified by every verifier.
- `claims.issuer_pid`: must equal `cert.issuer_pid`.

`nonce` is deterministic issuer-generated uniqueness material. It is not
secret, not a replay key, and not an authorization input. The issuer derives it
from caller, prepare operation id, subject, issuer, and selected cert hash
without `raw_rand` or any management-canister call.

## 4. Canonical Encoding

Signed payloads use Canic's auth canonical encoding in
`ops/auth/delegated/canonical.rs`, not Candid bytes and not serde bytes.

Canonical hashes:

```rust
cert_hash = sha256(canonical_bytes(DelegationCert))
claims_hash = sha256(canonical_bytes(DelegatedTokenClaims))
role_hash = sha256(canonical_role_bytes(CanisterRole))
issuer_proof_binding_hash =
    sha256("canic-issuer-proof-binding-v1" ||
           issuer_pid ||
           issuer_proof_alg ||
           canonical_bytes(issuer_proof_binding))
```

Strict canonical rules:

- role grants must already be strictly sorted by role and duplicate-free
- scopes inside each grant must already be strictly sorted and duplicate-free
- role and scope strings must be non-empty ASCII strings using only `[a-z0-9_:-]`
- project audience strings must be non-empty ASCII strings using only `[a-z0-9_:-.]`
- token `ext` payloads are optional opaque bytes and must not exceed 4096 bytes
- no verifier-role or verifier-principal audience exists
- verifier rejects noncanonical vectors rather than normalizing them

This is intentional: one semantic token must have one valid canonical encoding.

## 5. Root Proof Issuance

Entrypoint path:

```text
AuthApi::prepare_delegation_proof_batch_root
  -> root canic_prepare_delegation_proof_batch update
  -> AuthOps::prepare_delegation_proof_batch
  -> SignatureMap.add_signature
  -> set_certified_data(labeled_hash("sig", signatures.root_hash()))

canic_get_delegation_proof_batch query
  -> AuthOps::get_delegation_proof_batch
  -> SignatureMap.get_signature_as_cbor
  -> RootDelegationProofBatchGetResponse

canic_install_delegation_proof_batch update
  -> root broadcasts canic_install_active_delegation_proof to issuers
```

Root issuance steps:

1. Require local canister is root.
2. Require root-controller authorization for the MVP batch endpoints.
3. Validate each issuer against the root issuer registry.
4. Load `auth.delegated_tokens` config.
5. Bind each requested issuer canister to
   `IssuerProofAlgorithm::IcCanisterSignatureV1` with seed
   `b"canic-issuer-delegated-token"`.
6. Build each `DelegationCert`.
7. Enforce:
   - `cert.root_pid == self`
   - `cert.not_before_ns < cert.expires_at_ns`
   - cert TTL does not exceed `auth.delegated_tokens.max_ttl_secs`
   - `cert.max_token_ttl_ns > 0`
   - `cert.max_token_ttl_ns <= cert_ttl_ns`
   - audience shape is canonical
   - role grants are non-empty, bounded, sorted, and canonical
   - `cert.issuer_pid` equals the requested issuer
   - `cert.issuer_proof_binding_hash` matches the issuer proof authority
     fields
8. Add a canister-signature map entry for each `cert_hash`.
9. Commit certified data for the `"sig"` tree.
10. Return `RootDelegationProofBatchPrepareResponse` metadata.
11. In a direct root query, assemble `DelegationProof` values from the prepared
    metadata and root data certificate.
12. In a root update, validate submitted proofs against pending metadata and
    broadcast issuer installs.

Root proof creation input:

```rust
CanisterSigInputs {
    seed: b"canic-root-delegation-cert",
    domain: b"canic-root-delegation-cert",
    message: &cert_hash,
}
```

The verifier message is:

```text
domain_len || domain || cert_hash
```

That exact byte string is passed to
`ic_signature_verification::verify_canister_sig`.

### Replay and Retrieval

`canic_prepare_delegation_proof_batch` is request-id keyed. Repeating the same
batch provision request with the same request id returns the same batch
metadata. Reusing the request id with a different payload returns a replay
conflict.

Root batch provisioning is bounded in the MVP:

- at most 64 issuers per prepare batch
- at most 128 pending batches
- at most 16 pending root delegation proofs per issuer

Expired pending batch metadata is pruned opportunistically during prepare and
install. Uninstalled entries are removed after their retrieval window expires;
installed entries remain available for idempotent reinstall until certificate
expiry. The current MVP does not prune canister-signature map leaves.

`canic_get_delegation_proof_batch` is a direct root query over existing pending
batch metadata and is not separately replay-protected. The requested
`batch_id`, issuer, and `cert_hash` must match pending metadata, and
`now_ns < retrieval_expires_at_ns`.

The retired single-proof `canic_prepare_delegation_proof` and
`canic_get_delegation_proof` root endpoints are removed from the active
protocol. Issuer canisters must not retrieve root proof material through
composite-query wrappers.

The pending retrieval window is one minute, matching the upstream
`SignatureMap` retention period used by the root canister-signature map.

Root certifies only `"sig"` in this hard cut.

Root proof issuer canisters must not be deployed on a subnet whose canister
signatures are invalid. Deployment tooling enforces this; runtime may also trap
on an explicit deployment assertion.

## 6. Issuer Token Issuance

Entrypoint path:

```text
AuthApi::prepare_delegated_token
  -> reserve auth.prepare_delegated_token.v1 replay receipt
  -> add issuer canister-signature map entry
  -> set_certified_data(labeled_hash("sig", SIGNATURES.root_hash()))

AuthApi::get_delegated_token
  -> return the prepared claims plus issuer canister-signature proof
```

Issuer issuance steps:

1. Require caller-provided replay metadata.
2. Return the committed prepare response for the same operation id, actor, and
   payload.
3. Reject the same operation id with a different actor or payload.
4. Require an installed `ActiveDelegationProof` whose cert issuer is this
   canister.
5. Prepare `DelegatedTokenClaims`, including deterministic issuer-generated
   nonce material.
6. Enforce:
   - root proof verifies
   - cert is currently valid
   - token TTL is greater than zero
   - token TTL does not exceed `cert.max_token_ttl_ns`
   - token expiry does not exceed cert expiry
   - token audience is a subset of cert audience
   - token grants are a subset of cert grants
   - claims are canonical
7. Add an issuer canister-signature entry for the canonical claims hash.
8. Commit the exact prepare response.
9. Query retrieval is caller-bound and returns the self-contained
   `DelegatedToken`.

The normal auth surface has no single-call token issuance path. Fleet, CLI, and
test helpers choreograph prepare/get from off-canister code. Normal delegated
auth does not call `management_canister.sign_with_ecdsa`, `raw_rand`, or any
management-canister method during `prepare_delegated_token`.

## 7. Verifier Algorithm

Endpoint delegated auth is reached through endpoint guards in `access::auth`.

Verifier steps:

1. Decode the first ingress argument as `DelegatedToken`.
2. Resolve verifier trust config:
   - `auth.delegated_tokens.root_canister_id`, or initialized root env
   - parsed `auth.delegated_tokens.network`
   - `network = "mainnet"` requires the configured known mainnet raw IC root key
   - `network = "local"`, `"pocketic"`, or `"testnet"` requires a configured
     non-mainnet raw IC root key
   - issuer canister-signature proof embedded in the token
3. Verify certificate policy:
   - configured root principal
   - cert time window
   - cert TTL policy
   - max token TTL policy
   - audience shape
   - role grant shape
   - issuer proof algorithm and binding hash
4. Verify root canister-signature proof:
   - proof variant is `RootProof::IcCanisterSignatureV1`
   - public key DER embeds configured root principal and expected seed
   - verification message is `domain_len || domain || cert_hash`
   - `ic_root_public_key_raw` is the 96-byte raw IC BLS key, not DER
5. Verify claims:
   - `claims.issuer_pid == cert.issuer_pid`
   - `claims.cert_hash == cert_hash`
   - token window is valid
   - token does not outlive cert
   - token TTL does not exceed `cert.max_token_ttl_ns`
   - `claims.aud` is subset of `cert.aud`
   - local project accepts both `claims.aud` and `cert.aud`
   - `claims.grants` is subset of `cert.grants`
6. Verify issuer canister-signature proof:
   - proof variant is `IssuerProof::IcCanisterSignatureV1`
   - public key DER embeds `cert.issuer_pid` and expected issuer seed
   - verification message is `domain_len || domain || claims_hash`
   - `ic_root_public_key_raw` is the configured raw IC BLS key
7. Verify local role authorization:
   - configured local role is required
   - token grants include the local role
   - endpoint-required scopes are present in that local-role grant
8. Verify transport caller binding:

```rust
claims.subject == ic_cdk::caller()
```

If all checks pass, the endpoint receives a delegated subject identity.

`DelegatedToken` is not an on-behalf-of delegation mechanism. A user token is
valid only when presented by `claims.subject` as `msg.caller()`.
Canister-to-canister forwarding intentionally fails because the downstream
verifier sees the forwarding canister as `msg.caller()`. Service-to-service
calls use `SignedRoleAttestation` or a future explicit on-behalf-of protocol.

Plain query, composite-query, and update guards share this same verification
path. No step checks local proof presence, fetches root key material, or calls
root.

## 8. Delegated Sessions

Delegated sessions allow a wallet caller to temporarily bind an authenticated
delegated subject.

Entrypoint:

```text
AuthApi::set_delegated_session_subject
```

Rules:

- delegated token must verify through the same self-validating token path
- token subject must equal requested delegated subject
- wallet caller and delegated subject must not be infrastructure/canister
  principals rejected by `validate_delegated_session_subject`
- session expiry is clamped to:
  - token expiry
  - configured delegated-token max TTL
  - optional requested session TTL
- expiry boundary is strict: `now_ns >= expires_at_ns` means expired
- verifier future-skew allowance is allowed only for not-from-the-future checks:
  `AUTH_TIME_SKEW_ALLOWANCE_NS = 60_000_000_000`
- no expiry grace is added for delegated tokens, delegation certs, sessions, or
  role attestations
- bootstrap token fingerprint is stored to reject replay conflicts and allow
  idempotent same-session replay

Session storage is not delegated-token proof storage.

## 9. Role Attestation

Role attestation is separate from delegated-token proof validation. In 0.65,
role attestations use root canister signatures with the same update-then-query
shape as root delegation proofs.

Data:

```rust
pub struct RoleAttestation {
    pub subject: Principal,
    pub role: CanisterRole,
    pub subnet_id: Option<Principal>,
    pub audience: Principal,
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
    pub epoch: u64,
}

pub struct SignedRoleAttestation {
    pub payload: RoleAttestation,
    pub root_proof: RootProof,
}
```

Root canister-signature role attestations use:

```text
RootPayloadKind::RoleAttestation
```

Issuance flow:

- `canic_prepare_role_attestation` is an update call on the root canister
- `canic_get_role_attestation` is a query call by the same caller
- retrieval is caller-bound and returns the embedded root proof

Verifier behavior:

- hash the canonical `RoleAttestation` payload
- verify the embedded root canister-signature proof against the configured root
  canister id and raw IC root public key
- enforce subject, role, audience, subnet, time window, and minimum accepted
  epoch locally; `issued_at_ns` may be at most
  `AUTH_TIME_SKEW_ALLOWANCE_NS` ahead of verifier time, while
  `expires_at_ns` remains strict
- make no root, issuer, or management-canister call on the protected path

Current issuance rule:

- `canic_request_role_attestation` is not exposed in normal 0.65 auth
- `canic_request_internal_invocation_proof` is not exposed in normal 0.65 auth
- standalone capability proof DTOs are not part of the active protocol
- delegated tokens are the supported reusable endpoint-auth path

## 10. Configuration

Delegated tokens:

```toml
[auth.delegated_tokens]
enabled = true
root_canister_id = "..."
ic_root_public_key_raw_hex = "..."
network = "mainnet"
max_ttl_secs = 3600
```

Role attestation:

```toml
[auth.role_attestation]
max_ttl_secs = 300

[auth.role_attestation.min_accepted_epoch_by_role]
project_instance = 1
```

Per-canister auth roles:

```toml
[subnets.prime.canisters.user_shard.auth]
delegated_token_issuer = true
delegated_token_verifier = true

[subnets.prime.canisters.project_instance.auth]
delegated_token_verifier = true
```

Security boundaries:

- `auth.delegated_tokens.root_canister_id` or `EnvOps::root_pid()` is the
  delegated-token root identity trust boundary.
- `auth.delegated_tokens.network` and the effective raw IC root key are paired:
  mainnet requires the known mainnet raw key, while local/PocketIC/test
  verification requires a non-mainnet root key configured as
  `ic_root_public_key_raw_hex`.
- token issuers must set `delegated_token_issuer = true`; only those canisters
  expose delegated-token prepare/get/install provisioning endpoints.
- public delegated-token prepare self-issues only login/session material
  (`session` and `verify` scopes) for the caller subject. Privileged
  application scopes must be issued through a caller-authorized path or checked
  against verifier-local application state after authentication.
- protected endpoint verifiers must set `delegated_token_verifier = true`; the
  runtime delegated-token verifier rejects before proof verification when the
  current canister is not explicitly configured as a delegated-token verifier.
- verifier `local_role` config is trusted; a canister configured with the wrong
  role is compromised for delegated-auth purposes.

Feature requirements:

- root proof issuer: `control-plane`, `auth-root-canister-sig-create`
- endpoint verifier: `auth-delegated-token-verify`
- issuer token proof creator: `auth-issuer-canister-sig-create`
- role attestation issuer: `control-plane`, `auth-root-canister-sig-create`
- role attestation verifier: `auth-root-canister-sig-verify` with configured
  root canister id and raw IC root public key

## 11. Revocation and TTL

Delegated proofs and tokens are self-contained. A verifier that has the token
and the configured IC root key can verify without online root or issuer state.
Emergency revocation before `expires_at_ns` is not guaranteed.

The hard-cut mitigation is short cert/token TTLs, strict `max_ttl_secs`, and
local endpoint checks.

## 12. Removed Concepts

These concepts are not part of current Canic delegated auth:

- root threshold ECDSA proofs for `DelegationCert`
- legacy `root_sig` verifier acceptance
- local verifier proof cache as an auth condition
- proof fanout from root to verifiers
- creation-time verifier proof catch-up
- proof equality matching
- root-key fallback from embedded token material
- verifier-side root-key fetch-on-verify
- query-time key fetch from `requires(auth::authenticated())`
- delegated root-key background warmup timers
- fresh one-shot root ECDSA role-attestation issuance in normal auth
- fresh one-shot root ECDSA internal-invocation proof issuance in normal auth
- `RootKeyAuthority`
- root-key certificates
- implicit revocation by deleting proof/cache state
- relay envelope delegated auth
- single-call fresh-proof `mint_token`

Authenticated endpoint guards require a first argument of type
`DelegatedToken`. Candid `Reserved` placeholders are not part of the current
auth surface.

## 13. Failure Modes

Expected failures:

- disabled delegated auth config
- missing verifier feature or trust anchor
- malformed Candid token argument
- noncanonical role grants, noncanonical grant scopes, or invalid audience labels
- mismatched root principal
- malformed root canister-signature proof
- malformed issuer canister-signature proof
- wrong IC root public key
- issuer proof binding mismatch
- certificate expired or not yet valid
- token expired or not yet valid
- token TTL exceeds cert or config policy
- audience subset failure
- local project does not accept token or cert audience
- missing local role
- local role missing from token grants
- required scope missing from local-role grant
- token subject does not match transport caller
- delegated-session bootstrap replay conflict
- role-attestation epoch below configured minimum

These failures are cryptographic, temporal, policy, or config failures. There
is no "delegation proof miss" correctness failure in current Canic auth.

## 14. Developer Checklist

When changing auth code:

- keep auth checks at endpoint/access/API auth boundaries
- use one canonical encoding implementation for delegated-token prepare and
  verify
- reject noncanonical vectors instead of sorting during verification
- preserve `now_ns >= expires_at_ns` as the expiry boundary everywhere
- apply `AUTH_TIME_SKEW_ALLOWANCE_NS` only to not-from-the-future checks, never
  to expiry
- do not add verifier-local proof lookup
- do not add proof distribution as a correctness requirement
- do not retag root-provided attestation keys
- do not accept caller-provided arbitrary public keys
- do not reintroduce management-canister threshold ECDSA for normal auth
- do not hide query-certificate retrieval behind a one-shot update API
- update `docs/contracts/AUTH_DELEGATED_SIGNATURES.md` when wire structs or
  verification rules change
- update this document when trust boundaries or auth flows change
