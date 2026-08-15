# Canic 0.102 Runtime Auth Root Issuer And Batch Constructor Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger classifies all six production `InternalError`
constructor references in the root-issuer policy facade and root delegation-
batch policy sweep. It assigns no number and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/runtime/auth/root_issuer/mod.rs` | 5 |
| `workflow/runtime/auth/root_delegation_batch/mod.rs` | 1 |
| **Total** | **6** |

## Root Issuer Policy Facade

The five root-issuer sites add no exact meaning:

| Exact candidate or disposition | Sites | Producer function/branch | Required hard cut |
| --- | ---: | --- | --- |
| transparent typed Fleet-activation/storage cause | 1 | `protected_fleet`; protected Fleet lookup | Preserve the exact storage/activation diagnostic without a facade wrapper |
| transparent typed `AuthPolicyError` dispatch | 1 | `map_policy_upsert_error`; issuer-policy upsert | Exhaustively map each producer-reachable policy variant to its qualified identity |
| reuse `AUTH_ROOT_ISSUER_CERT_TTL_ZERO` | 1 | `map_renewal_template_upsert_error`; renewal-template TTL admission | Return the existing exact request-policy diagnostic |
| transparent typed `AuthPolicyError` dispatch | 2 | `map_renewal_template_upsert_error`; renewal-template Fleet/grant/policy admission | Preserve each exact policy identity, class and approved projection |

The current mapper changes typed policy meaning based on a catch-all branch:
some variants become `InvalidInput`, and all remaining variants become
`Forbidden`. B4 must replace that string dispatch with an exhaustive typed
mapping. Fleet mismatch, renewal-grant absence, issuer absence/disablement,
grant denial, TTL bounds and protected policy arithmetic already have distinct
qualified identities and retry actions.

The protected Fleet conversion is likewise transparent. The facade must not
create a second missing/conflicting Fleet authority merely because the source
is reached during issuer-policy mutation.

## Root Delegation-Batch Sweep

The batch sweep's one constructor also adds no meaning:

| Exact candidate or disposition | Sites | Producer function/branch | Required hard cut |
| --- | ---: | --- | --- |
| transparent typed `AuthPolicyError` dispatch | 1 | `prepare_due_chain_key_root_delegation_batch`; per-issuer batch approval | Preserve the exact policy diagnostic and protected issuer/batch correlation |

One rejected issuer policy must not be flattened to generic `Forbidden` or
silently authorize the rest of the batch. The typed policy result is computed
before `commit_chain_key_root_delegation_batch`; failure therefore remains a
pre-commit admission result and cannot be retried as though an external effect
may already have occurred.

## Dynamic Public Context

Four formatted `AuthPolicyError` values are classified as `DPC-250` through
`DPC-253` in
[dynamic-public-context.md](dynamic-public-context.md). Each is an
authoritatively typed cause; dynamic principal, role, scope, TTL and policy
fields follow the source identity's approved projection and never become
compact diagnostic text.

## Reconciliation

All six direct sites now have one disposition. They add no exact meaning, reuse
one existing identity and retain five transparent typed edges. The effective
constructor frontier moves from 2,293 to 2,299 classified sites and from 206 to
200 open sites. The qualified semantic set remains 2,504 exact candidates plus
31 safe projections: 2,535 current symbolic identities.

## Required Tests

- exhaustive producer-reachable `AuthPolicyError` mappings for policy upsert,
  renewal-template upsert and batch approval;
- no broad catch-all InvalidInput/Forbidden mapping survives;
- certificate-TTL zero reuses its exact existing identity;
- protected Fleet lookup preserves its typed activation/storage cause;
- policy dynamic fields obey their already-approved public projections; and
- batch policy rejection occurs before commit and cannot produce a partial
  approval set.

## Next Slice

Continue with root and non-root runtime lifecycle orchestration, separating
configuration, activation and typed application-hook boundaries.
