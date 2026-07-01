# Root Delegation Proof Renewal Audit

Date: 2026-06-29

Superseded: this audit describes the pre-0.76 bridge-backed
canister-signature renewal design. It is retained as historical evidence only.
The active 0.76 delegated-auth path is chain-key batch renewal and must not use
the bridge/direct-root-query flow described below.

Scope: Canic delegated auth root proof renewal, external liveness
dependencies, and migration impact for a bridge-free chain-key root proof.

This audit is repo-grounded. It does not implement a redesign.

## 1. Executive Summary

The current root-managed delegated proof renewal design has an external
liveness dependency. Root can schedule and prepare renewal batches from a timer,
but the current proof primitive is `RootProof::IcCanisterSignatureV1`, which
requires a direct root query to assemble proof material from root
`certified_data` and the IC `data_certificate`. The 0.74 design says root cannot
complete the proof flow entirely inside a timer or update call, and names the
renewal bridge as the component that performs the direct root query and submits
the proof batch back to root for install (`docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md:29`,
`docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md:35`,
`docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md:37`).

The current design cannot be made bridge-free without changing the proof
primitive. The repo explicitly records that issuer composite-query retrieval,
issuer wrappers, root self-calls, and canister-to-canister calls are not valid
substitutes for a direct root query (`docs/design/archive/0.68-root-proof-provisioning/0.68-design.md:96`,
`docs/design/archive/0.68-root-proof-provisioning/0.68-design.md:98`,
`docs/design/archive/0.68-root-proof-provisioning/0.68-design.md:101`).
The active auth ops module repeats the same shape: prepare runs in a root
update, get runs only as a direct root query so root has `data_certificate()`,
and install validates submitted proofs before issuer-local install
(`crates/canic-core/src/ops/auth/delegation/mod.rs:10`).

Recommended direction: replace the root delegation proof primitive with a
chain-key root proof, preferably a per-batch per-epoch signature if multiple
issuers can share one signed batch. A per-issuer per-epoch proof is acceptable
for small issuer counts. Per-login root signing is a design failure unless a
future product design documents and enforces very small bounded login volume.

Confidence: high for the conclusion that the current canister-signature proof
shape structurally requires an external direct root query. Confidence: medium
for the chain-key migration shape because the active repo no longer contains
chain-key auth wrappers, key-id config, or verifier code; those parts require
new implementation and platform-spec verification.

Major unknowns:

- Whether Canic should standardize on threshold ECDSA or threshold Schnorr.
- Exact management-canister Candid args and verification library choices for
  the selected chain-key algorithm. The active repo has no `sign_with_ecdsa`,
  `ecdsa_public_key`, `sign_with_schnorr`, or `schnorr_public_key` code under
  `crates/`; this is an external-spec dependency.
- Whether batch signing should be a simple signed vector hash or a Merkle tree
  with per-issuer witnesses.
- Operational key model: key id, derivation path, key rotation, and production
  rejection of test keys.
- Stable-state migration strategy for existing active
  `RootProof::IcCanisterSignatureV1` records.

## 2. Current Design Map

The active DTO model has one root proof variant:
`RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1)`
(`crates/canic-core/src/dto/auth.rs:28`). The proof stores
`signature_cbor` and `public_key_der` only
(`crates/canic-core/src/dto/auth.rs:46`). A `DelegationProof` embeds a
`DelegationCert` plus `root_proof` (`crates/canic-core/src/dto/auth.rs:93`).
Issuer-local active proof state stores that proof plus `cert_hash`, time
bounds, refresh threshold, and install metadata
(`crates/canic-core/src/dto/auth.rs:103`).

