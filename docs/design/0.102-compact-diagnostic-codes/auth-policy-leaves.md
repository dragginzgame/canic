# Canic 0.102 Authentication And Policy Diagnostic Leaves

Date: 2026-08-12

## Status

This provisional B1 ledger covers pure policy and the already-typed
authentication operations reachable from the terminal conversion graph at
`v0.101.53`. It allocates no numbers. It deliberately leaves
`AuthValidationError::Auth(String)` unallocated until its hidden producer graph
is separately typed and inventoried.

`self` in the public-projection column means the exact candidate leaf is safe
to return. `recent failure` means the exact internal numeric leaf is written to
the existing bounded runtime diagnostic owner while the safe projection is
returned. Dynamic principals, scopes, epochs, times, field names and proof
details never enter the compact error.

## Pure Auth Policy

All 17 `AuthPolicyError` variants have current producers.

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry | Observability |
| --- | --- | --- | --- | --- | --- |
| `AUTH_PUBLIC_SCOPE_NOT_SELF_GRANTABLE` | `PublicPrepareScopeNotSelfGrantable` | `Forbidden` / auth issuance policy | self | Request only public session/verify scope; corrected request only | public |
| `AUTH_ROOT_ISSUER_AUDIENCE_NOT_ALLOWED` | `RootIssuerAudienceNotAllowed` | `Forbidden` / issuer policy | self | Select an allowed audience or update protected policy; no unchanged retry | public |
| `AUTH_ROOT_ISSUER_FLEET_MISMATCH` | `RootIssuerFleetMismatch` | `Forbidden` / issuer Fleet binding | `AUTH_ROOT_ISSUER_BINDING_INVALID` | Use issuer evidence bound to the exact Fleet; no unchanged retry | recent failure |
| `AUTH_ROOT_ISSUER_AUDIENCE_REQUIRED` | `RootIssuerAudienceRequired` | `Invariant` / issuer policy | `AUTH_ROOT_ISSUER_POLICY_INVALID` | Correct protected issuer policy; no unchanged retry | recent failure |
| `AUTH_ROOT_ISSUER_CERT_TTL_ZERO` | `RootIssuerCertTtlZero` | `InvalidInput` / issuer request | self | Supply a positive certificate TTL | public |
| `AUTH_ROOT_ISSUER_CERT_TTL_EXCEEDS_MAX` | `RootIssuerCertTtlExceedsMax` | `InvalidInput` / issuer request | self | Reduce TTL to the policy ceiling | public |
| `AUTH_ROOT_ISSUER_DISABLED` | `RootIssuerDisabled` | `Unavailable` / issuer policy | self | Enable or select an active issuer; retry only after policy changes | public |
| `AUTH_ROOT_ISSUER_GRANT_NOT_ALLOWED` | `RootIssuerGrantNotAllowed` | `Forbidden` / issuer grant policy | self | Request only admitted grants; corrected request only | public |
| `AUTH_ROOT_ISSUER_GRANT_REQUIRED` | `RootIssuerGrantRequired` | `Invariant` / issuer policy | `AUTH_ROOT_ISSUER_POLICY_INVALID` | Configure at least one admitted grant; no unchanged retry | recent failure |
| `AUTH_ROOT_ISSUER_MAX_CERT_TTL_ZERO` | `RootIssuerMaxCertTtlZero` | `Invariant` / issuer policy | `AUTH_ROOT_ISSUER_POLICY_INVALID` | Configure a positive maximum TTL; no unchanged retry | recent failure |
| `AUTH_ROOT_ISSUER_POLICY_MISMATCH` | `RootIssuerPolicyMismatch` | `Conflict` / issuer identity | `AUTH_ROOT_ISSUER_BINDING_INVALID` | Reload exact issuer authority; no blind retry | recent failure |
| `AUTH_ROOT_ISSUER_REFRESH_AFTER_INVALID` | `RootIssuerRefreshAfterInvalid` | `Invariant` / issuer policy | `AUTH_ROOT_ISSUER_POLICY_INVALID` | Correct refresh window; no unchanged retry | recent failure |
| `AUTH_ROOT_ISSUER_REFRESH_AFTER_OVERFLOW` | `RootIssuerRefreshAfterOverflow` | `Invariant` / issuer policy arithmetic | `AUTH_ROOT_ISSUER_POLICY_INVALID` | Correct TTL/refresh values; no unchanged retry | recent failure |
| `AUTH_ROOT_ISSUER_REFRESH_RATIO_INVALID` | `RootIssuerRefreshRatioInvalid` | `Invariant` / issuer policy | `AUTH_ROOT_ISSUER_POLICY_INVALID` | Configure ratio in the accepted interval; no unchanged retry | recent failure |
| `AUTH_ROOT_ISSUER_UNREGISTERED` | `RootIssuerUnregistered` | `Unavailable` / issuer Registry | self | Register issuer, then retry | public |
| `AUTH_ROOT_ISSUER_RENEWAL_GRANT_REQUIRED` | `RootIssuerRenewalGrantRequired` | `InvalidInput` / renewal template | self | Supply at least one renewal grant | public |
| `AUTH_SUBJECT_CALLER_MISMATCH` | `SubjectCallerMismatch` | `Forbidden` / public issuance | self | Subject must equal transport caller; no unchanged retry | public |

