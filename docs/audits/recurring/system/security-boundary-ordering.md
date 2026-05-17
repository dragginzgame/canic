# Audit: Security Boundary Ordering

## Purpose

Audit security-sensitive ordering and invariant sequencing across auth, replay
protection, endpoint guards, RPC capability handling, and delegated-token
verification.

This is not a crypto audit. It is an enforcement-order and trust-boundary
audit.

## Key Invariant

Verification order matters more than individual checks.

For endpoint delegated tokens, the required order is:

1. decode token boundary material;
2. verify token material;
3. verify root/shard trust chain;
4. enforce caller/subject binding;
5. enforce audience/scope;
6. consume update-token replay marker for update calls;
7. dispatch the endpoint implementation;
8. record bounded success/denial metrics at the owning boundary.

For root RPC capabilities, replay reservation may happen before authorization
only when every authorization or execution failure aborts the reservation.
Replay commit must happen only after authorized execution succeeds.

## Scope

Primary scope:

- `crates/canic-core/src/access/auth/**`
- `crates/canic-core/src/ops/auth/**`
- `crates/canic-core/src/ops/rpc/**`
- `crates/canic-core/src/workflow/rpc/**`
- `crates/canic-macros/src/endpoint/**`

## Checklist

### 1. Endpoint Delegated Token Ordering

```bash
rg -n 'consume|scope|subject|verify|bind|return Err|return Ok' crates/canic-core/src/access/auth -g '*.rs'
```

Expected:

- no replay/update token consumption before token verification;
- no replay/update token consumption before subject binding and scope checks;
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
rg -n 'verify_delegated_token|verify_claims|verify_audience|verify_scopes|root_trust_anchor|verify_shard_key_binding|record_verify' crates/canic-core/src/ops/auth -g '*.rs'
```

Expected:

- config and local shard/root binding checks happen before verifier success;
- root cert signature verifies before claim authorization is accepted;
- audience and scope checks complete before success is returned;
- metrics record bounded outcomes but are not authorization inputs.

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
rg -n 'capability_hash|RootCapabilityEnvelope|NonrootCyclesCapabilityEnvelope|attestation|cache|cached_root_response_attestation' crates/canic-core/src/ops/rpc crates/canic-core/src/workflow/rpc -g '*.rs'
```

Expected:

- capability envelope hash covers the target canister and canonical request;
- root-issued attestation cache is reuse-only and checks root, audience,
  subject, role, epoch, and expiry before reuse;
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