Current documented root proof issuance is update-then-query because certified
data is committed during update execution and the data certificate is available
during query execution (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:186`).
The documented flow is:

```text
provisioner    -> root canic_prepare_delegation_proof_batch update
provisioner    -> root canic_get_delegation_proof_batch direct query
provisioner    -> root canic_install_delegation_proof_batch update
root           -> issuer canic_install_active_delegation_proof update
caller/session -> issuer canic_prepare_delegated_token update
caller/session -> issuer canic_get_delegated_token query
caller/session -> endpoint with DelegatedToken
```

Evidence: `docs/contracts/AUTH_DELEGATED_SIGNATURES.md:192`.

Root-managed renewal reuses the batch proof contract. Root stores renewal
templates, per-issuer attempts, and scheduled renewal batches, while
provisioners can retrieve proof material only through
`canic_get_delegation_renewal_proof_batch`
(`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:228`).
The operation runbook describes the long-term flow:

```text
root timer             -> root prepares due scheduled renewal attempts
bridge/provisioner     -> root canic_delegation_renewal_work query
bridge/provisioner     -> root canic_get_delegation_renewal_proof_batch direct query
bridge/provisioner     -> root canic_install_delegation_proof_batch update
root                   -> issuer canic_install_active_delegation_proof update
operator/medic         -> root canic_root_issuer_renewal_status query
operator/medic         -> issuer canic_active_delegation_proof_status query
```

Evidence: `docs/operations/root-proof-provisioning.md:41`.

Implementation map:

- Preparation:
  `prepare_delegation_proof_batch` calls
  `AuthOps::prepare_root_canister_signature` for each cert hash
  (`crates/canic-core/src/ops/auth/delegation/batch.rs:88`,
  `crates/canic-core/src/ops/auth/delegation/batch.rs:103`).
- Certified data / signature map:
  `prepare_root_canister_signature` adds a signature-map leaf and refreshes
  certified data (`crates/canic-core/src/ops/auth/root_canister_sig.rs:210`,
  `crates/canic-core/src/ops/auth/root_canister_sig.rs:217`,
  `crates/canic-core/src/ops/auth/root_canister_sig.rs:218`).
  `refresh_root_canister_sig_certified_data` calls
  `cdk::api::certified_data_set(labeled_hash(...))`
  (`crates/canic-core/src/ops/auth/root_canister_sig.rs:239`,
  `crates/canic-core/src/ops/auth/root_canister_sig.rs:243`).
- Proof assembly:
  `get_root_canister_signature_proof` calls
  `get_signature_as_cbor` and returns `RootProof::IcCanisterSignatureV1`
  (`crates/canic-core/src/ops/auth/root_canister_sig.rs:258`,
  `crates/canic-core/src/ops/auth/root_canister_sig.rs:285`,
  `crates/canic-core/src/ops/auth/root_canister_sig.rs:294`).
- Direct-query bridge step:
  `canic_get_delegation_renewal_proof_batch` is a query endpoint available to a
  controller or renewal provisioner
  (`crates/canic/src/macros/endpoints/root.rs:138`). It resolves scheduled
  proof refs from the batch id (`crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/retrieval.rs:55`).
- Install/update:
  `canic_install_delegation_proof_batch` is an update endpoint available to a
  controller or renewal provisioner
  (`crates/canic/src/macros/endpoints/root.rs:150`). The runtime workflow
  validates each proof, calls issuer install, marks successful pending metadata
  installed, and records renewal state
  (`crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs:31`,
  `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs:68`,
  `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs:80`,
  `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs:85`).
- Issuer storage:
  `install_active_delegation_proof` verifies the root proof, computes
  `cert_hash`, and stores active proof state
  (`crates/canic-core/src/ops/auth/delegation/active.rs:29`,
  `crates/canic-core/src/ops/auth/delegation/active.rs:42`,
  `crates/canic-core/src/ops/auth/delegation/active.rs:62`).
- Verifier consumption:
  normal endpoint auth verifies embedded proofs locally and does not call root
  (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:317`).

ASCII sequence diagram:

```text
Root timer/update
  -> AuthOps::prepare_due_delegation_renewals
  -> AuthOps::prepare_delegation_proof_batch
  -> AuthOps::prepare_root_canister_signature
  -> signature_map.add_signature(cert_hash)
  -> certified_data_set(labeled_hash("sig", root_hash))
  -> persist scheduled renewal batch and attempt refs

External bridge/provisioner
  -> query root canic_delegation_renewal_work
  -> direct query root canic_get_delegation_renewal_proof_batch
     -> get_signature_as_cbor needs root data_certificate()
     -> return DelegationProof batch
  -> update root canic_install_delegation_proof_batch

Root update
  -> validate proof matches pending metadata
  -> call issuer canic_install_active_delegation_proof update
  -> record scheduled renewal install outcome

Issuer update
  -> verify root canister-signature proof
  -> persist ActiveDelegationProof

Login
  -> caller update issuer canic_prepare_delegated_token
  -> issuer signs claims with issuer canister signature
  -> caller query issuer canic_get_delegated_token
  -> caller sends self-contained DelegatedToken to verifier endpoint
```

## 3. External Moving Parts Inventory