The two safe projections introduced here are:

- `AUTH_ROOT_ISSUER_BINDING_INVALID`: protected issuer/Fleet identity is
  inconsistent; and
- `AUTH_ROOT_ISSUER_POLICY_INVALID`: protected issuer policy cannot safely
  issue.

They reveal no issuer principal, Fleet identity, grant or timing value.

## Authority And Activation Fences

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `AUTHORITY_RESTORE_UPDATE_FENCED` | `AuthorityRestoreEndpointPolicyError::Fenced` | `Conflict` / restore fence | self | Resume the exact sealed operation; retry ordinary update only after unseal |
| `FLEET_ACTIVATION_ENDPOINT_FENCED` | `FleetActivationEndpointPolicyError::Fenced` | `Unavailable` / prepared-runtime fence | self | Complete activation using admitted recovery endpoints; retry after Active |

The two fences do not share a leaf. One protects snapshot restore authority and
the other protects pre-activation runtime authority; their permitted recovery
journeys differ.

## Environment And Placement Policy

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `ENV_REQUIRED_FIELDS_MISSING` | `EnvPolicyError::MissingEnvFields(String)` and required-field `EnvOpsError` variants | `Invariant` / runtime environment | `RUNTIME_ENVIRONMENT_INVALID` | Reinstall with a complete protected binding; no unchanged retry |
| `SCALING_DISABLED` | `ScalingPolicyError::ScalingDisabled` | `Unavailable` / scaling policy | self | Configure scaling before requesting a worker; retry after configuration changes |
| `SCALING_POOL_NOT_FOUND` | `ScalingPolicyError::PoolNotFound` | `InvalidInput` / scaling policy | self | Select a declared pool; corrected request only |
| `SHARDING_DISABLED` | `ShardingPolicyError::ShardingDisabled` | `Unavailable` / sharding policy | self | Configure sharding before requesting assignment; retry after configuration changes |
| `SHARDING_POOL_NOT_FOUND` | `ShardingPolicyError::PoolNotFound` | `InvalidInput` / sharding policy | self | Select a declared pool; corrected request only |
| `SHARDING_POOL_AT_CAPACITY` | `ShardCreationBlocked(CreateBlockedReason::PoolAtCapacity)` | `ResourceExhausted` / sharding capacity | self | Free capacity or use another admitted shard; retry after state changes |
| `SHARDING_NO_FREE_SLOTS` | `ShardCreationBlocked(CreateBlockedReason::NoFreeSlots)` | `ResourceExhausted` / sharding slots | self | Increase configured slots/rebalance; no blind retry |

`ENV_REQUIRED_FIELDS_MISSING` intentionally groups the exact missing-field set
across initial policy validation, import/restore validation and required
environment accessors: every current case has the same owner, exposure,
reinstall action and retry policy. The free-form field list must be removed
from the runtime error. The actual installed environment remains available
through its typed status and binding surfaces rather than a generic error
detail. Other environment/access decisions are mapped in
[runtime-ops-leaves.md](runtime-ops-leaves.md).

