# Canic 0.76 Design Feedback

Status: direction approved; contract tightening required before
implementation.

The architectural decision is correct. The design should stop trying to make
`RootProof::IcCanisterSignatureV1` bridge-free and should move toward a
bridge-free chain-key batch proof. The main thing to fix before implementation
is precision: trust anchors, canonical encoding, Merkle witnesses, issuer proof
binding, active proof storage keys, lazy repair authorization, signing quotas,
algorithm selection, and token expiry rules.

## Blocking Edits Before Implementation

### 1. Do Not Let The Proof Self-Supply The Trusted Public Key

Section 8 currently has:

```rust
pub struct ChainKeyRootSignatureV1 {
    pub algorithm: ChainKeyAlgorithm,
    pub key_id: ChainKeyKeyId,
    pub derivation_path: Vec<Vec<u8>>,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}
```

That is acceptable only if `public_key` is treated as a hint or cached copy,
never as the trust anchor.

Add explicit verifier language:

```text
The verifier MUST NOT trust the public_key embedded in the proof by itself.

The proof is accepted only if the embedded public_key exactly matches either:

1. the verifier-configured root chain-key public key; or
2. the public key derived under the verifier's accepted root_canister_id,
   algorithm, key_id, and derivation_path policy.

A proof-supplied public key that is not already configured or derivable under
accepted root key policy MUST be rejected.
```

This matters because chain-key verification shifts the trust anchor from the
IC certificate/certified-data path to explicit key metadata and verifier
configuration.

### 2. Bind The Signing Canister Identity Explicitly

The management-canister signing key is tied to the calling canister plus
derivation path and key id. For `sign_with_ecdsa`, the corresponding public key
is obtained using the caller's canister id and the same derivation path/key id.
For Schnorr, the requested master key, calling canister, and derivation path
similarly determine the signing key.

The header includes:

```rust
pub root_canister_id: Principal
```

Add this invariant:

```text
The canister that calls sign_with_ecdsa/sign_with_schnorr MUST be the root
canister whose id is signed in ChainKeyBatchHeaderV1.root_canister_id.

A helper signer canister is forbidden unless a future design introduces a
separate root_signing_canister_id field and verifier policy explicitly accepts
that signer.
```

Without this invariant, a later helper-canister implementation could
accidentally change the root authority key.

### 3. Freeze Canonical Encoding Before Signing Code

Section 9 is directionally right, but pseudo-code is not precise enough for a
signature contract.

This shape:

```text
issuer_leaf_hash = hash(
  domain_leaf,
  schema_version,
  root_canister_id,
  issuer_canister_id,
  audience,
  grants,
  ...
)
```

should become a contract like:

```text
issuer_leaf_bytes =
  canonical_encode(ChainKeyIssuerLeafV1)

issuer_leaf_hash =
  SHA256(
    "CANIC_ROOT_DELEGATION_CHAIN_KEY_ISSUER_LEAF_V1" ||
    len_u32_be(issuer_leaf_bytes) ||
    issuer_leaf_bytes
  )
```

The design must define:

1. Hash function.
2. Integer endian.
3. Principal byte encoding.
4. Domain separator byte encoding.
5. Vector ordering.
6. Grant ordering.
7. Duplicate grant behavior.
8. Audience encoding.
9. `registry_hash` derivation.
10. `batch_id` derivation.
11. String normalization, if any.

For grants, do not rely on natural `Vec` order unless the policy intentionally
treats grant order as semantic. Prefer canonical sorting plus duplicate
rejection.

### 4. Specify The Merkle Tree Format

The design should not only say "Merkle witness." Freeze the tree contract:

```text
leaf_hash =
  SHA256(0x00 || domain_leaf || canonical_leaf_bytes)

internal_node_hash =
  SHA256(0x01 || left_child_hash || right_child_hash)

leaf_sort_key =
  (issuer_canister_id_bytes, audience_hash, grants_hash, proof_epoch, batch_id)
```

Rules:

1. Leaves MUST be sorted by `leaf_sort_key`.
2. Duplicate `leaf_sort_key` values MUST be rejected.
3. Empty batches MUST NOT be signed.
4. Witness steps MUST encode direction: `LeftSibling` or `RightSibling`.

