# Audit: Security Boundary Ordering

## Purpose

Audit security-sensitive ordering and invariant sequencing across auth, replay
protection, endpoint guards, RPC capability handling, and delegated-token
verification.

This audit tracks the current hard-cut boundary split:

- public delegated-token endpoint auth uses `auth::authenticated(...)`,
  stable `DelegationAudience::{Canister, CanicSubnet, Project}`, signed
  local-role grant scope checks, subject/caller binding, and bearer-token
  verification without verifier-local token-use writes;
- root proof renewal uses controller/root-only issuer policy/template updates,
  root timer or internal registered-issuer updates for chain-key batch
  prepare/sign/install, and issuer-local active proof installation only after
  root proof verification;
- signed role-attestation verification is explicit and local before any
  caller trusts an embedded attestation;
- endpoint multi-role policy is not delegated-token multi-role audience.

This is not a crypto audit. It is an enforcement-order and trust-boundary
audit.

## Key Invariant

Verification order matters more than individual checks.

For endpoint delegated tokens, the required order is:

1. decode token boundary material;
2. verify token material;
3. verify the configured root canister/root key and issuer canister-signature
   proof chain, or rerun local identity checks after a positive proof-cache
   hit;
4. enforce caller/subject binding;
5. enforce stable audience and local-role grant scope;
6. do not write verifier-local token-use state; replay-sensitive endpoint
   commands must use domain operation receipts;
7. dispatch the endpoint implementation;
8. record bounded success/denial metrics at the owning boundary.

For root proof renewal, the required order is:

1. root policy/template updates are root/controller update paths before state
   mutation;
2. batch prepare validates quotas, issuer registry policy, certificate TTL,
   audiences, and grants before persisting canonical chain-key batch metadata;
3. signing checks signer policy and persists one threshold signature over the
   canonical batch header;
4. install planning materializes issuer-specific proof/witness payloads from the
   signed batch without re-signing;
5. issuer install verifies the root proof against configured root trust anchors
   and local issuer binding before storing active proof state;
6. normal delegated-token prepare/get stays issuer-local after active proof
   installation.

For root RPC capabilities, replay reservation may happen before authorization
only when every authorization or execution failure aborts the reservation.
Replay commit must happen only after authorized execution succeeds.

## Scope

Primary scope:

- `crates/canic-core/src/access/auth/**`
- `crates/canic-core/src/ops/auth/**`
- `crates/canic-core/src/ops/rpc/**`
- `crates/canic-core/src/workflow/rpc/**`
- `crates/canic-core/src/workflow/runtime/auth/**`
- `crates/canic-core/src/api/auth/**`
- `crates/canic-macros/src/endpoint/**`
- `crates/canic/src/macros/endpoints/**`

## Checklist

### 1. Endpoint Delegated Token Ordering

```bash
rg -n 'consume|scope|subject|verify|bind|return Err|return Ok' crates/canic-core/src/access/auth -g '*.rs'
```

Expected:

- no domain replay receipt lookup or reservation before token verification;
- no domain replay receipt lookup or reservation before subject binding and
  scope checks;
- delegated-token audience handling is limited to `Canister`, `CanicSubnet`,
  and `Project`;
- no verifier-local token-use store or consume path;
- handler dispatch occurs only after access evaluation succeeds.

### 2. Endpoint Macro Sequencing

```bash
rg -n 'eval_access|eval_default_app_guard|dispatch_|return Err' crates/canic-macros/src/endpoint -g '*.rs'
```

Expected:

- generated wrappers build call context;
- evaluate default/app or explicit access;
- return on denial;
- dispatch only after successful access evaluation.

### 3. Delegated Token Material Verification

```bash
rg -n 'verify_delegated_token|verify_claims|verify_audience|verify_scopes|auth_proof_verifier_config|verify_chain_key_batch_root_proof|verify_issuer_canister_signature_proof|positive_cache|record_verify' crates/canic-core/src/ops/auth -g '*.rs'
```

Expected:

- config and current-canister verifier checks happen before verifier success;
- explicit root canister/root-key config is resolved before embedded proof
  verification;