`CreateBlockedReason::PolicyViolation(String)` has no current producer; only a
metrics match arm refers to it. It is sediment and must be deleted rather than
assigned a code. This leaves two live `ShardCreationBlocked` reasons.

## Already-Typed Authentication Operations

`AuthOpsError` is a transparent wrapper and receives no code. The current
`AuthValidationError::Auth(String)` variant is excluded from this table and
handled in the next section. The other 22 operation variants are provisionally
mapped below.

### Validation And Signature

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry | Observability |
| --- | --- | --- | --- | --- | --- |
| `AUTH_ROOT_AUTHORITY_INVALID` | `AuthValidationError::InvalidRootAuthority` | `Forbidden` / delegation authority | `AUTH_PROOF_INVALID` | Reacquire proof from the exact root; no unchanged retry | recent failure |
| `AUTH_CANONICAL_ENCODING_FAILED` | `AuthValidationError::EncodeFailed` | `Invariant` / auth encoding | `AUTH_PROOF_INVALID` | Inspect typed encoder/context; do not retry unchanged | recent failure |
| `AUTH_ATTESTATION_SUBNET_UNAVAILABLE` | `AuthValidationError::AttestationSubnetUnavailable` | `Unavailable` / local-Subnet attestation | self | Obtain live receiver-Subnet evidence, then retry | public |
| `AUTH_ATTESTATION_SUBNET_REQUIRED` | `AuthValidationError::AttestationSubnetRequired` | `InvalidInput` / local-Subnet attestation | self | Supply the required subnet claim | public |
| `AUTH_ATTESTATION_FIELD_TOO_LARGE` | `AuthValidationError::AttestationProofFieldTooLarge` | `InvalidInput` / proof quota | self | Submit bounded key/signature evidence | public |
| `AUTH_ATTESTATION_WINDOW_INVALID` | `AuthValidationError::AttestationInvalidWindow` | `InvalidInput` / attestation time | self | Use an expiry later than issue time | public |
| `AUTH_DELEGATED_TOKENS_DISABLED` | `AuthValidationError::DelegatedTokenAuthDisabled` | `Unavailable` / auth configuration | self | Enable delegated tokens or use another auth mode | public |
| `AUTH_PROOF_UNAVAILABLE` | `AuthSignatureError::ProofUnavailable` | `Unavailable` / delegation proof | self | Wait for proof preparation/renewal; bounded retry | public |
| `AUTH_DELEGATION_PROOF_INVALID` | `AuthSignatureError::ProofInvalid(String)` | `InvalidInput` / delegation proof | `AUTH_PROOF_INVALID` | Reacquire a valid proof; no unchanged retry | recent failure |
| `AUTH_ROOT_DATA_CERTIFICATE_UNAVAILABLE` | `AuthSignatureError::RootDataCertificateUnavailable` | `Unavailable` / certified data | self | Retry in certified query context or after certificate availability | public |
| `AUTH_ATTESTATION_PROOF_INVALID` | `AuthSignatureError::AttestationProofInvalid(String)` | `InvalidInput` / local-Subnet proof | `AUTH_PROOF_INVALID` | Reacquire valid local proof; no unchanged retry | recent failure |

`AUTH_PROOF_INVALID` is the safe public projection for exact cryptographic,
encoding and root-authority failures. Proof strings and cryptographic library
errors are never public identities.