| External moving part | Evidence | Classification |
| --- | --- | --- |
| External direct root query for `canic_get_delegation_proof_batch` | Contract requires direct query in the issuance flow (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:192`) and says the get endpoint is a direct root query over pending metadata (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:238`). | Required for correctness of current canister-signature proof material. |
| External direct root query for `canic_get_delegation_renewal_proof_batch` | Renewal get is also a direct root query (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:243`). | Required for renewal liveness and proof correctness. |
| Renewal bridge / provisioner | 0.74 design says the bridge performs the direct root query and submits proof batch to root (`docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md:35`). | Required for renewal liveness under current proof primitive. |
| `canic auth renewal run-once` CLI | Runbook says the bridge can be the CLI (`docs/operations/root-proof-provisioning.md:54`). CLI implementation queries renewal work, queries proof batch, then calls install update (`crates/canic-cli/src/auth/mod.rs:711`). | Optional implementation of the required bridge role; required if no other bridge exists. |
| Renewal provisioner ACL | Endpoint macros expose work/get/install to controller or `caller::is_delegation_renewal_provisioner()` (`crates/canic/src/macros/endpoints/root.rs:112`, `crates/canic/src/macros/endpoints/root.rs:138`, `crates/canic/src/macros/endpoints/root.rs:150`). | Required only if a non-controller bridge is used. |
| Manual controller prepare/get/install | Runbook lists manual provisioning as explicit controller/operator action (`docs/operations/root-proof-provisioning.md:25`). | Required for initial provisioning and manual repair under current design; not normal-login correctness once active proof is installed. |
| Product frontend/client provisioning | 0.74 says product frontends must not run delegated-proof provisioning (`docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md:43`). | Forbidden for normal product auth; not an intended dependency. |
| Medic/status CLI | Runbook says status/medic observe drift and do not mutate root state (`docs/operations/root-proof-provisioning.md:124`, `docs/operations/root-proof-provisioning.md:130`). | Optional/debug/operations only. |
| CI packaged CLI probes | `scripts/ci/auth-renewal-cli-proof-lib.sh` runs `auth renewal run-once` against a fake fixture (`scripts/ci/auth-renewal-cli-proof-lib.sh:38`). | Test/proof only. |
| Off-chain signer | No active repo evidence of an off-chain signer in delegated auth. | Not present. |
| Host daemon | 0.74 says `daemon` remains a follow-up (`docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md:761`). | Not implemented; future optional bridge form, but forbidden by the new product invariant. |

## 4. Canister-Signature Proof Analysis

### Where Certified Data Is Set

Root proof preparation calls
`cdk::api::certified_data_set(labeled_hash(LABEL_SIG, signature_root_hash))`
in `refresh_root_canister_sig_certified_data`
(`crates/canic-core/src/ops/auth/root_canister_sig.rs:239`,
`crates/canic-core/src/ops/auth/root_canister_sig.rs:243`). Issuer token proof
creation uses the same pattern for issuer-local signatures
(`crates/canic-core/src/ops/auth/issuer_canister_sig.rs:229`,
`crates/canic-core/src/ops/auth/issuer_canister_sig.rs:234`).

### Where Data Certificate Is Retrieved

The repo does not call `ic_cdk::api::data_certificate()` directly in Canic
auth code. Instead, proof assembly calls
`signature_map.get_signature_as_cbor`, which can fail with a data-certificate
message that is mapped to `RootDataCertificateUnavailable`
(`crates/canic-core/src/ops/auth/root_canister_sig.rs:285`,
`crates/canic-core/src/ops/auth/root_canister_sig.rs:303`,
`crates/canic-core/src/ops/auth/root_canister_sig.rs:305`,
`crates/canic-core/src/ops/auth/root_canister_sig.rs:306`). This is the active
code-level indicator that retrieval depends on query data-certificate context.

### Can Root Retrieve Its Own Data Certificate From A Timer Or Update?

Repo answer: no. 0.74 states root can prepare canister-signature leaves during
an update, but cannot assemble root proof material in the same update because
that assembly needs the root data certificate from a direct root query
(`docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md:114`).
It explicitly forbids update-time proof assembly without a data certificate
(`docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md:118`).

The root timer implementation only prepares due renewals:
`RootDelegationRenewalWorkflow::sweep` calls
`AuthOps::prepare_due_delegation_renewals` and returns after logging a prepared
batch id (`crates/canic-core/src/workflow/runtime/auth/renewal.rs:73`,
`crates/canic-core/src/workflow/runtime/auth/renewal.rs:81`,
`crates/canic-core/src/workflow/runtime/auth/renewal.rs:92`). It does not call
proof retrieval or issuer install.

### Can Issuer Composite-Query Root And Get A Root Data Certificate?

Repo answer: no. The 0.68 design says the old issuer-local composite-query shape
failed because root must assemble proof in a query context where
`data_certificate()` is available, and the nested root query does not provide
that certificate context (`docs/design/archive/0.68-root-proof-provisioning/0.68-design.md:41`,
`docs/design/archive/0.68-root-proof-provisioning/0.68-design.md:48`). The same
design says `canic_get_delegation_proof_batch` must not be called through
issuer composite-query wrappers, issuer wrappers, root self-calls, or
canister-to-canister calls (`docs/design/archive/0.68-root-proof-provisioning/0.68-design.md:96`,
`docs/design/archive/0.68-root-proof-provisioning/0.68-design.md:98`).

Active contracts keep that invariant. The delegated signatures contract says
root proof retrieval must not be hidden behind issuer composite-query wrappers
because root needs its direct query data certificate
(`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:247`).

### Can A Composite Query Install Or Store A Proof?

Current issuer storage requires an update endpoint:
`canic_install_active_delegation_proof` is emitted as a `canic_update`
controller-gated endpoint (`crates/canic/src/macros/endpoints/nonroot.rs:45`).
Issuer active proof installation persists `ActiveDelegationProof` through
`AuthStateOps::set_active_delegation_proof` (`crates/canic-core/src/ops/auth/delegation/active.rs:62`,
`crates/canic-core/src/ops/auth/delegation/active.rs:78`). A query or composite
query cannot be the current persistent install path.

### Is Any Code Relying On A Nested/Composite Root Query Being Equivalent To A Direct Root Query?

No active code path found. The repo contains the opposite assertions: historical
design marks nested root retrieval as failed (`docs/design/archive/0.68-root-proof-provisioning/0.68-design.md:41`),
active contracts forbid it (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:247`),
and the root ops module states get runs only as a direct root query
(`crates/canic-core/src/ops/auth/delegation/mod.rs:10`).

