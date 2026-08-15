# Canic 0.102 Runtime Auth Renewal And Admission Constructor Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger classifies all fifteen production `InternalError`
constructor references in delegated-proof renewal and prepare admission. It
assigns no number and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/runtime/auth/renewal.rs` | 8 |
| `workflow/runtime/auth/prepare/admission.rs` | 7 |
| **Total** | **15** |

## Delegated-Proof Renewal

The eight sites add six exact meanings and reuse the existing required
chain-key configuration meaning:

| Exact candidate or disposition | Sites | Producer function/branch | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- | --- |
| `AUTH_RENEWAL_WORK_COUNT_OVERFLOW` | 2 | `RootIssuerRenewalWorkflow::sweep_configured` issuer-count conversion and `checked_work_count` addition | `Invariant` / renewal accounting | recent-failure only | Repair the bounded work counter; no unchanged retry |
| `AUTH_RENEWAL_STALLED` | 1 | `RootIssuerRenewalWorkflow::completed_result`; due-without-work branch | `Invariant` / renewal progress | recent-failure only | Inspect the due batch and durable progress before restarting the timer |
| `AUTH_RENEWAL_RETRY_DEADLINE_OVERFLOW` | 1 | `RootIssuerRenewalWorkflow::failed_result`; `retry_deadline_ns` failure | `Invariant` / retry time arithmetic | recent-failure only | Repair the protected time/retry inputs; no unchanged retry |
| `AUTH_RENEWAL_RETRY_DEADLINE_MISSING` | 1 | `RootIssuerRenewalWorkflow::failed_result`; absent persisted `next_deadline_ns` | `Invariant` / durable retry state | recent-failure only | Recover the exact batch journal; never synthesize another deadline |
| `AUTH_RENEWAL_RETRY_DEADLINE_REGRESSED` | 1 | `RootIssuerRenewalWorkflow::failed_result`; `exact_deadline_ns.checked_sub(now_ns)` failure | `Invariant` / durable retry ordering | recent-failure only | Reconcile the exact batch and observed time; never move the deadline backward |
| reuse `AUTH_CHAIN_KEY_CONFIG_REQUIRED` | 1 | `chain_key_min_accepted_proof_epoch`; missing configured epoch | `Invariant` / protected verifier configuration | `RUNTIME_CONFIGURATION_INVALID` | Configure the required minimum accepted proof epoch before renewal |
| `AUTH_DELEGATED_TOKEN_MAX_TTL_OVERFLOW` | 1 | `delegated_token_max_ttl_ns`; seconds-to-nanoseconds multiplication | `InvalidInput` / protected TTL configuration | `RUNTIME_CONFIGURATION_INVALID` | Configure a maximum delegated-token TTL representable in nanoseconds |

The two work-count constructors have the same counter owner, overflow meaning,
timer outcome and repair action, so they share one exact identity. Deadline
overflow, absence and regression remain distinct: they diagnose arithmetic,
lost durable state and ordering contradiction at different recovery
boundaries.

`AUTH_CHAIN_KEY_CONFIG_REQUIRED` already covers a missing required signer or
verifier field. The renewal-specific wording does not create another identity.
Configuration reads, renewal timing, batch preparation/signing/install and
retry deferral preserve their typed causes and receive no workflow wrapper
code.

The bounded recent-failure ring currently derives a string label from broad
class and origin. B4 must retain the exact registered identity instead; the
formatted class/origin label and error prose are not durable diagnostic
authority.

## Role-Attestation And Token Admission

The seven admission sites add four exact meanings, reuse two existing
attestation meanings and retain one typed policy edge:

| Exact candidate or disposition | Sites | Producer function/branch | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- | --- |
| `AUTH_ROLE_ATTESTATION_TTL_INVALID` | 1 | `validate_role_attestation_request`; zero-or-above-maximum branch | `InvalidInput` / attestation request | self | Request a positive TTL no greater than the configured maximum |
| reuse `AUTH_ATTESTATION_SUBJECT_MISMATCH` | 1 | `validate_active_component_subject`; subject/caller inequality | `Forbidden` / attestation subject | self | Bind the requested subject to the exact transport caller |
| `AUTH_ATTESTATION_MEMBER_CALLER_MISMATCH` | 1 | `validate_active_component_subject`; Registry member/caller inequality | `Forbidden` / active Registry membership | self | Invoke as the exact active Component or Component Child member |
| `AUTH_ATTESTATION_ROLE_MISMATCH` | 1 | `validate_active_component_subject`; registered/requested role inequality | `Forbidden` / registered role authority | self | Request only the role in the caller's protected active binding |
| reuse `AUTH_ATTESTATION_SUBNET_MISMATCH` | 1 | `validate_active_component_subject`; registered/requested Subnet inequality | `Forbidden` / protected placement | self | Bind the request to the caller's exact registered Subnet |
| `AUTH_ROLE_ATTESTATION_TTL_CONFIGURATION_OVERFLOW` | 1 | `role_attestation_max_ttl_ns`; seconds-to-nanoseconds multiplication | `InvalidInput` / protected TTL configuration | `RUNTIME_CONFIGURATION_INVALID` | Configure a role-attestation TTL representable in nanoseconds |
| transparent typed `AuthPolicyError` dispatch | 1 | `map_token_prepare_policy_error`; exact variant dispatch required in B4 | typed issuance policy | source mapping | Preserve `AUTH_SUBJECT_CALLER_MISMATCH` or `AUTH_PUBLIC_SCOPE_NOT_SELF_GRANTABLE` |

The Registry-member caller, role and Subnet predicates remain independently
testable. A correct caller principal cannot substitute for the registered role
or placement, and a request-presented role or Subnet never becomes authority.
Both top-level Components and Component Children pass through the same exact
protected `ManagedCanisterBinding` projection.

The token policy adapter currently flattens `AuthPolicyError` with
`to_string()`. It receives no wrapper identity. B4 must dispatch the two
producer-reachable typed variants directly and discard their formatted role
and scope values.

## Dynamic Public Context

The fourteen formatted values in admission are classified as `DPC-199`
through `DPC-212` in
[dynamic-public-context.md](dynamic-public-context.md). Request fields come
from the caller, while the exact active member binding and TTL ceiling are
present in the caller's protected Canic configuration/binding. They are
caller-derivable and are discarded from the compact diagnostic. Renewal
constructors are static; detailed cause text remains in guarded structured
logs or the operation-specific state that owns it.

## Reconciliation

All fifteen direct sites now have one disposition. They add ten exact meanings,
reuse three existing identities, retain one transparent typed edge and add no
projection. The effective constructor frontier moves from 2,229 to 2,244
classified sites and from 270 to 255 open sites. The qualified semantic set
reaches 2,480 exact candidates plus 31 safe projections: 2,511 current symbolic
identities.

## Required Tests

- both renewal work-count overflow paths map to one exact identity;
- stalled, overflowed, missing and regressed retry state remain distinct;
- typed batch/configuration errors pass through without workflow identities;
- TTL zero, ceiling and nanosecond-overflow boundaries reject exactly;
- subject, Registry member, role and Subnet authority reject independently;
- Component and Component Child active bindings receive identical admission;
- typed token policy variants survive without formatted wrapper dispatch; and
- timer recent-failure evidence retains the exact registered identity.

## Next Slice

Continue with prepare replay and response reconstruction, separating request
metadata, retained-receipt state and terminal response recovery.
