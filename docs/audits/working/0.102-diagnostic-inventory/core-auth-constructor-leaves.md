# Canic 0.102 Core Authentication Constructor Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger classifies all 39 production `InternalError`
constructor references in the core authentication token, delegation and
chain-key batch owners. It assigns no number and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `ops/auth/token/error.rs` | 11 |
| `ops/auth/token/verification.rs` | 7 |
| `ops/auth/delegation/errors.rs` | 4 |
| `ops/auth/delegation/active.rs` | 2 |
| `ops/auth/token/retention/mod.rs` | 2 |
| `ops/auth/delegation/chain_key_batch/mod.rs` | 7 |
| `ops/auth/delegation/chain_key_batch/merkle.rs` | 2 |
| `ops/auth/delegation/chain_key_batch/install.rs` | 1 |
| `ops/auth/delegation/chain_key_batch/selection.rs` | 1 |
| `ops/auth/delegation/chain_key_batch/signing.rs` | 1 |
| `ops/auth/delegation/chain_key_registry.rs` | 1 |
| **Total** | **39** |

## Token And Delegation Adapters

The first 26 sites add no symbolic identity. They link concrete constructors
to the exact leaves already qualified in
[auth-policy-leaves.md](auth-policy-leaves.md) and
[auth-string-frontier.md](auth-string-frontier.md):

| Disposition | Sites | Exact treatment |
| --- | ---: | --- |
| existing exact authentication meaning | 16 | Reuse certificate/token time, active-proof availability, root authority, verifier-policy and retained-token capacity leaves |
| transparent typed cause or exhaustive typed dispatch | 8 | Preserve storage, canonical, certificate-rule, proof, epoch-floor and verification causes without an adapter code |
| unreachable string fallback | 2 | Delete the `Valid`/`RefreshNeeded` active-proof fallback and the production-unreachable `String` proof-cause implementation |

The 16 exact reuses include `AUTH_CERT_EXPIRED`,
`AUTH_CERT_NOT_YET_VALID`, `AUTH_TOKEN_EXPIRED`,
`AUTH_TOKEN_OUTLIVES_CERT`, `AUTH_PROOF_UNAVAILABLE`,
`AUTH_ACTIVE_DELEGATION_PROOF_MISSING`, `AUTH_ROOT_AUTHORITY_INVALID`,
`AUTH_CHAIN_KEY_POLICY_UNAVAILABLE`,
`AUTH_TOKEN_RETENTION_ACTOR_CAPACITY` and
`AUTH_TOKEN_RETENTION_GLOBAL_CAPACITY`. Repeated sites are evidence for the
same meaning, not new allocation points.

The exhaustive conversion sites must map the source variants directly. In
particular:

- `PrepareDelegationCertError`, `VerifyDelegatedTokenError` and
  `ChainKeyRootProofError` keep the exact typed leaves already recorded in the
  auth string frontier;
- `RootProofInvalid`, `IssuerProofInvalid` and storage failures retain their
  nested registered cause;
- root-policy expiry remains distinct from proof expiry;
- policy versus proof `NotYetValid` is selected by a bounded typed target, not
  the current free-form target string; and
- proof-epoch and Registry-epoch floor checks reuse their corresponding
  chain-key stale-material leaves.

The active-proof fallback is unreachable after the maintained lookup: expired
proofs are removed before status mapping, a missing proof is explicit, and a
not-yet-valid proof is currently mislabeled `Valid`. B4 must expose the actual
time reason and remove the fallback rather than allocate stale/valid/refresh
codes. The default `String` root/issuer proof cause is likewise absent from the
production runtime graph; embedded verification uses `InternalError` and the
cached path cannot emit either proof-cause variant.

## Chain-Key Batch And Registry State

The remaining 13 sites contain twelve direct decisions and one transparent
signer-cause wrapper. The direct decisions reduce to eleven exact candidates:

| Exact candidate | Sites | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `AUTH_CHAIN_KEY_BATCH_EXPIRY_OVERFLOW` | 1 | `Invariant` / batch time arithmetic | self | Correct protected time/TTL policy; no unchanged retry |
| `AUTH_CHAIN_KEY_BATCH_EMPTY` | 2 | `Invariant` / selected batch construction | self | Repair selected-template/batch construction; no unchanged retry |
| `AUTH_CHAIN_KEY_BATCH_TTL_ZERO` | 1 | `Invariant` / protected batch policy | self | Configure positive certificate, verifier and revocation bounds |
| `AUTH_CHAIN_KEY_CERT_CANONICALIZATION_FAILED` | 1 | `Invariant` / prepared certificate evidence | self | Inspect the canonical encoder and protected certificate inputs |
| `AUTH_CHAIN_KEY_APPROVAL_COUNT_MISMATCH` | 1 | `Invariant` / workflow approval binding | self | Rebuild approvals for the exact opaque plan; no unchanged retry |
| `AUTH_CHAIN_KEY_APPROVAL_ISSUER_MISMATCH` | 1 | `Invariant` / issuer authority | self | Preserve plan order and exact issuer identity; no unchanged retry |
| `AUTH_CHAIN_KEY_APPROVAL_EXPIRY_MISMATCH` | 1 | `Invariant` / batch window authority | self | Bind every approval to the plan expiry; no unchanged retry |
| `AUTH_CHAIN_KEY_BATCH_ISSUER_DUPLICATED` | 1 | `Invariant` / canonical issuer set | self | Repair duplicate protected renewal state; no unchanged retry |
| `AUTH_CHAIN_KEY_BATCH_SIGNATURE_MISSING` | 1 | `Invariant` / durable signed-batch state | self | Repair contradictory signed state; never install unchanged |
| `AUTH_CHAIN_KEY_BATCH_CAPACITY_EXCEEDED` | 1 | `ResourceExhausted` / bounded pending batches | self | Wait for expiry/install pruning before bounded retry |
| `AUTH_CHAIN_KEY_REGISTRY_CANONICALIZATION_FAILED` | 1 | `Invariant` / protected delegated-auth Registry | self | Repair canonical Registry policy state before preparing another batch |

The two `AUTH_CHAIN_KEY_BATCH_EMPTY` sites are one invariant: a batch with no
selected issuer has no Merkle leaf. Different helper wording does not justify
a second identity.

The signing site is a transparent typed dispatch. It must preserve the exact
`ChainKeySignerError` meaning and the nested management diagnostic, apply the
terminal/retryable decision recorded in the auth string frontier, and stop
persisting formatted `batch.failure` text. It adds no wrapper identity.

## Dynamic Public Context

This site pass does not close the transitive auth formatter. Static prose and
closed typed variants add no dynamic row, but formatted proof, policy, time,
capacity and dependency values remain subject to the row-by-row ownership
classification in
[dynamic-public-context.md](dynamic-public-context.md). In particular, pending
batch counts and limits, nested auth enum fields, canonicalization causes and
signing causes cannot survive merely because this constructor pass identifies
their semantic leaf.

## Reconciliation

All 39 direct sites now have one disposition:

- 16 reuse an existing exact authentication meaning;
- eight preserve or exhaustively dispatch a typed cause;
- two are production-unreachable sediment;
- twelve direct chain-key decisions add eleven exact meanings; and
- one signer site is a transparent typed dispatch.

The effective constructor frontier moves from 2,159 to 2,198 classified sites
and from 340 to 301 open sites. The qualified semantic set gains eleven exact
candidates and no projection, reaching 2,447 exact candidates plus 31 safe
projections: 2,478 current symbolic identities.

## Required Tests

- exhaustive mappings for every retained token, certificate, active-proof and
  root-proof source variant;
- compile-time or residue guards deleting the two unreachable string
  fallbacks;
- typed policy/proof window-target tests proving their different diagnostics;
- exact batch-empty reuse at both private checks;
- separate approval count, issuer and expiry mismatch tests;
- a signed-without-signature durable-state rejection before proof install;
- pending-batch capacity release after expiry/install pruning; and
- signer tests proving typed terminal causes do not enter retry loops or
  durable prose while management causes preserve their nested disposition.

## Next Slice

Continue with runtime intent and RPC execution constructor owners. In parallel,
classify the transitive authentication formatter's dynamic values before any
numeric allocation.