Conclusion: a bridge is structurally required for the current
canister-signature proof shape. Root can own policy, scheduling, and install
validation, but it cannot complete proof material assembly from a timer/update
without an external direct root query.

## 5. Chain-Key Replacement Candidate

A bridge-free replacement should introduce a new proof primitive, for example:

- `RootProof::IcChainKeySignatureV1`
- `RootProof::IcChainKeyBatchSignatureV1`

The primitive should preserve Canic authorization semantics:

- root authorizes issuer eligibility;
- root authorizes audience and grants;
- root sets bounded validity windows;
- issuers cache/store active proofs;
- relying verifiers evaluate embedded, self-contained token material locally.

It will not preserve the exact current canister-signature proof guarantees. The
proof primitive changes from "IC-certified canister state plus witness/hash tree
under the configured IC root key" to "IC threshold signature over a canonical
root-authored payload or batch hash under a configured chain-key public key."

Candidate timer flow:

```text
root timer/update
  -> select due templates under root issuer policy
  -> build canonical delegation certs or batch leaves
  -> call management canister sign_with_ecdsa or sign_with_schnorr
  -> verify returned signature defensively
  -> persist signed batch/proof state
  -> call issuer canic_install_active_delegation_proof update
  -> issuers verify and store active proof
  -> normal logins reuse stored proof
```

Candidate lazy repair:

```text
issuer login/update
  -> sees missing/stale active proof
  -> issuer update-calls root get_or_create_active_delegation_proof
  -> root returns cached active proof if valid
  -> root signs only if no valid signed proof/batch exists and no signing is in flight
  -> issuer stores proof
  -> login continues using issuer-local token prepare
```

This lazy repair path needs careful gating so it does not turn root signing into
per-login signing. It should use cached proof epochs and singleflight
deduplication.

Original 2026-06-29 repo blockers for this migration:

- Active DTOs only define `RootProof::IcCanisterSignatureV1`
  (`crates/canic-core/src/dto/auth.rs:28`).
- Stable records only define `RootProofRecord::IcCanisterSignatureV1`
  (`crates/canic-core/src/storage/stable/auth/records.rs:101`).
- Active features enable canister-signature create/verify dependencies but no
  chain-key auth features (`crates/canic-core/Cargo.toml:19`).
- Active management infra has lifecycle/cycles/randomness/snapshots/status
  modules, but no chain-key signing module
  (`crates/canic-core/src/infra/ic/mgmt/mod.rs:7`).
- A repo search under `crates/` found no active `sign_with_ecdsa`,
  `ecdsa_public_key`, `sign_with_schnorr`, or `schnorr_public_key` usage.

Close-out status is tracked in `docs/design/0.76-auth/0.76-design.md` and
`docs/status/current.md`; these baseline blockers are no longer the current
0.76 release state.

## 6. Signing-Volume Analysis

### Would Current Login Flow Call Root Per Login?

No. Current login/token issuance is issuer-local after active proof install.
`prepare_delegated_token_issuer_proof` reads
`Self::active_delegation_proof(now_ns)` and then prepares issuer canister
signature material over claims (`crates/canic-core/src/ops/auth/token.rs:83`,
`crates/canic-core/src/ops/auth/token.rs:96`,
`crates/canic-core/src/ops/auth/token.rs:110`). The public auth architecture
also says normal delegated auth does not call the management canister during
`prepare_delegated_token` (`docs/architecture/authentication.md:377`).

### Where Could Per-Login Signing Accidentally Happen?

Risk points in a chain-key redesign:

- adding root signing directly to `RuntimeAuthWorkflow::prepare_delegated_token`
  or `AuthOps::prepare_delegated_token_issuer_proof`;
- making issuer login repair always request a fresh root signature rather than a
  cached epoch proof;
- signing `DelegatedTokenClaims`, nonce, subject, or session/login data at root;
- failing to dedupe simultaneous missing-proof repairs;
- treating stale-but-valid proof refresh as mandatory for every login instead
  of a background/lazy epoch operation.

### What Data Should Root Sign?

Root should sign stable delegation-epoch authorization data:

- root canister id;
- issuer canister id or batch issuer leaf;
- audience and grants;
- registry epoch/hash if used for anti-stale topology binding;
- proof epoch/batch id;
- not-before and expiry;
- algorithm/key metadata;
- issuer proof algorithm/binding if issuer tokens remain canister signatures.

This maps to existing `DelegationCert` semantics
(`crates/canic-core/src/dto/auth.rs:74`) and existing root issuer policy checks
(`crates/canic-core/src/domain/policy/auth/root_provisioning.rs:186`).

### What Data Must Not Be Signed By Root?

Root should not sign per-login data by default:

- `DelegatedTokenClaims.subject`;
- token nonce;
- login operation id;
- token `issued_at_ns`/`expires_at_ns`;
- user session-specific `ext`;
- endpoint payloads;
- delegated-token claim hash.

Those are issuer-local token issuance fields today
(`crates/canic-core/src/dto/auth.rs:162`) and are intentionally bounded by the
root-certified `DelegationCert` instead of root-signed per login.

### Proof Caching

Cache signed root proof material by:

- proof epoch or batch id;
- template fingerprint;
- registry hash;
- key version and key id;
- issuer set and leaf hashes for batch mode;
- not-before/expires-at.

