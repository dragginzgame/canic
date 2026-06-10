# Authentication Subnet-State Addendum

- **Status:** superseded by the current canister-signature delegated-token contract
- **Applies to:** historical ECDSA delegated-token root trust-anchor distribution
- **Canonical parent:** [Authentication](authentication.md)

## Summary

This addendum records the old delegated-token trust-anchor design. It is no
longer the current verifier contract.

The ECDSA-era model distributed
`SubnetState.auth.delegated_root_public_key` so endpoint guards could verify a
root ECDSA signature over `DelegationCert` without fetching key material during
queries. That was correct for the old root-threshold-ECDSA design, but 0.65
hard-cuts delegated root proofs to IC canister signatures.

Current delegated-token verification uses:

- configured root canister id
- configured or runtime raw IC root public key
- embedded `RootProof::IcCanisterSignatureV1`
- shard public key certified inside `DelegationCert`

It does not read `SubnetState.auth.delegated_root_public_key` for delegated-token
root proof verification.

## Kept Lessons

The following constraints still apply:

- endpoint guards must not fetch key material during query execution
- delegated-token verification must not require proof fanout or proof catch-up
- caches are performance hints, not authority
- root-provided role-attestation key material must not be retagged locally

The `SubnetState` root-public-key fields may remain in code for unrelated
historical or capability paths until the full auth churn cleanup removes or
isolates them. They are not delegated-token root proof authority.

## Removed Current-Contract Roles

These old responsibilities are superseded:

- distributing delegated-token root ECDSA public keys through `SubnetState`
- treating `SubnetState.auth.delegated_root_public_key` as the delegated-token
  root proof trust anchor
- denying otherwise valid tokens because a verifier has not imported a cascaded
  root ECDSA key
- refreshing delegated-token root keys from endpoint guards