- root certificate proof and issuer proof verify before token acceptance on
  cache misses;
- positive proof-cache hits rerun local canonical, audience, grant, subject,
  and scope identity checks before success;
- audience, grant, and required-scope checks complete before success is
  returned;
- `DelegationAudience::{Canister, CanicSubnet, Project}` is the only accepted
  delegated-token audience shape;
- no role/principal or plural role audience DTO is accepted;
- metrics record bounded outcomes but are not authorization inputs.

### 3a. Root Proof Renewal Ordering

```bash
rg -n 'upsert_root_issuer_policy|upsert_root_issuer_renewal_template|prepare_due_chain_key_root_delegation_batch|sign_next_chain_key_root_delegation_batch|get_or_create_chain_key_delegation_proof|start_next_chain_key_root_delegation_batch_install|install_active_delegation_proof|ChainKeyRootDelegationBatch|RootDelegationProofBatch' crates/canic-core/src/api/auth crates/canic-core/src/ops/auth/delegation crates/canic-core/src/workflow/runtime/auth crates/canic/src -g '*.rs'
```

Expected:

- root policy/template updates require root/controller authority before state
  mutation;
- timer renewal and internal registered-issuer lazy repair validate chain-key
  mode, signer policy, quotas, issuer policy, TTL, audience, and grants before
  root proof preparation;
- root signing covers canonical batch header material only, not login/session
  payloads;
- install planning validates persisted signed batch state and materializes
  issuer proof/witness payloads before issuer install calls;
- issuer install verifies the root proof and local issuer binding before
  storing active proof state;
- bridge/direct-query root proof retrieval is not treated as a supported proof
  assembly path.

### 3b. Delegated Token Issuer Prepare/Get Ordering

```bash
rg -n 'prepare_delegated_token|prepare_delegated_token_issuer_proof|get_delegated_token|get_delegated_token_issuer_proof|active_delegation_proof|prepare_issuer_canister_signature|get_issuer_canister_signature_proof|prepared_by' crates/canic-core/src/api/auth crates/canic-core/src/ops/auth crates/canic-core/src/workflow/runtime/auth -g '*.rs'
```

Expected:

- issuer endpoint checks delegated-token issuer config before prepare/get;
- prepare requires active delegation proof, verifies `issuer_pid == self`, and
  only then prepares token claims and issuer canister-signature material;
- get retrieves pending token material by `claims_hash` plus caller/prepared-by
  binding before attaching issuer proof;
- normal delegated-token issuance does not call root after active proof
  installation.

### 4. RPC Replay Sequencing

```bash
rg -n 'check_replay|reserve|authorize|execute|commit_replay|abort_replay|Cached|Duplicate' crates/canic-core/src/workflow/rpc crates/canic-core/src/ops/replay -g '*.rs'
```

Expected:

- replay commit only after successful authorized execution;
- replay reservation is aborted on authorization or execution failure;
- cached replay decode failure does not partially accept a response.

### 5. Capability Envelope And Attestation Cache

```bash
rg -n 'capability_hash|RootCapabilityEnvelope|NonrootCyclesCapabilityEnvelope|attestation|cache|cached_root_response_attestation|CapabilityProof::' crates/canic-core/src/ops/rpc crates/canic-core/src/workflow/rpc crates/canic-core/src/dto/capability -g '*.rs'
```

Expected:

- capability envelope hash covers the target canister and canonical request;
- retained root-issued attestation caches, if any, are reuse-only and check
  root, audience, subject, role, epoch, and expiry before reuse;
- cached attestations do not skip target capability hash construction.

## Output Requirements

Reports must include:

- executive summary;
- verification ordering map;
- trust-boundary table;
- replay sequencing analysis;
- endpoint macro sequencing analysis;
- RPC capability handling review;
- residual watchpoints;
- recommended guard additions.

## Final Verdict

Choose one:

- Pass - ordering invariants hold;
- Pass with watchpoints - ordering holds, but hotspots remain;
- Fail - a boundary can authorize or mutate out of order.
