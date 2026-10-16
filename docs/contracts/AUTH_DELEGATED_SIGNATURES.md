# Auth Contract: Root Delegation Proofs

Canic retains Root delegation proof infrastructure and role attestations.
Portable application session admission is unfinished and exposes no live
establishment endpoint. The [authentication design](../architecture/authentication.md)
separates these supported boundaries from the planned application integration.

## Authority and ownership

Root configuration binds the exact issuer, Fleet audience, grants, chain-key
policy, epochs and validity window. IC Auth owns canonical encoding and proof
verification. Canic supplies protected authority and bounded inputs; submitted
proofs cannot enlarge configured authority.

Root prepares signed batches, installs verified proofs on issuers and renews
configured issuers. Installation accepts only the exact Root authority and
validates the proof before storage. Active proof status reports that local
material; it is not evidence of an admitted application session.

## Root Chain-Key Batch Proof

Root delegation proof creation uses IC management-canister chain-key
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

The embedded `ChainKeyDelegationCertV1` must agree with `DelegationCert` on
issuer identity, proof algorithm and binding, audience, grants, validity and TTL
bounds, registry epoch and registry hash. The current `RootProof` uses the
chain-key batch variant. Role attestations carry their separate Root
canister-signature proof.

## Provisioning and operational recovery

The [Root proof runbook](../operations/root-proof-provisioning.md) owns setup,
renewal, status and retry procedures. Identical configuration retries preserve
usable signed material and renewal state. Paid signing retains intent and
retry-after evidence; stale callbacks cannot overwrite newer authority.

## Application integration boundary

Fleet Root manages approved issuer/key enrollment for portable Auth. Canic's
bounded session/login adapter preserves IC Auth's account binding and replay
custody, including atomic refusal and same-release restoration. Endpoint wiring,
clock/floor persistence and installed acceptance remain incomplete. Public local
application authorization refuses while that integration is unavailable.

Transport authorization remains independent: application credentials cannot
supply controller, placement, funding or lifecycle authority.

## Continue From Here

- [Authentication design](../architecture/authentication.md)
- [Access contract](ACCESS_ARCHITECTURE.md)