Issuer caches active proof in existing active proof state
(`crates/canic-core/src/dto/auth.rs:103`). Root should also persist signed batch
state so lazy repair can return a cached proof and so timer retries do not
re-sign.

### Singleflight / In-Flight Signing Deduplication

Root state should move from today's `Prepared` transport-window model
(`crates/canic-core/src/domain/policy/auth/root_provisioning.rs:115`) to a
signing-aware state machine:

- if a signing request is in flight for `(proof_epoch, template_fingerprint,
  registry_hash, key_version)`, later requests attach to the same batch;
- unknown signing result should transition to a repair state that first checks
  whether a signed proof was persisted before retrying;
- failed management calls should back off and not create one signing request per
  login.

The old 0.61 replay design documented a prior threshold-ECDSA in-flight marker
for root proof signing (`docs/design/archive/0.61-replay-protection/0.61-design.md:990`).
That old design is not active, but the migration should recover the useful
singleflight idea.

### Expected Signing Frequency

| Mode | Expected signing frequency | Assessment |
| --- | --- | --- |
| Per-login signing | One root threshold signature per login/session. | Fails the product requirement by default. Recreates prior constant signing pressure unless the product has very small bounded login volume. |
| Per-issuer per-epoch signing | One signature per issuer per renewal epoch. | Acceptable for small issuer counts. Simpler than batch witnesses but scales linearly with issuers. |
| Per-batch per-epoch signing | One signature for a batch root per renewal epoch, with per-issuer inclusion witness. | Preferred if Canic can define stable batch leaves and verifier witness checks. Minimizes chain-key calls. |

## 7. Proposed Canonical Signed Payload

The new payload must make explicit the authority data that current
canister-signature verification derives from the IC certificate, the root
canister-signature public key, and the existing `DelegationCert` hash.

Single-issuer payload:

```text
CanicRootDelegationChainKeyV1 {
  domain_separator: "CANIC-ROOT-DELEGATION-CHAIN-KEY-V1",
  schema_version: 1,
  root_canister_id: Principal,
  issuer_canister_id: Principal,
  issuer_proof_algorithm: IssuerProofAlgorithm,
  issuer_proof_binding_hash: [u8; 32],
  issuer_proof_binding: IssuerProofBinding,
  audience: DelegationAudience,
  grants: Vec<DelegatedRoleGrant>,
  registry_epoch: u64,
  registry_hash: [u8; 32],
  proof_epoch: u64,
  batch_id: [u8; 32],
  not_before_ns: u64,
  issued_at_ns: u64,
  expires_at_ns: u64,
  max_token_ttl_ns: u64,
  key_version: u64,
  algorithm: ChainKeyAlgorithm,
  key_id: String,
  derivation_path_hash: [u8; 32],
}
```

Batch signing proposal:

```text
issuer_leaf_hash =
  sha256("CANIC-ROOT-DELEGATION-BATCH-LEAF-V1" ||
         issuer_canister_id ||
         issuer_proof_algorithm ||
         issuer_proof_binding_hash ||
         canonical_audience ||
         canonical_grants ||
         not_before_ns ||
         issued_at_ns ||
         expires_at_ns ||
         max_token_ttl_ns ||
         registry_epoch ||
         registry_hash)

batch_header =
  "CANIC-ROOT-DELEGATION-BATCH-V1" ||
  schema_version ||
  root_canister_id ||
  proof_epoch ||
  batch_id ||
  tree_root ||
  not_before_ns ||
  expires_at_ns ||
  key_version ||
  algorithm ||
  key_id ||
  derivation_path_hash

signature_message = sha256(batch_header)
```

The proof carried to issuers/verifiers would contain:

- batch header;
- issuer leaf payload or existing `DelegationCert` plus extra epoch/key fields;
- Merkle witness from issuer leaf to tree root;
- chain-key signature over the batch header hash;
- public-key metadata needed by verifier.

Verifier checks:

- canonical issuer leaf hash recomputes;
- witness reconstructs batch `tree_root`;
- signature verifies over `signature_message`;
- root id, key metadata, epoch, audience, grants, issuer id, and time windows
  match local expectations and the token claims.

Open question: current canonicalization has `CanonicalDomain::DelegationCert`,
`DelegatedTokenClaims`, `DelegationProof`, `RoleHash`, and `IssuerProof`
(`crates/canic-core/src/ops/auth/delegated/canonical.rs:28`). Migration should
add new canonical domains for chain-key root payloads/batches instead of
overloading the old `DelegationCert` hash without key/epoch metadata.

## 8. Verifier-Contract Delta

### Current Canister-Signature Verifier Obligations

Current contract checks:

- root proof variant is `RootProof::IcCanisterSignatureV1`;
- public key DER embeds configured root canister id and expected seed;
- canister signature verifies under configured raw IC root key;
- issuer proof variant is `IssuerProof::IcCanisterSignatureV1`;
- issuer public key DER embeds issuer id and expected seed;
- cert/token time windows, audience, grants, subject binding, local role, and
  scopes all pass (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:286`).

Implementation evidence:

- root canister-signature verification parses public key DER, checks root id,
  checks seed, builds `domain_len || domain || payload_hash`, and calls
  `ic_signature_verification::verify_canister_sig`
  (`crates/canic-core/src/ops/auth/root_canister_sig.rs:323`,
  `crates/canic-core/src/ops/auth/root_canister_sig.rs:331`,
  `crates/canic-core/src/ops/auth/root_canister_sig.rs:347`);
- token verifier calls root proof verifier over `cert_hash`, then issuer proof
  verifier over `claims_hash`
  (`crates/canic-core/src/ops/auth/delegated/verify.rs:121`,
  `crates/canic-core/src/ops/auth/delegated/verify.rs:128`).

The current verifier gets these properties from the IC certificate/canister
signature proof:

- root canister id bound into the canister-signature public key;
- root seed/domain binding;
- verification under the configured IC root public key;
- certified-data witness shape inside `signature_cbor`, through the
  `ic-signature-verification` library.

### Proposed Chain-Key Verifier Obligations

A chain-key verifier must check explicitly:

- proof variant is `RootProof::IcChainKeySignatureV1` or
  `RootProof::IcChainKeyBatchSignatureV1`;
- configured root canister id is present in signed payload/header;
- algorithm is accepted;
- key id is accepted;
- derivation path or derivation-path hash is accepted;
- key version is not below minimum accepted version;
- public key is configured or derivable and matches the expected key metadata;
- signature verifies over canonical payload or batch header hash;
- issuer leaf inclusion witness is valid in batch mode;
- issuer id matches cert/token issuer;
- audience and grants match the signed payload and remain valid for local
  verifier;
- registry epoch/hash satisfy local anti-stale policy;
- not-before/expires-at use strict expiry;
- proof epoch is not stale or downgraded;
- issuer proof algorithm/binding remains as expected if issuer tokens continue
  to use canister signatures.

Anything current verification gets from `public_key_der` and the IC certificate
must become explicit signed payload fields or verifier config in the chain-key
design. In particular, time/freshness/epoch binding should not be assumed from
the threshold signature; it must be signed and verified.

## 9. Migration Scope

No files were modified for migration in this audit. Likely change set:

### Type Definitions

- `crates/canic-core/src/dto/auth.rs`: add root proof chain-key variants,
  payload/header/witness DTOs, algorithm/key metadata DTOs, possibly batch
  proof DTOs.
- `crates/canic-core/src/storage/stable/auth/records.rs`: add stable record
  variants for chain-key root proofs and signed batch state.
- `crates/canic-core/src/ops/storage/auth/mapper.rs`: map new DTO/record
  variants and migration compatibility.
- `crates/canic-core/src/ops/auth/delegated/canonical.rs`: add canonical
  domains and encoders for chain-key payloads, batch leaves, headers, and
  witnesses.

### Root Canister State Machine

- `crates/canic-core/src/domain/policy/auth/root_provisioning.rs`: extend
  renewal attempt statuses and state for `Signing`, `Signed`, `Installing`, and
  signed batch metadata.
- `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/schedule.rs`:
  replace prepare-only sweep with signing-aware batch selection and state
  transition.
- `crates/canic-core/src/workflow/runtime/auth/renewal.rs`: timer should drive
  prepare/sign/install progress, not just prepare.
- `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs`: install
  path may need to accept root-owned signed batches directly without requiring
  external submitted proof material.
- `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/install.rs`:
  record partial installs and retry signed batch fanout.

### Chain-Key Management-Canister Integration

- `crates/canic-core/src/infra/ic/mgmt/*`: add raw management calls for selected
  chain-key public key and signing APIs.
- `crates/canic-core/src/ops/ic/mgmt/*`: add metrics-wrapped ops layer for
  chain-key calls.
- `crates/canic-core/src/workflow/ic/mgmt.rs` or a new auth-specific workflow:
  orchestrate async signing, defensive verification, retry, and state updates.
- `crates/canic-core/Cargo.toml` and `crates/canic/Cargo.toml`: add carefully
  scoped chain-key signing/verify features and dependencies.

### Issuer Install Endpoint

- `crates/canic-core/src/ops/auth/delegation/active.rs`: verify chain-key root
  proof variant on install.
- `crates/canic-core/src/ops/auth/delegated/active_proof.rs`: keep issuer/time
  validation; ensure root proof callback can handle new variants.
- `crates/canic/src/macros/endpoints/nonroot.rs`: endpoint shape may remain
  update/install, but lazy repair may require a new issuer-to-root repair
  update.

### Verifier Library

- `crates/canic-core/src/ops/auth/root_canister_sig.rs`: either generalize to
  `root_proof` verification or add separate chain-key verifier module.
- `crates/canic-core/src/ops/auth/token.rs`: `AuthProofVerifierConfig` must
  include key metadata in addition to or instead of raw IC root key.
- `crates/canic-core/src/ops/auth/types.rs`: extend verifier config with
  chain-key public key, key id, derivation path policy, key version, and mode.
- `crates/canic-core/src/ops/auth/delegated/verify.rs`: branch root proof
  verification by variant and enforce anti-downgrade mode.
- `crates/canic-core/src/access/auth/token.rs`: no root/management call should
  be introduced on endpoint hot path.

### Docs and Contracts

- `docs/contracts/AUTH_DELEGATED_SIGNATURES.md`: replace or version the trust
  model, proof types, issuance flow, verifier contract, forbidden patterns, and
  config contract.
- `docs/architecture/authentication.md`: update root proof lifecycle and
  verifier algorithm.
- `docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md`:
  supersede or mark bridge design as legacy.
- `docs/operations/root-proof-provisioning.md`: remove bridge as liveness
  requirement for new mode and add key operations/runbook.
- `docs/audits/release-lines/0.65-zero-ecdsa-cleanup.md` and archived 0.65
  docs should remain historical, but new docs must explicitly state why the
  product invariant reverses the earlier zero-ECDSA auth decision.

### Tests

- `crates/canic-tests/tests/root_cases/sharding.rs`: add no-external-liveness
  PocketIC test and replace bridge-required renewal assertion for new mode.
- `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/tests.rs`:
  add signing state machine and singleflight tests.
- `crates/canic-core/src/ops/auth/delegated/verify.rs`: add chain-key proof
  verifier negative tests.
- `crates/canic/tests/protocol_surface.rs`: update endpoint/protocol proof
  surface tests for mode flags and legacy rejection.
- CLI proof scripts should be updated only after bridge removal/deprecation
  strategy is decided.

### Bridge Removal/Deprecation

- `crates/canic-cli/src/auth/mod.rs`: `auth renewal run-once` should become
  legacy/manual repair for canister-signature mode, not required for liveness.
- `crates/canic-cli/src/auth/render.rs` and `crates/canic-cli/src/auth/tests.rs`:
  update output and warnings for bridge-free mode.
- `scripts/ci/auth-renewal-cli-proof-lib.sh`: keep legacy proof or replace with
  proof that bridge-free mode does not need run-once.

### Configuration

- `crates/canic-core/src/config/schema/mod.rs` and
  `crates/canic-core/src/config/validation/auth.rs`: add chain-key algorithm,
  key id, derivation path, public key/trust anchor, key version, min proof
  epoch, test-key policy, and migration mode.
- Current config validates `ic_root_public_key_raw_hex` and network pairing for
  canister-signature verification (`crates/canic-core/src/config/validation/auth.rs:54`).
  Chain-key mode needs equivalent fail-closed validation.

### Migration/Compatibility Flags

- Add mode flags such as `root_proof_mode = "canister_signature" |
  "chain_key_batch"`.
- Add a verifier setting to reject legacy bridge/canister-signature proofs when
  bridge-free mode is enabled.
- Define one-time migration or compatibility for stored active proofs and root
  renewal records.

## 10. State-Machine Recommendation

Use a root-owned, idempotent, retry-safe batch state machine:

```text
Idle
  -> Prepared(batch_id, hash)
  -> Signing(batch_id)
  -> Signed(batch_id, signature)
  -> Installing(batch_id, remaining_issuers)
  -> Installed(batch_id)
  -> FailedRetryable(batch_id, reason)
```

Recommended details:

- `Idle`: no due work or not yet past refresh threshold.
- `Prepared(batch_id, hash)`: canonical batch leaves/header computed and
  persisted before signing.
- `Signing(batch_id)`: management-canister signing requested; duplicate timer
  ticks and lazy repairs must not issue another signature for the same batch.
- `Signed(batch_id, signature)`: signature and public-key metadata persisted
  only after defensive verification succeeds.
- `Installing(batch_id, remaining_issuers)`: root update fanout is in progress
  or partially complete.
- `Installed(batch_id)`: all required issuer installs succeeded or already had
  the same proof.
- `FailedRetryable(batch_id, reason)`: retry after backoff; stale batch is
  abandoned if template fingerprint, registry hash, key version, or time window
  changed.

Handling requirements:

- Unknown signing result: on recovery, check whether `Signed` state exists
  before retrying. If not, retry with same persisted batch hash only while the
  batch remains valid.
- Duplicate callbacks/timer ticks: state transitions must be compare-and-set
  style over `(batch_id, hash, key_version)`.
- Partial issuer install failure: persist `remaining_issuers`; retry only failed
  issuers with the same signed proof until expiry/backoff.
- Stale batches: if template fingerprint or registry hash changes before
  signature/install completes, mark old batch stale and prepare a new batch.
- Registry changes during signing: signed payload must bind `registry_epoch` and
  `registry_hash`; root must not install a signature produced for an old
  registry if policy says the old registry is no longer acceptable.

## 11. Tests And Acceptance Criteria

### Positive No-External-Liveness Test

PocketIC/integration test:

1. deploy root, issuer, and relying verifier in local IC/PocketIC;
2. enable chain-key root proof mode;
3. do not run bridge, CLI, worker, host daemon, external signer, or direct root
   query;
4. advance time past renewal threshold;
5. root timer signs or reuses a signed batch;
6. root installs active proof on issuer by update call;
7. issuer stores active proof;
8. relying verifier accepts issuer auth.

This test should fail today because current renewal test explicitly bridges
scheduled work by querying root and then installing the retrieved batch
(`crates/canic-tests/tests/root_cases/sharding.rs:109`,
`crates/canic-tests/tests/root_cases/sharding.rs:126`,
`crates/canic-tests/tests/root_cases/sharding.rs:363`).

### Lazy Repair Test

1. issuer starts with missing/stale proof;
2. login/update triggers issuer-to-root update repair;
3. root returns cached proof or creates one;
4. issuer stores proof;
5. login succeeds;
6. repeated logins do not call `sign_with_ecdsa` or `sign_with_schnorr`.

### Negative Tests

- expired proof rejected;
- wrong issuer rejected;
- wrong audience rejected;
- wrong grants rejected;
- stale proof epoch rejected;
- wrong registry hash rejected;
- wrong root public key rejected;
- wrong key id rejected;
- wrong derivation path rejected;
- invalid batch witness rejected;
- altered delegation cert rejected;
- legacy bridge/canister-signature proof rejected when bridge-free mode is
  enabled;
- production config rejects test keys if applicable.

### Signing-Volume Tests

- N simultaneous missing-proof login/update calls collapse to one signing
  request.
- M issuers in the same epoch require one batch signature, not M signatures, if
  batch mode is implemented.
- repeated logins under a fresh proof require zero root threshold signatures.

### Existing Tests To Keep Or Update

- Root batch provisioning and issuer-local token issuance in
  `crates/canic-tests/tests/root_cases/sharding.rs:94`.
- Scheduled renewal bridge test should become legacy-mode coverage or be
  replaced by the no-external-liveness test
  (`crates/canic-tests/tests/root_cases/sharding.rs:109`).
- Protocol surface tests for root renewal endpoints and provisioner ACLs in
  `crates/canic/tests/protocol_surface.rs` should be updated for the new mode.

## 12. Risk Register

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Canonical encoding mismatch | Verifiers and issuers disagree on signed bytes. | Add canonical byte golden tests and cross-language fixtures before rollout. |
| Domain separation mistakes | Signature replay across proof families or versions. | New explicit domains for single proof, batch leaf, and batch header. |
| Missing expiry or epoch binding | Stale but structurally valid proof remains usable. | Sign not-before, expires-at, proof epoch, registry epoch/hash, and key version. |
| Accepting wrong key id or derivation path | Wrong IC chain-key authority accepted. | Verifier config must bind algorithm, key id, derivation path/hash, and public key. |
| Replay of stale signed proof | Old proof reintroduced after policy/registry change. | Include template fingerprint and registry hash; persist min accepted proof epoch/key version. |
| Accidental per-login signing | Cost/liveness regression and possible rate pressure. | Root signs only batch/epoch payloads; add signing-volume tests and code guards. |
| Thundering-herd signing | Many stale issuers or logins trigger duplicate signing. | Singleflight state keyed by batch hash and epoch. |
| Signing-cost regression | Restores the problem 0.65 removed. | Prefer batch signatures; record signing metrics and enforce quotas. |
| Partial issuer install failure | Some issuers remain stale despite signed batch. | Persist `remaining_issuers` and retry update fanout until expiry/backoff. |
| Bridge accidentally still required | Product invariant violated. | Acceptance test forbids bridge/CLI/direct root query for liveness. |
| Downgrade to legacy proof path | Legacy canister-signature bridge path bypasses new invariant. | Mode flag with fail-closed legacy rejection. |
| Test key use in production | Production auth trusts test chain-key material. | Config validation equivalent to current IC root key network checks. |
| Verifier library divergence | Rust/Motoko/TypeScript verifier inconsistency. | Golden canonical/signature fixtures and versioned verifier spec. |
| Root key rotation | Valid proofs become unverifiable or stale proofs stay valid. | Include key version and min accepted key version. |
| Management-call uncertainty | Unknown result after signing call traps or times out. | Persist `Signing` state before call; retry only after checking signed state and batch validity. |

## 13. Final Recommendation

Replace the current root proof with a chain-key batch proof.

Given the new product invariant, the current design should not be kept unless
the product accepts the external bridge as a liveness component. The repo
evidence is strong that bridge-free renewal cannot be achieved with the current
canister-signature/certified-data proof primitive:

- 0.74 says root cannot complete the proof flow inside timer/update and needs a
  bridge direct query (`docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md:29`);
- 0.68 says issuer composite-query/root self-call/canister-to-canister
  substitutes fail for root `data_certificate()` (`docs/design/archive/0.68-root-proof-provisioning/0.68-design.md:96`);
- active code implements timer preparation only
  (`crates/canic-core/src/workflow/runtime/auth/renewal.rs:81`);
- active CLI implements the bridge loop
  (`crates/canic-cli/src/auth/mod.rs:711`);
- active tests validate renewal by explicitly bridging scheduled work
  (`crates/canic-tests/tests/root_cases/sharding.rs:363`).

The migration must not claim equivalent security guarantees. It should say:
Canic preserves the authorization semantics of root-bounded issuer/audience/grant
delegation windows, but changes the proof primitive from IC-certified canister
state to IC chain-key threshold signatures over explicit canonical payloads.

Per-batch per-epoch signing is preferred. Per-issuer per-epoch signing is a
fallback if batch witness design is too large for the first migration. Per-login
root signing should be rejected by design and test.
