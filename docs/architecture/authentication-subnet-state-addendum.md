# Authentication Subnet-State Addendum

- **Status:** accepted design addendum
- **Applies to:** delegated-token root trust-anchor distribution
- **Canonical parent:** [Authentication](authentication.md)

## Problem

Delegated-token verification needs one local trust anchor: the delegated root
public key.

The token is self-contained for proof and shard authority, but the verifier
must still verify that root signed the embedded `DelegationCert`. If a verifier
does not already have the delegated root public key, the old first-use behavior
can try to fetch key material from inside an authenticated endpoint guard.

That is invalid for plain replicated queries on the IC. A query guard must not
perform an inter-canister or management key fetch.

## Design Decision

The delegated root public key is `SubnetState`.

It is not `Env`:

- `Env` is near-immutable identity/runtime context.
- the delegated root public key is stable in normal operation but can change
  when delegated-auth key configuration changes.

It is not `AppState`:

- `AppState` controls app-mode runtime behavior.
- `SubnetState` carries subnet-scoped shared control-plane data.
- combining them would cascade unrelated state when one structure is enough.

It is not proof state:

- no verifier-local delegation proof cache is reintroduced.
- no proof fanout or proof catch-up is allowed.

## Target Shape

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

Field rules:

- `public_key_sec1`: root delegated-auth public key bytes.
- `key_name`: `auth.delegated_tokens.ecdsa_key_name` used to derive the key.
- `key_hash`: `sha256(public_key_sec1)`.

## Cascade Contract

Root is the owner of delegated root trust-anchor state.

Sequence:

1. Root resolves the delegated root public key from current delegated-auth
   config.
2. Root writes the identity-bound key record into `SubnetState.auth`.
3. Root cascades `SubnetState` through the existing subnet state propagation
   path.
4. Non-root canisters import cascaded `SubnetState`.
5. Authenticated endpoint guards read the delegated root public key directly
   from local `SubnetState`.

Query, composite-query, and update guards share the same verification contract:

```text
root key present and identity matches -> verify
root key missing or stale             -> deny cleanly
```

No access guard may fetch the delegated root public key.

## State Invariant

Delegated root trust-anchor correctness is identity-bound:

```text
SubnetState.auth.delegated_root_public_key.key_name
  == auth.delegated_tokens.ecdsa_key_name
sha256(SubnetState.auth.delegated_root_public_key.public_key_sec1)
  == SubnetState.auth.delegated_root_public_key.key_hash
SubnetState.auth.delegated_root_public_key.key_hash
  == cert.root_key_hash
```

`SubnetState` is both the distribution source and the local verifier read
model. The certificate remains the delegated capability.

## Failure Modes

Expected failures:

- `SubnetState.auth.delegated_root_public_key` is absent.
- local `SubnetState` has not imported the latest cascade.
- cascaded key name differs from current delegated-auth config.
- cascaded key hash differs from `cert.root_key_hash`.
- root signature does not verify under the cascaded key.

All failures must surface as Canic auth errors. They must not surface as IC
execution errors caused by attempted query-time calls.

## Implementation Removal List

After this design is implemented, these delegated-auth mechanisms can be
removed or kept out:

- verifier-side root-key fetch from `access::auth` token verification.
- first-use `EcdsaOps::public_key_sec1` call during delegated-token verify.
- any call-kind split where updates fetch but queries do not.
- delegated root-key background warmup timer.
- non-root delegated root-key prewarm API.
- proof fanout, proof catch-up, proof equality matching, and proof-cache repair
  code.
- root-key fallback from token-embedded public key material.
- `RootKeyAuthority` and root-key certificate surfaces.

These mechanisms remain valid:

- root-side ECDSA public-key resolution for signing/cert issuance.
- shard-side ECDSA public-key resolution for shard signing key identity.
- role-attestation key-set refresh, because it is a separate attestation trust
  surface.
- stable auth key cache for shard and role-attestation material.

## Implemented

- Extended `SubnetStateRecord`, `SubnetStateInput`, and `SubnetStateResponse`
  with `SubnetAuthState`.
- Added ops mappers for `SubnetAuthState` without exposing storage records
  outside ops.
- Root delegated-key publishing writes
  `SubnetState.auth.delegated_root_public_key`.
- `SubnetState` continues to cascade using the existing state propagation path.
- Delegated root public keys are read directly from local `SubnetState`.
- Delegated-token access verification no longer fetches root key material.

## Remaining Test Work

- Add dedicated regression coverage for cold authenticated query on a verifier
  canister. Existing sharding coverage asserts signer key material is warmed,
  but does not yet exercise the cold-verifier query denial path directly.
