# Canic 0.102 Runtime Auth Prepare Replay Constructor Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger classifies all thirty-two production
`InternalError` constructor references in authentication prepare replay. It
assigns no number and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/runtime/auth/prepare/replay.rs` | 32 |
| **Total** | **32** |

## Request Metadata And Expiry

The seven metadata sites add six exact meanings:

| Exact candidate | Sites | Producer function/branch | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- | --- |
| `AUTH_PREPARE_REPLAY_EXPIRY_OVERFLOW` | 1 | `replay_reserve_input`; `now_ns.checked_add(ttl_ns)` failure | `InvalidInput` / replay expiry arithmetic | self | Correct request time or replay TTL before reserving |
| `AUTH_PREPARE_OPERATION_ID_REQUIRED` | 2 | `role_attestation_replay_metadata` and `token_replay_metadata`; absent metadata | `InvalidInput` / replay identity | self | Supply one exact nonzero request identity |
| `AUTH_ROLE_ATTESTATION_REPLAY_TTL_ZERO` | 1 | `role_attestation_replay_metadata`; zero TTL branch | `InvalidInput` / role-attestation replay metadata | self | Supply a positive replay TTL |
| `AUTH_ROLE_ATTESTATION_REPLAY_TTL_EXCEEDED` | 1 | `role_attestation_replay_metadata`; `MAX_ROLE_ATTESTATION_REPLAY_TTL_NS` ceiling branch | `InvalidInput` / role-attestation replay metadata | self | Reduce replay TTL to the maintained ceiling |
| `AUTH_TOKEN_PREPARE_REPLAY_TTL_ZERO` | 1 | `token_replay_metadata`; zero TTL branch | `InvalidInput` / delegated-token replay metadata | self | Supply a positive replay TTL |
| `AUTH_TOKEN_PREPARE_REPLAY_TTL_EXCEEDED` | 1 | `token_replay_metadata`; `MAX_TOKEN_REPLAY_TTL_NS` ceiling branch | `InvalidInput` / delegated-token replay metadata | self | Reduce replay TTL to the maintained ceiling |

Role-attestation and delegated-token TTL identities remain distinct because
their command kinds, request DTOs and future policy ceilings are independently
owned. Operation identity is the same replay requirement for both commands and
therefore shares one exact meaning.

The expiry helper's caller-supplied prose is not authority. B4 must replace the
message parameter with the exact registered identity while retaining checked
`u64` time arithmetic.

## Replay Decisions

The sixteen decision sites reduce to eight exact meanings shared by both
authentication prepare commands:

| Exact candidate | Sites | Producer function/branch | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- | --- |
| `REPLAY_UNEXPECTED_FRESH_DECISION` | 2 | `map_token_prepare_replay_decision` and `map_role_attestation_replay_decision`; escaped `ReplayReceiptDecision::Fresh` | `Invariant` / workflow dispatch | self | Repair the reserve/dispatch boundary; no unchanged retry |
| `REPLAY_OPERATION_IN_PROGRESS` | 2 | both replay-decision mappers; `ReplayReceiptDecision::OperationInProgress` | `Conflict` / exact replay operation | self | Retry later with the same request ID and payload |
| `REPLAY_ACTOR_MISMATCH` | 2 | both replay-decision mappers; `ReplayReceiptDecision::ActorMismatch` | `Conflict` / replay actor identity | self | Never reuse another actor's request ID |
| `REPLAY_PAYLOAD_MISMATCH` | 2 | both replay-decision mappers; `ReplayReceiptDecision::PayloadMismatch` | `Conflict` / replay payload identity | self | Replay only the exact original payload or use a new request ID |
| `REPLAY_RECEIPT_EXPIRED` | 2 | both replay-decision mappers; `ReplayReceiptDecision::Expired` | `Conflict` / replay retention | self | Start a new operation with a new request ID |
| `AUTH_PREPARE_REPLAY_RECOVERY_REASON_INVALID` | 2 | both replay-decision mappers; non-`RecoveryReason::ResponseCommitFailed` recovery branch | `Conflict` / auth response recovery | self | Inspect the exact retained receipt; auth preparation may auto-recover only response-commit failure |
| `REPLAY_PENDING_ACTOR_CAPACITY` | 2 | both replay-decision mappers; `ReplayReceiptDecision::PendingActorQuotaExceeded` | `ResourceExhausted` / per-actor pending receipts | self | Wait for exact pending operations to settle before retry |
| `REPLAY_PENDING_COMMAND_CAPACITY` | 2 | both replay-decision mappers; `ReplayReceiptDecision::PendingCommandQuotaExceeded` | `ResourceExhausted` / per-command pending receipts | self | Wait for command-kind capacity before retry |

`ResponseCommitFailed` is not an error decision here: the workflow commits the
already-staged response and decodes the terminal receipt. Every other recovery
reason contradicts the auth-prepare state machine; the exact typed reason
remains in the retained replay receipt. Generic replay decisions otherwise share exact identities across
commands because their durable owner, retry contract and public exposure are
identical.

## Receipt State And Response Reconstruction

The remaining nine sites add seven exact meanings:

| Exact candidate | Sites | Producer function/branch | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- | --- |
| `REPLAY_RECEIPT_MISSING` | 1 | `map_auth_prepare_replay_store_error`; `ReplayReceiptStoreError::ReceiptMissing` | `Workflow` / retained replay state | self | Preserve the operation and inspect missing durable receipt state |
| `REPLAY_RECEIPT_DECODE_FAILED` | 1 | `map_auth_prepare_replay_store_error`; `ReplayReceiptStoreError::ReceiptDecodeFailed` | `Workflow` / retained replay encoding | self | Preserve malformed bytes and fail closed |
| `REPLAY_RECEIPT_TOKEN_MISMATCH` | 1 | `map_auth_prepare_replay_store_error`; `ReplayReceiptStoreError::ReceiptTokenMismatch` | `Workflow` / protected receipt identity | self | Reload the exact receipt token; never commit through stale identity |
| `REPLAY_STAGED_RESPONSE_MISSING` | 1 | `map_auth_prepare_replay_store_error`; `ReplayReceiptStoreError::StagedResponseMissing` | `Workflow` / response-commit recovery | self | Preserve receipt state and reconstruct only from exact staged bytes |
| `REPLAY_COST_GUARD_SETTLEMENT_MISSING` | 1 | `map_auth_prepare_replay_store_error`; `ReplayReceiptStoreError::CostGuardSettlementMissing` | `Workflow` / cost settlement identity | self | Preserve receipt state and fail before settlement or response commit |
| `REPLAY_RESPONSE_ENCODE_FAILED` | 2 | `encode_token_prepare_response` and `encode_role_attestation_prepare_response`; `ReplayCommitError::EncodeFailed` | `Workflow` / canonical response encoding | self | Repair the exact response contract; no unchanged retry |
| `REPLAY_RESPONSE_DECODE_FAILED` | 2 | `decode_token_prepare_response` and `decode_role_attestation_prepare_response`; `ReplayDecodeError::DecodeFailed` | `Workflow` / terminal response reconstruction | self | Preserve terminal bytes and repair the exact response decoder |

`ReplayReceiptStoreError`, `ReplayCommitError` and `ReplayDecodeError` are typed
owners. Their current string fields must become finite typed causes where
needed; the auth workflow must not allocate token-versus-attestation wrapper
codes. Response encoding and decoding share identities across both response
types because the replay receipt and recovery action are the same and the
command kind retains the qualified response contract.

## Dynamic Public Context

The twenty-three dynamic values in this owner are classified as `DPC-213`
through `DPC-235` in
[dynamic-public-context.md](dynamic-public-context.md). Sixteen are
caller-derivable request, command or quota values. Seven are typed recovery or
encoding/decoding causes and must remain typed rather than formatted. No replay
receipt bytes, actor identity or payload hash enter a compact diagnostic.

## Reconciliation

All thirty-two direct sites now have one disposition. They add twenty-one exact
meanings and no projection. The effective constructor frontier moves from
2,244 to 2,276 classified sites and from 255 to 223 open sites. The qualified
semantic set reaches 2,501 exact candidates plus 31 safe projections: 2,532
current symbolic identities.

## Required Tests

- checked expiry arithmetic and separate command-specific TTL boundaries;
- one shared operation-ID requirement across both prepare commands;
- exhaustive two-command mapping for every `ReplayReceiptDecision` variant;
- response-commit failure recovers, while every other recovery reason fails
  closed with its retained typed reason;
- actor and payload conflicts remain independent;
- actor and command quotas remain independent and bounded;
- exhaustive typed mappings for every receipt-store error;
- response encoding/decoding identities are shared without losing command
  kind or response schema; and
- malformed, missing, stale-token and missing-staged-response state never
  fabricates a terminal response.

## Next Slice

Continue with the prepare orchestrator and provisioning adapter, preserving
cost-guard settlement, lazy repair and issuer-install causes across awaits.