### Scope And Time

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `AUTH_ISSUER_PRINCIPAL_MISMATCH` | `AuthScopeError::IssuerPidMismatch` | `Forbidden` / token scope | self | Use token issued by exact protected issuer; no unchanged retry |
| `AUTH_ATTESTATION_SUBJECT_MISMATCH` | `AuthScopeError::AttestationSubjectMismatch` | `Forbidden` / attestation subject | self | Bind proof subject to transport caller |
| `AUTH_ATTESTATION_AUDIENCE_MISMATCH` | `AuthScopeError::AttestationAudienceMismatch` | `Forbidden` / attestation audience | self | Bind proof audience to receiver Canister |
| `AUTH_ATTESTATION_SUBNET_MISMATCH` | `AuthScopeError::AttestationSubnetMismatch` | `Forbidden` / attestation Subnet | self | Bind proof to the live receiver Subnet |
| `AUTH_CERT_EXPIRED` | `AuthExpiryError::CertExpired` | `Unauthorized` / delegation certificate | self | Renew certificate; retry with new proof |
| `AUTH_TOKEN_EXPIRED` | `AuthExpiryError::TokenExpired` | `Unauthorized` / delegated token | self | Renew token; retry with new token |
| `AUTH_TOKEN_NOT_YET_VALID` | `AuthExpiryError::TokenNotYetValid` | `Unauthorized` / delegated token | self | Wait until issue time or correct clock/evidence |
| `AUTH_TOKEN_TTL_EXCEEDED` | `AuthExpiryError::TokenTtlExceeded` | `Forbidden` / delegated token policy | self | Request token within maximum TTL |
| `AUTH_ATTESTATION_EXPIRED` | `AuthExpiryError::AttestationExpired` | `Unauthorized` / local-Subnet attestation | self | Renew attestation |
| `AUTH_ATTESTATION_NOT_YET_VALID` | `AuthExpiryError::AttestationNotYetValid` | `Unauthorized` / local-Subnet attestation | self | Wait or correct issue-time evidence |
| `AUTH_ATTESTATION_EPOCH_REJECTED` | `AuthExpiryError::AttestationEpochRejected` | `Unauthorized` / role epoch | self | Renew proof at an accepted role epoch |

The expiry leaves stay distinct because renewal subject, retry timing and
machine decisions differ. They cannot be collapsed back into the current broad
`AuthProofExpired`/`AuthTokenExpired` classes.

## Untyped Authentication Frontier

`AuthValidationError::Auth(String)` is not one leaf, and it is not the complete
string frontier. Current producers span:

- attestation request/binding and Candid validation;
- delegated-token parsing and verification;
- issuer/root proof preparation, lookup and pruning;
- chain-key request construction and signing evidence;
- verifier configuration and IC-root-key validation; and
- conversion from already-typed token/proof error enums through `to_string()`.

`ProofInvalid(String)` and `AttestationProofInvalid(String)` may retain one safe
public projection because their public action is deliberately indistinguishable,
but their exact internal leaves above remain separate. `Auth(String)` is broader:
it mixes invalid input, unavailable prepared state, configuration invariants and
cryptographic failures. Adjacent paths also flatten typed errors directly into
`InternalError` or durable chain-key failure text without passing through
`Auth(String)`.

[auth-string-frontier.md](auth-string-frontier.md) records and reconciles that
expanded graph: ten additional typed owners, 97 declared variants, 96 non-test
structural variants and 43 direct `Auth(...)` construction sites. It reduces
them to 84 new exact candidates and two new safe projections after wrapper
removal, same-semantics reuse and current-path sediment. B4/B5 must replace the
prose constructors with those finite reasons, preserve typed cause carriers and
separate retryable management failures from terminal protected-policy failures.
A generic `AUTH_VALIDATION_FAILED` code is forbidden.

## Current Count

This family provisionally records:

- 17 exact pure auth-policy leaves;
- two exact authority/activation fences;
- seven environment/scaling/sharding leaves;
- 22 already-typed auth-operation leaves; and
- four new safe public projection leaves:
  `AUTH_ROOT_ISSUER_BINDING_INVALID`, `AUTH_ROOT_ISSUER_POLICY_INVALID`,
  `RUNTIME_ENVIRONMENT_INVALID` and `AUTH_PROOF_INVALID`.

This base table contains **48 exact candidate leaves plus four safe
projections**. The unproduced sharding policy reason is excluded. Including the
reconciled string frontier, the complete authentication/policy family now
contains **132 provisional exact candidates plus six distinct safe
projections**.

## Required Tests

- exhaustive mappings for all 17 `AuthPolicyError` variants and the 22 typed
  auth-operation variants;
- exact fence tests preserving restore-versus-activation recovery behavior;
- proof that environment missing-field text is absent while typed environment
  status remains sufficient;
- deletion guard for unproduced `CreateBlockedReason::PolicyViolation`;
- public projection tests proving cryptographic, issuer-binding and policy
  details remain masked;
- numeric recent-failure evidence for every masked exact leaf; and
- residue guards rejecting `AuthValidationError::Auth(String)` and all
  text-derived classification after its nested frontier is typed.