Without this, signer and verifier can both "use Merkle" and still disagree.

### 5. Add Issuer Proof Binding Back Into The DTO

Current issuer tokens still appear to use issuer canister signatures. Root is
not only saying "issuer canister X exists"; root is authorizing an issuer proof
mechanism. The chain-key root proof should bind the issuer's signing/proof
policy.

Add fields like:

```rust
pub issuer_proof_algorithm: IssuerProofAlgorithm,
pub issuer_proof_binding_hash: [u8; 32],
pub max_token_ttl_ns: u64,
```

Potentially also:

```rust
pub issuer_seed_hash: [u8; 32],
pub issuer_public_key_policy_hash: [u8; 32],
```

depending on how issuer canister signatures are represented.

Important rule:

```text
Root proof expiry must cap issuer token expiry.

Issuer may not issue a delegated token whose expires_at_ns exceeds the active
root proof expires_at_ns or root-signed max_token_ttl_ns.
```

### 6. Clarify Active Proof Storage Key

The design often says "issuer-specific proof," but the leaf includes issuer,
audience, and grants. That means active proof storage keying matters.

Decide explicitly among:

1. One active proof per issuer, containing all authorized audience/grant state.
2. One active proof per issuer/audience/grants hash.
3. One batch proof installed per issuer, with multiple leaves for that issuer.

A safer storage key is closer to:

```rust
ActiveRootProofKey {
    issuer_canister_id,
    audience_hash,
    grants_hash,
    registry_epoch,
    key_version,
}
```

Alternatively, use a single issuer proof that contains a canonical set of all
authorized grants.

### 7. Make Lazy Repair Authorization Fail Closed

For:

```text
issuer -> root get_or_create_active_delegation_proof
```

Root MUST require:

1. `caller == requested issuer_canister_id`.
2. Caller is registered/enabled under root issuer policy.
3. Requested audience/grants are allowed by root policy.
4. `proof mode == chain_key_batch`.

Do not allow:

1. Caller asks root to produce proof for arbitrary issuer X.
2. Caller supplies arbitrary grants.
3. Caller forces new signing instead of cached batch retrieval.

Pending behavior:

```text
If signing is already in flight, root returns Pending/RetryAfter or awaits the
same in-flight operation. It MUST NOT start a second signing request for the
same batch scope.
```

Queries are free but not persisted. Updates are the correct path for repair and
install because state changes need consensus-backed execution.

### 8. Add Signing Budgets And Quotas

Add this invariant:

```text
Root MUST enforce a maximum number of threshold signing requests per epoch and
per wall-clock window.
```

Root MUST expose metrics:

```text
chain_key_sign_requests_started
chain_key_sign_requests_succeeded
chain_key_sign_requests_failed
chain_key_sign_requests_deduplicated
chain_key_signed_batches
issuer_install_retries
```

Add config:

```text
max_signatures_per_epoch
max_signatures_per_day
min_batch_size_for_proactive_signing
allow_single_issuer_repair_signing
```

Without quotas, a lazy-repair bug could recreate the per-login signing failure
the design is trying to avoid.

### 9. Move ECDSA Versus Schnorr To A Pre-Implementation Gate

The design-lock draft may leave this open, but production signer
implementation must not start until it is resolved. The choice affects:

1. Signature format.
2. Public key encoding.
3. Verification crate.
4. Canonical message handling.
5. Test vectors.
6. Config schema.
7. PocketIC/fake signer behavior.

The spec differences matter. ECDSA signs a 32-byte `message_hash` and returns
`r || s`. Schnorr signs a message blob and returns a 64-byte signature whose
encoding depends on the algorithm, with Ed25519 and BIP340 variants documented
separately.

Add this gate:

```text
No production signer implementation may land until 0.76 chooses:

1. algorithm
2. key_id policy
3. derivation_path policy
4. public-key encoding
5. verifier crate
6. signature test-vector format
```

### 10. Replace "Freshness Equivalent" Phrasing

This line is close but dangerous:

```text
Certified state freshness equivalent, represented by explicit proof epoch,
registry epoch/hash, not-before time, and expiry.
```

It is not equivalent to IC certificate time. It is a replacement freshness
policy.

Suggested wording:

```text
Current IC certificate time/freshness is not preserved as the same primitive.
In chain-key mode, freshness is represented by explicit signed validity fields:
proof_epoch, registry_epoch/hash, not_before_ns, expires_at_ns, and verifier
min-epoch/key-version policy.
```

## Section-By-Section Comments

### Sections 1-2

Keep the wording:

```text
preserves Canic's root-authorized issuer, audience, grant, and bounded
validity-window semantics under a different IC threshold-signature primitive
```

This is the right security framing.

### Section 4

Add this clarifier:

```text
Management-canister threshold signing is not considered an external signer for
this invariant because it is a protocol service callable by canisters.
```

### Section 7

Add default and migration behavior:

```text
New deployments may select chain_key_batch.
Existing deployments remain canister_signature until migration is explicit.
Verifier mode MUST be fail-closed.
Mixed-mode acceptance requires an explicit compatibility window.
```

### Section 8

The DTO is directionally good but needs the public-key trust-anchor fix, issuer
proof binding, and possibly `max_token_ttl_ns`.

Avoid `domain: Vec<u8>` in the runtime DTO unless there is a reason to make it
variable. Prefer a fixed domain implied by `schema_version` and type. If a
domain field is included, the verifier must reject anything except the exact
expected domain bytes.

### Section 9

Good structure, but it needs exact canonicalization and Merkle rules. This
should be the most precise part of the document before implementation.

### Section 10

Add:

```text
The verifier MUST reject proof-supplied public keys that are not configured or
derived under accepted root key policy.

The verifier MUST reject tokens whose token expiry exceeds the root proof expiry
or root-signed max_token_ttl_ns.
```

### Section 11

Keep the signing-volume section. It is one of the strongest parts of the
design.

### Section 12

Replace:

```text
Unknown signing results must not trigger unbounded new signatures. Root may
retry only after the retry policy classifies the previous attempt as unknown
and safe to supersede.
```

with:

```text
Unknown signing results must not trigger a new payload signature. Root should
retry the same persisted batch hash while the batch remains valid, or abandon
the batch as stale. It must not advance to a new batch merely because the
previous signing result was unknown.
```

The IC spec warns that for `SYS_UNKNOWN` or `CANISTER_ERROR`, the signature may
exist even though the canister did not receive it.

### Section 13

Add caller binding and root policy checks.

Also define client behavior when repair is pending:

```text
If root returns Pending/RetryAfter, issuer login/update returns a retryable auth
state rather than issuing a token without a valid active root proof.
```

### Section 14

Add:

```text
Phase 1.5: Golden Fixtures
  - canonical issuer leaf fixtures
  - canonical batch header fixtures
  - Merkle witness fixtures
  - valid/invalid signature fixtures
  - wrong-key/wrong-path fixtures
```

### Section 16

Add tests:

```text
public_key_in_proof_does_not_define_trust_anchor
proof_signed_by_helper_canister_key_rejected
token_expiry_after_root_proof_expiry_rejected
duplicate grant order canonicalizes or rejects deterministically
duplicate issuer/audience/grants leaf rejected
batch with altered leaf ordering rejected
lazy repair for unregistered caller rejected
lazy repair for caller != requested issuer rejected
signing quota exceeded returns retryable/fail-closed state
```

Also check whether PocketIC supports the real management signing API needed by
the selected algorithm. If not, the implementation plan should use:

```text
unit/integration tests with a fake signer trait for state-machine and
signing-volume assertions
```

plus:

```text
a smaller platform test for real sign_with_ecdsa/sign_with_schnorr behavior
where the environment supports it
```

## Suggested Contract Snippets

### Root Chain-Key Trust Anchor Rule

```text
The root chain-key signing authority is identified by:

  root_canister_id
  algorithm
  key_id
  derivation_path
  key_version
  public_key

The signature-producing canister MUST be root_canister_id. In normal operation
root calls sign_with_ecdsa/sign_with_schnorr directly. A signer helper canister
is not allowed unless a future design explicitly adds root_signing_canister_id
and verifier policy accepts it.

The proof may carry public_key for transport/debugging, but verifiers MUST NOT
trust this field by itself. Verifiers accept the proof only when public_key
matches configured root key material or a derived key accepted by local root key
policy.

The signed batch header MUST include root_canister_id, algorithm, key_id,
derivation_path_hash, key_version, proof_epoch, registry_epoch, registry_hash,
tree_root, not_before_ns, and expires_at_ns.
```

### Canonical Batch Rule

```text
leaf_payload = canonical_encode(ChainKeyIssuerLeafV1)
leaf_hash    = SHA256(0x00 || DOMAIN_LEAF || len(leaf_payload) || leaf_payload)

node_hash    = SHA256(0x01 || left_hash || right_hash)

leaves are sorted by:
  issuer_canister_id_bytes
  audience_hash
  grants_hash

duplicate sort keys are rejected
empty batches are rejected
witness steps encode sibling direction
```

## Cost Model Feedback

Reading "traditional ECDSA" as ordinary software/off-chain ECDSA, and
"chain-key" as IC management-canister `sign_with_ecdsa` or
`sign_with_schnorr`:

| Approach | Cost and security posture |
| --- | --- |
| Traditional/off-chain ECDSA | Cheapest on-chain, but violates the no-external-signer and key-custody requirement. |
| Canister software ECDSA | Cheap in cycles, but the private key lives in replicated canister state; not equivalent security. |
| IC chain-key ECDSA/Schnorr | Expensive per signature, but autonomous and threshold-secured. |
| IC canister signatures | Cheap per proof, but the current root-proof flow requires an external direct-query bridge. |

The cost-safe model is still per-batch per-epoch signing, not per-login
signing, and preferably not per-issuer signing if issuers can share one signed
batch.

### Unit Costs From Feedback

These numbers should be revalidated against the official IC pricing docs before
implementation.

| Operation | Cost |
| --- | --- |
| Query call | Free |
| Update message execution base, 13-node subnet | `5_000_000` cycles |
| 1B Wasm instructions, 13-node subnet | `1_000_000_000` cycles |
| Inter-canister call overhead, 13-node subnet | `260_000` cycles plus bytes |
| Ingress reception, 13-node subnet | `1_200_000` cycles plus bytes |
| Production chain-key `sign_with_ecdsa` or `sign_with_schnorr`, `key_1` | `26_153_846_153` cycles, about `$0.0357` |
| Test chain-key `test_key_1` | `10_000_000_000` cycles, about `$0.0137` |

The feedback states that the ICP docs list the same production fee for
threshold ECDSA and threshold Schnorr under `key_1`, and that
`ecdsa_public_key` and `schnorr_public_key` have no cycle cost. Ordinary
update/query/call costs are separate from the signing fee.

### Formula

Let:

```text
L = logins per day
I = issuers
E = proof epochs per day
P = number of independent batches/partitions per epoch
```

Then:

```text
per-login signatures/day           = L
per-issuer per-epoch signatures/day = I * E
per-batch per-epoch signatures/day  = P * E

chain_key_cost/day =
  signatures/day * 26_153_846_153 cycles

chain_key_usd/day =
  signatures/day * $0.0357
```

Example assumptions:

```text
L = 100,000 logins/day
I = 100 issuers
E = 24 epochs/day
P = 1 batch/epoch
```

Chain-key ECDSA/Schnorr signing fee only:

| Model | Signatures/day | Cycles/day | Approx USD/day | Assessment |
| --- | ---: | ---: | ---: | --- |
| Per login | 100,000 | 2,615.38T | $3,570.00 | Hard no |
| Per issuer per hour | 2,400 | 62.77T | $85.68 | Possibly acceptable only if issuer count is small |
| Per issuer per day | 100 | 2.62T | $3.57 | Reasonable for small issuer count |
| Per batch per hour | 24 | 0.628T | $0.86 | Good |
| Per batch per 15 min | 96 | 2.51T | $3.43 | Still reasonable |
| Per batch per day | 1 | 0.026T | $0.036 | Very cheap |

## Final Assessment

Once the trust-anchor, signing identity, canonical encoding, Merkle format,
issuer proof binding, active-proof key, lazy-repair authorization, signing
quotas, algorithm gate, and token-expiry constraints are tightened, this is a
solid 0.76 design doc.
